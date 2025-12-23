use crate::{Exchange, utils::calculate_percent_change};
use anyhow::Context;
use api_schemes::{FundingRateHistoryResponse, FundingRateInfoResponse, OpenInterestStatisticsResponse};
use tracing::{debug, error, warn};
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

		debug!("Fetching open interest data for {symbol}: 5m={url_5m}, 1h={url_1h}, 1d={url_1d}");

		let (data_5m, data_1h, data_1d): (
			Vec<OpenInterestStatisticsResponse>,
			Vec<OpenInterestStatisticsResponse>,
			Vec<OpenInterestStatisticsResponse>,
		) = match tokio::try_join!(
			fetch_json(&self.client, &url_5m),
			fetch_json(&self.client, &url_1h),
			fetch_json(&self.client, &url_1d)
		) {
			Ok(data) => {
				debug!("Received data: 5m={} entries, 1h={} entries, 1d={} entries", data.0.len(), data.1.len(), data.2.len());
				data
			},
			Err(e) => {
				error!("Failed to fetch open interest data for {symbol}: {e:?}");
				return Err(e.context("Failed to fetch open interest data from Binance API"));
			},
		};

		if data_5m.is_empty() {
			warn!("No 5-minute open interest data returned for {symbol}");
			anyhow::bail!("No 5-minute open interest data available for symbol '{symbol}'");
		}

		let current_oi = match data_5m.last() {
			Some(entry) => {
				debug!("Last 5m entry: timestamp={}, sum_open_interest={}", entry.timestamp, entry.sum_open_interest);
				entry
					.sum_open_interest
					.parse::<f64>()
					.with_context(|| format!("Failed to parse sum_open_interest '{}' as f64", entry.sum_open_interest))?
			},
			None => {
				error!("data_5m array is empty but passed is_empty() check - this should not happen");
				anyhow::bail!("No current open interest data for symbol '{symbol}'");
			},
		};

		debug!("Current open interest: {current_oi}");

		let oi_15m = find_oi_at_time(&data_5m, fifteen_min_ago, current_oi);
		debug!("Open interest 15min ago: {oi_15m} (target_time={fifteen_min_ago})");

		let combined_short_term: Vec<_> = data_5m.iter().chain(data_1h.iter()).collect();
		let oi_1h = combined_short_term
			.iter()
			.find(|d| parse_timestamp(&d.timestamp).unwrap_or(0) <= one_hour_ago)
			.and_then(|d| match parse_rate(&d.sum_open_interest) {
				Ok(rate) => Some(rate),
				Err(e) => {
					warn!("Failed to parse sum_open_interest '{}' for 1h data: {}", d.sum_open_interest, e);
					None
				},
			})
			.unwrap_or_else(|| {
				warn!("No 1h open interest data found, using current_oi as fallback");
				current_oi
			});
		debug!("Open interest 1h ago: {oi_1h} (target_time={one_hour_ago})");

		let oi_4h = find_oi_at_time(&data_1h, four_hours_ago, current_oi);
		debug!("Open interest 4h ago: {oi_4h} (target_time={four_hours_ago})");

		let oi_30d = data_1d
			.first()
			.and_then(|d| match parse_rate(&d.sum_open_interest) {
				Ok(rate) => {
					debug!("30d open interest from first entry: timestamp={}, oi={rate}", d.timestamp);
					Some(rate)
				},
				Err(e) => {
					warn!("Failed to parse sum_open_interest '{}' for 30d data: {}", d.sum_open_interest, e);
					None
				},
			})
			.unwrap_or_else(|| {
				warn!("No 30d open interest data found, using current_oi as fallback");
				current_oi
			});
		debug!("Open interest 30d ago: {oi_30d}");

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

fn ms_ago(ms: i64) -> anyhow::Result<i64> {
	Ok(now_ms()? - ms)
}

async fn fetch_json<T: serde::de::DeserializeOwned>(client: &reqwest::Client, url: &str) -> anyhow::Result<T> {
	debug!("Fetching: {url}");
	let response = client.get(url).send().await.with_context(|| format!("Network error while fetching {url}"))?;

	let status = response.status();
	if !status.is_success() {
		let error_text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
		error!("HTTP error {} for {}: {}", status, url, error_text);
		anyhow::bail!("HTTP {} error from {}: {}", status, url, error_text);
	}

	response.json().await.with_context(|| format!("Failed to parse JSON response from {url}")).map_err(Into::into)
}

fn parse_timestamp(ts: &str) -> anyhow::Result<i64> {
	ts.parse::<i64>().map_err(|e| anyhow::anyhow!("Failed to parse timestamp '{}': {}", ts, e))
}

fn parse_rate(rate: &str) -> anyhow::Result<f64> {
	rate.parse::<f64>().map_err(|e| anyhow::anyhow!("Failed to parse rate '{}': {}", rate, e))
}

fn find_rate_at_time(history: &[FundingRateHistoryResponse], target_time: i64, fallback: f64) -> f64 {
	history
		.iter()
		.find(|h| h.funding_time <= target_time)
		.and_then(|h| parse_rate(&h.funding_rate).ok())
		.unwrap_or(fallback)
}

fn find_oi_at_time(data: &[OpenInterestStatisticsResponse], target_time: i64, fallback: f64) -> f64 {
	for d in data {
		match parse_timestamp(&d.timestamp) {
			Ok(ts) if ts <= target_time => match parse_rate(&d.sum_open_interest) {
				Ok(oi) => {
					debug!("Found OI at timestamp {}: {}", ts, oi);
					return oi;
				},
				Err(e) => warn!("Failed to parse sum_open_interest '{}': {}", d.sum_open_interest, e),
			},
			Ok(ts) => debug!("Skipping entry with timestamp {} (target: {})", ts, target_time),
			Err(e) => warn!("Failed to parse timestamp '{}': {}", d.timestamp, e),
		}
	}

	warn!("No matching OI data found for target_time {}, using fallback {}", target_time, fallback);
	fallback
}
