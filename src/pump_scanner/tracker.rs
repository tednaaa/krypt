use crate::exchange::{Candle, DerivativesMetrics, Symbol};
use crate::indicators::{MultiEma, PivotLevels};
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct SymbolTracker {
	pub symbol: Symbol,
	pub price_history: VecDeque<PricePoint>,
	pub volume_history: VecDeque<f64>,
	pub ema_1m: MultiEma,
	pub ema_5m: MultiEma,
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
			price_history: VecDeque::with_capacity(1200),
			volume_history: VecDeque::with_capacity(60),
			ema_1m: MultiEma::new(ema_periods),
			ema_5m: MultiEma::new(ema_periods),
			pivot_levels: None,
			last_derivatives: None,
			baseline_derivatives: None,
			pump_state: PumpState::Normal,
			last_alert_time: None,
			last_update: Utc::now(),
		}
	}

	pub fn update_from_price(&mut self, price: f64, timestamp: DateTime<Utc>) {
		let price_point = PricePoint { timestamp, price, volume: 0.0 };

		self.price_history.push_back(price_point);
		self.last_update = timestamp;

		let cutoff = timestamp - Duration::seconds(1200);
		while self.price_history.front().is_some_and(|p| p.timestamp < cutoff) {
			self.price_history.pop_front();
		}
	}

	pub fn update_pivot_levels(&mut self, candles: &[Candle]) {
		if let Some(pivots) = PivotLevels::from_candles(candles) {
			self.pivot_levels = Some(pivots);
		}
	}

	pub fn update_derivatives(&mut self, metrics: DerivativesMetrics) {
		if self.baseline_derivatives.is_none() {
			self.baseline_derivatives = Some(metrics.clone());
		}

		self.last_derivatives = Some(metrics);
	}

	pub fn price_change_in_window(&self, window_secs: u64) -> Option<PriceChange> {
		if self.price_history.len() < 2 {
			return None;
		}

		let now = self.last_update;
		let window_start = now - Duration::seconds(i64::try_from(window_secs).unwrap_or(i64::MAX));

		let start_point = self.price_history.iter().find(|p| p.timestamp >= window_start)?;

		let end_point = self.price_history.back()?;

		let price_change_pct = ((end_point.price - start_point.price) / start_point.price) * 100.0;
		let time_elapsed_mins = (end_point.timestamp - start_point.timestamp).num_seconds() / 60;

		Some(PriceChange {
			start_price: start_point.price,
			change_pct: price_change_pct,
			time_elapsed_mins: u64::try_from(time_elapsed_mins).unwrap_or(0),
		})
	}

	pub fn average_volume(&self) -> f64 {
		if self.volume_history.is_empty() {
			return 0.0;
		}

		let sum: f64 = self.volume_history.iter().sum();
		sum / self.volume_history.len() as f64
	}

	pub fn baseline_average_volume(&self, exclude_last_mins: u64) -> f64 {
		if self.volume_history.is_empty() {
			return 0.0;
		}

		let exclude_count = exclude_last_mins.min(self.volume_history.len() as u64) as usize;

		if exclude_count >= self.volume_history.len() {
			return self.average_volume();
		}

		let baseline_volumes: Vec<f64> =
			self.volume_history.iter().take(self.volume_history.len() - exclude_count).copied().collect();

		if baseline_volumes.is_empty() {
			return self.average_volume();
		}

		let sum: f64 = baseline_volumes.iter().sum();
		sum / baseline_volumes.len() as f64
	}

	pub fn current_volume(&self) -> f64 {
		let cutoff = self.last_update - Duration::seconds(300);
		self.price_history.iter().filter(|p| p.timestamp >= cutoff).map(|p| p.volume).sum()
	}

	pub fn volume_in_window(&self, window_secs: u64) -> f64 {
		let cutoff = self.last_update - Duration::seconds(i64::try_from(window_secs).unwrap_or(i64::MAX));
		self.price_history.iter().filter(|p| p.timestamp >= cutoff).map(|p| p.volume).sum()
	}

	pub fn volume_ratio_for_window(&self, window_secs: u64) -> f64 {
		let window_volume = self.volume_in_window(window_secs);
		let window_mins = window_secs / 60;

		let baseline_avg_per_min = self.baseline_average_volume(window_mins);

		if baseline_avg_per_min > 0.0 {
			let expected_baseline = baseline_avg_per_min * window_mins as f64;
			window_volume / expected_baseline
		} else {
			0.0
		}
	}

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

	pub fn funding_rate(&self) -> Option<f64> {
		self.last_derivatives.as_ref().map(|d| d.funding_rate)
	}

	pub fn long_ratio(&self) -> Option<f64> {
		self
			.last_derivatives
			.as_ref()
			.and_then(|d| d.long_short_ratio.as_ref().map(super::super::exchange::types::LongShortRatio::account_ratio))
	}

	pub fn is_ema_extended(&self, price: f64, ema_periods: &[u32]) -> bool {
		self.ema_1m.price_above_emas(price, ema_periods) || self.ema_5m.price_above_emas(price, ema_periods)
	}

	pub fn is_near_pivot_resistance(&self, price: f64, threshold_pct: f64) -> Option<String> {
		let pivots = self.pivot_levels.as_ref()?;

		pivots.is_near_resistance(price, threshold_pct).map_or_else(
			|| {
				if pivots.is_extended_to_resistance(price) {
					Some("Above R1".to_string())
				} else {
					None
				}
			},
			|level| Some(format!("Pivot {level}")),
		)
	}

	pub const fn reset_pump_state(&mut self) {
		self.pump_state = PumpState::Normal;
	}

	pub fn mark_alerted(&mut self) {
		self.pump_state = PumpState::Alerted { alerted_at: Utc::now() };
		self.last_alert_time = Some(Utc::now());
	}

	pub fn is_in_cooldown(&self, cooldown_secs: u64) -> bool {
		self.last_alert_time.is_some_and(|last_alert| {
			let elapsed = (Utc::now() - last_alert).num_seconds();
			elapsed < i64::try_from(cooldown_secs).unwrap_or(i64::MAX)
		})
	}

	pub fn current_price(&self) -> Option<f64> {
		self.price_history.back().map(|p| p.price)
	}
}

#[derive(Debug, Clone)]
pub struct PriceChange {
	pub start_price: f64,
	pub change_pct: f64,
	pub time_elapsed_mins: u64,
}

pub struct TrackerManager {
	trackers: HashMap<Symbol, SymbolTracker>,
	ema_periods: Vec<u32>,
}

impl TrackerManager {
	pub fn new(ema_periods: Vec<u32>) -> Self {
		Self { trackers: HashMap::new(), ema_periods }
	}

	pub fn get_or_create(&mut self, symbol: Symbol) -> &mut SymbolTracker {
		self.trackers.entry(symbol.clone()).or_insert_with(|| SymbolTracker::new(symbol, &self.ema_periods))
	}

	pub fn get_mut(&mut self, symbol: &Symbol) -> Option<&mut SymbolTracker> {
		self.trackers.get_mut(symbol)
	}

	pub fn cleanup_stale(&mut self, max_age_secs: u64) {
		let cutoff = Utc::now() - Duration::seconds(i64::try_from(max_age_secs).unwrap_or(i64::MAX));
		self.trackers.retain(|_, tracker| tracker.last_update > cutoff);
	}

	pub fn count(&self) -> usize {
		self.trackers.len()
	}
}
