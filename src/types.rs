use serde::Deserialize;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Timestamp = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
	Tier1,
	Tier2,
	Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketState {
	Idle,
	Accumulation,
	BreakoutLong,
	Distribution,
	BreakdownShort,
	PumpDetected,
	DumpDetected,
}

#[derive(Debug, Clone)]
pub struct SymbolData {
	pub symbol: String,

	// From ticker stream
	pub price: f64,
	pub price_change_pct_24h: f64,
	pub volume_24h: f64,
	pub quote_volume_24h: f64,
	pub trades_24h: u64,
	pub high_24h: f64,
	pub low_24h: f64,
	pub open_24h: f64,

	// Calculated
	pub score: f64,
	pub tier: Tier,

	// CVD tracking (for Tier 1 only)
	pub cvd: f64,
	pub cvd_history: VecDeque<(Timestamp, f64)>,

	// Short-term windows (for pattern detection)
	pub price_window: VecDeque<(Timestamp, f64)>,
	pub volume_window: VecDeque<(Timestamp, f64)>,

	// State machine
	pub state: MarketState,
	pub last_alert_time: Option<Timestamp>,
	pub last_update_time: Timestamp,

	// Accumulation/Distribution tracking
	pub accumulation_start: Option<Timestamp>,
	pub accumulation_high: Option<f64>,
	pub accumulation_low: Option<f64>,
	pub distribution_start: Option<Timestamp>,
	pub distribution_high: Option<f64>,
	pub distribution_low: Option<f64>,
}

impl SymbolData {
	pub fn new(symbol: String) -> Self {
		Self {
			symbol,
			price: 0.0,
			price_change_pct_24h: 0.0,
			volume_24h: 0.0,
			quote_volume_24h: 0.0,
			trades_24h: 0,
			high_24h: 0.0,
			low_24h: 0.0,
			open_24h: 0.0,
			score: 0.0,
			tier: Tier::Ignored,
			cvd: 0.0,
			cvd_history: VecDeque::new(),
			price_window: VecDeque::new(),
			volume_window: VecDeque::new(),
			state: MarketState::Idle,
			last_alert_time: None,
			last_update_time: current_timestamp(),
			accumulation_start: None,
			accumulation_high: None,
			accumulation_low: None,
			distribution_start: None,
			distribution_high: None,
			distribution_low: None,
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct TickerData {
	#[serde(rename = "s")]
	pub symbol: String,
	#[serde(rename = "c")]
	pub current_price: String,
	#[serde(rename = "p")]
	pub price_change: String,
	#[serde(rename = "P")]
	pub price_change_percent: String,
	#[serde(rename = "v")]
	pub volume: String,
	#[serde(rename = "q")]
	pub quote_volume: String,
	#[serde(rename = "o")]
	pub open_price: String,
	#[serde(rename = "h")]
	pub high_price: String,
	#[serde(rename = "l")]
	pub low_price: String,
	#[serde(rename = "n")]
	pub number_of_trades: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradeData {
	#[serde(rename = "s")]
	pub symbol: String,
	#[serde(rename = "p")]
	pub price: String,
	#[serde(rename = "q")]
	pub quantity: String,
	#[serde(rename = "m")]
	pub is_buyer_maker: bool,
	#[serde(rename = "T")]
	pub trade_time: u64,
}

#[derive(Debug, Clone)]
pub enum StreamMessage {
	Ticker(Vec<TickerData>),
	Trade(TradeData),
}

#[derive(Debug, Clone)]
pub struct Alert {
	pub alert_type: AlertType,
	pub symbol: String,
	pub price: f64,
	pub details: AlertDetails,
	pub timestamp: Timestamp,
	pub exchange: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlertType {
	PumpDetected,
	DumpDetected,
	AccumulationDetected,
	DistributionDetected,
	LongSetupConfirmed,
	ShortSetupConfirmed,
}

#[derive(Debug, Clone)]
pub struct AlertDetails {
	pub price_change_pct: Option<f64>,
	pub volume_ratio: Option<f64>,
	pub cvd_change: Option<f64>,
	pub timeframe: Option<String>,
}

impl Alert {
	pub fn format_telegram(&self) -> String {
		let emoji = match self.alert_type {
			AlertType::PumpDetected => "🚀",
			AlertType::DumpDetected => "📉",
			AlertType::AccumulationDetected => "📊",
			AlertType::DistributionDetected => "⚠️",
			AlertType::LongSetupConfirmed => "✅",
			AlertType::ShortSetupConfirmed => "✅",
		};

		let alert_name = match self.alert_type {
			AlertType::PumpDetected => "PUMP DETECTED",
			AlertType::DumpDetected => "DUMP DETECTED",
			AlertType::AccumulationDetected => "ACCUMULATION DETECTED",
			AlertType::DistributionDetected => "DISTRIBUTION DETECTED",
			AlertType::LongSetupConfirmed => "LONG SETUP CONFIRMED",
			AlertType::ShortSetupConfirmed => "SHORT SETUP CONFIRMED",
		};

		// Generate coinglass link based on exchange
		let coinglass_url = self.get_coinglass_url();

		let mut message = format!("<code>{emoji} {alert_name}</code>\n");
		message.push_str(&format!("Exchange: {}\n", self.exchange));
		message.push_str(&format!("Symbol: <a href=\"{}\">{}</a>\n\n", coinglass_url, self.symbol));

		// Start code block
		message.push_str("<pre>");
		message.push_str(&format!("Price: ${:.8}", self.price));

		if let Some(pct) = self.details.price_change_pct {
			message.push_str(&format!(" ({:+.2}%)", pct));
		}

		if let Some(vol_ratio) = self.details.volume_ratio {
			message.push_str(&format!("\nVolume: {:.1}x average", vol_ratio));
		}

		if let Some(cvd_change) = self.details.cvd_change {
			message.push_str(&format!("\nCVD Change: {:.2}", cvd_change));
		}

		if let Some(ref timeframe) = self.details.timeframe {
			message.push_str(&format!("\nTimeframe: {}", timeframe));
		}

		let time = format_timestamp(self.timestamp);
		message.push_str(&format!("\nTime: {}", time));

		// End code block
		message.push_str("</pre>");

		message
	}

	fn get_coinglass_url(&self) -> String {
		match self.exchange.as_str() {
			"Binance" => format!("https://www.coinglass.com/tv/Binance_{}", self.symbol),
			"Bybit" => format!("https://www.coinglass.com/tv/Bybit_{}", self.symbol),
			"OKX" => format!("https://www.coinglass.com/tv/OKX_{}", self.symbol),
			_ => format!("https://www.coinglass.com/tv/{}_{}", self.exchange, self.symbol),
		}
	}
}

pub fn current_timestamp() -> Timestamp {
	SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub fn format_timestamp(ts: Timestamp) -> String {
	let datetime =
		chrono::DateTime::from_timestamp(ts as i64, 0).unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
	datetime.format("%H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_alert_format_telegram() {
		let alert = Alert {
			alert_type: AlertType::AccumulationDetected,
			symbol: "RESOLVUSDT".to_string(),
			price: 0.08940000,
			details: AlertDetails {
				price_change_pct: Some(0.11),
				volume_ratio: Some(86547.1),
				cvd_change: Some(6.35),
				timeframe: Some("2m".to_string()),
			},
			timestamp: 1704558271, // This will show as 19:44:31 UTC
			exchange: "Binance".to_string(),
		};

		let formatted = alert.format_telegram();

		// Print the formatted message
		println!("\n=== Telegram Message Format ===\n{}\n", formatted);

		// Assertions to verify format
		assert!(formatted.contains("<code>📊 ACCUMULATION DETECTED</code>"));
		assert!(formatted.contains("Exchange: Binance"));
		assert!(formatted.contains("<a href=\"https://www.coinglass.com/tv/Binance_RESOLVUSDT\">RESOLVUSDT</a>"));
		assert!(formatted.contains("Price: $0.08940000 (+0.11%)"));
		assert!(formatted.contains("Volume: 86547.1x average"));
		assert!(formatted.contains("CVD Change: 6.35"));
		assert!(formatted.contains("Timeframe: 2m"));
		assert!(formatted.contains("<pre>"));
		assert!(formatted.contains("</pre>"));
	}

	#[test]
	fn test_coinglass_urls() {
		let test_cases = vec![
			("Binance", "BTCUSDT", "https://www.coinglass.com/tv/Binance_BTCUSDT"),
			("Bybit", "ETHUSDT", "https://www.coinglass.com/tv/Bybit_ETHUSDT"),
			("OKX", "SOLUSDT", "https://www.coinglass.com/tv/OKX_SOLUSDT"),
		];

		for (exchange, symbol, expected_url) in test_cases {
			let alert = Alert {
				alert_type: AlertType::PumpDetected,
				symbol: symbol.to_string(),
				price: 100.0,
				details: AlertDetails { price_change_pct: None, volume_ratio: None, cvd_change: None, timeframe: None },
				timestamp: 1000,
				exchange: exchange.to_string(),
			};

			let url = alert.get_coinglass_url();
			assert_eq!(url, expected_url, "URL mismatch for exchange: {}", exchange);
		}
	}
}
