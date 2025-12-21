use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
	pub base: String,
	pub quote: String,
	pub exchange: String,
}

impl Symbol {
	pub fn new(base: impl Into<String>, quote: impl Into<String>, exchange: impl Into<String>) -> Self {
		Self { base: base.into(), quote: quote.into(), exchange: exchange.into() }
	}

	#[allow(dead_code)]
	pub fn pair(&self) -> String {
		format!("{}/{}", self.base, self.quote)
	}

	pub fn exchange_symbol(&self) -> String {
		format!("{}{}", self.base, self.quote)
	}
}

impl std::fmt::Display for Symbol {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}/{}", self.base, self.quote)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
	pub symbol: Symbol,
	pub timestamp: DateTime<Utc>,
	pub open: f64,
	pub high: f64,
	pub low: f64,
	pub close: f64,
	pub volume: f64,
	pub interval: String,
}

impl Candle {
	#[allow(dead_code)]
	pub fn typical_price(&self) -> f64 {
		(self.high + self.low + self.close) / 3.0
	}

	#[allow(dead_code)]
	pub fn range_pct(&self) -> f64 {
		if self.low > 0.0 {
			((self.high - self.low) / self.low) * 100.0
		} else {
			0.0
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPrice {
	pub symbol: Symbol,
	pub timestamp: DateTime<Utc>,
	pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivativesMetrics {
	pub symbol: Symbol,
	pub timestamp: DateTime<Utc>,
	pub open_interest: f64,
	pub open_interest_value: f64,
	pub funding_rate: f64,
	pub long_short_ratio: Option<LongShortRatio>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct LongShortRatio {
	pub long_account_pct: f64,
	pub short_account_pct: f64,
	pub long_position_pct: f64,
	pub short_position_pct: f64,
}

impl LongShortRatio {
	pub fn account_ratio(&self) -> f64 {
		self.long_account_pct / (self.long_account_pct + self.short_account_pct)
	}

	#[allow(dead_code)]
	pub fn position_ratio(&self) -> f64 {
		self.long_position_pct / (self.long_position_pct + self.short_position_pct)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
	pub symbol: Symbol,
	pub timestamp: DateTime<Utc>,
	pub last_price: f64,
	pub volume_24h: f64,
	pub price_change_24h_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PriceSnapshot {
	pub symbol: Symbol,
	pub timestamp: DateTime<Utc>,
	pub price: f64,
	pub volume: f64,
}

#[derive(Debug, Clone)]
pub enum ExchangeMessage {
	Candle(Candle),
	#[allow(dead_code)]
	MarkPrice(MarkPrice),
	#[allow(dead_code)]
	Ticker(Ticker),
	Error(String),
}
