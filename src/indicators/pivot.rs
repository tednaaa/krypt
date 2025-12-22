use crate::exchange::Candle;

#[derive(Debug, Clone)]
pub struct PivotLevels {
	#[allow(dead_code)]
	pub pivot: f64,
	pub resistance1: f64,
	pub resistance2: f64,
	pub resistance3: f64,
	#[allow(dead_code)]
	pub support1: f64,
	#[allow(dead_code)]
	pub support2: f64,
	#[allow(dead_code)]
	pub support3: f64,
	#[allow(dead_code)]
	pub calculated_at: chrono::DateTime<chrono::Utc>,
}

impl PivotLevels {
	pub fn from_candle(candle: &Candle) -> Self {
		let high = candle.high;
		let low = candle.low;
		let close = candle.close;

		let pivot = (high + low + close) / 3.0;

		let resistance1 = 2.0f64.mul_add(pivot, -low);
		let resistance2 = pivot + (high - low);
		let resistance3 = 2.0f64.mul_add(pivot - low, high);

		let support1 = 2.0f64.mul_add(pivot, -high);
		let support2 = pivot - (high - low);
		let support3 = 2.0f64.mul_add(-(high - pivot), low);

		Self { pivot, resistance1, resistance2, resistance3, support1, support2, support3, calculated_at: candle.timestamp }
	}

	pub fn from_candles(candles: &[Candle]) -> Option<Self> {
		if candles.is_empty() {
			return None;
		}

		let candle = candles.last()?;
		Some(Self::from_candle(candle))
	}

	#[allow(dead_code)]
	pub fn from_hlc(high: f64, low: f64, close: f64) -> Self {
		let pivot = (high + low + close) / 3.0;

		let resistance1 = 2.0f64.mul_add(pivot, -low);
		let resistance2 = pivot + (high - low);
		let resistance3 = 2.0f64.mul_add(pivot - low, high);

		let support1 = 2.0f64.mul_add(pivot, -high);
		let support2 = pivot - (high - low);
		let support3 = 2.0f64.mul_add(-(high - pivot), low);

		Self {
			pivot,
			resistance1,
			resistance2,
			resistance3,
			support1,
			support2,
			support3,
			calculated_at: chrono::Utc::now(),
		}
	}

	pub fn is_near_resistance(&self, price: f64, threshold_pct: f64) -> Option<ResistanceLevel> {
		let levels = [
			(ResistanceLevel::R1, self.resistance1),
			(ResistanceLevel::R2, self.resistance2),
			(ResistanceLevel::R3, self.resistance3),
		];

		for (level, level_price) in levels {
			// Only consider prices that are approaching resistance from below
			// Price must be within threshold_pct of the resistance level
			// and must be at or above (level_price - threshold_pct)
			let distance_pct = ((level_price - price) / level_price) * 100.0;
			if distance_pct >= 0.0 && distance_pct < threshold_pct {
				return Some(level);
			}
		}

		None
	}

	#[allow(dead_code)]
	pub fn is_near_support(&self, price: f64, threshold_pct: f64) -> Option<SupportLevel> {
		let levels =
			[(SupportLevel::S1, self.support1), (SupportLevel::S2, self.support2), (SupportLevel::S3, self.support3)];

		for (level, level_price) in levels {
			// Only consider prices that are approaching support from above
			// Price must be within threshold_pct of the support level
			// and must be at or below (level_price + threshold_pct)
			let distance_pct = ((price - level_price) / level_price) * 100.0;
			if distance_pct >= 0.0 && distance_pct < threshold_pct {
				return Some(level);
			}
		}

		None
	}

	#[allow(dead_code)]
	pub fn is_near_pivot(&self, price: f64, threshold_pct: f64) -> bool {
		let distance_pct = ((price - self.pivot).abs() / self.pivot) * 100.0;
		distance_pct <= threshold_pct
	}

	#[allow(dead_code)]
	pub fn distance_to_resistance(&self, price: f64) -> Option<(ResistanceLevel, f64)> {
		let levels = [
			(ResistanceLevel::R1, self.resistance1),
			(ResistanceLevel::R2, self.resistance2),
			(ResistanceLevel::R3, self.resistance3),
		];

		levels
			.iter()
			.filter(|(_, level_price)| price <= *level_price)
			.map(|(level, level_price)| {
				let distance_pct = ((level_price - price) / price) * 100.0;
				(*level, distance_pct)
			})
			.min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
	}

	#[allow(dead_code)]
	pub fn distance_to_support(&self, price: f64) -> Option<(SupportLevel, f64)> {
		let levels =
			[(SupportLevel::S1, self.support1), (SupportLevel::S2, self.support2), (SupportLevel::S3, self.support3)];

		levels
			.iter()
			.filter(|(_, level_price)| price >= *level_price)
			.map(|(level, level_price)| {
				let distance_pct = ((price - level_price) / price) * 100.0;
				(*level, distance_pct)
			})
			.min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
	}

	pub fn is_extended_to_resistance(&self, price: f64) -> bool {
		price >= self.resistance1
	}

	#[allow(dead_code)]
	pub fn is_extended_to_support(&self, price: f64) -> bool {
		price <= self.support1
	}

	#[allow(dead_code)]
	pub fn resistance_levels(&self) -> Vec<f64> {
		vec![self.resistance1, self.resistance2, self.resistance3]
	}

	#[allow(dead_code)]
	pub fn support_levels(&self) -> Vec<f64> {
		vec![self.support1, self.support2, self.support3]
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResistanceLevel {
	R1,
	R2,
	R3,
}

impl std::fmt::Display for ResistanceLevel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::R1 => write!(f, "R1"),
			Self::R2 => write!(f, "R2"),
			Self::R3 => write!(f, "R3"),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SupportLevel {
	S1,
	S2,
	S3,
}

impl std::fmt::Display for SupportLevel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::S1 => write!(f, "S1"),
			Self::S2 => write!(f, "S2"),
			Self::S3 => write!(f, "S3"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::exchange::Symbol;
	use chrono::Utc;

	fn create_test_candle(high: f64, low: f64, close: f64) -> Candle {
		Candle {
			symbol: Symbol::new("BTC", "USDT", "binance"),
			timestamp: Utc::now(),
			open: close,
			high,
			low,
			close,
			volume: 1000.0,
			interval: "1h".to_string(),
		}
	}

	#[test]
	fn test_pivot_calculation() {
		let candle = create_test_candle(50000.0, 48000.0, 49000.0);
		let pivots = PivotLevels::from_candle(&candle);

		// P = (50000 + 48000 + 49000) / 3 = 49000
		assert!((pivots.pivot - 49000.0).abs() < 1e-10);

		// R1 = 2 * P - L = 2 * 49000 - 48000 = 50000
		assert!((pivots.resistance1 - 50000.0).abs() < 1e-10);

		// S1 = 2 * P - H = 2 * 49000 - 50000 = 48000
		assert!((pivots.support1 - 48000.0).abs() < 1e-10);

		// R2 = P + (H - L) = 49000 + 2000 = 51000
		assert!((pivots.resistance2 - 51000.0).abs() < 1e-10);

		// S2 = P - (H - L) = 49000 - 2000 = 47000
		assert!((pivots.support2 - 47000.0).abs() < 1e-10);
	}

	#[test]
	fn test_near_resistance() {
		let pivots = PivotLevels::from_hlc(50000.0, 48000.0, 49000.0);

		// Price near R1 (50000)
		assert_eq!(pivots.is_near_resistance(49950.0, 1.0), Some(ResistanceLevel::R1));

		// Price not near any resistance
		assert_eq!(pivots.is_near_resistance(49500.0, 1.0), None);
	}

	#[test]
	fn test_near_support() {
		let pivots = PivotLevels::from_hlc(50000.0, 48000.0, 49000.0);

		// Price near S1 (48000)
		assert_eq!(pivots.is_near_support(48050.0, 1.0), Some(SupportLevel::S1));

		// Price not near any support
		assert_eq!(pivots.is_near_support(49500.0, 1.0), None);
	}

	#[test]
	fn test_extended_to_resistance() {
		let pivots = PivotLevels::from_hlc(50000.0, 48000.0, 49000.0);

		assert!(pivots.is_extended_to_resistance(50100.0));
		assert!(pivots.is_extended_to_resistance(50000.0));
		assert!(!pivots.is_extended_to_resistance(49900.0));
	}

	#[test]
	fn test_distance_to_resistance() {
		let pivots = PivotLevels::from_hlc(50000.0, 48000.0, 49000.0);

		let (level, distance) = pivots.distance_to_resistance(49000.0).unwrap();
		assert_eq!(level, ResistanceLevel::R1);
		assert!((distance - 2.04).abs() < 0.1); // ~2% to R1
	}
}
