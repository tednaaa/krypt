use anyhow::Context;
use coinglass::Coinglass;
use exchanges::{BinanceExchange, Exchange, MarketLiquidationsInfo};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::{
	config::Config,
	telegram::{TelegramBot, TokenAlert},
	utils::extract_coin_from_pair,
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

	let min_liquidation_usd_price = config.scanner.min_liquidation_usd_price;

	let telegram_bot = TelegramBot::new(config.telegram);
	info!("✅ Telegram bot initialized");

	let coinglass = Coinglass::new()?;
	info!("✅ Coinglass initialized");

	// Use separate clients: one for the WS stream and one for REST calls in the alert worker.
	let binance_stream = BinanceExchange::new();
	let binance_rest = BinanceExchange::new();
	info!("✅ Binance exchange initialized");

	// Keep the stream callback synchronous/cheap: forward events to an async worker.
	// Bounded channel prevents unbounded backlog if the stream is noisy.
	let (alert_tx, mut alert_rx) = mpsc::channel::<MarketLiquidationsInfo>(128);

	tokio::spawn(async move {
		while let Some(liquidation_info) = alert_rx.recv().await {
			let symbol = liquidation_info.symbol.clone();
			let coin = utils::extract_coin_from_pair(&symbol);

			// Coinglass screenshot is a blocking operation; avoid blocking the async runtime.
			let liquidation_heatmap_screenshot =
				tokio::task::block_in_place(|| coinglass.get_liquidation_heatmap_screenshot(coin));

			let liquidation_heatmap_screenshot = match liquidation_heatmap_screenshot {
				Ok(screenshot) => screenshot,
				Err(e) => {
					error!("Failed to get liquidation heatmap screenshot for {}: {}", symbol, e);
					warn!("Skipping alert for {symbol}: no liquidation heatmap screenshot available");
					continue;
				},
			};

			let open_interest_info = match binance_rest.get_open_interest_info(&symbol).await {
				Ok(info) => info,
				Err(e) => {
					error!("Failed to get open interest info for {}: {}", symbol, e);
					continue;
				},
			};

			let token_alert = TokenAlert {
				symbol: extract_coin_from_pair(&symbol).to_string(),
				open_interest_info,
				liquidation_info,
				liquidation_heatmap_screenshot,
			};

			if let Err(e) = telegram_bot.send_alert(&token_alert).await {
				error!("Failed to send alert for {}: {}", token_alert.liquidation_info.symbol, e);
			}
		}
	});

	binance_stream
		.watch_market_liquidations(move |liquidation| {
			if liquidation.usd_price >= min_liquidation_usd_price {
				match alert_tx.try_send(liquidation) {
					Ok(()) => {},
					Err(tokio::sync::mpsc::error::TrySendError::Full(liquidation)) => {
						warn!("Alert queue is full; dropping alert for {}", liquidation.symbol);
					},
					Err(tokio::sync::mpsc::error::TrySendError::Closed(liquidation)) => {
						warn!("Alert worker is down; dropping alert for {}", liquidation.symbol);
					},
				}
			}
		})
		.await?;

	Ok(())
}
