use crate::config::ScoringConfig;
use crate::types::{SymbolData, Tier};

pub struct Scorer {
	config: ScoringConfig,
}

impl Scorer {
	pub fn new(config: ScoringConfig) -> Self {
		Self { config }
	}

	pub fn calculate_score(&self, symbol: &SymbolData) -> f64 {
		let volume_score = self.calculate_volume_score(symbol.quote_volume_24h);
		let volatility_score = self.calculate_volatility_score(symbol.price_change_pct_24h);
		let activity_score = self.calculate_activity_score(symbol.trades_24h);

		let weights = &self.config.weights;
		let total_score = weights.volume_weight * volume_score
			+ weights.volatility_weight * volatility_score
			+ weights.activity_weight * activity_score;

		total_score.clamp(0.0, 1.0)
	}

	fn calculate_volume_score(&self, quote_volume: f64) -> f64 {
		if quote_volume <= 0.0 {
			return 0.0;
		}

		// Logarithmic scale for volume: log10(volume_in_millions)
		let volume_millions = quote_volume / 1_000_000.0;
		let log_score = volume_millions.log10();

		// Normalize: 10M = 1.0, 100M = 2.0, 1B = 3.0
		// Map to 0-1 range (assuming 1B is near perfect)
		(log_score / 3.0).clamp(0.0, 1.0)
	}

	fn calculate_volatility_score(&self, price_change_pct: f64) -> f64 {
		// Higher volatility = higher score
		// 10% change = 1.0 score
		let abs_change = price_change_pct.abs();
		(abs_change / 10.0).clamp(0.0, 1.0)
	}

	fn calculate_activity_score(&self, trades_24h: u64) -> f64 {
		// More trades = higher score
		// 10,000 trades = 1.0 score
		let score = trades_24h as f64 / 10_000.0;
		score.clamp(0.0, 1.0)
	}

	pub fn assign_tier(&self, score: f64) -> Tier {
		if score >= self.config.tier1_threshold {
			Tier::Tier1
		} else if score >= self.config.tier2_threshold {
			Tier::Tier2
		} else {
			Tier::Ignored
		}
	}

	pub fn select_tier1_symbols(&self, symbols: &mut [SymbolData]) -> Vec<String> {
		// Sort by score descending
		symbols.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

		// Take top N symbols that meet tier1 threshold
		symbols
			.iter()
			.filter(|s| s.score >= self.config.tier1_threshold)
			.take(self.config.max_tier1_symbols)
			.map(|s| s.symbol.clone())
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::ScoringWeights;

	fn test_config() -> ScoringConfig {
		ScoringConfig {
			tier1_threshold: 0.7,
			tier2_threshold: 0.4,
			max_tier1_symbols: 20,
			rescore_interval_secs: 10,
			weights: ScoringWeights { volume_weight: 0.4, volatility_weight: 0.4, activity_weight: 0.2 },
		}
	}

	#[test]
	fn test_volume_score() {
		let scorer = Scorer::new(test_config());

		// 10M should give ~0.33 score
		assert!(scorer.calculate_volume_score(10_000_000.0) > 0.3);
		assert!(scorer.calculate_volume_score(10_000_000.0) < 0.4);

		// 100M should give ~0.66 score
		assert!(scorer.calculate_volume_score(100_000_000.0) > 0.6);
		assert!(scorer.calculate_volume_score(100_000_000.0) < 0.7);

		// 1B should give 1.0 score
		assert_eq!(scorer.calculate_volume_score(1_000_000_000.0), 1.0);
	}

	#[test]
	fn test_volatility_score() {
		let scorer = Scorer::new(test_config());

		// 5% change should give 0.5 score
		assert_eq!(scorer.calculate_volatility_score(5.0), 0.5);

		// 10% change should give 1.0 score
		assert_eq!(scorer.calculate_volatility_score(10.0), 1.0);

		// Negative changes should use absolute value
		assert_eq!(scorer.calculate_volatility_score(-5.0), 0.5);
	}

	#[test]
	fn test_activity_score() {
		let scorer = Scorer::new(test_config());

		// 5,000 trades should give 0.5 score
		assert_eq!(scorer.calculate_activity_score(5000), 0.5);

		// 10,000 trades should give 1.0 score
		assert_eq!(scorer.calculate_activity_score(10000), 1.0);
	}

	#[test]
	fn test_tier_assignment() {
		let scorer = Scorer::new(test_config());

		assert_eq!(scorer.assign_tier(0.8), Tier::Tier1);
		assert_eq!(scorer.assign_tier(0.7), Tier::Tier1);
		assert_eq!(scorer.assign_tier(0.5), Tier::Tier2);
		assert_eq!(scorer.assign_tier(0.4), Tier::Tier2);
		assert_eq!(scorer.assign_tier(0.3), Tier::Ignored);
	}
}
