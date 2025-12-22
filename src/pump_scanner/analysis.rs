use super::detector::PumpCandidate;
use super::tracker::SymbolTracker;
use crate::config::{DerivativesConfig, TechnicalConfig};

#[derive(Debug, Clone)]
pub struct SignalAnalysis {
	pub open_interest: OpenInterestSignal,
	pub funding_rate: FundingRateSignal,
	pub long_short_ratio: LongShortSignal,
	pub volume: VolumeSignal,
	pub ema_status: EmaSignal,
	pub pivot_status: PivotSignal,
	pub total_score: u32,
}

#[derive(Debug, Clone)]
pub struct OpenInterestSignal {
	pub value: Option<f64>,
	pub increase_pct: Option<f64>,
	pub is_overheated: bool,
}

#[derive(Debug, Clone)]
pub struct FundingRateSignal {
	pub value: Option<f64>,
	pub is_overheated: bool,
}

#[derive(Debug, Clone)]
pub struct LongShortSignal {
	pub long_pct: Option<f64>,
	pub short_pct: Option<f64>,
	pub is_overheated: bool,
}

#[derive(Debug, Clone)]
pub struct VolumeSignal {
	pub ratio: f64,
	pub is_significant: bool,
}

#[derive(Debug, Clone)]
pub struct EmaSignal {
	pub ema50_distance: Option<f64>,
	pub ema200_distance: Option<f64>,
	pub is_extended: bool,
}

#[derive(Debug, Clone)]
pub struct PivotSignal {
	pub level: Option<String>,
	pub is_near_resistance: bool,
}

impl SignalAnalysis {
	pub fn analyze(
		candidate: &PumpCandidate,
		tracker: &SymbolTracker,
		derivatives_config: &DerivativesConfig,
		technical_config: &TechnicalConfig,
	) -> Self {
		let mut total_score = 0;

		let open_interest = Self::analyze_open_interest(tracker, derivatives_config);
		if open_interest.is_overheated {
			total_score += 1;
		}

		let funding_rate = Self::analyze_funding_rate(tracker, derivatives_config);
		if funding_rate.is_overheated {
			total_score += 1;
		}

		let long_short_ratio = Self::analyze_long_short_ratio(tracker, derivatives_config);
		if long_short_ratio.is_overheated {
			total_score += 1;
		}

		let volume = Self::analyze_volume(candidate);
		if volume.is_significant {
			total_score += 1;
		}

		let ema_status = Self::analyze_ema(candidate, tracker, technical_config);
		if ema_status.is_extended {
			total_score += 1;
		}

		let pivot_status = Self::analyze_pivot(candidate, tracker, technical_config);
		if pivot_status.is_near_resistance {
			total_score += 1;
		}

		Self { open_interest, funding_rate, long_short_ratio, volume, ema_status, pivot_status, total_score }
	}

	fn analyze_open_interest(tracker: &SymbolTracker, config: &DerivativesConfig) -> OpenInterestSignal {
		let value = tracker.last_derivatives.as_ref().map(|d| d.open_interest);
		let increase_pct = tracker.oi_increase_pct();

		let is_overheated = increase_pct.is_some_and(|pct| pct >= config.min_oi_increase_pct);

		OpenInterestSignal { value, increase_pct, is_overheated }
	}

	fn analyze_funding_rate(tracker: &SymbolTracker, config: &DerivativesConfig) -> FundingRateSignal {
		let value = tracker.funding_rate();
		let is_overheated = value.is_some_and(|rate| rate >= config.min_funding_rate);

		FundingRateSignal { value, is_overheated }
	}

	fn analyze_long_short_ratio(tracker: &SymbolTracker, config: &DerivativesConfig) -> LongShortSignal {
		let long_ratio = tracker.long_ratio();

		let (long_pct, short_pct) =
			long_ratio.map_or((None, None), |ratio| (Some(ratio * 100.0), Some((1.0 - ratio) * 100.0)));

		let is_overheated = long_ratio.is_some_and(|ratio| ratio >= config.min_long_ratio);

		LongShortSignal { long_pct, short_pct, is_overheated }
	}

	fn analyze_volume(candidate: &PumpCandidate) -> VolumeSignal {
		let ratio = candidate.volume_ratio;
		let is_significant = ratio >= 2.0;

		VolumeSignal { ratio, is_significant }
	}

	fn analyze_ema(candidate: &PumpCandidate, tracker: &SymbolTracker, config: &TechnicalConfig) -> EmaSignal {
		let current_price = candidate.current_price;

		let ema50_distance = tracker.ema_1m.get(50).map(|ema| ((current_price - ema) / ema) * 100.0);

		let ema200_distance = tracker.ema_1m.get(200).map(|ema| ((current_price - ema) / ema) * 100.0);

		let is_extended = if config.ema_extension { tracker.is_ema_extended(current_price, &[50, 200]) } else { false };

		EmaSignal { ema50_distance, ema200_distance, is_extended }
	}

	fn analyze_pivot(candidate: &PumpCandidate, tracker: &SymbolTracker, config: &TechnicalConfig) -> PivotSignal {
		let current_price = candidate.current_price;

		let level = if config.pivot_proximity { tracker.is_near_pivot_resistance(current_price, 2.0) } else { None };

		let is_near_resistance = level.is_some();

		PivotSignal { level, is_near_resistance }
	}
}
