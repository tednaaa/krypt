use super::{Candle, DerivativesMetrics, Exchange, ExchangeMessage, LongShortRatio, MessageStream, Symbol, Ticker};
use crate::config::BybitConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::{stream, SinkExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct BybitExchange {
	config: BybitConfig,
	client: Client,
}

impl BybitExchange {
	pub fn new(config: BybitConfig) -> Result<Self> {
		let client =
			Client::builder().timeout(std::time::Duration::from_secs(10)).build().context("Failed to create HTTP client")?;

		Ok(Self { config, client })
	}
}

#[async_trait]
impl Exchange for BybitExchange {
	async fn symbols(&self) -> Result<Vec<Symbol>> {
		let url = format!("{}/v5/market/instruments-info?category=linear", self.config.api_url);
		let response: InstrumentsResponse = self.client.get(&url).send().await?.json().await?;

		if response.ret_code != 0 {
			anyhow::bail!("Bybit API error: {}", response.ret_msg);
		}

		Ok(
			response
				.result
				.list
				.into_iter()
				.filter(|s| s.status == "Trading" && s.quote_coin == "USDT" && s.contract_type == "LinearPerpetual")
				.map(|s| Symbol::new(s.base_coin, s.quote_coin, "bybit"))
				.collect(),
		)
	}

	async fn stream_prices(&self, symbols: &[Symbol]) -> Result<MessageStream> {
		if symbols.is_empty() {
			return Ok(Box::pin(stream::empty()));
		}

		let (ws_stream, _) = connect_async(&self.config.ws_url).await.context("Failed to connect to Bybit WebSocket")?;

		let (mut write, read) = ws_stream.split();

		let mut topics = Vec::new();
		for symbol in symbols {
			let symbol_str = symbol.exchange_symbol();
			topics.push(format!("tickers.{symbol_str}"));
		}

		let subscribe_msg = serde_json::json!({
			"op": "subscribe",
			"args": topics
		});

		write
			.send(Message::Text(subscribe_msg.to_string().into()))
			.await
			.context("Failed to send Bybit subscription message")?;

		let message_stream = read.filter_map(|msg| async move {
			match msg {
				Ok(Message::Text(text)) => match serde_json::from_str::<Value>(&text) {
					Ok(json) => {
						if json.get("op").and_then(|o| o.as_str()) == Some("subscribe") {
							tracing::info!("Bybit price subscription confirmed");
							return None;
						}

						if let Some(topic) = json.get("topic").and_then(|t| t.as_str()) {
							if topic.starts_with("tickers.") {
								if let Some(data) = json.get("data") {
									if let Some(data_array) = data.as_array() {
										if let Some(ticker_data) = data_array.first() {
											if let Some(symbol_str) = topic.strip_prefix("tickers.") {
												if let Some((base, quote)) = parse_bybit_symbol(symbol_str) {
													if let Some(last_price_str) = ticker_data.get("lastPrice").and_then(|p| p.as_str()) {
														if let Ok(price) = last_price_str.parse::<f64>() {
															let ticker = Ticker {
																symbol: Symbol::new(base, quote, "bybit"),
																timestamp: Utc::now(),
																last_price: price,
																volume_24h: ticker_data
																	.get("volume24h")
																	.and_then(|v| v.as_str())
																	.and_then(|v| v.parse::<f64>().ok())
																	.unwrap_or(0.0),
																price_change_24h_pct: ticker_data
																	.get("price24hPcnt")
																	.and_then(|p| p.as_str())
																	.and_then(|p| p.parse::<f64>().ok())
																	.map_or(0.0, |p| p * 100.0),
															};
															return Some(ExchangeMessage::Ticker(ticker));
														}
													}
												}
											}
										}
									}
								}
							}
						}
						None
					},
					Err(e) => {
						tracing::warn!("Failed to parse Bybit price message: {}", e);
						Some(ExchangeMessage::Error(format!("Parse error: {e}")))
					},
				},
				Ok(Message::Close(_)) => {
					tracing::info!("Bybit price WebSocket closed");
					Some(ExchangeMessage::Error("Connection closed".to_string()))
				},
				Err(e) => {
					tracing::error!("Bybit price WebSocket error: {}", e);
					Some(ExchangeMessage::Error(format!("WebSocket error: {e}")))
				},
				_ => None,
			}
		});

		Ok(Box::pin(message_stream))
	}

	async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics> {
		let symbol_str = symbol.exchange_symbol();

		let oi_url = format!(
			"{}/v5/market/open-interest?category=linear&symbol={}&intervalTime=5min",
			self.config.api_url, symbol_str
		);
		let oi_response: OpenInterestResponse = self.client.get(&oi_url).send().await?.json().await?;

		if oi_response.ret_code != 0 {
			anyhow::bail!("Bybit OI API error: {}", oi_response.ret_msg);
		}

		let funding_url = format!("{}/v5/market/tickers?category=linear&symbol={}", self.config.api_url, symbol_str);
		let funding_response: TickerResponse = self.client.get(&funding_url).send().await?.json().await?;

		if funding_response.ret_code != 0 {
			anyhow::bail!("Bybit funding API error: {}", funding_response.ret_msg);
		}

		let ratio_url =
			format!("{}/v5/market/account-ratio?category=linear&symbol={}&period=5min", self.config.api_url, symbol_str);
		let ratio_response: LongShortRatioResponse =
			self.client.get(&ratio_url).send().await?.json().await.unwrap_or_else(|_| LongShortRatioResponse {
				ret_code: 0,
				ret_msg: String::new(),
				result: LongShortRatioResult { list: vec![] },
			});

		let oi_data = oi_response.result.list.first();
		let ticker_data = funding_response.result.list.first();
		let ratio_data = ratio_response.result.list.first();

		let open_interest = oi_data.and_then(|d| d.open_interest.parse::<f64>().ok()).unwrap_or(0.0);
		let mark_price = ticker_data.and_then(|d| d.mark_price.parse::<f64>().ok()).unwrap_or(0.0);
		let funding_rate = ticker_data.and_then(|d| d.funding_rate.parse::<f64>().ok()).unwrap_or(0.0);

		let long_short_ratio = ratio_data.map(|r| {
			let buy_ratio = r.buy_ratio.parse::<f64>().unwrap_or(0.5);
			let sell_ratio = r.sell_ratio.parse::<f64>().unwrap_or(0.5);

			LongShortRatio {
				account_long: buy_ratio * 100.0,
				account_short: sell_ratio * 100.0,
				position_long: buy_ratio * 100.0,
				position_short: sell_ratio * 100.0,
			}
		});

		Ok(DerivativesMetrics {
			symbol: symbol.clone(),
			timestamp: Utc::now(),
			open_interest,
			open_interest_value: open_interest * mark_price,
			funding_rate,
			long_short_ratio,
		})
	}

	async fn fetch_historical_candles(&self, symbol: &Symbol, interval: &str, limit: u32) -> Result<Vec<Candle>> {
		let symbol_str = symbol.exchange_symbol();
		let url = format!(
			"{}/v5/market/kline?category=linear&symbol={}&interval={}&limit={}",
			self.config.api_url, symbol_str, interval, limit
		);

		let response: KlineResponse = self.client.get(&url).send().await?.json().await?;

		if response.ret_code != 0 {
			anyhow::bail!("Bybit kline API error: {}", response.ret_msg);
		}

		let candles = response
			.result
			.list
			.into_iter()
			.filter_map(|k| {
				Some(Candle {
					symbol: symbol.clone(),
					timestamp: DateTime::from_timestamp_millis(k.0.parse().ok()?)?,
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

fn parse_bybit_symbol(symbol: &str) -> Option<(String, String)> {
	let symbol_upper = symbol.to_uppercase();
	if symbol_upper.ends_with("USDT") {
		let base = symbol_upper.trim_end_matches("USDT").to_string();
		return Some((base, "USDT".to_string()));
	}
	None
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstrumentsResponse {
	ret_code: i32,
	ret_msg: String,
	result: InstrumentsResult,
}

#[derive(Debug, Deserialize)]
struct InstrumentsResult {
	list: Vec<InstrumentInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstrumentInfo {
	#[allow(dead_code)]
	symbol: String,
	contract_type: String,
	status: String,
	base_coin: String,
	quote_coin: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInterestResponse {
	ret_code: i32,
	ret_msg: String,
	result: OpenInterestResult,
}

#[derive(Debug, Deserialize)]
struct OpenInterestResult {
	list: Vec<OpenInterestData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInterestData {
	open_interest: String,
	#[allow(dead_code)]
	timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TickerResponse {
	ret_code: i32,
	ret_msg: String,
	result: TickerResult,
}

#[derive(Debug, Deserialize)]
struct TickerResult {
	list: Vec<TickerData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TickerData {
	#[allow(dead_code)]
	symbol: String,
	mark_price: String,
	funding_rate: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LongShortRatioResponse {
	#[allow(dead_code)]
	ret_code: i32,
	#[allow(dead_code)]
	ret_msg: String,
	result: LongShortRatioResult,
}

#[derive(Debug, Deserialize)]
struct LongShortRatioResult {
	list: Vec<LongShortRatioData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LongShortRatioData {
	buy_ratio: String,
	sell_ratio: String,
	#[allow(dead_code)]
	timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KlineResponse {
	ret_code: i32,
	ret_msg: String,
	result: KlineResult,
}

#[derive(Debug, Deserialize)]
struct KlineResult {
	list: Vec<KlineData>,
}

type KlineData = (String, String, String, String, String, String, String);
