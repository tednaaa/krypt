use super::tracker::{PriceChange, PumpState, SymbolTracker};
use crate::config::PumpConfig;
use crate::exchange::Symbol;
use chrono::Utc;
use tracing::{debug, info};

pub struct PumpDetector {
	config: PumpConfig,
}

impl PumpDetector {
	pub const fn new(config: PumpConfig) -> Self {
		Self { config }
	}

	pub fn analyze(&self, tracker: &mut SymbolTracker) -> Option<PumpCandidate> {
		match tracker.pump_state {
			PumpState::Candidate { .. } | PumpState::Alerted { .. } => return None,
			PumpState::Normal => {},
		}

		if tracker.price_history.len() < 10 {
			return None;
		}

		let current_price = tracker.current_price()?;

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

	fn is_pump_trigger(&self, price_change: &PriceChange, tracker: &SymbolTracker) -> bool {
		if price_change.change_pct < self.config.price_threshold_pct {
			debug!(
				symbol = %tracker.symbol,
				change_pct = price_change.change_pct,
				threshold = self.config.price_threshold_pct,
				"Price change below threshold"
			);
			return false;
		}

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

	fn calculate_volume_ratio(&self, tracker: &SymbolTracker, window_secs: u64) -> f64 {
		tracker.volume_ratio_for_window(window_secs)
	}

	pub fn update_candidate(&self, tracker: &mut SymbolTracker) {
		if let PumpState::Candidate { detected_at, entry_price, max_price, total_volume } = tracker.pump_state {
			if let Some(current_price) = tracker.current_price() {
				let new_max = max_price.max(current_price);

				// TODO: move to fn
				// Check if pump has faded (price dropped significantly from max)
				let drop_from_max_pct = ((new_max - current_price) / new_max) * 100.0;

				// TODO: move to fn
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
