use super::tracker::{PriceChange, PumpState, SymbolTracker};
use crate::config::PumpConfig;
use crate::exchange::Symbol;
use chrono::Utc;
use tracing::{debug, info};

/// Detects pump events based on price movement and volume
pub struct PumpDetector {
	config: PumpConfig,
}

impl PumpDetector {
	pub fn new(config: PumpConfig) -> Self {
		Self { config }
	}

	/// Analyzes a symbol to detect pump candidates
	pub fn analyze(&self, tracker: &mut SymbolTracker) -> Option<PumpCandidate> {
		// Skip if already in pump state or recently alerted
		match tracker.pump_state {
			PumpState::Candidate { .. } | PumpState::Alerted { .. } => return None,
			PumpState::Normal => {},
		}

		// Need enough price history
		if tracker.price_history.len() < 10 {
			return None;
		}

		let current_price = tracker.current_price()?;

		// Check for pump in different time windows
		for window_mins in self.config.min_window_mins..=self.config.max_window_mins {
			let window_secs = window_mins * 60;

			if let Some(price_change) = tracker.price_change_in_window(window_secs) {
				if self.is_pump_trigger(&price_change, tracker) {
					info!(
						symbol = %tracker.symbol,
						change_pct = price_change.change_pct,
						window_mins = price_change.time_elapsed_mins,
						"Pump candidate detected"
					);

					// Update pump state
					tracker.pump_state = PumpState::Candidate {
						detected_at: Utc::now(),
						entry_price: price_change.start_price,
						max_price: current_price,
						total_volume: tracker.current_volume(),
					};

					return Some(PumpCandidate {
						symbol: tracker.symbol.clone(),
						price_change,
						volume_ratio: self.calculate_volume_ratio(tracker),
						current_price,
					});
				}
			}
		}

		None
	}

	/// Checks if price change qualifies as a pump trigger
	fn is_pump_trigger(&self, price_change: &PriceChange, tracker: &SymbolTracker) -> bool {
		// Check price threshold
		if price_change.change_pct < self.config.price_threshold_pct {
			debug!(
				symbol = %tracker.symbol,
				change_pct = price_change.change_pct,
				threshold = self.config.price_threshold_pct,
				"Price change below threshold"
			);
			return false;
		}

		// Check time window
		if price_change.time_elapsed_mins < self.config.min_window_mins
			|| price_change.time_elapsed_mins > self.config.max_window_mins
		{
			debug!(
				symbol = %tracker.symbol,
				elapsed_mins = price_change.time_elapsed_mins,
				"Time window outside range"
			);
			return false;
		}

		// Check volume spike
		let volume_ratio = self.calculate_volume_ratio(tracker);
		if volume_ratio < self.config.volume_multiplier {
			debug!(
				symbol = %tracker.symbol,
				volume_ratio = volume_ratio,
				threshold = self.config.volume_multiplier,
				"Volume spike insufficient"
			);
			return false;
		}

		true
	}

	/// Calculates volume ratio (current vs average)
	fn calculate_volume_ratio(&self, tracker: &SymbolTracker) -> f64 {
		let avg_volume = tracker.average_volume();
		if avg_volume > 0.0 {
			tracker.current_volume() / avg_volume
		} else {
			0.0
		}
	}

	/// Updates pump candidate state if still active
	pub fn update_candidate(&self, tracker: &mut SymbolTracker) {
		if let PumpState::Candidate { detected_at, entry_price, max_price, total_volume } = tracker.pump_state {
			if let Some(current_price) = tracker.current_price() {
				// Update max price if new high
				let new_max = max_price.max(current_price);

				// Check if pump has faded (price dropped significantly from max)
				let drop_from_max_pct = ((new_max - current_price) / new_max) * 100.0;

				// Reset if pump has faded or too much time has passed
				let elapsed_mins = (Utc::now() - detected_at).num_minutes();
				if drop_from_max_pct > 3.0 || elapsed_mins > 30 {
					debug!(
						symbol = %tracker.symbol,
						drop_pct = drop_from_max_pct,
						elapsed_mins = elapsed_mins,
						"Pump candidate expired"
					);
					tracker.reset_pump_state();
				} else {
					// Update state with new max
					tracker.pump_state = PumpState::Candidate {
						detected_at,
						entry_price,
						max_price: new_max,
						total_volume: total_volume + tracker.current_volume(),
					};
				}
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct PumpCandidate {
	pub symbol: Symbol,
	pub price_change: PriceChange,
	pub volume_ratio: f64,
	pub current_price: f64,
}

impl PumpCandidate {
	/// Returns a human-readable summary
	pub fn summary(&self) -> String {
		format!(
			"{} pumped {:.2}% in {}m with {:.1}x volume",
			self.symbol, self.price_change.change_pct, self.price_change.time_elapsed_mins, self.volume_ratio
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::exchange::{Candle, Symbol};
	use chrono::{Duration, Utc};

	fn create_test_config() -> PumpConfig {
		PumpConfig { price_threshold_pct: 5.0, min_window_mins: 5, max_window_mins: 15, volume_multiplier: 2.5 }
	}

	fn create_test_candle(symbol: Symbol, price: f64, volume: f64, timestamp: chrono::DateTime<Utc>) -> Candle {
		Candle { symbol, timestamp, open: price, high: price, low: price, close: price, volume, interval: "1m".to_string() }
	}

	#[test]
	fn test_pump_detection() {
		let config = create_test_config();
		let detector = PumpDetector::new(config);
		let symbol = Symbol::new("BTC", "USDT", "binance");
		let mut tracker = SymbolTracker::new(symbol.clone(), &[7, 14, 28, 50, 200]);

		// Add baseline volume
		let base_time = Utc::now() - Duration::minutes(20);
		for i in 0..10 {
			let candle = create_test_candle(symbol.clone(), 50000.0, 1000.0, base_time + Duration::minutes(i));
			tracker.update_from_candle(&candle);
		}

		// Add pump movement: 5% increase in 10 minutes with 3x volume
		let pump_start = Utc::now() - Duration::minutes(10);
		for i in 0..10 {
			let price = 50000.0 + (i as f64 * 250.0); // 5% increase over 10 candles
			let volume = 3000.0; // 3x volume
			let candle = create_test_candle(symbol.clone(), price, volume, pump_start + Duration::minutes(i));
			tracker.update_from_candle(&candle);
		}

		// Should detect pump
		let candidate = detector.analyze(&mut tracker);
		assert!(candidate.is_some());

		let candidate = candidate.unwrap();
		assert!(candidate.price_change.change_pct >= 5.0);
		assert!(candidate.volume_ratio >= 2.5);
	}

	#[test]
	fn test_no_pump_insufficient_price_change() {
		let config = create_test_config();
		let detector = PumpDetector::new(config);
		let symbol = Symbol::new("BTC", "USDT", "binance");
		let mut tracker = SymbolTracker::new(symbol.clone(), &[7, 14, 28, 50, 200]);

		// Add candles with only 2% increase (below threshold)
		let base_time = Utc::now() - Duration::minutes(10);
		for i in 0..10 {
			let price = 50000.0 + (i as f64 * 100.0); // 2% increase
			let candle = create_test_candle(symbol.clone(), price, 3000.0, base_time + Duration::minutes(i));
			tracker.update_from_candle(&candle);
		}

		// Should NOT detect pump
		let candidate = detector.analyze(&mut tracker);
		assert!(candidate.is_none());
	}

	#[test]
	fn test_no_pump_insufficient_volume() {
		let config = create_test_config();
		let detector = PumpDetector::new(config);
		let symbol = Symbol::new("BTC", "USDT", "binance");
		let mut tracker = SymbolTracker::new(symbol.clone(), &[7, 14, 28, 50, 200]);

		// Add baseline volume
		let base_time = Utc::now() - Duration::minutes(20);
		for i in 0..10 {
			let candle = create_test_candle(symbol.clone(), 50000.0, 1000.0, base_time + Duration::minutes(i));
			tracker.update_from_candle(&candle);
		}

		// Add pump movement but with low volume
		let pump_start = Utc::now() - Duration::minutes(10);
		for i in 0..10 {
			let price = 50000.0 + (i as f64 * 250.0); // 5% increase
			let volume = 1000.0; // Same volume (no spike)
			let candle = create_test_candle(symbol.clone(), price, volume, pump_start + Duration::minutes(i));
			tracker.update_from_candle(&candle);
		}

		// Should NOT detect pump
		let candidate = detector.analyze(&mut tracker);
		assert!(candidate.is_none());
	}
}
