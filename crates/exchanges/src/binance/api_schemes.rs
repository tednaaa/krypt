use serde::{Deserialize, Serialize};

// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/All-Market-Tickers-Streams
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TickerStream {
	#[serde(rename = "e")]
	pub event_type: String,
	#[serde(rename = "E")]
	pub event_time: u64,
	#[serde(rename = "s")]
	pub symbol: String,
	#[serde(rename = "p")]
	pub price_change: String,
	#[serde(rename = "P")]
	pub price_change_percent: String,
	#[serde(rename = "w")]
	pub weighted_average_price: String,
	#[serde(rename = "c")]
	pub last_price: String,
	#[serde(rename = "Q")]
	pub last_quantity: String,
	#[serde(rename = "o")]
	pub open_price: String,
	#[serde(rename = "h")]
	pub high_price: String,
	#[serde(rename = "l")]
	pub low_price: String,
	#[serde(rename = "v")]
	pub total_traded_base_asset_volume: String,
	#[serde(rename = "q")]
	pub total_traded_quote_asset_volume: String,
	#[serde(rename = "O")]
	pub statistics_open_time: u64,
	#[serde(rename = "C")]
	pub statistics_close_time: u64,
	#[serde(rename = "F")]
	pub first_trade_id: u64,
	#[serde(rename = "L")]
	pub last_trade_id: u64,
	#[serde(rename = "n")]
	pub total_number_of_trades: u64,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Liquidation-Order-Streams
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ForceOrderStream {
	#[serde(rename = "e")]
	pub event_type: String,
	#[serde(rename = "E")]
	pub event_time: u64,
	#[serde(rename = "o")]
	pub order: ForceOrderInfo,
}
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ForceOrderInfo {
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
#[allow(dead_code)]
pub struct DailyTickerStream {
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
pub struct FundingRateHistoryRequestParams {
	pub symbol: String,
	pub start_time: Option<u64>,
	pub end_time: Option<u64>,
	/// Default 100, max 1000
	pub limit: Option<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRateHistoryResponse {
	pub funding_rate: String,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Open-Interest-Statistics
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenInterestStatisticsRequestParams {
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
#[allow(dead_code)]
pub struct OpenInterestStatisticsResponse {
	symbol: String,
	pub sum_open_interest: String,
	pub sum_open_interest_value: String,
	#[serde(rename = "CMCCirculatingSupply")]
	pub cmc_circulating_supply: String,
	pub timestamp: i64,
}

// https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Long-Short-Ratio
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LongShortRatioRequestParams {
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
#[allow(dead_code)]
pub struct LongShortRatioResponse {
	pub symbol: String,
	pub long_short_ratio: String,
	pub long_account: String,
	pub short_account: String,
	pub timestamp: String,
}
