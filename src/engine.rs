use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::config::{Config, FilterConfig};
use crate::detection::Detector;
use crate::scoring::Scorer;
use crate::types::{current_timestamp, Alert, MarketState, StreamMessage, SymbolData, TickerData, Tier, TradeData};

pub struct SignalEngine {
	config: Config,
	symbols: HashMap<String, SymbolData>,
	scorer: Scorer,
	detector: Detector,
}

impl SignalEngine {
	pub fn new(config: Config) -> Self {
		let scorer = Scorer::new(config.scoring.clone());
		let detector = Detector::new(config.detection.clone());

		Self { config, symbols: HashMap::new(), scorer, detector }
	}

	/// Process a ticker update for all symbols
	pub fn process_ticker(&mut self, tickers: Vec<TickerData>) -> Vec<String> {
		let mut tier1_symbols = Vec::new();

		for ticker in tickers {
			// Apply hard filters
			if !self.passes_hard_filters(&ticker) {
				continue;
			}

			// Update or create symbol data
			let symbol = self.symbols.entry(ticker.symbol.clone()).or_insert_with(|| SymbolData::new(ticker.symbol.clone()));

			self.update_symbol_from_ticker(symbol, &ticker);
		}

		tier1_symbols
	}

	/// Process a trade update for a specific symbol
	pub fn process_trade(&mut self, trade: &TradeData) -> Vec<Alert> {
		let symbol = match self.symbols.get_mut(&trade.symbol) {
			Some(s) => s,
			None => {
				warn!("Received trade for unknown symbol: {}", trade.symbol);
				return Vec::new();
			},
		};

		// Only process trades for Tier 1 symbols
		if symbol.tier != Tier::Tier1 {
			return Vec::new();
		}

		// Update CVD
		self.update_cvd(symbol, trade);

		// Run detection algorithms
		self.detector.detect(symbol)
	}

	/// Recalculate scores and tiers for all symbols
	pub fn rescore_symbols(&mut self) -> Vec<String> {
		let now = current_timestamp();

		// Remove stale symbols
		let stale_threshold = self.config.filters.stale_data_threshold_secs;
		self.symbols.retain(|_, symbol| {
			let elapsed = now.saturating_sub(symbol.last_update_time);
			elapsed <= stale_threshold
		});

		// Calculate scores
		for symbol in self.symbols.values_mut() {
			symbol.score = self.scorer.calculate_score(symbol);
		}

		// Get all symbols as a vector for sorting
		let mut symbol_vec: Vec<SymbolData> = self.symbols.values().cloned().collect();

		// Select Tier 1 symbols
		let tier1_symbols = self.scorer.select_tier1_symbols(&mut symbol_vec);

		// Update tiers
		for symbol in self.symbols.values_mut() {
			if tier1_symbols.contains(&symbol.symbol) {
				symbol.tier = Tier::Tier1;
			} else {
				let tier = self.scorer.assign_tier(symbol.score);
				symbol.tier = tier;

				// Clear Tier 1-specific data if downgraded
				if tier != Tier::Tier1 {
					symbol.cvd = 0.0;
					symbol.cvd_history.clear();
					// Reset state if not Tier 1
					if symbol.state != MarketState::Idle {
						debug!("Resetting state for {} (downgraded from Tier 1)", symbol.symbol);
						symbol.state = MarketState::Idle;
					}
				}
			}
		}

		info!(
			"Rescoring complete: {} total symbols, {} Tier 1, {} Tier 2",
			self.symbols.len(),
			self.symbols.values().filter(|s| s.tier == Tier::Tier1).count(),
			self.symbols.values().filter(|s| s.tier == Tier::Tier2).count()
		);

		tier1_symbols
	}

	/// Check if ticker passes hard filters
	fn passes_hard_filters(&self, ticker: &TickerData) -> bool {
		// Must be USDT pair
		if !ticker.symbol.ends_with("USDT") {
			return false;
		}

		// Parse numeric values
		let quote_volume = match ticker.quote_volume.parse::<f64>() {
			Ok(v) => v,
			Err(_) => return false,
		};

		let current_price = match ticker.current_price.parse::<f64>() {
			Ok(p) => p,
			Err(_) => return false,
		};

		let filters = &self.config.filters;

		// Apply filters
		if quote_volume < filters.min_quote_volume {
			return false;
		}

		if current_price < filters.min_price {
			return false;
		}

		if ticker.number_of_trades < filters.min_trades_24h {
			return false;
		}

		true
	}

	/// Update symbol data from ticker
	fn update_symbol_from_ticker(&mut self, symbol: &mut SymbolData, ticker: &TickerData) {
		// Parse values
		let price = ticker.current_price.parse::<f64>().unwrap_or(0.0);
		let price_change_pct = ticker.price_change_percent.parse::<f64>().unwrap_or(0.0);
		let volume_24h = ticker.volume.parse::<f64>().unwrap_or(0.0);
		let quote_volume_24h = ticker.quote_volume.parse::<f64>().unwrap_or(0.0);
		let high_24h = ticker.high_price.parse::<f64>().unwrap_or(0.0);
		let low_24h = ticker.low_price.parse::<f64>().unwrap_or(0.0);
		let open_24h = ticker.open_price.parse::<f64>().unwrap_or(0.0);

		let now = current_timestamp();

		// Update basic fields
		symbol.price = price;
		symbol.price_change_pct_24h = price_change_pct;
		symbol.volume_24h = volume_24h;
		symbol.quote_volume_24h = quote_volume_24h;
		symbol.trades_24h = ticker.number_of_trades;
		symbol.high_24h = high_24h;
		symbol.low_24h = low_24h;
		symbol.open_24h = open_24h;
		symbol.last_update_time = now;

		// Update price window
		symbol.price_window.push_back((now, price));
		while symbol.price_window.len() > self.config.performance.price_window_size {
			symbol.price_window.pop_front();
		}

		// Update volume window (use quote volume)
		symbol.volume_window.push_back((now, quote_volume_24h));
		while symbol.volume_window.len() > self.config.performance.price_window_size {
			symbol.volume_window.pop_front();
		}
	}

	/// Update CVD from trade data
	fn update_cvd(&mut self, symbol: &mut SymbolData, trade: &TradeData) {
		let price = trade.price.parse::<f64>().unwrap_or(0.0);
		let quantity = trade.quantity.parse::<f64>().unwrap_or(0.0);

		if price == 0.0 || quantity == 0.0 {
			return;
		}

		let volume_usd = price * quantity;

		// is_buyer_maker = true means this was a sell (maker was selling)
		// is_buyer_maker = false means this was a buy (maker was buying)
		let delta = if trade.is_buyer_maker {
			-volume_usd // Sell
		} else {
			volume_usd // Buy
		};

		symbol.cvd += delta;

		// Update CVD history
		let now = current_timestamp();
		symbol.cvd_history.push_back((now, symbol.cvd));

		// Keep only recent history
		while symbol.cvd_history.len() > self.config.performance.cvd_history_size {
			symbol.cvd_history.pop_front();
		}
	}

	/// Get statistics about the engine state
	pub fn get_stats(&self) -> EngineStats {
		let tier1_count = self.symbols.values().filter(|s| s.tier == Tier::Tier1).count();

		let tier2_count = self.symbols.values().filter(|s| s.tier == Tier::Tier2).count();

		let state_counts = self.count_states();

		EngineStats {
			total_symbols: self.symbols.len(),
			tier1_symbols: tier1_count,
			tier2_symbols: tier2_count,
			state_counts,
		}
	}

	fn count_states(&self) -> HashMap<MarketState, usize> {
		let mut counts = HashMap::new();

		for symbol in self.symbols.values() {
			*counts.entry(symbol.state).or_insert(0) += 1;
		}

		counts
	}
}

#[derive(Debug, Clone)]
pub struct EngineStats {
	pub total_symbols: usize,
	pub tier1_symbols: usize,
	pub tier2_symbols: usize,
	pub state_counts: HashMap<MarketState, usize>,
}

/// Main signal engine task
pub async fn signal_engine_task(
	config: Config,
	mut stream_rx: mpsc::Receiver<StreamMessage>,
	alert_tx: mpsc::Sender<Alert>,
	tier1_tx: mpsc::Sender<Vec<String>>,
) {
	info!("Starting signal engine task");

	let mut engine = SignalEngine::new(config.clone());

	// Setup periodic rescoring
	let rescore_interval = config.scoring.rescore_interval_secs;
	let mut rescore_timer = interval(Duration::from_secs(rescore_interval));

	// Stats reporting interval
	let mut stats_timer = interval(Duration::from_secs(60));

	loop {
		tokio::select! {
				Some(msg) = stream_rx.recv() => {
						match msg {
								StreamMessage::Ticker(tickers) => {
										engine.process_ticker(tickers);
								}
								StreamMessage::Trade(trade) => {
										let alerts = engine.process_trade(&trade);
										for alert in alerts {
												if let Err(e) = alert_tx.send(alert).await {
														error!("Failed to send alert: {}", e);
												}
										}
								}
						}
				}

				_ = rescore_timer.tick() => {
						debug!("Running periodic rescoring");
						let tier1_symbols = engine.rescore_symbols();

						// Notify subscription manager
						if let Err(e) = tier1_tx.send(tier1_symbols).await {
								error!("Failed to send tier1 symbols update: {}", e);
						}
				}

				_ = stats_timer.tick() => {
						let stats = engine.get_stats();
						info!(
								"Engine stats: {} total, {} T1, {} T2 | States: {:?}",
								stats.total_symbols,
								stats.tier1_symbols,
								stats.tier2_symbols,
								stats.state_counts
						);
				}
		}
	}
}
