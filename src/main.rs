use anyhow::{Context, Result};
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber;

mod config;
mod detection;
mod engine;
mod scoring;
mod streams;
mod telegram;
mod types;

use config::Config;
use engine::signal_engine_task;
use streams::{BinanceStreamManager, TradeStreamSubscriptionManager};
use telegram::alert_dispatcher_task;
use types::StreamMessage;

#[tokio::main]
async fn main() -> Result<()> {
	// Initialize logging
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	info!("🚀 Crypto Market Scanner starting...");

	// Load configuration
	let config =
		Config::load("config.toml").context("Failed to load configuration. Make sure config.toml exists and is valid")?;

	info!("✅ Configuration loaded successfully");

	// Create channels
	let (stream_tx, stream_rx) =
		mpsc::channel::<StreamMessage>(config.performance.ticker_channel_size + config.performance.trade_channel_size);
	let (alert_tx, alert_rx) = mpsc::channel(config.performance.alert_channel_size);
	let (tier1_tx, tier1_rx) = mpsc::channel(100);

	// Clone for tasks
	let stream_tx_ticker = stream_tx.clone();
	let stream_tx_trade = stream_tx;

	// Spawn ticker stream task
	let ticker_config = config.clone();
	tokio::spawn(async move {
		info!("Starting ticker stream task");
		let stream_manager =
			BinanceStreamManager::new(ticker_config.binance.ws_url.clone(), ticker_config.websocket.clone());

		if let Err(e) = stream_manager.connect_ticker_stream(stream_tx_ticker).await {
			error!("Ticker stream task failed: {}", e);
		}
	});

	// Spawn signal engine task
	let engine_config = config.clone();
	tokio::spawn(async move {
		signal_engine_task(engine_config, stream_rx, alert_tx, tier1_tx).await;
	});

	// Spawn alert dispatcher task
	let telegram_config = config.telegram.clone();
	tokio::spawn(async move {
		alert_dispatcher_task(telegram_config, alert_rx).await;
	});

	// Spawn trade subscription manager task
	let subscription_config = config.clone();
	tokio::spawn(async move {
		info!("Starting trade subscription manager task");

		let manager = TradeStreamSubscriptionManager::new(
			subscription_config.binance.ws_url.clone(),
			subscription_config.websocket.clone(),
		);

		let mut tier1_rx = tier1_rx;

		while let Some(tier1_symbols) = tier1_rx.recv().await {
			info!("Updating trade subscriptions for {} Tier 1 symbols", tier1_symbols.len());

			manager.update_subscriptions(tier1_symbols, stream_tx_trade.clone()).await;

			let active_count = manager.active_count().await;
			info!("Active trade streams: {}", active_count);
		}

		info!("Trade subscription manager task ended");
	});

	info!("✅ All tasks spawned successfully");
	info!("📊 Monitoring Binance USDT pairs...");
	info!("Press Ctrl+C to stop");

	// Wait for shutdown signal
	signal::ctrl_c().await?;

	info!("🛑 Shutdown signal received, stopping...");

	Ok(())
}
