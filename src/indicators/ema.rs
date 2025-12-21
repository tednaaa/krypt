use crate::exchange::Candle;
use std::collections::VecDeque;

/// Exponential Moving Average calculator
#[derive(Debug, Clone)]
pub struct EMA {
	period: usize,
	multiplier: f64,
	current_value: Option<f64>,
	is_initialized: bool,
	price_buffer: VecDeque<f64>,
}

impl EMA {
	/// Creates a new EMA calculator with the given period
	pub fn new(period: usize) -> Self {
		let multiplier = 2.0 / (period as f64 + 1.0);
		Self {
			period,
			multiplier,
			current_value: None,
			is_initialized: false,
			price_buffer: VecDeque::with_capacity(period),
		}
	}

	/// Updates the EMA with a new price
	pub fn update(&mut self, price: f64) -> Option<f64> {
		if !self.is_initialized {
			self.price_buffer.push_back(price);

			if self.price_buffer.len() >= self.period {
				// Calculate initial SMA
				let sum: f64 = self.price_buffer.iter().sum();
				let sma = sum / self.period as f64;
				self.current_value = Some(sma);
				self.is_initialized = true;
				return Some(sma);
			}

			None
		} else {
			// EMA formula: EMA = (Price - EMA_prev) * multiplier + EMA_prev
			if let Some(prev_ema) = self.current_value {
				let ema = (price - prev_ema) * self.multiplier + prev_ema;
				self.current_value = Some(ema);
				Some(ema)
			} else {
				None
			}
		}
	}

	/// Returns the current EMA value
	pub fn value(&self) -> Option<f64> {
		self.current_value
	}

	/// Returns true if the EMA has been initialized
	pub fn is_ready(&self) -> bool {
		self.is_initialized
	}

	/// Returns the period
	pub fn period(&self) -> usize {
		self.period
	}

	/// Resets the EMA calculator
	pub fn reset(&mut self) {
		self.current_value = None;
		self.is_initialized = false;
		self.price_buffer.clear();
	}
}

/// Multi-period EMA tracker for a symbol
#[derive(Debug, Clone)]
pub struct MultiEMA {
	emas: Vec<(u32, EMA)>,
}

impl MultiEMA {
	/// Creates a new multi-period EMA tracker
	pub fn new(periods: &[u32]) -> Self {
		let emas = periods.iter().map(|&p| (p, EMA::new(p as usize))).collect();
		Self { emas }
	}

	/// Updates all EMAs with a new price
	pub fn update(&mut self, price: f64) {
		for (_, ema) in &mut self.emas {
			ema.update(price);
		}
	}

	/// Updates all EMAs with a candle's close price
	pub fn update_from_candle(&mut self, candle: &Candle) {
		self.update(candle.close);
	}

	/// Returns the value of a specific EMA period
	pub fn get(&self, period: u32) -> Option<f64> {
		self.emas.iter().find(|(p, _)| *p == period).and_then(|(_, ema)| ema.value())
	}

	/// Returns all EMA values as a vector of (period, value) tuples
	pub fn all_values(&self) -> Vec<(u32, Option<f64>)> {
		self.emas.iter().map(|(p, ema)| (*p, ema.value())).collect()
	}

	/// Returns true if all EMAs are ready
	pub fn all_ready(&self) -> bool {
		self.emas.iter().all(|(_, ema)| ema.is_ready())
	}

	/// Returns true if at least one EMA is ready
	pub fn any_ready(&self) -> bool {
		self.emas.iter().any(|(_, ema)| ema.is_ready())
	}

	/// Checks if price is extended above a specific EMA
	pub fn is_price_above(&self, price: f64, period: u32, threshold_pct: f64) -> bool {
		if let Some(ema_value) = self.get(period) {
			let extension_pct = ((price - ema_value) / ema_value) * 100.0;
			extension_pct > threshold_pct
		} else {
			false
		}
	}

	/// Checks if price is extended below a specific EMA
	pub fn is_price_below(&self, price: f64, period: u32, threshold_pct: f64) -> bool {
		if let Some(ema_value) = self.get(period) {
			let extension_pct = ((ema_value - price) / ema_value) * 100.0;
			extension_pct > threshold_pct
		} else {
			false
		}
	}

	/// Checks if price is above multiple EMAs
	pub fn price_above_emas(&self, price: f64, periods: &[u32]) -> bool {
		periods.iter().all(|&period| if let Some(ema_value) = self.get(period) { price > ema_value } else { false })
	}

	/// Checks if price is below multiple EMAs
	pub fn price_below_emas(&self, price: f64, periods: &[u32]) -> bool {
		periods.iter().all(|&period| if let Some(ema_value) = self.get(period) { price < ema_value } else { false })
	}

	/// Resets all EMAs
	pub fn reset(&mut self) {
		for (_, ema) in &mut self.emas {
			ema.reset();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ema_initialization() {
		let mut ema = EMA::new(3);
		assert!(!ema.is_ready());
		assert_eq!(ema.value(), None);

		ema.update(10.0);
		assert!(!ema.is_ready());

		ema.update(11.0);
		assert!(!ema.is_ready());

		let value = ema.update(12.0);
		assert!(ema.is_ready());
		assert!(value.is_some());
		assert_eq!(value.unwrap(), 11.0); // SMA of 10, 11, 12
	}

	#[test]
	fn test_ema_calculation() {
		let mut ema = EMA::new(3);

		// Initialize with SMA
		ema.update(10.0);
		ema.update(11.0);
		let initial = ema.update(12.0).unwrap();
		assert_eq!(initial, 11.0);

		// Next update should use EMA formula
		let next = ema.update(15.0).unwrap();
		// EMA = (15 - 11) * 0.5 + 11 = 13.0
		assert_eq!(next, 13.0);
	}

	#[test]
	fn test_multi_ema() {
		let mut multi = MultiEMA::new(&[7, 14, 28]);

		// Update with some prices
		for price in 10..40 {
			multi.update(price as f64);
		}

		// Check that shorter period EMA reacts faster
		let ema7 = multi.get(7).unwrap();
		let ema14 = multi.get(14).unwrap();
		let ema28 = multi.get(28).unwrap();

		// In an uptrend, shorter EMAs should be higher
		assert!(ema7 > ema14);
		assert!(ema14 > ema28);
	}
}
