use anyhow::Context;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	pub scanner: ScannerConfig,
	pub telegram: TelegramConfig,
	#[allow(dead_code)]
	pub coinglass: CoinglassConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScannerConfig {
	pub min_liquidation_usd_price: f64,
	pub big_tokens: Vec<String>,
	pub big_tokens_min_liquidation_usd_price: f64,
	#[serde(default)]
	pub ticker_alerts: TickerAlertsConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TickerAlertsConfig {
	pub enabled: bool,
	/// Lookback window for price/volume change calculations.
	pub lookback_minutes: u64,
	/// Alert if abs(%) change over the lookback is >= this value.
	pub min_abs_percent_change: f64,
	/// Alert tiers; if volume spike multiplier meets any tier, alert.
	/// Example: [5.0, 10.0]
	pub volume_multipliers: Vec<f64>,
	/// Absolute floor for the (estimated) quote volume in the lookback window (USDT).
	pub min_quote_volume_in_window: f64,
	/// Don't alert again for the same symbol within this cooldown.
	pub alert_cooldown_minutes: u64,
	/// Per-symbol sampling interval; lower = more accurate, higher = cheaper.
	pub sample_every_seconds: u64,
	/// Bounded channel capacity between WS callback and async alert worker.
	pub channel_capacity: usize,
	/// Only consider tickers where the symbol ends with this suffix (e.g. "USDT").
	pub symbol_suffix: String,
}

impl Default for TickerAlertsConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			lookback_minutes: 15,
			min_abs_percent_change: 5.0,
			volume_multipliers: vec![5.0, 10.0],
			min_quote_volume_in_window: 50_000.0,
			alert_cooldown_minutes: 30,
			sample_every_seconds: 30,
			channel_capacity: 8,
			symbol_suffix: String::from("USDT"),
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
	pub bot_token: String,
	pub chat_id: String,
	pub thread_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CoinglassConfig {
	pub login: String,
	pub password: String,
}

impl Config {
	pub fn load(path: &str) -> anyhow::Result<Self> {
		let content = fs::read_to_string(path).context(format!("Failed to read config file: {path}"))?;

		let config: Self = toml::from_str(&content).context("Failed to parse config file")?;

		Ok(config)
	}
}
