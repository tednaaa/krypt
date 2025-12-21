use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	pub binance: BinanceConfig,
	pub bybit: BybitConfig,
	pub filters: FilterConfig,
	pub scoring: ScoringConfig,
	#[allow(dead_code)]
	pub detection: DetectionConfig,
	pub pump: PumpConfig,
	pub derivatives: DerivativesConfig,
	pub technical: TechnicalConfig,
	pub telegram: TelegramConfig,
	#[allow(dead_code)]
	pub performance: PerformanceConfig,
	#[allow(dead_code)]
	pub websocket: WebSocketConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceConfig {
	pub ws_url: String,
	pub api_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BybitConfig {
	pub ws_url: String,
	pub api_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FilterConfig {
	pub min_quote_volume: f64,
	#[allow(dead_code)]
	pub min_price: f64,
	#[allow(dead_code)]
	pub min_trades_24h: u64,
	#[allow(dead_code)]
	pub stale_data_threshold_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringConfig {
	pub tier1_threshold: f64,
	pub tier2_threshold: f64,
	pub max_tier1_symbols: usize,
	#[allow(dead_code)]
	pub rescore_interval_secs: u64,
	#[allow(dead_code)]
	pub weights: ScoringWeights,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
#[allow(clippy::struct_field_names)]
pub struct ScoringWeights {
	pub volume_weight: f64,
	pub volatility_weight: f64,
	pub activity_weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DetectionConfig {
	pub pump_threshold_pct: f64,
	pub dump_threshold_pct: f64,
	pub accumulation_range_pct: f64,
	pub volume_spike_ratio: f64,
	pub breakout_threshold_pct: f64,
	pub window_size_secs: u64,
	pub accumulation_window_secs: u64,
	pub distribution_window_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PumpConfig {
	pub price_threshold_pct: f64,
	pub min_window_mins: u64,
	pub max_window_mins: u64,
	pub volume_multiplier: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DerivativesConfig {
	pub min_funding_rate: f64,
	pub min_long_ratio: f64,
	pub min_oi_increase_pct: f64,
	pub poll_interval_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechnicalConfig {
	pub ema_extension: bool,
	pub pivot_proximity: bool,
	pub pivot_timeframe_mins: u64,
	pub emas: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
	pub bot_token: String,
	pub chat_id: String,
	pub pump_screener_topic_id: Option<String>,
	pub alert_cooldown_secs: u64,
	#[allow(dead_code)]
	pub max_alerts_per_minute: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
#[allow(clippy::struct_field_names)]
pub struct PerformanceConfig {
	pub ticker_channel_size: usize,
	pub trade_channel_size: usize,
	pub alert_channel_size: usize,
	pub price_window_size: usize,
	pub cvd_history_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WebSocketConfig {
	pub ping_interval_secs: u64,
	pub reconnect_base_delay_secs: u64,
	pub reconnect_max_delay_secs: u64,
	pub target_latency_ms: u64,
}

impl Config {
	pub fn load(path: &str) -> Result<Self> {
		let content = fs::read_to_string(path).with_context(|| format!("Failed to read config file: {path}"))?;

		let config: Self = toml::from_str(&content).with_context(|| "Failed to parse config file")?;

		config.validate()?;

		Ok(config)
	}

	fn validate(&self) -> Result<()> {
		if self.telegram.bot_token == "YOUR_BOT_TOKEN_HERE" {
			anyhow::bail!("Please set a valid Telegram bot token in config.toml");
		}

		if self.telegram.chat_id == "YOUR_CHAT_ID_HERE" {
			anyhow::bail!("Please set a valid Telegram chat ID in config.toml");
		}

		if self.filters.min_quote_volume <= 0.0 {
			anyhow::bail!("min_quote_volume must be positive");
		}

		if self.scoring.tier1_threshold <= self.scoring.tier2_threshold {
			anyhow::bail!("tier1_threshold must be greater than tier2_threshold");
		}

		if self.scoring.max_tier1_symbols == 0 {
			anyhow::bail!("max_tier1_symbols must be greater than 0");
		}

		if self.pump.price_threshold_pct <= 0.0 {
			anyhow::bail!("pump price_threshold_pct must be positive");
		}

		if self.pump.volume_multiplier <= 1.0 {
			anyhow::bail!("pump volume_multiplier must be greater than 1.0");
		}

		if self.derivatives.min_funding_rate < 0.0 {
			anyhow::bail!("derivatives min_funding_rate must be non-negative");
		}

		if self.derivatives.min_long_ratio < 0.0 || self.derivatives.min_long_ratio > 1.0 {
			anyhow::bail!("derivatives min_long_ratio must be between 0 and 1");
		}

		if self.technical.emas.is_empty() {
			anyhow::bail!("technical.emas must contain at least one period");
		}

		Ok(())
	}
}
