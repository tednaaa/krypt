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

use crate::exchange::Symbol;

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

	// Filter out invalid symbols (non-ASCII characters, special symbols)
	let initial_count = all_symbols.len();
	all_symbols.retain(Symbol::is_valid);
	if initial_count != all_symbols.len() {
		warn!("Filtered out {} invalid symbols (non-ASCII or special characters)", initial_count - all_symbols.len());
	}

	// Apply max_symbols limit (default to 400 to avoid WebSocket connection limits)
	// Binance supports ~20 WebSocket connections, each with 50 streams = ~500 symbols max
	// We use 400 as a safe default (16 connections for 2 intervals per symbol)
	let max_symbols = config.monitoring.max_symbols.unwrap_or(400);
	let tracked_symbols = if all_symbols.len() > max_symbols {
		info!("Limiting symbols from {} to {} (max_symbols limit)", all_symbols.len(), max_symbols);

		// Try to get balanced distribution between exchanges
		let mut binance_symbols: Vec<_> = all_symbols.iter().filter(|s| s.exchange == "binance").cloned().collect();
		let mut bybit_symbols: Vec<_> = all_symbols.iter().filter(|s| s.exchange == "bybit").cloned().collect();

		let binance_count = (max_symbols / 2).min(binance_symbols.len());
		let bybit_count = (max_symbols - binance_count).min(bybit_symbols.len());
		let final_binance_count = binance_count + (max_symbols - binance_count - bybit_count);

		binance_symbols.truncate(final_binance_count);
		bybit_symbols.truncate(bybit_count);

		let mut result = binance_symbols;
		result.extend(bybit_symbols);
		result
	} else {
		all_symbols
	};

	info!(
		"Tracking {} symbols across exchanges ({} Binance, {} Bybit)",
		tracked_symbols.len(),
		tracked_symbols.iter().filter(|s| s.exchange == "binance").count(),
		tracked_symbols.iter().filter(|s| s.exchange == "bybit").count()
	);

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

	// Calculate safe delay between requests to avoid rate limits
	// Binance: ~1200 req/min, Bybit: ~120 req/min
	// Use conservative 10 req/sec = 600 req/min per exchange
	let delay_per_request_ms = if symbols.len() > 100 {
		200 // 5 req/sec for many symbols
	} else if symbols.len() > 50 {
		150 // ~6.6 req/sec
	} else {
		100 // 10 req/sec for few symbols
	};

	info!(
		"Starting derivatives polling (interval: {}s, {} symbols, {}ms delay)",
		poll_interval_secs,
		symbols.len(),
		delay_per_request_ms
	);

	loop {
		poll_interval.tick().await;

		let start_time = std::time::Instant::now();
		let mut success_count = 0;
		let mut error_count = 0;

		for symbol in &symbols {
			// Find the correct exchange for this symbol
			let exchange = exchanges.iter().find(|e| e.name() == symbol.exchange.as_str());

			if let Some(exchange) = exchange {
				match exchange.fetch_derivatives_metrics(symbol).await {
					Ok(metrics) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(symbol) {
							tracker.update_derivatives(metrics);
							success_count += 1;
							debug!(
								symbol = %symbol,
								"Updated derivatives metrics"
							);
						}
					},
					Err(e) => {
						error_count += 1;
						debug!(
							symbol = %symbol,
							error = %e,
							"Failed to fetch derivatives metrics"
						);
					},
				}
			}

			// Rate limiting delay
			tokio::time::sleep(Duration::from_millis(delay_per_request_ms)).await;
		}

		let elapsed = start_time.elapsed();
		info!(
			"Derivatives poll completed: {} success, {} errors, took {:.1}s",
			success_count,
			error_count,
			elapsed.as_secs_f64()
		);
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

	// Calculate safe delay between requests to avoid rate limits
	let delay_per_request_ms = if symbols.len() > 100 {
		200 // 5 req/sec for many symbols
	} else if symbols.len() > 50 {
		150 // ~6.6 req/sec
	} else {
		100 // 10 req/sec for few symbols
	};

	info!(
		"Starting pivot update task (interval: {}m, {} symbols, {}ms delay)",
		pivot_interval_mins,
		symbols.len(),
		delay_per_request_ms
	);

	loop {
		update_interval.tick().await;

		let start_time = std::time::Instant::now();
		let mut success_count = 0;
		let mut error_count = 0;
		let mut error_samples: Vec<(String, String)> = Vec::new();
		let max_error_samples = 5;

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
							success_count += 1;
							debug!(
								symbol = %symbol,
								"Updated pivot levels"
							);
						}
					},
					Err(e) => {
						error_count += 1;

						// Collect sample errors for logging
						if error_samples.len() < max_error_samples {
							error_samples.push((symbol.to_string(), e.to_string()));
						}

						debug!(
							symbol = %symbol,
							error = %e,
							"Failed to fetch historical candles for pivots"
						);
					},
				}
			}

			// Rate limiting delay
			tokio::time::sleep(Duration::from_millis(delay_per_request_ms)).await;
		}

		let elapsed = start_time.elapsed();

		if error_count > 0 {
			warn!(
				"Pivot poll completed: {} success, {} errors, took {:.1}s",
				success_count,
				error_count,
				elapsed.as_secs_f64()
			);

			// Log sample errors to help diagnose issues
			if !error_samples.is_empty() {
				warn!("Sample pivot poll errors:");
				for (symbol, error) in error_samples.iter().take(3) {
					warn!("  {} → {}", symbol, error);
				}
			}
		} else {
			info!(
				"Pivot poll completed: {} success, {} errors, took {:.1}s",
				success_count,
				error_count,
				elapsed.as_secs_f64()
			);
		}
	}
}
