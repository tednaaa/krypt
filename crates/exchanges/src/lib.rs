pub use binance::BinanceExchange;

mod binance;

#[async_trait::async_trait]
pub trait Exchange {
	async fn get_all_usdt_pairs(&self) -> anyhow::Result<Vec<String>>;
	async fn watch_market_liquidations<F>(&self, callback: F) -> anyhow::Result<()>
	where
		F: FnMut(MarketLiquidationsInfo) + Send;
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
pub struct CandleInfo {
	pub open: f64,
	pub high: f64,
	pub low: f64,
	pub close: f64,
	pub volume: f64,
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
