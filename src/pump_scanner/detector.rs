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
	pub const fn new(config: PumpConfig) -> Self {
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
						volume_ratio: self.calculate_volume_ratio(tracker, window_secs),
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

		// Check volume spike (optional - pumps can happen without volume spike, especially for shitcoins)
		let volume_ratio = self.calculate_volume_ratio(tracker, price_change.time_elapsed_mins * 60);

		// If volume multiplier is set to 0 or very low, skip volume check
		if self.config.volume_multiplier > 0.5 && volume_ratio < self.config.volume_multiplier {
			// For shitcoins and low liquidity, allow pumps with lower volume if price change is significant
			let is_significant_pump = price_change.change_pct >= self.config.price_threshold_pct * 1.5;
			let has_some_volume = volume_ratio >= 1.0;

			if is_significant_pump && has_some_volume {
				debug!(
					symbol = %tracker.symbol,
					volume_ratio = volume_ratio,
					price_change = price_change.change_pct,
					"Allowing pump with lower volume due to significant price change"
				);
			} else {
				debug!(
					symbol = %tracker.symbol,
					volume_ratio = volume_ratio,
					threshold = self.config.volume_multiplier,
					price_change = price_change.change_pct,
					"Volume spike insufficient and not a significant pump"
				);
				return false;
			}
		}

		true
	}

	/// Calculates volume ratio (current window vs baseline average)
	fn calculate_volume_ratio(&self, tracker: &SymbolTracker, window_secs: u64) -> f64 {
		tracker.volume_ratio_for_window(window_secs)
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
	#[allow(dead_code)]
	pub fn summary(&self) -> String {
		format!(
			"{} pumped {:.2}% in {}m with {:.1}x volume",
			self.symbol, self.price_change.change_pct, self.price_change.time_elapsed_mins, self.volume_ratio
		)
	}
}

