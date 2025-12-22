use super::detector::PumpCandidate;
use super::tracker::SymbolTracker;
use crate::config::{DerivativesConfig, TechnicalConfig};
use tracing::{debug, info};

/// Qualifies pump candidates based on derivatives and technical overheating
pub struct OverheatingQualifier {
	derivatives_config: DerivativesConfig,
	technical_config: TechnicalConfig,
}

impl OverheatingQualifier {
	pub const fn new(derivatives_config: DerivativesConfig, technical_config: TechnicalConfig) -> Self {
		Self { derivatives_config, technical_config }
	}

	/// Qualifies a pump candidate by checking overheating conditions
	pub fn qualify(&self, candidate: &PumpCandidate, tracker: &SymbolTracker) -> Option<QualificationResult> {
		let mut conditions_met = Vec::new();
		let mut conditions_failed = Vec::new();
		let mut score = 0;

		// Check derivatives conditions
		let derivatives_result = self.check_derivatives(tracker);
		for condition in &derivatives_result.conditions_met {
			conditions_met.push(condition.clone());
			score += 1;
		}
		for condition in &derivatives_result.conditions_failed {
			conditions_failed.push(condition.clone());
		}

		// Check technical conditions
		let technical_result = self.check_technical(candidate, tracker);
		for condition in &technical_result.conditions_met {
			conditions_met.push(condition.clone());
			score += 1;
		}
		for condition in &technical_result.conditions_failed {
			conditions_failed.push(condition.clone());
		}

		// Require at least 2 conditions to be met
		if score >= 2 {
			info!(
				symbol = %candidate.symbol,
				score = score,
				conditions = ?conditions_met,
				"Pump qualified as overheating"
			);

			Some(QualificationResult {
				qualified: true,
				score,
				conditions_met,
				conditions_failed,
				derivatives_details: derivatives_result,
				technical_details: technical_result,
			})
		} else {
			debug!(
				symbol = %candidate.symbol,
				score = score,
				conditions_met = ?conditions_met,
				conditions_failed = ?conditions_failed,
				"Pump not qualified - insufficient conditions"
			);

			None
		}
	}

	/// Checks derivatives overheating conditions
	fn check_derivatives(&self, tracker: &SymbolTracker) -> DerivativesResult {
		let mut conditions_met = Vec::new();
		let mut conditions_failed = Vec::new();

		// Check Open Interest increase
		if let Some(oi_increase) = tracker.oi_increase_pct() {
			if oi_increase >= self.derivatives_config.min_oi_increase_pct {
				conditions_met.push(format!("OI increased {oi_increase:.1}%"));
			} else {
				conditions_failed
					.push(format!("OI increase {:.1}% < {:.1}%", oi_increase, self.derivatives_config.min_oi_increase_pct));
			}
		} else {
			conditions_failed.push("OI data unavailable".to_string());
		}

		// Check Funding Rate
		if let Some(funding_rate) = tracker.funding_rate() {
			if funding_rate >= self.derivatives_config.min_funding_rate {
				conditions_met.push(format!("Funding rate {funding_rate:.4}"));
			} else {
				conditions_failed
					.push(format!("Funding {:.4} < {:.4}", funding_rate, self.derivatives_config.min_funding_rate));
			}
		} else {
			conditions_failed.push("Funding rate unavailable".to_string());
		}

		// Check Long/Short Ratio
		if let Some(long_ratio) = tracker.long_ratio() {
			if long_ratio >= self.derivatives_config.min_long_ratio {
				let long_pct = long_ratio * 100.0;
				let short_pct = (1.0 - long_ratio) * 100.0;
				conditions_met.push(format!("Long ratio {long_pct:.0}% / {short_pct:.0}%"));
			} else {
				conditions_failed.push(format!("Long ratio {:.2} < {:.2}", long_ratio, self.derivatives_config.min_long_ratio));
			}
		} else {
			conditions_failed.push("Long/Short ratio unavailable".to_string());
		}

		DerivativesResult {
			conditions_met,
			conditions_failed,
			oi_increase_pct: tracker.oi_increase_pct(),
			funding_rate: tracker.funding_rate(),
			long_ratio: tracker.long_ratio(),
		}
	}

	/// Checks technical overheating conditions
	fn check_technical(&self, candidate: &PumpCandidate, tracker: &SymbolTracker) -> TechnicalResult {
		let mut conditions_met = Vec::new();
		let mut conditions_failed = Vec::new();

		let current_price = candidate.current_price;

		// Check EMA extension (if enabled)
		if self.technical_config.ema_extension {
			let key_emas = [50u32, 200u32];
			let is_extended = tracker.is_ema_extended(current_price, &key_emas);

			if is_extended {
				// Get specific EMA values for context
				let mut ema_info = Vec::new();
				if let Some(ema50) = tracker.ema_1m.get(50) {
					let ext_pct = ((current_price - ema50) / ema50) * 100.0;
					ema_info.push(format!("EMA50: +{ext_pct:.1}%"));
				}
				if let Some(ema200) = tracker.ema_1m.get(200) {
					let ext_pct = ((current_price - ema200) / ema200) * 100.0;
					ema_info.push(format!("EMA200: +{ext_pct:.1}%"));
				}

				if !ema_info.is_empty() {
					conditions_met.push(format!("Price above {}", ema_info.join(", ")));
				}
			} else {
				conditions_failed.push("Price not extended above key EMAs".to_string());
			}
		}

		// Check pivot proximity (if enabled)
		if self.technical_config.pivot_proximity {
			if let Some(pivot_context) = tracker.is_near_pivot_resistance(current_price, 2.0) {
				conditions_met.push(format!("Near {pivot_context}"));
			} else {
				conditions_failed.push("Not near pivot resistance".to_string());
			}
		}

		// Check momentum slowing (price action patterns)
		let momentum_status = self.check_momentum(tracker);
		match &momentum_status {
			MomentumStatus::Slowing(reason) => {
				conditions_met.push(format!("Momentum slowing: {reason}"));
			},
			MomentumStatus::Strong => {
				conditions_failed.push("Momentum still strong".to_string());
			},
			MomentumStatus::Unknown => {
				// Neutral - don't count as pass or fail
			},
		}

		TechnicalResult {
			conditions_met,
			conditions_failed,
			ema_extended: tracker.is_ema_extended(current_price, &[50, 200]),
			near_pivot_resistance: tracker.is_near_pivot_resistance(current_price, 2.0),
			momentum_status,
		}
	}

	/// Checks if momentum is slowing based on recent price action
	fn check_momentum(&self, tracker: &SymbolTracker) -> MomentumStatus {
		// Check if recent price action shows deceleration
		let recent_change = tracker.price_change_in_window(60); // Last 1 minute
		let previous_change = tracker.price_change_in_window(180); // Last 3 minutes

		if let (Some(recent), Some(previous)) = (recent_change, previous_change) {
			// Calculate rate of change (price change per minute)
			let recent_rate = recent.change_pct / recent.time_elapsed_mins.max(1) as f64;
			let previous_rate = previous.change_pct / previous.time_elapsed_mins.max(1) as f64;

			// If recent rate is significantly slower than previous rate
			if previous_rate > 0.5 && recent_rate < previous_rate * 0.6 {
				return MomentumStatus::Slowing("deceleration detected".to_string());
			}

			// Check if price is making lower highs (rejection)
			if recent.change_pct < 0.0 {
				return MomentumStatus::Slowing("price rejected".to_string());
			}
		}

		// Check EMA momentum
		if let Some(current_price) = tracker.current_price() {
			// If price falling below fast EMAs, momentum is slowing
			if let Some(ema7) = tracker.ema_1m.get(7) {
				if current_price < ema7 {
					return MomentumStatus::Slowing("price below EMA7".to_string());
				}
			}
		}

		MomentumStatus::Unknown
	}
}

#[derive(Debug, Clone)]
pub struct QualificationResult {
	#[allow(dead_code)]
	pub qualified: bool,
	pub score: u32,
	#[allow(dead_code)]
	pub conditions_met: Vec<String>,
	#[allow(dead_code)]
	pub conditions_failed: Vec<String>,
	pub derivatives_details: DerivativesResult,
	pub technical_details: TechnicalResult,
}

#[derive(Debug, Clone)]
pub struct DerivativesResult {
	pub conditions_met: Vec<String>,
	pub conditions_failed: Vec<String>,
	pub oi_increase_pct: Option<f64>,
	pub funding_rate: Option<f64>,
	pub long_ratio: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TechnicalResult {
	pub conditions_met: Vec<String>,
	pub conditions_failed: Vec<String>,
	#[allow(dead_code)]
	pub ema_extended: bool,
	#[allow(dead_code)]
	pub near_pivot_resistance: Option<String>,
	pub momentum_status: MomentumStatus,
}

#[derive(Debug, Clone)]
pub enum MomentumStatus {
	#[allow(dead_code)]
	Strong,
	Slowing(String),
	Unknown,
}

impl QualificationResult {
	/// Returns a formatted summary of the qualification
	#[allow(dead_code)]
	pub fn summary(&self) -> String {
		format!(
			"Qualified with score {}/5. Met: [{}]. Failed: [{}]",
			self.score,
			self.conditions_met.join(", "),
			self.conditions_failed.join(", ")
		)
	}

	/// Returns derivatives context for alert message
	#[allow(dead_code)]
	pub fn derivatives_context(&self) -> String {
		let mut parts = Vec::new();

		if let Some(oi) = self.derivatives_details.oi_increase_pct {
			parts.push(format!("OI: +{oi:.1}%"));
		}

		if let Some(funding) = self.derivatives_details.funding_rate {
			parts.push(format!("Funding: {funding:.4}"));
		}

		if let Some(ratio) = self.derivatives_details.long_ratio {
			let long_pct = ratio * 100.0;
			let short_pct = (1.0 - ratio) * 100.0;
			parts.push(format!("L/S: {long_pct:.0}% / {short_pct:.0}%"));
		}

		if parts.is_empty() {
			"N/A".to_string()
		} else {
			parts.join(", ")
		}
	}

	/// Returns technical context for alert message
	pub fn technical_context(&self) -> Vec<String> {
		let mut context = Vec::new();

		for condition in &self.technical_details.conditions_met {
			context.push(format!("• {condition}"));
		}

		// Add momentum status if slowing
		if let MomentumStatus::Slowing(reason) = &self.technical_details.momentum_status {
			context.push(format!("• Momentum slowing: {reason}"));
		}

		context
	}
}
