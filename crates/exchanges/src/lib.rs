mod binance;

pub use binance::BinanceExchange;

#[async_trait::async_trait]
pub trait Exchange {
	async fn watch_market_tickers<F>(&self, callback: F) -> anyhow::Result<()>
	where
		F: FnMut(Vec<TickerInfo>) + Send;
	async fn watch_market_liquidations<F>(&self, callback: F) -> anyhow::Result<()>
	where
		F: FnMut(MarketLiquidationsInfo) + Send;
	async fn get_open_interest_info(&self, symbol: &str) -> anyhow::Result<OpenInterestInfo>;
	async fn get_funding_rate_info(&self, symbol: &str) -> anyhow::Result<FundingRateInfo>;
}

#[derive(Debug)]
pub struct TickerInfo {
	pub symbol: String,
	pub price_change: String,
	pub price_change_percent: String,
	pub weighted_average_price: String,
	pub last_price: String,
	pub last_quantity: String,
	pub open_price: String,
	pub high_price: String,
	pub low_price: String,
	pub total_traded_base_asset_volume: String,
	pub total_traded_quote_asset_volume: String,
	pub statistics_open_time: u64,
	pub statistics_close_time: u64,
	pub total_number_of_trades: u64,
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
