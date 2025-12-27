use anyhow::Context;
use coinglass::Coinglass;
use exchanges::{BinanceExchange, Exchange};
use tracing::info;

use crate::{
	config::Config,
	telegram::{TelegramBot, TokenAlert},
};

mod config;
mod telegram;
mod utils;

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

	// coinglass::login(&config.coinglass.login, &config.coinglass.password)?;
	// info!("✅ Successfully logged in to CoinGlass");

	let binance = BinanceExchange::new();

	binance
		.watch_market_liquidations(|liquidation| {
			if liquidation.usd_price >= config.scanner.min_liquidation_usd_price {
				println!("{liquidation:?}");
			}
		})
		.await?;

	// let test_symbol = "LIGHTUSDT";

	// let coinglass = Coinglass::new()?;

	// let liquidation_heatmap_screenshot =
	// 	coinglass.get_liquidation_heatmap_screenshot(utils::extract_coin_from_pair(test_symbol))?;

	// let open_interest_info = binance.get_open_interest_info(test_symbol).await?;

	// let token_alert = TokenAlert {
	// 	symbol: String::from(test_symbol),
	// 	open_interest_info,
	// 	chart_screenshot: None,
	// 	liquidation_heatmap_screenshot: Some(liquidation_heatmap_screenshot),
	// };

	// telegram_bot.send_alert(&token_alert).await?;

	Ok(())
}
