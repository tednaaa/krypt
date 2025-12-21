use crate::exchange::{Candle, DerivativesMetrics, Symbol};
use crate::indicators::{MultiEMA, PivotLevels};
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, VecDeque};

/// Tracks state for a single symbol during pump detection
#[derive(Debug, Clone)]
pub struct SymbolTracker {
	pub symbol: Symbol,
	pub price_history: VecDeque<PricePoint>,
	pub volume_history: VecDeque<f64>,
	pub ema_1m: MultiEMA,
	pub ema_5m: MultiEMA,
	pub pivot_levels: Option<PivotLevels>,
	pub last_derivatives: Option<DerivativesMetrics>,
	pub baseline_derivatives: Option<DerivativesMetrics>,
	pub pump_state: PumpState,
	pub last_alert_time: Option<DateTime<Utc>>,
	pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PricePoint {
	pub timestamp: DateTime<Utc>,
	pub price: f64,
	pub volume: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PumpState {
	Normal,
	Candidate { detected_at: DateTime<Utc>, entry_price: f64, max_price: f64, total_volume: f64 },
	Alerted { alerted_at: DateTime<Utc> },
}

impl SymbolTracker {
	pub fn new(symbol: Symbol, ema_periods: &[u32]) -> Self {
		Self {
			symbol,
			price_history: VecDeque::with_capacity(1200), // ~20 minutes at 1 update/sec
			volume_history: VecDeque::with_capacity(60),  // 1 hour of 1m candles
			ema_1m: MultiEMA::new(ema_periods),
			ema_5m: MultiEMA::new(ema_periods),
			pivot_levels: None,
			last_derivatives: None,
			baseline_derivatives: None,
			pump_state: PumpState::Normal,
			last_alert_time: None,
			last_update: Utc::now(),
		}
	}

	/// Updates price history with a new candle
	pub fn update_from_candle(&mut self, candle: &Candle) {
		let price_point = PricePoint { timestamp: candle.timestamp, price: candle.close, volume: candle.volume };

		self.price_history.push_back(price_point);
		self.last_update = candle.timestamp;

		// Keep last 20 minutes of price data (to support 15 min detection window + buffer)
		let cutoff = candle.timestamp - Duration::seconds(1200);
		while self.price_history.front().map_or(false, |p| p.timestamp < cutoff) {
			self.price_history.pop_front();
		}

		// Update volume history
		self.volume_history.push_back(candle.volume);
		if self.volume_history.len() > 60 {
			self.volume_history.pop_front();
		}

		// Update EMAs based on interval
		match candle.interval.as_str() {
			"1m" => self.ema_1m.update_from_candle(candle),
			"5m" => self.ema_5m.update_from_candle(candle),
			_ => {},
		}
	}

	/// Updates pivot levels from historical candles
	pub fn update_pivot_levels(&mut self, candles: &[Candle]) {
		if let Some(pivots) = PivotLevels::from_candles(candles) {
			self.pivot_levels = Some(pivots);
		}
	}

	/// Updates derivatives metrics
	pub fn update_derivatives(&mut self, metrics: DerivativesMetrics) {
		// Store baseline if not set (first fetch)
		if self.baseline_derivatives.is_none() {
			self.baseline_derivatives = Some(metrics.clone());
		}

		self.last_derivatives = Some(metrics);
	}

	/// Calculates price change over a time window
	pub fn price_change_in_window(&self, window_secs: u64) -> Option<PriceChange> {
		if self.price_history.len() < 2 {
			return None;
		}

		let now = self.last_update;
		let window_start = now - Duration::seconds(window_secs as i64);

		// Find first price point within window
		let start_point = self.price_history.iter().find(|p| p.timestamp >= window_start)?;

		let end_point = self.price_history.back()?;

		let price_change_pct = ((end_point.price - start_point.price) / start_point.price) * 100.0;
		let time_elapsed_mins = (end_point.timestamp - start_point.timestamp).num_seconds() / 60;

		Some(PriceChange {
			start_price: start_point.price,
			end_price: end_point.price,
			change_pct: price_change_pct,
			time_elapsed_mins: time_elapsed_mins as u64,
			start_time: start_point.timestamp,
			end_time: end_point.timestamp,
		})
	}

	/// Calculates average volume over history
	pub fn average_volume(&self) -> f64 {
		if self.volume_history.is_empty() {
			return 0.0;
		}

		let sum: f64 = self.volume_history.iter().sum();
		sum / self.volume_history.len() as f64
	}

	/// Gets current volume from recent candles
	pub fn current_volume(&self) -> f64 {
		// Sum volume from last 5 minutes
		let cutoff = self.last_update - Duration::seconds(300);
		self.price_history.iter().filter(|p| p.timestamp >= cutoff).map(|p| p.volume).sum()
	}

	/// Calculates OI increase percentage from baseline
	pub fn oi_increase_pct(&self) -> Option<f64> {
		let baseline = self.baseline_derivatives.as_ref()?;
		let current = self.last_derivatives.as_ref()?;

		if baseline.open_interest > 0.0 {
			let change_pct = ((current.open_interest - baseline.open_interest) / baseline.open_interest) * 100.0;
			Some(change_pct)
		} else {
			None
		}
	}

	/// Gets current funding rate
	pub fn funding_rate(&self) -> Option<f64> {
		self.last_derivatives.as_ref().map(|d| d.funding_rate)
	}

	/// Gets current long ratio
	pub fn long_ratio(&self) -> Option<f64> {
		self.last_derivatives.as_ref().and_then(|d| d.long_short_ratio.as_ref().map(|r| r.account_ratio()))
	}

	/// Checks if price is extended above key EMAs
	pub fn is_ema_extended(&self, price: f64, ema_periods: &[u32]) -> bool {
		self.ema_1m.price_above_emas(price, ema_periods) || self.ema_5m.price_above_emas(price, ema_periods)
	}

	/// Checks if price is near pivot resistance
	pub fn is_near_pivot_resistance(&self, price: f64, threshold_pct: f64) -> Option<String> {
		let pivots = self.pivot_levels.as_ref()?;

		if let Some(level) = pivots.is_near_resistance(price, threshold_pct) {
			Some(format!("Pivot {}", level))
		} else if pivots.is_extended_to_resistance(price) {
			Some("Above R1".to_string())
		} else {
			None
		}
	}

	/// Resets pump state to normal
	pub fn reset_pump_state(&mut self) {
		self.pump_state = PumpState::Normal;
	}

	/// Marks symbol as alerted
	pub fn mark_alerted(&mut self) {
		self.pump_state = PumpState::Alerted { alerted_at: Utc::now() };
		self.last_alert_time = Some(Utc::now());
	}

	/// Checks if symbol is in cooldown period
	pub fn is_in_cooldown(&self, cooldown_secs: u64) -> bool {
		if let Some(last_alert) = self.last_alert_time {
			let elapsed = (Utc::now() - last_alert).num_seconds();
			elapsed < cooldown_secs as i64
		} else {
			false
		}
	}

	/// Gets current price
	pub fn current_price(&self) -> Option<f64> {
		self.price_history.back().map(|p| p.price)
	}
}

#[derive(Debug, Clone)]
pub struct PriceChange {
	pub start_price: f64,
	pub end_price: f64,
	pub change_pct: f64,
	pub time_elapsed_mins: u64,
	pub start_time: DateTime<Utc>,
	pub end_time: DateTime<Utc>,
}

/// Manages all symbol trackers
pub struct TrackerManager {
	trackers: HashMap<Symbol, SymbolTracker>,
	ema_periods: Vec<u32>,
}

impl TrackerManager {
	pub fn new(ema_periods: Vec<u32>) -> Self {
		Self { trackers: HashMap::new(), ema_periods }
	}

	/// Gets or creates a tracker for a symbol
	pub fn get_or_create(&mut self, symbol: Symbol) -> &mut SymbolTracker {
		self.trackers.entry(symbol.clone()).or_insert_with(|| SymbolTracker::new(symbol, &self.ema_periods))
	}

	/// Gets a tracker for a symbol if it exists
	pub fn get(&self, symbol: &Symbol) -> Option<&SymbolTracker> {
		self.trackers.get(symbol)
	}

	/// Gets a mutable tracker for a symbol if it exists
	pub fn get_mut(&mut self, symbol: &Symbol) -> Option<&mut SymbolTracker> {
		self.trackers.get_mut(symbol)
	}

	/// Returns all trackers
	pub fn all(&self) -> impl Iterator<Item = &SymbolTracker> {
		self.trackers.values()
	}

	/// Returns all mutable trackers
	pub fn all_mut(&mut self) -> impl Iterator<Item = &mut SymbolTracker> {
		self.trackers.values_mut()
	}

	/// Removes stale trackers that haven't been updated recently
	pub fn cleanup_stale(&mut self, max_age_secs: u64) {
		let cutoff = Utc::now() - Duration::seconds(max_age_secs as i64);
		self.trackers.retain(|_, tracker| tracker.last_update > cutoff);
	}

	/// Returns count of tracked symbols
	pub fn count(&self) -> usize {
		self.trackers.len()
	}
}
