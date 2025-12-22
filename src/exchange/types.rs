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

	pub fn exchange_symbol(&self) -> String {
		format!("{}{}", self.base, self.quote)
	}

	pub fn is_valid(&self) -> bool {
		self.base.chars().all(|c| c.is_ascii_alphanumeric())
			&& self.quote.chars().all(|c| c.is_ascii_alphanumeric())
			&& !self.base.is_empty()
			&& !self.quote.is_empty()
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
pub struct LongShortRatio {
	pub account_long: f64,
	pub account_short: f64,
	pub position_long: f64,
	pub position_short: f64,
}

impl LongShortRatio {
	pub fn account_ratio(&self) -> f64 {
		self.account_long / (self.account_long + self.account_short)
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

#[derive(Debug, Clone)]
pub enum ExchangeMessage {
	Ticker(Ticker),
	Error(String),
}
