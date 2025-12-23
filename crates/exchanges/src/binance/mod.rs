use crate::{Exchange, utils::calculate_percent_change};
use api_schemes::{FundingRateHistoryResponse, FundingRateInfoResponse, OpenInterestStatisticsResponse};
mod api_schemes;

const BINANCE_FUTURES_API_BASE: &str = "https://fapi.binance.com";
const ONE_HOUR_MS: i64 = 60 * 60 * 1000;
const FOUR_HOURS_MS: i64 = 4 * ONE_HOUR_MS;
const FIFTEEN_MIN_MS: i64 = 15 * 60 * 1000;
const THIRTY_DAYS_MS: i64 = 30 * 24 * ONE_HOUR_MS;

pub struct BinanceExchange {
	client: reqwest::Client,
}

impl BinanceExchange {
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
		let now = now_ms()?;
		let four_hours_ago = ms_ago(FOUR_HOURS_MS)?;
		let one_hour_ago = ms_ago(ONE_HOUR_MS)?;

		let info_url = format!("{BINANCE_FUTURES_API_BASE}/fapi/v1/fundingInfo?symbol={symbol}");
		let history_url =
			format!("{BINANCE_FUTURES_API_BASE}/fapi/v1/fundingRate?symbol={symbol}&startTime={four_hours_ago}&limit=1000");

		let (info_response, mut history): (Vec<FundingRateInfoResponse>, Vec<FundingRateHistoryResponse>) =
			tokio::try_join!(fetch_json(&self.client, &info_url), fetch_json(&self.client, &history_url))?;

		let info = info_response
			.into_iter()
			.find(|i| i.symbol == symbol)
			.ok_or_else(|| anyhow::anyhow!("Symbol '{symbol}' not found"))?;

		if history.is_empty() {
			anyhow::bail!("No funding rate history available for symbol '{symbol}'");
		}

		history.sort_by(|a, b| b.funding_time.cmp(&a.funding_time));

		let current_rate = parse_rate(&history[0].funding_rate)?;

		let rate_1h = find_rate_at_time(&history, one_hour_ago, current_rate);
		let rate_4h = find_rate_at_time(&history, four_hours_ago, current_rate);

		Ok(crate::FundingRateInfo {
			funding_rate: format!("{current_rate:.6}"),
			funding_interval_hours: info.funding_interval_hours.to_string(),
			funding_rate_percent_change_1_hour: calculate_percent_change(rate_1h, current_rate),
			funding_rate_percent_change_4_hours: calculate_percent_change(rate_4h, current_rate),
		})
	}

	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<crate::OpenInterestInfo> {
		let now = now_ms()?;

		// Calculate timestamps
		let thirty_days_ago = ms_ago(THIRTY_DAYS_MS)?;
		let five_hours_ago = ms_ago(5 * ONE_HOUR_MS)?;
		let four_hours_ago = ms_ago(FOUR_HOURS_MS)?;
		let one_hour_ago = ms_ago(ONE_HOUR_MS)?;
		let fifteen_min_ago = ms_ago(FIFTEEN_MIN_MS)?;

		let url_5m = format!(
			"{BINANCE_FUTURES_API_BASE}/futures/data/openInterestHist?symbol={symbol}&period=5m&limit=500&startTime={five_hours_ago}"
		);
		let url_1h = format!(
			"{BINANCE_FUTURES_API_BASE}/futures/data/openInterestHist?symbol={symbol}&period=1h&limit=500&startTime={five_hours_ago}"
		);
		let url_1d = format!(
			"{BINANCE_FUTURES_API_BASE}/futures/data/openInterestHist?symbol={symbol}&period=1d&limit=35&startTime={thirty_days_ago}"
		);

		let (data_5m, data_1h, data_1d): (
			Vec<OpenInterestStatisticsResponse>,
			Vec<OpenInterestStatisticsResponse>,
			Vec<OpenInterestStatisticsResponse>,
		) = tokio::try_join!(
			fetch_json(&self.client, &url_5m),
			fetch_json(&self.client, &url_1h),
			fetch_json(&self.client, &url_1d)
		)?;

		let current_oi = data_5m
			.last()
			.ok_or_else(|| anyhow::anyhow!("No current open interest data for symbol '{symbol}'"))?
			.sum_open_interest
			.parse::<f64>()?;

		let oi_15m = find_oi_at_time(&data_5m, fifteen_min_ago, current_oi);

		let combined_short_term: Vec<_> = data_5m.iter().chain(data_1h.iter()).collect();
		let oi_1h = combined_short_term
			.iter()
			.find(|d| parse_timestamp(&d.timestamp).unwrap_or(0) <= one_hour_ago)
			.and_then(|d| parse_rate(&d.sum_open_interest).ok())
			.unwrap_or(current_oi);

		let oi_4h = find_oi_at_time(&data_1h, four_hours_ago, current_oi);
		let oi_30d = data_1d.first().and_then(|d| parse_rate(&d.sum_open_interest).ok()).unwrap_or(current_oi);

		Ok(crate::OpenInterestInfo {
			open_interest_percent_change_15_minutes: calculate_percent_change(oi_15m, current_oi),
			open_interest_percent_change_1_hour: calculate_percent_change(oi_1h, current_oi),
			open_interest_percent_change_4_hours: calculate_percent_change(oi_4h, current_oi),
			open_interest_percent_change_30_days: calculate_percent_change(oi_30d, current_oi),
		})
	}
}

fn now_ms() -> anyhow::Result<i64> {
	Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as i64)
}

/// Calculate timestamp for N milliseconds ago
fn ms_ago(ms: i64) -> anyhow::Result<i64> {
	Ok(now_ms()? - ms)
}

/// Fetch JSON from a URL with proper error handling
async fn fetch_json<T: serde::de::DeserializeOwned>(client: &reqwest::Client, url: &str) -> anyhow::Result<T> {
	client.get(url).send().await?.error_for_status()?.json().await.map_err(Into::into)
}

/// Parse timestamp string to i64, with error context
fn parse_timestamp(ts: &str) -> anyhow::Result<i64> {
	ts.parse::<i64>().map_err(|e| anyhow::anyhow!("Failed to parse timestamp '{}': {}", ts, e))
}

/// Parse funding rate string to f64, with error context
fn parse_rate(rate: &str) -> anyhow::Result<f64> {
	rate.parse::<f64>().map_err(|e| anyhow::anyhow!("Failed to parse rate '{}': {}", rate, e))
}

/// Find the most recent funding rate at or before a given timestamp
fn find_rate_at_time(history: &[FundingRateHistoryResponse], target_time: i64, fallback: f64) -> f64 {
	history
		.iter()
		.find(|h| h.funding_time <= target_time)
		.and_then(|h| parse_rate(&h.funding_rate).ok())
		.unwrap_or(fallback)
}

/// Find open interest at or before a given timestamp
fn find_oi_at_time(data: &[OpenInterestStatisticsResponse], target_time: i64, fallback: f64) -> f64 {
	data
		.iter()
		.find(|d| parse_timestamp(&d.timestamp).unwrap_or(0) <= target_time)
		.and_then(|d| parse_rate(&d.sum_open_interest).ok())
		.unwrap_or(fallback)
}
