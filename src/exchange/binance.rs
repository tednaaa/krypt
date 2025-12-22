use super::{Candle, DerivativesMetrics, Exchange, ExchangeMessage, LongShortRatio, MessageStream, Symbol, Ticker};
use crate::config::BinanceConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// Binance has a limit of ~200 streams per connection, and URL length limits
// Split into chunks of 50 streams to be safe
const MAX_STREAMS_PER_CONNECTION: usize = 50;

pub struct BinanceExchange {
	config: BinanceConfig,
	client: Client,
}

impl BinanceExchange {
	pub fn new(config: BinanceConfig) -> Result<Self> {
		let client =
			Client::builder().timeout(std::time::Duration::from_secs(10)).build().context("Failed to create HTTP client")?;

		Ok(Self { config, client })
	}

	fn parse_kline_message(symbol_str: &str, data: &Value) -> Option<Candle> {
		let k_value = data.get("k")?;
		let k: BinanceKlineData = serde_json::from_value(k_value.clone()).ok()?;

		// Parse symbol (e.g., "BTCUSDT" -> "BTC", "USDT")
		let (base, quote) = parse_binance_symbol(symbol_str)?;

		Some(Candle {
			symbol: Symbol::new(base, quote, "binance"),
			timestamp: DateTime::from_timestamp_millis(k.t)?,
			open: k.o.parse().ok()?,
			high: k.h.parse().ok()?,
			low: k.l.parse().ok()?,
			close: k.c.parse().ok()?,
			volume: k.v.parse().ok()?,
			interval: k.i,
		})
	}
}

#[async_trait]
impl Exchange for BinanceExchange {
	fn name(&self) -> &'static str {
		"binance"
	}

	async fn symbols(&self) -> Result<Vec<Symbol>> {
		let url = format!("{}/fapi/v1/exchangeInfo", self.config.api_url);
		let response: ExchangeInfo = self.client.get(&url).send().await?.json().await?;

		Ok(
			response
				.symbols
				.into_iter()
				.filter(|s| s.status == "TRADING" && s.quote_asset == "USDT" && s.contract_type == "PERPETUAL")
				.map(|s| Symbol::new(s.base_asset, s.quote_asset, "binance"))
				.collect(),
		)
	}

	async fn stream_prices(&self, symbols: &[Symbol]) -> Result<MessageStream> {
		if symbols.is_empty() {
			return Ok(Box::pin(stream::empty()));
		}

		// Build stream names: btcusdt@ticker
		let mut streams = Vec::new();
		for symbol in symbols {
			let symbol_lower = symbol.exchange_symbol().to_lowercase();
			streams.push(format!("{symbol_lower}@ticker"));
		}

		let chunks: Vec<_> = streams.chunks(MAX_STREAMS_PER_CONNECTION).collect();

		tracing::info!(
			"Connecting to Binance price stream with {} streams for {} symbols across {} connection(s)",
			streams.len(),
			symbols.len(),
			chunks.len()
		);

		// Create multiple WebSocket connections if needed
		let mut connection_streams: Vec<MessageStream> = Vec::new();

		for (i, chunk) in chunks.iter().enumerate() {
			let stream_param = chunk.join("/");
			let ws_url = format!("{}/stream?streams={}", self.config.ws_url, stream_param);

			tracing::debug!("Price stream connection {} URL length: {} chars", i + 1, ws_url.len());

			let (ws_stream, response) = connect_async(&ws_url).await.map_err(|e| {
				tracing::error!("Failed to connect to Binance price WebSocket (connection {}): {}", i + 1, e);
				anyhow::anyhow!("Failed to connect to Binance price WebSocket: {e}")
			})?;

			tracing::info!("Binance price WebSocket connection {} established. Response status: {:?}", i + 1, response.status());

			let (_write, read) = ws_stream.split();

			let message_stream = read.filter_map(|msg| async move {
				match msg {
					Ok(Message::Text(text)) => {
						match serde_json::from_str::<Value>(&text) {
							Ok(json) => {
								if let Some(data) = json.get("data") {
									if let Some(stream_name) = json.get("stream").and_then(|s| s.as_str()) {
										// Extract symbol from stream name (e.g., "btcusdt@ticker")
										if let Some(symbol_part) = stream_name.split('@').next() {
											if let Some((base, quote)) = parse_binance_symbol(symbol_part) {
												if let Some(price) = data.get("c").and_then(|c| c.as_str()).and_then(|c| c.parse::<f64>().ok()) {
													let ticker = Ticker {
														symbol: Symbol::new(base, quote, "binance"),
														timestamp: Utc::now(),
														last_price: price,
														volume_24h: data.get("v").and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
														price_change_24h_pct: data.get("P").and_then(|p| p.as_str()).and_then(|p| p.parse::<f64>().ok()).unwrap_or(0.0),
													};
													return Some(ExchangeMessage::Ticker(ticker));
												}
											}
										}
									}
								}
								None
							},
							Err(e) => {
								tracing::warn!("Failed to parse Binance price message: {}", e);
								Some(ExchangeMessage::Error(format!("Parse error: {e}")))
							},
						}
					},
					Ok(Message::Close(_)) => {
						tracing::info!("Binance price WebSocket closed");
						Some(ExchangeMessage::Error("Connection closed".to_string()))
					},
					Err(e) => {
						tracing::error!("Binance price WebSocket error: {}", e);
						Some(ExchangeMessage::Error(format!("WebSocket error: {e}")))
					},
					_ => None,
				}
			});

			connection_streams.push(Box::pin(message_stream));
		}

		// Merge all connection streams into one
		if connection_streams.len() == 1 {
			connection_streams.into_iter().next().ok_or_else(|| anyhow::anyhow!("No streams created"))
		} else {
			let merged_stream = futures_util::stream::select_all(connection_streams);
			Ok(Box::pin(merged_stream))
		}
	}

	async fn stream_candles(&self, symbols: &[Symbol], intervals: &[&str]) -> Result<MessageStream> {
		if symbols.is_empty() {
			return Ok(Box::pin(stream::empty()));
		}

		// Build stream names: btcusdt@kline_1m
		let mut streams = Vec::new();
		for symbol in symbols {
			for interval in intervals {
				let symbol_lower = symbol.exchange_symbol().to_lowercase();
				streams.push(format!("{symbol_lower}@kline_{interval}"));
			}
		}

		let chunks: Vec<_> = streams.chunks(MAX_STREAMS_PER_CONNECTION).collect();

		tracing::info!(
			"Connecting to Binance WebSocket with {} streams for {} symbols across {} connection(s)",
			streams.len(),
			symbols.len(),
			chunks.len()
		);

		// Create multiple WebSocket connections if needed
		let mut connection_streams: Vec<MessageStream> = Vec::new();

		for (i, chunk) in chunks.iter().enumerate() {
			let stream_param = chunk.join("/");
			let ws_url = format!("{}/stream?streams={}", self.config.ws_url, stream_param);

			tracing::debug!("Connection {} URL length: {} chars", i + 1, ws_url.len());

			let (ws_stream, response) = connect_async(&ws_url).await.map_err(|e| {
				tracing::error!("Failed to connect to Binance WebSocket (connection {}): {}", i + 1, e);
				tracing::error!("URL: {}", ws_url);
				tracing::error!("Possible causes: network issues, firewall blocking, or invalid stream names");
				anyhow::anyhow!(
					"Failed to connect to Binance WebSocket: {e}. Check network connectivity and firewall settings.",
				)
			})?;

			tracing::info!("Binance WebSocket connection {} established. Response status: {:?}", i + 1, response.status());

			let (_write, read) = ws_stream.split();

			let message_stream = read.filter_map(|msg| async move {
				match msg {
					Ok(Message::Text(text)) => {
						match serde_json::from_str::<Value>(&text) {
							Ok(json) => {
								if let Some(data) = json.get("data") {
									if let Some(stream_name) = json.get("stream").and_then(|s| s.as_str()) {
										// Extract symbol from stream name (e.g., "btcusdt@kline_1m")
										if let Some(symbol_part) = stream_name.split('@').next() {
											if data.get("e").and_then(|e| e.as_str()) == Some("kline") {
												if let Some(candle) = Self::parse_kline_message(symbol_part, data) {
													return Some(ExchangeMessage::Candle(candle));
												}
											}
										}
									}
								}
								None
							},
							Err(e) => {
								tracing::warn!("Failed to parse Binance message: {}", e);
								Some(ExchangeMessage::Error(format!("Parse error: {e}")))
							},
						}
					},
					Ok(Message::Close(_)) => {
						tracing::info!("Binance WebSocket closed");
						Some(ExchangeMessage::Error("Connection closed".to_string()))
					},
					Err(e) => {
						tracing::error!("Binance WebSocket error: {}", e);
						Some(ExchangeMessage::Error(format!("WebSocket error: {e}")))
					},
					_ => None,
				}
			});

			connection_streams.push(Box::pin(message_stream));
		}

		// Merge all connection streams into one
		if connection_streams.len() == 1 {
			connection_streams.into_iter().next().ok_or_else(|| anyhow::anyhow!("No streams created"))
		} else {
			let merged_stream = futures_util::stream::select_all(connection_streams);
			Ok(Box::pin(merged_stream))
		}
	}

	async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics> {
		let symbol_str = symbol.exchange_symbol();

		// Fetch Open Interest
		let oi_url = format!("{}/fapi/v1/openInterest?symbol={}", self.config.api_url, symbol_str);
		let oi_response: OpenInterestResponse = self.client.get(&oi_url).send().await?.json().await?;

		// Fetch Funding Rate
		let funding_url = format!("{}/fapi/v1/premiumIndex?symbol={}", self.config.api_url, symbol_str);
		let funding_response: PremiumIndexResponse = self.client.get(&funding_url).send().await?.json().await?;

		// Fetch Long/Short Ratio (Global)
		let ratio_url =
			format!("{}/futures/data/globalLongShortAccountRatio?symbol={}&period=5m", self.config.api_url, symbol_str);
		let ratio_response: Vec<LongShortRatioResponse> =
			self.client.get(&ratio_url).send().await?.json().await.unwrap_or_default();

		let long_short_ratio = ratio_response.first().map(|r| LongShortRatio {
			long_account_pct: r.long_account * 100.0,
			short_account_pct: r.short_account * 100.0,
			long_position_pct: r.long_account * 100.0, // Binance doesn't separate position/account clearly
			short_position_pct: r.short_account * 100.0,
		});

		Ok(DerivativesMetrics {
			symbol: symbol.clone(),
			timestamp: Utc::now(),
			open_interest: oi_response.open_interest.parse().unwrap_or(0.0),
			open_interest_value: oi_response.open_interest.parse::<f64>().unwrap_or(0.0)
				* funding_response.mark_price.parse::<f64>().unwrap_or(0.0),
			funding_rate: funding_response.last_funding_rate.parse().unwrap_or(0.0),
			long_short_ratio,
		})
	}

	async fn fetch_historical_candles(&self, symbol: &Symbol, interval: &str, limit: u32) -> Result<Vec<Candle>> {
		let symbol_str = symbol.exchange_symbol();
		let url =
			format!("{}/fapi/v1/klines?symbol={}&interval={}&limit={}", self.config.api_url, symbol_str, interval, limit);

		let response: Vec<KlineResponse> = self.client.get(&url).send().await?.json().await?;

		let candles = response
			.into_iter()
			.filter_map(|k| {
				Some(Candle {
					symbol: symbol.clone(),
					timestamp: DateTime::from_timestamp_millis(k.0)?,
					open: k.1.parse().ok()?,
					high: k.2.parse().ok()?,
					low: k.3.parse().ok()?,
					close: k.4.parse().ok()?,
					volume: k.5.parse().ok()?,
					interval: interval.to_string(),
				})
			})
			.collect();

		Ok(candles)
	}

	fn format_interval(&self, minutes: u32) -> String {
		// Binance uses format: 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M
		match minutes {
			1 => "1m".to_string(),
			3 => "3m".to_string(),
			5 => "5m".to_string(),
			15 => "15m".to_string(),
			30 => "30m".to_string(),
			60 => "1h".to_string(),
			120 => "2h".to_string(),
			240 => "4h".to_string(),
			360 => "6h".to_string(),
			480 => "8h".to_string(),
			720 => "12h".to_string(),
			1440 => "1d".to_string(),
			4320 => "3d".to_string(),
			10080 => "1w".to_string(),
			43200 => "1M".to_string(),
			_ => format!("{minutes}m"),
		}
	}
}

// Helper function to parse Binance symbols
fn parse_binance_symbol(symbol: &str) -> Option<(String, String)> {
	// Most futures symbols end with USDT
	let symbol_upper = symbol.to_uppercase();
	if symbol_upper.ends_with("USDT") {
		let base = symbol_upper.trim_end_matches("USDT").to_string();
		return Some((base, "USDT".to_string()));
	}
	None
}

// Binance API Response Types
#[derive(Debug, Deserialize)]
struct ExchangeInfo {
	symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolInfo {
	#[allow(dead_code)]
	symbol: String,
	status: String,
	base_asset: String,
	quote_asset: String,
	contract_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInterestResponse {
	open_interest: String,
	#[allow(dead_code)]
	symbol: String,
	#[allow(dead_code)]
	time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PremiumIndexResponse {
	#[allow(dead_code)]
	symbol: String,
	mark_price: String,
	#[allow(dead_code)]
	index_price: String,
	last_funding_rate: String,
	#[allow(dead_code)]
	next_funding_time: i64,
	#[allow(dead_code)]
	time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LongShortRatioResponse {
	#[allow(dead_code)]
	symbol: String,
	long_account: f64,
	short_account: f64,
	#[allow(dead_code)]
	timestamp: i64,
}

// Kline response: [timestamp, open, high, low, close, volume, close_time, ...]
type KlineResponse = (i64, String, String, String, String, String, i64, String, i64, String, String, String);

// Binance WebSocket Kline Data
#[derive(Debug, Deserialize)]
struct BinanceKlineData {
	/// Kline start time
	#[serde(rename = "t")]
	pub t: i64,
	/// Open price
	#[serde(rename = "o")]
	pub o: String,
	/// High price
	#[serde(rename = "h")]
	pub h: String,
	/// Low price
	#[serde(rename = "l")]
	pub l: String,
	/// Close price
	#[serde(rename = "c")]
	pub c: String,
	/// Volume
	#[serde(rename = "v")]
	pub v: String,
	/// Interval
	#[serde(rename = "i")]
	pub i: String,
}
