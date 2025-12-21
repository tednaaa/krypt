mod config;
mod exchange;
mod indicators;
mod pump_scanner;
mod telegram;

use anyhow::{Context, Result};
use config::Config;
use exchange::{create_exchange, Candle, Exchange, ExchangeMessage};
use futures_util::StreamExt;
use pump_scanner::{OverheatingQualifier, PumpDetector, TrackerManager};
use std::sync::Arc;
use telegram::TelegramBot;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
	// Initialize tracing
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	info!("🚀 Starting Pump Scanner → Short Bias Alert Bot");

	// Load configuration
	let config = Config::load("config.toml").context("Failed to load configuration")?;
	info!("✅ Configuration loaded");

	// Initialize Telegram bot
	let telegram = TelegramBot::new(config.telegram.clone());

	// Test Telegram connection
	if let Err(e) = telegram.test_connection().await {
		error!("Failed to connect to Telegram: {}", e);
		error!("Please verify your bot token and chat ID in config.toml");
		return Err(e);
	}
	info!("✅ Telegram bot connected");

	// Initialize components
	let pump_detector = Arc::new(PumpDetector::new(config.pump.clone()));
	let qualifier = Arc::new(OverheatingQualifier::new(config.derivatives.clone(), config.technical.clone()));
	let tracker_manager = Arc::new(RwLock::new(TrackerManager::new(config.technical.emas.clone())));

	// Create exchange instances
	let binance = create_exchange("binance", &config)?;
	let bybit = create_exchange("bybit", &config)?;

	info!("✅ Exchange connections initialized");

	// Fetch symbols from exchanges
	let mut all_symbols = Vec::new();

	match binance.symbols().await {
		Ok(symbols) => {
			info!("Fetched {} symbols from Binance", symbols.len());
			all_symbols.extend(symbols);
		},
		Err(e) => {
			warn!("Failed to fetch Binance symbols: {}", e);
		},
	}

	match bybit.symbols().await {
		Ok(symbols) => {
			info!("Fetched {} symbols from Bybit", symbols.len());
			all_symbols.extend(symbols);
		},
		Err(e) => {
			warn!("Failed to fetch Bybit symbols: {}", e);
		},
	}

	if all_symbols.is_empty() {
		error!("No symbols available from any exchange");
		return Err(anyhow::anyhow!("No symbols available"));
	}

	// Filter to top symbols (limit to prevent overwhelming)
	let tracked_symbols = all_symbols.into_iter().take(50).collect::<Vec<_>>();
	info!("Tracking {} symbols across exchanges", tracked_symbols.len());

	// Spawn background tasks
	let telegram_arc = Arc::new(telegram);

	// Task 1: Stream candles and detect pumps
	let candle_task = {
		let tracker_manager = Arc::clone(&tracker_manager);
		let pump_detector = Arc::clone(&pump_detector);
		let qualifier = Arc::clone(&qualifier);
		let telegram = Arc::clone(&telegram_arc);
		let symbols = tracked_symbols.clone();
		let cooldown_secs = config.telegram.alert_cooldown_secs;

		tokio::spawn(async move {
			if let Err(e) =
				run_candle_stream_task(binance, symbols, tracker_manager, pump_detector, qualifier, telegram, cooldown_secs)
					.await
			{
				error!("Candle stream task failed: {}", e);
			}
		})
	};

	// Task 2: Periodically fetch derivatives metrics
	let derivatives_task = {
		let tracker_manager = Arc::clone(&tracker_manager);
		let tracked_symbols = tracked_symbols.clone();
		let poll_interval_secs = config.derivatives.poll_interval_secs;
		let config = config.clone();

		tokio::spawn(async move {
			if let Err(e) = run_derivatives_polling_task(tracked_symbols, tracker_manager, poll_interval_secs, config).await {
				error!("Derivatives polling task failed: {}", e);
			}
		})
	};

	// Task 3: Periodically fetch pivot levels
	let pivot_task = {
		let tracker_manager = Arc::clone(&tracker_manager);
		let tracked_symbols = tracked_symbols.clone();
		let pivot_interval_mins = config.technical.pivot_timeframe_mins;
		let config_clone = config.clone();

		tokio::spawn(async move {
			if let Err(e) = run_pivot_update_task(tracked_symbols, tracker_manager, pivot_interval_mins, config_clone).await {
				error!("Pivot update task failed: {}", e);
			}
		})
	};

	// Task 4: Cleanup stale trackers
	let cleanup_task = {
		let tracker_manager = Arc::clone(&tracker_manager);

		tokio::spawn(async move {
			let mut cleanup_interval = interval(Duration::from_secs(300)); // Every 5 minutes

			loop {
				cleanup_interval.tick().await;

				let mut manager = tracker_manager.write().await;
				let before_count = manager.count();
				manager.cleanup_stale(1800); // Remove trackers older than 30 minutes
				let after_count = manager.count();
				drop(manager);

				if before_count != after_count {
					info!("Cleaned up {} stale trackers ({} -> {})", before_count - after_count, before_count, after_count);
				}
			}
		})
	};

	info!("✅ All tasks started");
	info!("🔍 Monitoring markets for pump signals...");

	// Wait for all tasks
	tokio::select! {
		_ = candle_task => warn!("Candle stream task ended"),
		_ = derivatives_task => warn!("Derivatives task ended"),
		_ = pivot_task => warn!("Pivot task ended"),
		_ = cleanup_task => warn!("Cleanup task ended"),
	}

	Ok(())
}

/// Runs the candle streaming and pump detection task
async fn run_candle_stream_task(
	exchange: Box<dyn Exchange>,
	symbols: Vec<exchange::Symbol>,
	tracker_manager: Arc<RwLock<TrackerManager>>,
	pump_detector: Arc<PumpDetector>,
	qualifier: Arc<OverheatingQualifier>,
	telegram: Arc<TelegramBot>,
	cooldown_secs: u64,
) -> Result<()> {
	let intervals = vec!["1m", "5m"];

	info!("Starting candle stream for {} symbols", symbols.len());

	let mut stream = exchange.stream_candles(&symbols, &intervals).await?;

	while let Some(message) = stream.next().await {
		match message {
			ExchangeMessage::Candle(candle) => {
				process_candle(candle, &tracker_manager, &pump_detector, &qualifier, &telegram, cooldown_secs).await;
			},
			ExchangeMessage::Error(err) => {
				warn!("Exchange stream error: {}", err);
			},
			_ => {},
		}
	}

	warn!("Candle stream ended");
	Ok(())
}

/// Processes a candle update and checks for pump signals
async fn process_candle(
	candle: Candle,
	tracker_manager: &Arc<RwLock<TrackerManager>>,
	pump_detector: &Arc<PumpDetector>,
	qualifier: &Arc<OverheatingQualifier>,
	telegram: &Arc<TelegramBot>,
	cooldown_secs: u64,
) {
	let mut manager = tracker_manager.write().await;
	let tracker = manager.get_or_create(candle.symbol.clone());

	// Update tracker with new candle
	tracker.update_from_candle(&candle);

	// Skip if in cooldown
	if tracker.is_in_cooldown(cooldown_secs) {
		drop(manager);
		return;
	}

	// Detect pump candidate
	if let Some(candidate) = pump_detector.analyze(tracker) {
		debug!(
			symbol = %candidate.symbol,
			change = %candidate.price_change.change_pct,
			"Pump candidate detected, checking qualification..."
		);

		// Qualify the pump
		if let Some(qualification) = qualifier.qualify(&candidate, tracker) {
			info!(
				symbol = %candidate.symbol,
				score = qualification.score,
				"Pump qualified! Sending alert..."
			);

			// Send Telegram alert
			if let Err(e) = telegram.post_alert(&candidate, &qualification).await {
				error!(
					symbol = %candidate.symbol,
					error = %e,
					"Failed to send Telegram alert"
				);
			} else {
				// Mark as alerted
				tracker.mark_alerted();
				info!(
					symbol = %candidate.symbol,
					"Alert sent successfully"
				);
			}
		} else {
			debug!(
				symbol = %candidate.symbol,
				"Pump not qualified - insufficient overheating conditions"
			);
		}
	}

	// Update pump candidate state
	pump_detector.update_candidate(tracker);
	drop(manager);
}

/// Runs the derivatives metrics polling task
async fn run_derivatives_polling_task(
	symbols: Vec<exchange::Symbol>,
	tracker_manager: Arc<RwLock<TrackerManager>>,
	poll_interval_secs: u64,
	config: Config,
) -> Result<()> {
	let mut poll_interval = interval(Duration::from_secs(poll_interval_secs));

	// Create exchange instances for REST API calls
	let exchanges = [create_exchange("binance", &config)?, create_exchange("bybit", &config)?];

	info!("Starting derivatives polling (interval: {}s)", poll_interval_secs);

	loop {
		poll_interval.tick().await;

		for symbol in &symbols {
			// Find the correct exchange for this symbol
			let exchange = exchanges.iter().find(|e| e.name() == symbol.exchange.as_str());

			if let Some(exchange) = exchange {
				match exchange.fetch_derivatives_metrics(symbol).await {
					Ok(metrics) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(symbol) {
							tracker.update_derivatives(metrics);
							debug!(
								symbol = %symbol,
								"Updated derivatives metrics"
							);
						}
					},
					Err(e) => {
						debug!(
							symbol = %symbol,
							error = %e,
							"Failed to fetch derivatives metrics"
						);
					},
				}
			}

			// Small delay to avoid rate limiting
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	}
}

/// Runs the pivot levels update task
async fn run_pivot_update_task(
	symbols: Vec<exchange::Symbol>,
	tracker_manager: Arc<RwLock<TrackerManager>>,
	pivot_interval_mins: u64,
	config: Config,
) -> Result<()> {
	let mut update_interval = interval(Duration::from_secs(pivot_interval_mins * 60));

	// Create exchange instances for REST API calls
	let exchanges = [create_exchange("binance", &config)?, create_exchange("bybit", &config)?];

	info!("Starting pivot update task (interval: {}m)", pivot_interval_mins);

	loop {
		update_interval.tick().await;

		for symbol in &symbols {
			// Find the correct exchange for this symbol
			let exchange = exchanges.iter().find(|e| e.name() == symbol.exchange.as_str());

			if let Some(exchange) = exchange {
				// Fetch historical candles for pivot calculation
				let interval = format!("{pivot_interval_mins}m");
				match exchange.fetch_historical_candles(symbol, &interval, 10).await {
					Ok(candles) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(symbol) {
							tracker.update_pivot_levels(&candles);
							debug!(
								symbol = %symbol,
								"Updated pivot levels"
							);
						}
					},
					Err(e) => {
						debug!(
							symbol = %symbol,
							error = %e,
							"Failed to fetch historical candles for pivots"
						);
					},
				}
			}

			// Small delay to avoid rate limiting
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	}
}
