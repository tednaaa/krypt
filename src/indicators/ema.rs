use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Ema {
	#[allow(dead_code)]
	period: usize,
	current_value: Option<f64>,
	#[allow(dead_code)]
	is_initialized: bool,
	#[allow(dead_code)]
	price_buffer: VecDeque<f64>,
}

impl Ema {
	pub fn new(period: usize) -> Self {
		Self {
			period,
			current_value: None,
			is_initialized: false,
			price_buffer: VecDeque::with_capacity(period),
		}
	}

	#[allow(dead_code)]
	pub fn update(&mut self, price: f64) -> Option<f64> {
		if self.is_initialized {
			if let Some(prev_ema) = self.current_value {
				let multiplier = 2.0 / (self.period as f64 + 1.0);
				let ema = (price - prev_ema).mul_add(multiplier, prev_ema);
				self.current_value = Some(ema);
				Some(ema)
			} else {
				None
			}
		} else {
			self.price_buffer.push_back(price);

			if self.price_buffer.len() >= self.period {
				let sum: f64 = self.price_buffer.iter().sum();
				let sma = sum / self.period as f64;
				self.current_value = Some(sma);
				self.is_initialized = true;
				return Some(sma);
			}

			None
		}
	}

	pub const fn value(&self) -> Option<f64> {
		self.current_value
	}
}

#[derive(Debug, Clone)]
pub struct MultiEma {
	emas: Vec<(u32, Ema)>,
}

impl MultiEma {
	pub fn new(periods: &[u32]) -> Self {
		let emas = periods.iter().map(|&p| (p, Ema::new(p as usize))).collect();
		Self { emas }
	}

	pub fn get(&self, period: u32) -> Option<f64> {
		self.emas.iter().find(|(p, _)| *p == period).and_then(|(_, ema)| ema.value())
	}

	pub fn price_above_emas(&self, price: f64, periods: &[u32]) -> bool {
		periods.iter().all(|&period| self.get(period).is_some_and(|ema_value| price > ema_value))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ema_calculation() {
		let mut ema = Ema::new(3);

		ema.update(10.0);
		ema.update(11.0);
		let initial = ema.update(12.0).unwrap();
		assert!((initial - 11.0).abs() < 1e-10);

		let next = ema.update(15.0).unwrap();
		assert!((next - 13.0).abs() < 1e-10);
	}
}
