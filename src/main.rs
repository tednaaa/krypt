mod config;
mod exchange;
mod indicators;
mod pump_scanner;
mod telegram;

use anyhow::{Context, Result};
use config::Config;
use exchange::{create_exchange, Exchange, ExchangeMessage, ExchangeType, Ticker};
use futures_util::StreamExt;
use pump_scanner::{PumpDetector, SignalAnalysis, TrackerManager};
use std::collections::HashSet;
use std::sync::Arc;
use telegram::TelegramBot;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::exchange::Symbol;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	info!("🚀 Starting krypt");

	let config = Config::load("config.toml").context("Failed to load configuration")?;
	info!("✅ Configuration loaded");

	let telegram = TelegramBot::new(config.telegram.clone());

	if let Err(e) = telegram.test_connection().await {
		error!("Failed to connect to Telegram: {}", e);
		error!("Please verify your bot token and chat ID in config.toml");
		return Err(e);
	}
	info!("✅ Telegram bot connected");

	let pump_detector = Arc::new(PumpDetector::new(config.pump.clone()));
	let tracker_manager = Arc::new(RwLock::new(TrackerManager::new(config.technical.emas.clone())));

	let binance = create_exchange(ExchangeType::Binance, &config)?;
	let bybit = create_exchange(ExchangeType::Bybit, &config)?;

	info!("✅ Exchange connections initialized");

	let mut all_symbols = Vec::new();

	match binance.symbols().await {
		Ok(symbols) => {
			info!("Fetched {} symbols from Binance", symbols.len());
			all_symbols.extend(symbols);
		},
		Err(e) => warn!("Failed to fetch Binance symbols: {}", e),
	}

	match bybit.symbols().await {
		Ok(symbols) => {
			info!("Fetched {} symbols from Bybit", symbols.len());
			all_symbols.extend(symbols);
		},
		Err(e) => warn!("Failed to fetch Bybit symbols: {}", e),
	}

	if all_symbols.is_empty() {
		error!("No symbols available from any exchange");
		return Err(anyhow::anyhow!("No symbols available"));
	}

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

	let telegram_arc = Arc::new(telegram);

	let price_stream_task = {
		let params = PriceStreamParams {
			symbols: tracked_symbols.clone(),
			tracker_manager: Arc::clone(&tracker_manager),
			pump_detector: Arc::clone(&pump_detector),
			telegram: Arc::clone(&telegram_arc),
			cooldown_secs: config.telegram.alert_cooldown_secs,
			price_threshold_pct: config.pump.price_threshold_pct,
			price_window_mins: config.pump.max_window_mins,
			config: config.clone(),
		};

		tokio::spawn(async move {
			if let Err(e) = run_price_stream_task(params).await {
				error!("Price stream task failed: {}", e);
			}
		})
	};

	// Task 3: Cleanup stale trackers
	let cleanup_task = {
		let tracker_manager = Arc::clone(&tracker_manager);

		tokio::spawn(async move {
			let mut cleanup_interval = interval(Duration::from_secs(300));

			loop {
				cleanup_interval.tick().await;

				let mut manager = tracker_manager.write().await;
				let before_count = manager.count();
				manager.cleanup_stale(1800);
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

	tokio::select! {
		_ = price_stream_task => warn!("Price stream task ended"),
		_ = cleanup_task => warn!("Cleanup task ended"),
	}

	Ok(())
}

struct PriceStreamParams {
	symbols: Vec<exchange::Symbol>,
	tracker_manager: Arc<RwLock<TrackerManager>>,
	pump_detector: Arc<PumpDetector>,
	telegram: Arc<TelegramBot>,
	cooldown_secs: u64,
	price_threshold_pct: f64,
	price_window_mins: u64,
	config: Config,
}

async fn run_price_stream_task(params: PriceStreamParams) -> Result<()> {
	let PriceStreamParams {
		symbols,
		tracker_manager,
		pump_detector,
		telegram,
		cooldown_secs,
		price_threshold_pct,
		price_window_mins,
		config,
	} = params;
	info!("Starting price stream for {} symbols", symbols.len());
	info!(
		"Will fetch detailed metrics when price change >= {}% in {} minutes (using pump config)",
		price_threshold_pct, price_window_mins
	);
	let binance_symbols: Vec<_> = symbols.iter().filter(|s| s.exchange == "binance").cloned().collect();
	let bybit_symbols: Vec<_> = symbols.iter().filter(|s| s.exchange == "bybit").cloned().collect();

	let fetched_symbols: Arc<RwLock<HashSet<Symbol>>> = Arc::new(RwLock::new(HashSet::new()));

	let mut tasks = Vec::new();

	if !binance_symbols.is_empty() {
		let tracker_manager_clone = Arc::clone(&tracker_manager);
		let pump_detector_clone = Arc::clone(&pump_detector);
		let telegram_clone = Arc::clone(&telegram);
		let fetched_symbols_clone = Arc::clone(&fetched_symbols);
		let config_clone = config.clone();
		let binance_symbols_clone = binance_symbols.clone();

		let binance_task = tokio::spawn(async move {
			let binance_ws = match create_exchange(ExchangeType::Binance, &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Binance WebSocket client: {}", e);
					return;
				},
			};

			let binance_rest = match create_exchange(ExchangeType::Binance, &config_clone) {
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
						process_price_update(PriceUpdateParams {
							ticker,
							tracker_manager: &tracker_manager_clone,
							pump_detector: &pump_detector_clone,
							telegram: &telegram_clone,
							cooldown_secs,
							price_threshold_pct,
							price_window_mins,
							exchange: binance_rest.as_ref(),
							fetched_symbols: &fetched_symbols_clone,
							config: &config_clone,
						})
						.await;
					},
					ExchangeMessage::Error(err) => {
						warn!("Binance price stream error: {}", err);
					},
				}
			}
		});
		tasks.push(binance_task);
	}

	if !bybit_symbols.is_empty() {
		let tracker_manager_clone = Arc::clone(&tracker_manager);
		let pump_detector_clone = Arc::clone(&pump_detector);
		let telegram_clone = Arc::clone(&telegram);
		let fetched_symbols_clone = Arc::clone(&fetched_symbols);
		let config_clone = config.clone();
		let bybit_symbols_clone = bybit_symbols.clone();

		let bybit_task = tokio::spawn(async move {
			let bybit_ws = match create_exchange(ExchangeType::Bybit, &config_clone) {
				Ok(e) => e,
				Err(e) => {
					error!("Failed to create Bybit WebSocket client: {}", e);
					return;
				},
			};

			let bybit_rest = match create_exchange(ExchangeType::Bybit, &config_clone) {
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
						process_price_update(PriceUpdateParams {
							ticker,
							tracker_manager: &tracker_manager_clone,
							pump_detector: &pump_detector_clone,
							telegram: &telegram_clone,
							cooldown_secs,
							price_threshold_pct,
							price_window_mins,
							exchange: bybit_rest.as_ref(),
							fetched_symbols: &fetched_symbols_clone,
							config: &config_clone,
						})
						.await;
					},
					ExchangeMessage::Error(err) => {
						warn!("Bybit price stream error: {}", err);
					},
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

struct PriceUpdateParams<'a> {
	ticker: Ticker,
	tracker_manager: &'a Arc<RwLock<TrackerManager>>,
	pump_detector: &'a Arc<PumpDetector>,
	telegram: &'a Arc<TelegramBot>,
	cooldown_secs: u64,
	price_threshold_pct: f64,
	price_window_mins: u64,
	exchange: &'a dyn Exchange,
	fetched_symbols: &'a Arc<RwLock<HashSet<Symbol>>>,
	config: &'a Config,
}

async fn process_price_update(params: PriceUpdateParams<'_>) {
	let PriceUpdateParams {
		ticker,
		tracker_manager,
		pump_detector,
		telegram,
		cooldown_secs,
		price_threshold_pct,
		price_window_mins,
		exchange,
		fetched_symbols,
		config,
	} = params;
	let mut manager = tracker_manager.write().await;
	let tracker = manager.get_or_create(ticker.symbol.clone());

	tracker.update_from_price(ticker.last_price, ticker.timestamp);
	let price_window_secs = price_window_mins * 60;
	let should_check_pump = if let Some(price_change) = tracker.price_change_in_window(price_window_secs) {
		if price_change.change_pct >= price_threshold_pct {
			let fetched = fetched_symbols.read().await;
			let needs_fetch = !fetched.contains(&ticker.symbol);
			drop(fetched);
			drop(manager);

			if needs_fetch {
				info!(
					symbol = %ticker.symbol,
					change_pct = price_change.change_pct,
					"Price pump detected, fetching detailed metrics and pivot levels..."
				);

				// Fetch derivatives metrics (OI, funding, long/short ratio)
				match exchange.fetch_derivatives_metrics(&ticker.symbol).await {
					Ok(metrics) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(&ticker.symbol) {
							tracker.update_derivatives(metrics);
						}
						drop(manager);
					},
					Err(e) => {
						warn!(
							symbol = %ticker.symbol,
							error = %e,
							"Failed to fetch derivatives metrics"
						);
					},
				}

				// Fetch historical candles for pivot levels
				let pivot_interval_mins = config.technical.pivot_timeframe_mins;
				let interval = exchange.format_interval(pivot_interval_mins as u32);
				match exchange.fetch_historical_candles(&ticker.symbol, &interval, 10).await {
					Ok(candles) => {
						let mut manager = tracker_manager.write().await;
						if let Some(tracker) = manager.get_mut(&ticker.symbol) {
							tracker.update_pivot_levels(&candles);
							debug!(
								symbol = %ticker.symbol,
								"Updated pivot levels for pump"
							);
						}
					},
					Err(e) => {
						warn!(
							symbol = %ticker.symbol,
							error = %e,
							"Failed to fetch historical candles for pivots"
						);
					},
				}

				// Mark symbol as fetched
				let mut fetched = fetched_symbols.write().await;
				fetched.insert(ticker.symbol.clone());
			}

			true
		} else {
			false
		}
	} else {
		false
	};

	if should_check_pump {
		let mut manager = tracker_manager.write().await;
		if let Some(tracker) = manager.get_mut(&ticker.symbol) {
			if tracker.is_in_cooldown(cooldown_secs) {
				drop(manager);
				return;
			}

			if let Some(candidate) = pump_detector.analyze(tracker) {
				info!(
					symbol = %candidate.symbol,
					change = %candidate.price_change.change_pct,
					"Pump detected! Analyzing and sending alert..."
				);

				let analysis = SignalAnalysis::analyze(&candidate, tracker, &config.derivatives, &config.technical);

				if let Err(e) = telegram.post_alert(&candidate, &analysis).await {
					error!(
						symbol = %candidate.symbol,
						error = %e,
						"Failed to send Telegram alert"
					);
				} else {
					tracker.mark_alerted();
					info!(
						symbol = %candidate.symbol,
						score = analysis.total_score,
						"Alert sent successfully"
					);
				}
			}

			pump_detector.update_candidate(tracker);
		}
	}
}
