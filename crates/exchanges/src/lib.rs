mod binance;

pub use binance::BinanceExchange;

#[async_trait::async_trait]
pub trait Exchange {
	async fn watch_market_liquidations<F>(&self, callback: F) -> anyhow::Result<()> where F: FnMut(MarketLiquidationsInfo) + Send;
	async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> anyhow::Result<Vec<CandleInfo>>;
	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<OpenInterestInfo>;
	async fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<FundingRateInfo>;
}

#[derive(Debug)]
pub struct MarketLiquidationsInfo {
	pub symbol: String,
	pub side: String,
	pub symbol_price: f64,
	pub usd_price: f64,
	pub quantity: f64,
	pub time: u64,
}

#[derive(Debug)]
struct CandleInfo {
	timestamp: i64,
	open: f64,
	high: f64,
	low: f64,
	close: f64,
	volume: f64,
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
