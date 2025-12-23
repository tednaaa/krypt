use anyhow::Context;
use exchanges::{BinanceExchange, Exchange};
use tracing::info;

use crate::{config::Config, telegram::TelegramBot};

mod config;
mod telegram;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	info!("✅ Starting pump/dump scanner");

	let config = Config::load("config.toml").context("Failed to load configuration")?;
	info!("✅ Configuration loaded");

	let telegram_bot = TelegramBot::new(config.telegram);

	telegram_bot.send_alert().await?;

	let binance = BinanceExchange::new();

	let test_symbol = "ANIMEUSDT";
	info!("{:?}", binance.get_funding_rate_info(test_symbol).await?);
	info!("{:?}", binance.get_open_interest_info(test_symbol).await?);

	Ok(())
}
