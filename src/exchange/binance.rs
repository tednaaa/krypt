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
}

#[async_trait]
impl Exchange for BinanceExchange {
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

		let mut connection_streams: Vec<MessageStream> = Vec::new();

		for (i, chunk) in chunks.iter().enumerate() {
			let stream_param = chunk.join("/");
			let ws_url = format!("{}/stream?streams={}", self.config.ws_url, stream_param);

			tracing::debug!("Price stream connection {} URL length: {} chars", i + 1, ws_url.len());

			let (ws_stream, response) = connect_async(&ws_url).await.map_err(|e| {
				tracing::error!("Failed to connect to Binance price WebSocket (connection {}): {}", i + 1, e);
				anyhow::anyhow!("Failed to connect to Binance price WebSocket: {e}")
			})?;

			tracing::info!(
				"Binance price WebSocket connection {} established. Response status: {:?}",
				i + 1,
				response.status()
			);

			let (_write, read) = ws_stream.split();

			let message_stream = read.filter_map(|msg| async move {
				match msg {
					Ok(Message::Text(text)) => match serde_json::from_str::<Value>(&text) {
						Ok(json) => {
							if let Some(data) = json.get("data") {
								if let Some(stream_name) = json.get("stream").and_then(|s| s.as_str()) {
									if let Some(symbol_part) = stream_name.split('@').next() {
										if let Some((base, quote)) = parse_binance_symbol(symbol_part) {
											if let Some(price) = data.get("c").and_then(|c| c.as_str()).and_then(|c| c.parse::<f64>().ok()) {
												let ticker = Ticker {
													symbol: Symbol::new(base, quote, "binance"),
													timestamp: Utc::now(),
													last_price: price,
													volume_24h: data
														.get("v")
														.and_then(|v| v.as_str())
														.and_then(|v| v.parse::<f64>().ok())
														.unwrap_or(0.0),
													price_change_24h_pct: data
														.get("P")
														.and_then(|p| p.as_str())
														.and_then(|p| p.parse::<f64>().ok())
														.unwrap_or(0.0),
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

		if connection_streams.len() == 1 {
			connection_streams.into_iter().next().ok_or_else(|| anyhow::anyhow!("No streams created"))
		} else {
			let merged_stream = futures_util::stream::select_all(connection_streams);
			Ok(Box::pin(merged_stream))
		}
	}

	async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics> {
		let symbol_str = symbol.exchange_symbol();

		let oi_url = format!("{}/fapi/v1/openInterest?symbol={}", self.config.api_url, symbol_str);
		let oi_response: OpenInterestResponse = self.client.get(&oi_url).send().await?.json().await?;

		let funding_url = format!("{}/fapi/v1/premiumIndex?symbol={}", self.config.api_url, symbol_str);
		let funding_response: PremiumIndexResponse = self.client.get(&funding_url).send().await?.json().await?;

		let ratio_url =
			format!("{}/futures/data/globalLongShortAccountRatio?symbol={}&period=5m", self.config.api_url, symbol_str);
		let ratio_response: Vec<LongShortRatioResponse> =
			self.client.get(&ratio_url).send().await?.json().await.unwrap_or_default();

		let long_short_ratio = ratio_response.first().map(|r| LongShortRatio {
			account_long: r.long_account * 100.0,
			account_short: r.short_account * 100.0,
			position_long: r.long_account * 100.0,
			position_short: r.short_account * 100.0,
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

fn parse_binance_symbol(symbol: &str) -> Option<(String, String)> {
	let symbol_upper = symbol.to_uppercase();
	if symbol_upper.ends_with("USDT") {
		let base = symbol_upper.trim_end_matches("USDT").to_string();
		return Some((base, "USDT".to_string()));
	}
	None
}

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

type KlineResponse = (i64, String, String, String, String, String, i64, String, i64, String, String, String);
