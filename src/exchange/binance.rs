use super::{Candle, DerivativesMetrics, Exchange, ExchangeMessage, LongShortRatio, MessageStream, Symbol};
use crate::config::BinanceConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
		let k = data.get("k")?;

		let timestamp = k.get("t")?.as_i64()?;
		let open = k.get("o")?.as_str()?.parse::<f64>().ok()?;
		let high = k.get("h")?.as_str()?.parse::<f64>().ok()?;
		let low = k.get("l")?.as_str()?.parse::<f64>().ok()?;
		let close = k.get("c")?.as_str()?.parse::<f64>().ok()?;
		let volume = k.get("v")?.as_str()?.parse::<f64>().ok()?;
		let interval = k.get("i")?.as_str()?.to_string();

		// Parse symbol (e.g., "BTCUSDT" -> "BTC", "USDT")
		let (base, quote) = parse_binance_symbol(symbol_str)?;

		Some(Candle {
			symbol: Symbol::new(base, quote, "binance"),
			timestamp: DateTime::from_timestamp_millis(timestamp)?,
			open,
			high,
			low,
			close,
			volume,
			interval,
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

	async fn stream_candles(&self, symbols: &[Symbol], intervals: &[&str]) -> Result<MessageStream> {
		if symbols.is_empty() {
			return Ok(Box::pin(stream::empty()));
		}

		// Build stream names: btcusdt@kline_1m
		let mut streams = Vec::new();
		for symbol in symbols {
			for interval in intervals {
				let symbol_lower = symbol.exchange_symbol().to_lowercase();
				streams.push(format!("{}@kline_{}", symbol_lower, interval));
			}
		}

		let stream_param = streams.join("/");
		let ws_url = format!("{}/stream?streams={}", self.config.ws_url, stream_param);

		let (ws_stream, _) = connect_async(&ws_url).await.context("Failed to connect to Binance WebSocket")?;

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
											if let Some(candle) = BinanceExchange::parse_kline_message(symbol_part, data) {
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
							Some(ExchangeMessage::Error(format!("Parse error: {}", e)))
						},
					}
				},
				Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => None,
				Ok(Message::Close(_)) => {
					tracing::info!("Binance WebSocket closed");
					Some(ExchangeMessage::Error("Connection closed".to_string()))
				},
				Err(e) => {
					tracing::error!("Binance WebSocket error: {}", e);
					Some(ExchangeMessage::Error(format!("WebSocket error: {}", e)))
				},
				_ => None,
			}
		});

		Ok(Box::pin(message_stream))
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
	symbol: String,
	time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PremiumIndexResponse {
	symbol: String,
	mark_price: String,
	index_price: String,
	last_funding_rate: String,
	next_funding_time: i64,
	time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LongShortRatioResponse {
	symbol: String,
	long_account: f64,
	short_account: f64,
	timestamp: i64,
}

// Kline response: [timestamp, open, high, low, close, volume, close_time, ...]
type KlineResponse = (i64, String, String, String, String, String, i64, String, i64, String, String, String);
