mod config;
mod exchange;
mod indicators;
mod pump_scanner;
mod telegram;

use anyhow::{Context, Result};
use config::Config;
use exchange::{create_exchange, Exchange, ExchangeMessage, Ticker};
use futures_util::StreamExt;
use pump_scanner::{OverheatingQualifier, PumpDetector, TrackerManager};
use std::collections::HashSet;
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

	let tracked_symbols = all_symbols;

	info!(
		"Tracking {} symbols across exchanges ({} Binance, {} Bybit)",
		tracked_symbols.len(),
		tracked_symbols.iter().filter(|s| s.exchange == "binance").count(),
		tracked_symbols.iter().filter(|s| s.exchange == "bybit").count()
	);

	// Spawn background tasks
	let telegram_arc = Arc::new(telegram);

	// Task 1: Stream prices and detect pumps, fetch detailed metrics on-demand
	let price_stream_task = {
		let tracker_manager = Arc::clone(&tracker_manager);
		let pump_detector = Arc::clone(&pump_detector);
		let qualifier = Arc::clone(&qualifier);
		let telegram = Arc::clone(&telegram_arc);
		let symbols = tracked_symbols.clone();
		let cooldown_secs = config.telegram.alert_cooldown_secs;
		let price_threshold_pct = config.pump.price_threshold_pct;
		let price_window_mins = config.pump.max_window_mins;
		let config_clone = config.clone();

		tokio::spawn(async move {
			if let Err(e) = run_price_stream_task(
				binance,
				bybit,
				symbols,
				tracker_manager,
				pump_detector,
				qualifier,
				telegram,
				cooldown_secs,
				price_threshold_pct,
				price_window_mins,
				config_clone,
			)
			.await
			{
				error!("Price stream task failed: {}", e);
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
		_ = price_stream_task => warn!("Price stream task ended"),
		_ = pivot_task => warn!("Pivot task ended"),
		_ = cleanup_task => warn!("Cleanup task ended"),
	}

	Ok(())
}

/// Runs the price streaming and pump detection task
/// Only fetches detailed metrics (OI, volume, etc.) when price pump threshold is hit
async fn run_price_stream_task(
	_binance: Box<dyn Exchange>,
	_bybit: Box<dyn Exchange>,
	symbols: Vec<exchange::Symbol>,
	tracker_manager: Arc<RwLock<TrackerManager>>,
	pump_detector: Arc<PumpDetector>,
	qualifier: Arc<OverheatingQualifier>,
	telegram: Arc<TelegramBot>,
	cooldown_secs: u64,
	price_threshold_pct: f64,
	price_window_mins: u64,
	config: Config,
) -> Result<()> {
	info!("Starting price stream for {} symbols", symbols.len());
	info!("Will fetch detailed metrics when price change >= {}% in {} minutes (using pump config)", price_threshold_pct, price_window_mins);

	// Split symbols by exchange
	let binance_symbols: Vec<_> = symbols.iter().filter(|s| s.exchange == "binance").cloned().collect();
	let bybit_symbols: Vec<_> = symbols.iter().filter(|s| s.exchange == "bybit").cloned().collect();

	// Track symbols that have triggered detailed metrics fetch (to avoid duplicate fetches)
	let fetched_symbols: Arc<RwLock<HashSet<Symbol>>> = Arc::new(RwLock::new(HashSet::new()));

	// Spawn tasks for each exchange's price stream
	let mut tasks = Vec::new();

	if !binance_symbols.is_empty() {
		let tracker_manager_clone = Arc::clone(&tracker_manager);
		let pump_detector_clone = Arc::clone(&pump_detector);
		let qualifier_clone = Arc::clone(&qualifier);
		let telegram_clone = Arc::clone(&telegram);
		let fetched_symbols_clone = Arc::clone(&fetched_symbols);
		let config_clone = config.clone();
		let binance_symbols_clone = binance_symbols.clone();

		let binance_task = tokio::spawn(async move {
			let binance_ws = match create_exchange("binance", &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Binance WebSocket client: {}", e);
					return;
				},
			};

			let binance_rest = match create_exchange("binance", &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Binance REST client: {}", e);
					return;
				},
			};

			let mut stream = match binance_ws.stream_prices(&binance_symbols_clone).await {
				Ok(s) => s,
				Err(e) => {
					error!("Failed to create Binance price stream: {}", e);
					return;
				},
			};

			while let Some(message) = stream.next().await {
				match message {
					ExchangeMessage::Ticker(ticker) => {
						process_price_update(
							ticker,
							&tracker_manager_clone,
							&pump_detector_clone,
							&qualifier_clone,
							&telegram_clone,
							cooldown_secs,
							price_threshold_pct,
							price_window_mins,
							&binance_rest,
							&fetched_symbols_clone,
						)
						.await;
					},
					ExchangeMessage::Error(err) => {
						warn!("Binance price stream error: {}", err);
					},
					_ => {},
				}
			}
		});
		tasks.push(binance_task);
	}

	if !bybit_symbols.is_empty() {
		let tracker_manager_clone = Arc::clone(&tracker_manager);
		let pump_detector_clone = Arc::clone(&pump_detector);
		let qualifier_clone = Arc::clone(&qualifier);
		let telegram_clone = Arc::clone(&telegram);
		let fetched_symbols_clone = Arc::clone(&fetched_symbols);
		let config_clone = config.clone();
		let bybit_symbols_clone = bybit_symbols.clone();

		let bybit_task = tokio::spawn(async move {
			let bybit_ws = match create_exchange("bybit", &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Bybit WebSocket client: {}", e);
					return;
				},
			};

			let bybit_rest = match create_exchange("bybit", &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Bybit REST client: {}", e);
					return;
				},
			};

			let mut stream = match bybit_ws.stream_prices(&bybit_symbols_clone).await {
				Ok(s) => s,
				Err(e) => {
					error!("Failed to create Bybit price stream: {}", e);
					return;
				},
			};

			while let Some(message) = stream.next().await {
				match message {
					ExchangeMessage::Ticker(ticker) => {
						process_price_update(
							ticker,
							&tracker_manager_clone,
							&pump_detector_clone,
							&qualifier_clone,
							&telegram_clone,
							cooldown_secs,
							price_threshold_pct,
							price_window_mins,
							&bybit_rest,
							&fetched_symbols_clone,
						)
						.await;
					},
					ExchangeMessage::Error(err) => {
						warn!("Bybit price stream error: {}", err);
					},
					_ => {},
				}
			}
		});
		tasks.push(bybit_task);
	}

	// Wait for all tasks
	futures_util::future::join_all(tasks).await;

	warn!("Price stream ended");
	Ok(())
}

/// Processes a price update and checks for pump signals
/// Fetches detailed metrics via REST API when price threshold is hit
async fn process_price_update(
	ticker: Ticker,
	tracker_manager: &Arc<RwLock<TrackerManager>>,
	pump_detector: &Arc<PumpDetector>,
	qualifier: &Arc<OverheatingQualifier>,
	telegram: &Arc<TelegramBot>,
	cooldown_secs: u64,
	price_threshold_pct: f64,
	price_window_mins: u64,
	exchange: &Box<dyn Exchange>,
	fetched_symbols: &Arc<RwLock<HashSet<Symbol>>>,
) {
	let mut manager = tracker_manager.write().await;
	let tracker = manager.get_or_create(ticker.symbol.clone());

	// Update tracker with new price
	tracker.update_from_price(ticker.last_price, ticker.timestamp);

	// Check if price change exceeds threshold
	let price_window_secs = price_window_mins * 60;
	let should_check_pump = if let Some(price_change) = tracker.price_change_in_window(price_window_secs) {
		if price_change.change_pct >= price_threshold_pct {
			// Check if we've already fetched detailed metrics for this symbol
			let fetched = fetched_symbols.read().await;
			let needs_fetch = !fetched.contains(&ticker.symbol);
			drop(fetched);
			drop(manager);

			if needs_fetch {
				// Fetch detailed metrics via REST API
				info!(
					symbol = %ticker.symbol,
					change_pct = price_change.change_pct,
					"Price pump detected, fetching detailed metrics..."
				);

				match exchange.fetch_derivatives_metrics(&ticker.symbol).await {
					Ok(metrics) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(&ticker.symbol) {
							tracker.update_derivatives(metrics);
						}
						drop(manager);

						let mut fetched = fetched_symbols.write().await;
						fetched.insert(ticker.symbol.clone());
					},
					Err(e) => {
						warn!(
							symbol = %ticker.symbol,
							error = %e,
							"Failed to fetch detailed metrics"
						);
					},
				}
			}

			true
		} else {
			false
		}
	} else {
		false
	};

	// Check for pump and send alerts if threshold was hit
	if should_check_pump {
		let mut manager = tracker_manager.write().await;
		if let Some(tracker) = manager.get_mut(&ticker.symbol) {
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
				let interval = exchange.format_interval(pivot_interval_mins as u32);
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
