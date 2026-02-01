use anyhow::Context;
use coinglass::Coinglass;
use exchanges::{BinanceExchange, Exchange, MarketLiquidationsInfo};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

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
		.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
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
		async fn handle_alert(
			telegram_bot: &TelegramBot,
			coinglass: &Coinglass,
			binance_rest: &BinanceExchange,
			liquidation_info: MarketLiquidationsInfo,
		) -> anyhow::Result<()> {
			let symbol = liquidation_info.symbol.clone();
			let coin = utils::extract_coin_from_pair(&symbol);

			let liquidation_heatmap_screenshot = tokio::task::block_in_place(|| {
				coinglass
					.get_liquidation_heatmap_screenshot(coin)
					.map_err(|error| anyhow::anyhow!("Failed to get liquidation heatmap screenshot for {symbol}: {error}"))
			})?;

			let open_interest_info = binance_rest
				.get_open_interest_info(&symbol)
				.await
				.map_err(|error| anyhow::anyhow!("Failed to get open interest info for {symbol}: {error}"))?;

			let token_alert = TokenAlert {
				symbol: extract_coin_from_pair(&symbol).to_string(),
				open_interest_info,
				liquidation_info,
				liquidation_heatmap_screenshot,
			};

			telegram_bot.send_alert(&token_alert).await.map_err(|error| {
				anyhow::anyhow!("Failed to send alert for {}: {error}", token_alert.liquidation_info.symbol)
			})?;

			Ok(())
		}

		while let Some(liquidation_info) = alert_rx.recv().await {
			if let Err(error) = handle_alert(&telegram_bot, &coinglass, &binance_rest, liquidation_info).await {
				error!("{error:#}");
				warn!("Skipping alert due to error");
			}
		}
	});

	binance_stream
		.watch_market_liquidations(move |liquidation| {
			if config.scanner.big_tokens.contains(&extract_coin_from_pair(&liquidation.symbol).to_string())
				&& liquidation.usd_price < config.scanner.big_tokens_min_liquidation_usd_price
			{
				return;
			}

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
