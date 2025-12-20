use crate::config::DetectionConfig;
use crate::types::{current_timestamp, Alert, AlertDetails, AlertType, MarketState, SymbolData, Timestamp};
use std::collections::VecDeque;

pub struct Detector {
	config: DetectionConfig,
}

impl Detector {
	pub fn new(config: DetectionConfig) -> Self {
		Self { config }
	}

	/// Main detection entry point - returns alerts if any patterns are detected
	pub fn detect(&self, symbol: &mut SymbolData) -> Vec<Alert> {
		let mut alerts = Vec::new();

		// Check for pump/dump (momentum-based, any tier)
		if let Some(alert) = self.detect_pump(symbol) {
			symbol.state = MarketState::PumpDetected;
			alerts.push(alert);
		} else if let Some(alert) = self.detect_dump(symbol) {
			symbol.state = MarketState::DumpDetected;
			alerts.push(alert);
		}

		// Check for accumulation/distribution (Tier 1 only, requires CVD)
		if !symbol.cvd_history.is_empty() {
			match symbol.state {
				MarketState::Idle | MarketState::PumpDetected | MarketState::DumpDetected => {
					if let Some(alert) = self.detect_accumulation(symbol) {
						symbol.state = MarketState::Accumulation;
						symbol.accumulation_start = Some(current_timestamp());
						symbol.accumulation_high = Some(symbol.price);
						symbol.accumulation_low = Some(symbol.price);
						alerts.push(alert);
					} else if let Some(alert) = self.detect_distribution(symbol) {
						symbol.state = MarketState::Distribution;
						symbol.distribution_start = Some(current_timestamp());
						symbol.distribution_high = Some(symbol.price);
						symbol.distribution_low = Some(symbol.price);
						alerts.push(alert);
					}
				},
				MarketState::Accumulation => {
					// Update accumulation range
					if let Some(high) = symbol.accumulation_high {
						symbol.accumulation_high = Some(high.max(symbol.price));
					}
					if let Some(low) = symbol.accumulation_low {
						symbol.accumulation_low = Some(low.min(symbol.price));
					}

					// Check for breakout
					if let Some(alert) = self.detect_breakout_long(symbol) {
						symbol.state = MarketState::BreakoutLong;
						alerts.push(alert);
					}
				},
				MarketState::Distribution => {
					// Update distribution range
					if let Some(high) = symbol.distribution_high {
						symbol.distribution_high = Some(high.max(symbol.price));
					}
					if let Some(low) = symbol.distribution_low {
						symbol.distribution_low = Some(low.min(symbol.price));
					}

					// Check for breakdown
					if let Some(alert) = self.detect_breakdown_short(symbol) {
						symbol.state = MarketState::BreakdownShort;
						alerts.push(alert);
					}
				},
				MarketState::BreakoutLong => {
					// Check if we should transition back to idle
					if self.should_reset_state(symbol) {
						symbol.state = MarketState::Idle;
						symbol.accumulation_start = None;
						symbol.accumulation_high = None;
						symbol.accumulation_low = None;
					}
				},
				MarketState::BreakdownShort => {
					// Check if we should transition back to idle
					if self.should_reset_state(symbol) {
						symbol.state = MarketState::Idle;
						symbol.distribution_start = None;
						symbol.distribution_high = None;
						symbol.distribution_low = None;
					}
				},
			}
		}

		alerts
	}

	/// Detect pump: rapid price increase with volume spike
	fn detect_pump(&self, symbol: &SymbolData) -> Option<Alert> {
		let now = current_timestamp();
		let window_start = now.saturating_sub(self.config.window_size_secs);

		// Get prices in the window
		let recent_prices: Vec<f64> =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).collect();

		if recent_prices.len() < 10 {
			return None;
		}

		let oldest_price = recent_prices[0];
		let current_price = symbol.price;

		// Calculate price change percentage
		let price_change_pct = ((current_price - oldest_price) / oldest_price) * 100.0;

		if price_change_pct < self.config.pump_threshold_pct {
			return None;
		}

		// Check volume spike
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);
		if volume_ratio < self.config.volume_spike_ratio {
			return None;
		}

		// Check if making new high
		if !self.is_new_high(symbol, 300) {
			// 5 minutes
			return None;
		}

		Some(Alert {
			alert_type: AlertType::PumpDetected,
			symbol: symbol.symbol.clone(),
			price: current_price,
			details: AlertDetails {
				price_change_pct: Some(price_change_pct),
				volume_ratio: Some(volume_ratio),
				cvd_change: None,
				timeframe: Some("60s".to_string()),
			},
			timestamp: now,
		})
	}

	/// Detect dump: rapid price decrease with volume spike
	fn detect_dump(&self, symbol: &SymbolData) -> Option<Alert> {
		let now = current_timestamp();
		let window_start = now.saturating_sub(self.config.window_size_secs);

		// Get prices in the window
		let recent_prices: Vec<f64> =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).collect();

		if recent_prices.len() < 10 {
			return None;
		}

		let oldest_price = recent_prices[0];
		let current_price = symbol.price;

		// Calculate price change percentage
		let price_change_pct = ((current_price - oldest_price) / oldest_price) * 100.0;

		if price_change_pct > self.config.dump_threshold_pct {
			return None;
		}

		// Check volume spike
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);
		if volume_ratio < self.config.volume_spike_ratio {
			return None;
		}

		// Check if making new low
		if !self.is_new_low(symbol, 300) {
			// 5 minutes
			return None;
		}

		Some(Alert {
			alert_type: AlertType::DumpDetected,
			symbol: symbol.symbol.clone(),
			price: current_price,
			details: AlertDetails {
				price_change_pct: Some(price_change_pct),
				volume_ratio: Some(volume_ratio),
				cvd_change: None,
				timeframe: Some("60s".to_string()),
			},
			timestamp: now,
		})
	}

	/// Detect accumulation: flat price with rising CVD
	fn detect_accumulation(&self, symbol: &SymbolData) -> Option<Alert> {
		let now = current_timestamp();
		let window_start = now.saturating_sub(self.config.accumulation_window_secs);

		// Get prices in window
		let recent_prices: Vec<f64> =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).collect();

		if recent_prices.len() < 20 {
			return None;
		}

		// Check price range is tight
		let max_price = recent_prices.iter().copied().fold(f64::NEG_INFINITY, f64::max);
		let min_price = recent_prices.iter().copied().fold(f64::INFINITY, f64::min);
		let price_range_pct = ((max_price - min_price) / symbol.price) * 100.0;

		if price_range_pct >= self.config.accumulation_range_pct {
			return None;
		}

		// Check CVD is rising
		let cvd_slope = self.calculate_cvd_slope(symbol, window_start);
		if cvd_slope <= 0.0 {
			return None;
		}

		// Check volume is elevated
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);
		if volume_ratio < 1.5 {
			return None;
		}

		// Check not making new lows
		if self.is_new_low(symbol, self.config.accumulation_window_secs) {
			return None;
		}

		Some(Alert {
			alert_type: AlertType::AccumulationDetected,
			symbol: symbol.symbol.clone(),
			price: symbol.price,
			details: AlertDetails {
				price_change_pct: Some(price_range_pct),
				volume_ratio: Some(volume_ratio),
				cvd_change: Some(cvd_slope),
				timeframe: Some("2m".to_string()),
			},
			timestamp: now,
		})
	}

	/// Detect distribution: stalling price with negative CVD
	fn detect_distribution(&self, symbol: &SymbolData) -> Option<Alert> {
		let now = current_timestamp();
		let window_start = now.saturating_sub(self.config.distribution_window_secs);

		// Get prices in window
		let recent_prices: Vec<f64> =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).collect();

		if recent_prices.len() < 30 {
			return None;
		}

		// Check price stopped making higher highs
		let price_trend = self.calculate_price_trend(symbol, window_start);
		if price_trend > 0.1 {
			// Still trending up
			return None;
		}

		// Check CVD flattening or negative
		let cvd_slope = self.calculate_cvd_slope(symbol, window_start);
		if cvd_slope > 0.0 {
			return None;
		}

		// Check volume remains elevated
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);
		if volume_ratio < 1.5 {
			return None;
		}

		Some(Alert {
			alert_type: AlertType::DistributionDetected,
			symbol: symbol.symbol.clone(),
			price: symbol.price,
			details: AlertDetails {
				price_change_pct: None,
				volume_ratio: Some(volume_ratio),
				cvd_change: Some(cvd_slope),
				timeframe: Some("3m".to_string()),
			},
			timestamp: now,
		})
	}

	/// Detect breakout from accumulation
	fn detect_breakout_long(&self, symbol: &SymbolData) -> Option<Alert> {
		let accumulation_high = symbol.accumulation_high?;

		// Check price breaks above accumulation range
		let breakout_threshold = accumulation_high * (1.0 + self.config.breakout_threshold_pct / 100.0);
		if symbol.price < breakout_threshold {
			return None;
		}

		// Check volume spike in last 30 seconds
		let now = current_timestamp();
		let window_start = now.saturating_sub(30);
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);

		if volume_ratio < self.config.volume_spike_ratio {
			return None;
		}

		// Check CVD continues rising
		let cvd_slope = self.calculate_cvd_slope(symbol, window_start);
		if cvd_slope <= 0.0 {
			return None;
		}

		let price_change_pct = ((symbol.price - accumulation_high) / accumulation_high) * 100.0;

		Some(Alert {
			alert_type: AlertType::LongSetupConfirmed,
			symbol: symbol.symbol.clone(),
			price: symbol.price,
			details: AlertDetails {
				price_change_pct: Some(price_change_pct),
				volume_ratio: Some(volume_ratio),
				cvd_change: Some(cvd_slope),
				timeframe: Some("breakout".to_string()),
			},
			timestamp: now,
		})
	}

	/// Detect breakdown from distribution
	fn detect_breakdown_short(&self, symbol: &SymbolData) -> Option<Alert> {
		let distribution_low = symbol.distribution_low?;

		// Check price breaks below distribution range
		let breakdown_threshold = distribution_low * (1.0 - self.config.breakout_threshold_pct / 100.0);
		if symbol.price > breakdown_threshold {
			return None;
		}

		// Check sell volume spike in last 30 seconds
		let now = current_timestamp();
		let window_start = now.saturating_sub(30);
		let volume_ratio = self.calculate_volume_ratio(symbol, window_start);

		if volume_ratio < self.config.volume_spike_ratio {
			return None;
		}

		// Check CVD sharply negative
		let cvd_slope = self.calculate_cvd_slope(symbol, window_start);
		if cvd_slope >= 0.0 {
			return None;
		}

		let price_change_pct = ((symbol.price - distribution_low) / distribution_low) * 100.0;

		Some(Alert {
			alert_type: AlertType::ShortSetupConfirmed,
			symbol: symbol.symbol.clone(),
			price: symbol.price,
			details: AlertDetails {
				price_change_pct: Some(price_change_pct),
				volume_ratio: Some(volume_ratio),
				cvd_change: Some(cvd_slope),
				timeframe: Some("breakdown".to_string()),
			},
			timestamp: now,
		})
	}

	// Helper functions

	fn calculate_volume_ratio(&self, symbol: &SymbolData, window_start: Timestamp) -> f64 {
		let recent_volumes: Vec<f64> =
			symbol.volume_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, v)| *v).collect();

		if recent_volumes.is_empty() {
			return 0.0;
		}

		let recent_avg = recent_volumes.iter().sum::<f64>() / recent_volumes.len() as f64;

		// Compare to 24h baseline
		let baseline_volume = symbol.quote_volume_24h / (24.0 * 3600.0); // Per second

		if baseline_volume == 0.0 {
			return 0.0;
		}

		recent_avg / baseline_volume
	}

	fn calculate_cvd_slope(&self, symbol: &SymbolData, window_start: Timestamp) -> f64 {
		let recent_cvd: Vec<(Timestamp, f64)> =
			symbol.cvd_history.iter().filter(|(ts, _)| *ts >= window_start).copied().collect();

		if recent_cvd.len() < 2 {
			return 0.0;
		}

		let first_cvd = recent_cvd[0].1;
		let last_cvd = recent_cvd[recent_cvd.len() - 1].1;

		last_cvd - first_cvd
	}

	fn calculate_price_trend(&self, symbol: &SymbolData, window_start: Timestamp) -> f64 {
		let recent_prices: Vec<f64> =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).collect();

		if recent_prices.len() < 2 {
			return 0.0;
		}

		let first_price = recent_prices[0];
		let last_price = recent_prices[recent_prices.len() - 1];

		((last_price - first_price) / first_price) * 100.0
	}

	fn is_new_high(&self, symbol: &SymbolData, window_secs: u64) -> bool {
		let now = current_timestamp();
		let window_start = now.saturating_sub(window_secs);

		let max_price = symbol
			.price_window
			.iter()
			.filter(|(ts, _)| *ts >= window_start)
			.map(|(_, p)| *p)
			.fold(f64::NEG_INFINITY, f64::max);

		symbol.price >= max_price * 0.999 // Within 0.1% of high
	}

	fn is_new_low(&self, symbol: &SymbolData, window_secs: u64) -> bool {
		let now = current_timestamp();
		let window_start = now.saturating_sub(window_secs);

		let min_price =
			symbol.price_window.iter().filter(|(ts, _)| *ts >= window_start).map(|(_, p)| *p).fold(f64::INFINITY, f64::min);

		symbol.price <= min_price * 1.001 // Within 0.1% of low
	}

	fn should_reset_state(&self, symbol: &SymbolData) -> bool {
		// Reset state after 5 minutes to avoid stale states
		let state_start = match symbol.state {
			MarketState::BreakoutLong => symbol.accumulation_start,
			MarketState::BreakdownShort => symbol.distribution_start,
			_ => return false,
		};

		if let Some(start) = state_start {
			let now = current_timestamp();
			now.saturating_sub(start) > 300 // 5 minutes
		} else {
			false
		}
	}
}
