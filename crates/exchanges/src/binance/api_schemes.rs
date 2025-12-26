use serde::{Deserialize, Serialize};

// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Liquidation-Order-Streams
#[derive(Debug, Deserialize)]
pub struct ForceOrderStream {
	#[serde(rename = "e")]
	event_type: String,
	#[serde(rename = "E")]
	event_time: u64,
	#[serde(rename = "o")]
	order: ForceOrderInfo,
}
#[derive(Debug, Deserialize)]
struct ForceOrderInfo {
	#[serde(rename = "s")]
	pub symbol: String,
	#[serde(rename = "S")]
	pub side: String,
	#[serde(rename = "o")]
	pub order_type: String,
	#[serde(rename = "f")]
	pub time_in_force: String,
	#[serde(rename = "q")]
	pub original_quantity: String,
	#[serde(rename = "p")]
	pub price: String,
	#[serde(rename = "ap")]
	pub average_price: String,
	#[serde(rename = "X")]
	pub order_status: String,
	#[serde(rename = "l")]
	pub order_last_filled_quantity: String,
	#[serde(rename = "z")]
	pub order_filled_accumulated_quantity: String,
	#[serde(rename = "T")]
	pub order_trade_time: u64,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/All-Market-Tickers-Streams
#[derive(Deserialize)]
pub(crate) struct DailyTickerStream {
	#[serde(rename = "e")]
	event_type: String,
	#[serde(rename = "E")]
	event_time: u64,
	#[serde(rename = "s")]
	symbol: String,
	#[serde(rename = "p")]
	price_change: String,
	#[serde(rename = "P")]
	price_change_percent: String,
	#[serde(rename = "w")]
	weighted_average_price: String,
	#[serde(rename = "c")]
	last_price: String,
	#[serde(rename = "Q")]
	last_quantity: String,
	#[serde(rename = "o")]
	open_price: String,
	#[serde(rename = "h")]
	high_price: String,
	#[serde(rename = "l")]
	low_price: String,
	#[serde(rename = "v")]
	total_traded_base_asset_volume: String,
	#[serde(rename = "q")]
	total_traded_quote_asset_volume: String,
	#[serde(rename = "O")]
	statistics_open_time: u64,
	#[serde(rename = "C")]
	statistics_close_time: u64,
	#[serde(rename = "F")]
	first_trade_id: u64,
	#[serde(rename = "L")]
	last_trade_id: u64,
	#[serde(rename = "n")]
	total_number_of_trades: u64,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Get-Funding-Rate-History
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FundingRateHistoryRequestParams {
	pub symbol: String,
	pub start_time: Option<u64>,
	pub end_time: Option<u64>,
	/// Default 100, max 1000
	pub limit: Option<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FundingRateHistoryResponse {
	pub funding_rate: String,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Open-Interest-Statistics
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenInterestStatisticsRequestParams {
	pub symbol: String,
	/// 5m, 15m, 30m, 1h, 2h, 4h, 6h, 12h, 1d
	pub period: String,
	/// default 30, max 500, array latest item is current
	pub limit: Option<i64>,
	pub start_time: Option<i64>,
	pub end_time: Option<i64>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenInterestStatisticsResponse {
	symbol: String,
	pub sum_open_interest: String,
	pub sum_open_interest_value: String,
	#[serde(rename = "CMCCirculatingSupply")]
	pub cmc_circulating_supply: String,
	pub timestamp: i64,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Kline-Candlestick-Data
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KlineCandlestickRequestParams {
	symbol: String,
	/// 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M
	interval: String,
	/// default 500, max 1500
	limit: Option<i64>,
	start_time: Option<i64>,
	end_time: Option<i64>,
}
pub(crate) type KlineCandlestickResponse = (
	u64,    // Open time
	String, // Open
	String, // High
	String, // Low
	String, // Close
	String, // Volume
	u64,    // Close time
	String, // Quote asset volume
	u64,    // Number of trades
	String, // Taker buy base asset volume
	String, // Taker buy quote asset volume
	String, // Ignore
);

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Long-Short-Ratio
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LongShortRatioRequestParams {
	pub symbol: String,
	/// 5m, 15m, 30m, 1h, 2h, 4h, 6h, 12h, 1d
	pub period: String,
	/// default 30, max 500
	pub limit: Option<i64>,
	pub start_time: Option<i64>,
	pub end_time: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LongShortRatioResponse {
	pub symbol: String,
	pub long_short_ratio: String,
	pub long_account: String,
	pub short_account: String,
	pub timestamp: String,
}
