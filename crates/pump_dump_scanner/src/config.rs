use anyhow::Context;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
	pub scanner: ScannerConfig,
	pub telegram: TelegramConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScannerConfig {
	pub price_change_percent: f64,
	pub volume_multiplier: f64,
	pub min_window_mins: u64,
	pub max_window_mins: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
	pub bot_token: String,
	pub chat_id: String,
	pub thread_id: Option<i32>,
}

impl Config {
	pub fn load(path: &str) -> anyhow::Result<Self> {
		let content = fs::read_to_string(path).context(format!("Failed to read config file: {path}"))?;

		let config: Self = toml::from_str(&content).context("Failed to parse config file")?;

		config.validate()?;

		Ok(config)
	}

	fn validate(&self) -> anyhow::Result<()> {
		if self.telegram.bot_token.is_empty() {
			anyhow::bail!("Please set a valid Telegram bot token in config.toml");
		}

		if self.telegram.chat_id.is_empty() {
			anyhow::bail!("Please set a valid Telegram chat ID in config.toml");
		}

		if self.scanner.price_change_percent <= 0.0 {
			anyhow::bail!("pump/dump price_change_percent must be positive");
		}

		if self.scanner.volume_multiplier <= 1.0 {
			anyhow::bail!("pump volume_multiplier must be greater than 1.0");
		}

		Ok(())
	}
}
