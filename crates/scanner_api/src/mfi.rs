use exchanges::CandleInfo;

pub fn calculate_mfi(candles: &[CandleInfo], length: usize) -> Option<f64> {
	if candles.len() < length + 1 {
		return None;
	}

	let mut positive_flow = 0.0;
	let mut negative_flow = 0.0;

	// Calculate typical prices and money flow
	for i in (candles.len() - length)..candles.len() {
		let current_typical = (candles[i].high + candles[i].low + candles[i].close) / 3.0;
		let previous_typical = (candles[i - 1].high + candles[i - 1].low + candles[i - 1].close) / 3.0;

		let raw_money_flow = current_typical * candles[i].volume;

		if current_typical > previous_typical {
			positive_flow += raw_money_flow;
		} else if current_typical < previous_typical {
			negative_flow += raw_money_flow;
		}
	}

	// Avoid division by zero
	if negative_flow == 0.0 {
		return Some(100.0);
	}

	let money_flow_ratio = positive_flow / negative_flow;
	let mfi = 100.0 - (100.0 / (1.0 + money_flow_ratio));

	Some(mfi)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mfi_basic() {
		let candles = vec![
			CandleInfo { open: 100.0, high: 105.0, low: 99.0, close: 103.0, volume: 1000.0 },
			CandleInfo { open: 103.0, high: 108.0, low: 102.0, close: 107.0, volume: 1500.0 },
			CandleInfo { open: 107.0, high: 110.0, low: 106.0, close: 108.0, volume: 1200.0 },
			CandleInfo { open: 108.0, high: 109.0, low: 105.0, close: 106.0, volume: 800.0 },
			CandleInfo { open: 106.0, high: 107.0, low: 104.0, close: 105.0, volume: 900.0 },
			CandleInfo { open: 105.0, high: 106.0, low: 103.0, close: 104.0, volume: 1100.0 },
			CandleInfo { open: 104.0, high: 105.0, low: 102.0, close: 103.0, volume: 1000.0 },
			CandleInfo { open: 103.0, high: 104.0, low: 101.0, close: 102.0, volume: 950.0 },
			CandleInfo { open: 102.0, high: 103.0, low: 100.0, close: 101.0, volume: 1050.0 },
			CandleInfo { open: 101.0, high: 102.0, low: 99.0, close: 100.0, volume: 1100.0 },
			CandleInfo { open: 100.0, high: 101.0, low: 98.0, close: 99.0, volume: 1200.0 },
			CandleInfo { open: 99.0, high: 100.0, low: 97.0, close: 98.0, volume: 1300.0 },
			CandleInfo { open: 98.0, high: 99.0, low: 96.0, close: 97.0, volume: 1400.0 },
			CandleInfo { open: 97.0, high: 98.0, low: 95.0, close: 96.0, volume: 1500.0 },
			CandleInfo { open: 96.0, high: 97.0, low: 94.0, close: 95.0, volume: 1600.0 },
		];

		let mfi = calculate_mfi(&candles, 14);
		assert!(mfi.is_some());

		let mfi_value = mfi.unwrap();
		assert!(mfi_value >= 0.0 && mfi_value <= 100.0);
	}

	#[test]
	fn test_insufficient_data() {
		let candles = vec![CandleInfo { open: 100.0, high: 105.0, low: 99.0, close: 103.0, volume: 1000.0 }];

		let mfi = calculate_mfi(&candles, 14);
		assert!(mfi.is_none());
	}
}
