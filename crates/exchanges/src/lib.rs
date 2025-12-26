mod binance;

pub use binance::BinanceExchange;

#[async_trait::async_trait]
pub trait Exchange {
	async fn watch_market_liquidations(&self) -> anyhow::Result<MarketLiquidationsInfo>;
	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<OpenInterestInfo>;
	async fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<FundingRateInfo>;
}

#[derive(Debug)]
pub struct MarketLiquidationsInfo {
	pub symbol: String,
	pub side: String,
	pub price: String,
	pub quantity: String,
	pub time: String,
}

#[derive(Debug)]
pub struct OpenInterestInfo {
	pub percent_change_5_minutes: f64,
	pub percent_change_15_minutes: f64,
	pub percent_change_1_hour: f64,
	pub percent_change_4_hours: f64,
	pub percent_change_1_day: f64,
	pub percent_change_7_days: f64,
	pub percent_change_30_days: f64,
}

#[derive(Debug)]
pub struct FundingRateInfo {
	pub funding_rate: String,
	pub average_funding_rate: String,
}
