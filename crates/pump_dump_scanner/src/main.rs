use anyhow::Context;
use exchanges::{BinanceExchange, Exchange, TickerInfo};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
	config::Config,
	telegram::{MarketTickerAlert, TelegramBot},
	ticker_scanner::MarketTickerScanner,
};

mod config;
mod telegram;
mod ticker_scanner;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
		.init();

	info!("✅ Starting pump/dump scanner");

	let config = Config::load("config.toml").context("Failed to load configuration")?;
	info!("✅ Configuration loaded");

	let telegram_bot = TelegramBot::new(config.telegram);
	info!("✅ Telegram bot initialized");

	// Use separate clients: one for the WS stream and one for REST calls in the alert worker.
	let binance_stream = BinanceExchange::new();
	info!("✅ Binance exchange initialized");

	let ticker_cfg = config.scanner.ticker_alerts.clone();

	// Keep the stream callback synchronous/cheap: forward batches to an async worker.
	let channel_capacity = ticker_cfg.channel_capacity.max(1);
	let (ticker_tx, mut ticker_rx) = mpsc::channel::<Vec<TickerInfo>>(channel_capacity);
	let telegram_worker = telegram_bot.clone();

	tokio::spawn(async move {
		let mut scanner = MarketTickerScanner::new(ticker_cfg);

		while let Some(batch) = ticker_rx.recv().await {
			for ticker in batch {
				if let Some(candidate) = scanner.on_ticker(&ticker) {
					let alert = MarketTickerAlert {
						symbol: candidate.symbol,
						direction: candidate.direction,
						window_minutes: candidate.window_minutes,
						percent_change_window: candidate.percent_change_window,
						price_now: candidate.price_now,
						quote_volume_window: candidate.quote_volume_window,
						quote_volume_24h: candidate.quote_volume_24h,
						volume_multiplier: candidate.volume_multiplier,
						volume_tier: candidate.volume_tier,
					};

					if let Err(e) = telegram_worker.send_market_ticker_alert(&alert).await {
						error!("Failed to send ticker alert for {}: {}", alert.symbol, e);
					}
				}
			}
		}
	});

	binance_stream
		.watch_market_tickers(move |data| match ticker_tx.try_send(data) {
			Ok(()) => {},
			Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => warn!("Ticker queue is full; dropping ticker batch"),
			Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => warn!("Ticker worker is down; dropping ticker batch"),
		})
		.await?;

	// Keep the stream callback synchronous/cheap: forward events to an async worker.
	// Bounded channel prevents unbounded backlog if the stream is noisy.
	// let (alert_tx, mut alert_rx) = mpsc::channel::<MarketLiquidationsInfo>(128);

	// tokio::spawn(async move {
	// 	while let Some(liquidation_info) = alert_rx.recv().await {
	// 		let symbol = liquidation_info.symbol.clone();
	// 		let coin = utils::extract_coin_from_pair(&symbol);

	// 		// Coinglass screenshot is a blocking operation; avoid blocking the async runtime.
	// 		let liquidation_heatmap_screenshot =
	// 			tokio::task::block_in_place(|| coinglass.get_liquidation_heatmap_screenshot(coin));

	// 		let liquidation_heatmap_screenshot = match liquidation_heatmap_screenshot {
	// 			Ok(screenshot) => screenshot,
	// 			Err(e) => {
	// 				error!("Failed to get liquidation heatmap screenshot for {}: {}", symbol, e);
	// 				warn!("Skipping alert for {symbol}: no liquidation heatmap screenshot available");
	// 				continue;
	// 			},
	// 		};

	// 		let open_interest_info = match binance_rest.get_open_interest_info(&symbol).await {
	// 			Ok(info) => info,
	// 			Err(e) => {
	// 				error!("Failed to get open interest info for {}: {}", symbol, e);
	// 				continue;
	// 			},
	// 		};

	// 		let token_alert = TokenAlert {
	// 			symbol: extract_coin_from_pair(&symbol).to_string(),
	// 			open_interest_info,
	// 			liquidation_info,
	// 			liquidation_heatmap_screenshot,
	// 		};

	// 		if let Err(e) = telegram_bot.send_alert(&token_alert).await {
	// 			error!("Failed to send alert for {}: {}", token_alert.liquidation_info.symbol, e);
	// 		}
	// 	}
	// });

	// binance_stream
	// 	.watch_market_liquidations(move |liquidation| {
	// 		if config.scanner.big_tokens.contains(&extract_coin_from_pair(&liquidation.symbol).to_string())
	// 			&& liquidation.usd_price < config.scanner.big_tokens_min_liquidation_usd_price
	// 		{
	// 			return;
	// 		}

	// 		if liquidation.usd_price >= min_liquidation_usd_price {
	// 			match alert_tx.try_send(liquidation) {
	// 				Ok(()) => {},
	// 				Err(tokio::sync::mpsc::error::TrySendError::Full(liquidation)) => {
	// 					warn!("Alert queue is full; dropping alert for {}", liquidation.symbol);
	// 				},
	// 				Err(tokio::sync::mpsc::error::TrySendError::Closed(liquidation)) => {
	// 					warn!("Alert worker is down; dropping alert for {}", liquidation.symbol);
	// 				},
	// 			}
	// 		}
	// 	})
	// 	.await?;

	Ok(())
}
