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
	funding_interval_hours: String,
	funding_rate_percent_change_1_hour: String,
	funding_rate_percent_change_4_hours: String,
}

#[derive(Debug)]
pub struct OpenInterestInfo {
	open_interest_percent_change_15_minutes: String,
	open_interest_percent_change_1_hour: String,
	open_interest_percent_change_4_hours: String,
	open_interest_percent_change_30_days: String,
}
