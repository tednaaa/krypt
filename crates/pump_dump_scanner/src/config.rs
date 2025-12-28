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
