mod binance;
mod utils;

pub use binance::BinanceExchange;

#[async_trait::async_trait]
pub trait Exchange {
	async fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<FundingRateInfo>;
	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<OpenInterestInfo>;
}

#[derive(Debug)]
pub struct FundingRateInfo {
	funding_rate: String,
	average_funding_rate: String,
}

#[derive(Debug)]
pub struct OpenInterestInfo {
	open_interest_percent_change_5_minutes: f64,
	open_interest_percent_change_15_minutes: f64,
	open_interest_percent_change_1_hour: f64,
	open_interest_percent_change_4_hours: f64,
	open_interest_percent_change_1_day: f64,
	open_interest_percent_change_7_days: f64,
	open_interest_percent_change_30_days: f64,
}
