use futures_util::{SinkExt, StreamExt};
use tokio::time::{Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::error;

use crate::{
	CandleInfo, Exchange, MarketLiquidationsInfo,
	binance::api_schemes::{
		ForceOrderStream, FundingRateHistoryRequestParams, KlineCandlestickRequestParams,
		OpenInterestStatisticsRequestParams,
	},
};
use anyhow::{Context, bail};
use api_schemes::{FundingRateHistoryResponse, OpenInterestStatisticsResponse};
mod api_schemes;

const BINANCE_FUTURES_API_BASE: &str = "https://fapi.binance.com";

const WS_URL: &str = "wss://fstream.binance.com/ws/!forceOrder@arr";
const HOURS_24: Duration = Duration::from_secs(24 * 60 * 60);
const PING_EVERY: Duration = Duration::from_secs(60);
const PONG_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub struct BinanceExchange {
	client: reqwest::Client,
}

impl BinanceExchange {
	#[must_use]
	pub fn new() -> Self {
		Self { client: reqwest::Client::new() }
	}
}

impl Default for BinanceExchange {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait::async_trait]
impl Exchange for BinanceExchange {
	async fn watch_market_liquidations<F>(&self, mut callback: F) -> anyhow::Result<()>
	where
		F: FnMut(MarketLiquidationsInfo) + Send,
	{
		loop {
			if let Err(e) = run_stream(&mut callback).await {
				error!("Stream error: {}, reconnecting in 5s...", e);
				tokio::time::sleep(Duration::from_secs(5)).await;
			}
		}
	}

	async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> anyhow::Result<Vec<CandleInfo>> {
		let url = format!("{BINANCE_FUTURES_API_BASE}/fapi/v1/klines");
		let response: Vec<FundingRateHistoryResponse> = self
			.client
			.get(&url)
			.query(&KlineCandlestickRequestParams {
				symbol: String::from(symbol),
				limit: Some(100),
				interval: String::from(interval),
				..Default::default()
			})
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
			.context(format!("Failed to fetch funding rate info for {symbol}"))?;

		let response = reqwest::get(&url).await?;
		let data: Vec<serde_json::Value> = response.json().await?;

		let candles = data
			.iter()
			.map(|k| CandleInfo {
				timestamp: k[0].as_i64().unwrap(),
				open: k[1].as_str().unwrap().parse().unwrap(),
				high: k[2].as_str().unwrap().parse().unwrap(),
				low: k[3].as_str().unwrap().parse().unwrap(),
				close: k[4].as_str().unwrap().parse().unwrap(),
				volume: k[5].as_str().unwrap().parse().unwrap(),
			})
			.collect();

		Ok(candles)
	}

	async fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<crate::FundingRateInfo> {
		let url = format!("{BINANCE_FUTURES_API_BASE}/fapi/v1/fundingRate");
		let response: Vec<FundingRateHistoryResponse> = self
			.client
			.get(&url)
			.query(&FundingRateHistoryRequestParams { symbol: String::from(symbol), limit: Some(100), ..Default::default() })
			.send()
			.await?
			.error_for_status()?
			.json()
			.await
			.context(format!("Failed to fetch funding rate info for {symbol}"))?;

		let current_funding_rate = response.first().map(|item| item.funding_rate.clone()).unwrap_or_default();

		let rates: Vec<f64> = response.iter().filter_map(|item| item.funding_rate.parse::<f64>().ok()).collect();

		let average_funding_rate = if rates.is_empty() {
			String::from("0.0000")
		} else {
			let sum: f64 = rates.iter().sum();
			let average = sum / rates.len() as f64;
			average.to_string()
		};

		Ok(crate::FundingRateInfo { funding_rate: current_funding_rate, average_funding_rate })
	}

	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<crate::OpenInterestInfo> {
		let url = format!("{BINANCE_FUTURES_API_BASE}/futures/data/openInterestHist");

		let (response_5m, response_1d) = tokio::join!(
			async {
				let limit = Some(48); // to get 5m - 4h distance
				self
					.client
					.get(&url)
					.query(&OpenInterestStatisticsRequestParams {
						symbol: String::from(symbol),
						period: String::from("5m"),
						limit,
						..Default::default()
					})
					.send()
					.await?
					.error_for_status()?
					.json::<Vec<OpenInterestStatisticsResponse>>()
					.await
					.context(format!("Failed to fetch open interest info for {symbol} (5m)"))
			},
			async {
				let limit = Some(30); // 30 days
				self
					.client
					.get(&url)
					.query(&OpenInterestStatisticsRequestParams {
						symbol: String::from(symbol),
						period: String::from("1d"),
						limit,
						..Default::default()
					})
					.send()
					.await?
					.error_for_status()?
					.json::<Vec<OpenInterestStatisticsResponse>>()
					.await
					.context(format!("Failed to fetch open interest info for {symbol} (1d)"))
			}
		);

		let response_5m = response_5m?;
		let response_1d = response_1d?;

		// Calculate percent changes for 5m period data
		// Latest item is at the end of the array (highest index), oldest at index 0
		let percent_change_5_minutes = calculate_percent_change(&response_5m, 1)?;
		let percent_change_15_minutes = calculate_percent_change(&response_5m, 3)?;
		let percent_change_1_hour = calculate_percent_change(&response_5m, 12)?;
		let percent_change_4_hours = calculate_percent_change(&response_5m, 47)?;

		// Calculate percent changes for 1d period data
		let percent_change_1_day = calculate_percent_change(&response_1d, 1)?;
		let percent_change_7_days = calculate_percent_change(&response_1d, 7)?;
		let percent_change_30_days = calculate_percent_change(&response_1d, 29)?;

		Ok(crate::OpenInterestInfo {
			percent_change_5_minutes,
			percent_change_15_minutes,
			percent_change_1_hour,
			percent_change_4_hours,
			percent_change_1_day,
			percent_change_7_days,
			percent_change_30_days,
		})
	}
}

/// Calculate percent change between the most recent value (last index) and a value at a given offset back in time
fn calculate_percent_change(data: &[OpenInterestStatisticsResponse], offset: usize) -> anyhow::Result<f64> {
	if data.len() <= offset {
		bail!("Insufficient data: need at least {} items, got {}", offset + 1, data.len());
	}

	let last_idx = data.len() - 1;
	let previous_idx = last_idx - offset;

	let current = data[last_idx]
		.sum_open_interest
		.parse::<f64>()
		.context(format!("Failed to parse current open interest: {}", data[last_idx].sum_open_interest))?;

	let previous = data[previous_idx]
		.sum_open_interest
		.parse::<f64>()
		.context(format!("Failed to parse previous open interest: {}", data[previous_idx].sum_open_interest))?;

	if previous == 0.0 {
		return Ok(0.0);
	}

	let percent_change = ((current - previous) / previous) * 100.0;
	Ok(percent_change)
}

async fn run_stream<F>(callback: &mut F) -> anyhow::Result<()>
where
	F: FnMut(MarketLiquidationsInfo),
{
	let (ws, _) = connect_async(WS_URL).await.context("Failed to connect")?;
	let (mut tx, mut rx) = ws.split();

	let start = Instant::now();
	let mut last_ping = Instant::now();
	let mut last_pong = Instant::now();

	loop {
		if start.elapsed() >= HOURS_24 {
			let _ = tx.send(Message::Close(None)).await;
			return Ok(());
		}

		if last_ping.elapsed() >= PING_EVERY {
			tx.send(Message::Ping(vec![].into())).await?;
			last_ping = Instant::now();
		}

		if last_pong.elapsed() > PONG_TIMEOUT {
			return Err(anyhow::anyhow!("Server timeout"));
		}

		match tokio::time::timeout(Duration::from_secs(30), rx.next()).await {
			Ok(Some(Ok(Message::Text(text)))) => {
				if let Ok(info) = parse_liquidation(&text) {
					callback(info);
				}
			},
			Ok(Some(Ok(Message::Pong(_)))) => last_pong = Instant::now(),
			Ok(Some(Ok(Message::Ping(p)))) => {
				tx.send(Message::Pong(p)).await?;
				last_pong = Instant::now();
			},
			Ok(Some(Ok(Message::Close(_))) | None) => return Ok(()),
			Ok(Some(Err(e))) => return Err(e.into()),
			_ => {},
		}
	}
}

fn parse_liquidation(text: &str) -> anyhow::Result<MarketLiquidationsInfo> {
	let data: ForceOrderStream = serde_json::from_str(text)?;
	let symbol_price = data.order.price.parse::<f64>().context(format!("Failed to parse price: {}", data.order.price))?;
	let quantity = data
		.order
		.original_quantity
		.parse::<f64>()
		.context(format!("Failed to parse quantity: {}", data.order.original_quantity))?;
	let usd_price = symbol_price * quantity;

	Ok(MarketLiquidationsInfo {
		symbol: data.order.symbol,
		side: data.order.side,
		symbol_price,
		usd_price,
		quantity,
		time: data.event_time,
	})
}
