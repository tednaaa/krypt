use anyhow::Context;
use coinglass::login;
use exchanges::{BinanceExchange, Exchange};
use tracing::info;

use crate::{
	config::Config,
	telegram::{TelegramBot, TokenAlert},
};

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
	info!("✅ Telegram bot initialized");

	coinglass::login(&config.coinglass.login, &config.coinglass.password)?;
	info!("✅ Successfully logged in to CoinGlass");

	// let binance = BinanceExchange::new();

	// let test_symbol = "ZBTUSDT";

	// let open_interest_info = binance.get_open_interest_info(test_symbol).await?;
	// let chart_screenshot = coinglass::get_chart_screenshot(test_symbol)?;

	// let token_alert = TokenAlert { symbol: String::from(test_symbol), open_interest_info };

	// telegram_bot.send_alert(&token_alert, chart_screenshot).await?;

	Ok(())
}
