use crate::{
	Exchange,
	binance::api_schemes::{FundingRateHistoryRequestParams, OpenInterestStatisticsRequestParams},
};
use anyhow::{Context, bail};
use api_schemes::{FundingRateHistoryResponse, OpenInterestStatisticsResponse};
mod api_schemes;

const BINANCE_FUTURES_API_BASE: &str = "https://fapi.binance.com";

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
