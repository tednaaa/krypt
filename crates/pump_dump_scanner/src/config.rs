use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	pub scanner: ScannerConfig,
	pub telegram: TelegramConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScannerConfig {
	pub price_threshold_pct: f64,
	pub min_window_mins: u64,
	pub max_window_mins: u64,
	pub volume_multiplier: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
	pub bot_token: String,
	pub chat_id: String,
	pub pump_screener_topic_id: Option<String>,
	pub alert_cooldown_secs: u64,
}

impl Config {
	pub fn load(path: &str) -> Result<Self> {
		let content = fs::read_to_string(path).context(format!("Failed to read config file: {path}"))?;

		let config: Self = toml::from_str(&content).context("Failed to parse config file")?;

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
