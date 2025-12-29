use std::collections::{HashMap, VecDeque};

use exchanges::TickerInfo;

use crate::config::TickerAlertsConfig;

#[derive(Debug, Clone)]
struct Sample {
	time_ms: u64,
	last_price: f64,
	quote_volume_24h: f64,
}

#[derive(Debug)]
struct SymbolState {
	samples: VecDeque<Sample>,
	last_alert_time_ms: Option<u64>,
}

impl SymbolState {
	const fn new() -> Self {
		Self { samples: VecDeque::new(), last_alert_time_ms: None }
	}
}

#[derive(Debug, Clone)]
pub struct MarketTickerAlertCandidate {
	pub symbol: String,
	pub direction: String,
	pub window_minutes: u64,
	pub percent_change_window: f64,
	pub price_now: f64,
	pub quote_volume_window: f64,
	pub quote_volume_24h: f64,
	pub volume_multiplier: f64,
	pub volume_tier: f64,
}

/// Maintains a tiny in-memory sliding window per symbol and emits alert candidates.
///
/// Notes:
/// - Binance `!ticker@arr` provides rolling 24h volume; we estimate "volume in last N minutes"
///   via delta in rolling 24h quote volume between now and N minutes ago. This is an approximation,
///   but spikes still stand out, especially with an absolute volume floor.
pub struct MarketTickerScanner {
	cfg: TickerAlertsConfig,
	per_symbol: HashMap<String, SymbolState>,
}

impl MarketTickerScanner {
	#[must_use]
	pub fn new(cfg: TickerAlertsConfig) -> Self {
		Self { cfg, per_symbol: HashMap::new() }
	}

	#[must_use]
	pub fn on_ticker(&mut self, ticker: &TickerInfo) -> Option<MarketTickerAlertCandidate> {
		if !self.cfg.enabled {
			return None;
		}

		if !self.cfg.symbol_suffix.is_empty() && !ticker.symbol.ends_with(&self.cfg.symbol_suffix) {
			return None;
		}

		let now_ms = ticker.statistics_close_time;
		let price_now = ticker.last_price.parse::<f64>().ok()?;
		let quote_volume_24h = ticker.total_traded_quote_asset_volume.parse::<f64>().ok()?;

		let lookback_ms = self.cfg.lookback_minutes.saturating_mul(60_000);
		if lookback_ms == 0 {
			return None;
		}

		let sample_every_ms = self.cfg.sample_every_seconds.saturating_mul(1_000).max(1);
		let retention_ms = lookback_ms.saturating_add(sample_every_ms.saturating_mul(2));

		let state = self.per_symbol.entry(ticker.symbol.clone()).or_insert_with(SymbolState::new);

		// Upsert a downsampled sample for this symbol.
		match state.samples.back_mut() {
			Some(last) if now_ms.saturating_sub(last.time_ms) < sample_every_ms => {
				*last = Sample { time_ms: now_ms, last_price: price_now, quote_volume_24h };
			},
			_ => state.samples.push_back(Sample { time_ms: now_ms, last_price: price_now, quote_volume_24h }),
		}

		// Prune old samples.
		while let Some(front) = state.samples.front() {
			if now_ms.saturating_sub(front.time_ms) > retention_ms {
				state.samples.pop_front();
			} else {
				break;
			}
		}

		// Find the latest sample that is <= (now - lookback).
		let target_time = now_ms.saturating_sub(lookback_ms);
		let mut anchor: Option<&Sample> = None;
		for s in &state.samples {
			if s.time_ms <= target_time {
				anchor = Some(s);
			} else {
				break;
			}
		}
		let anchor = anchor?;

		// Cooldown.
		let cooldown_ms = self.cfg.alert_cooldown_minutes.saturating_mul(60_000);
		if cooldown_ms > 0
			&& let Some(last_alert) = state.last_alert_time_ms
			&& now_ms.saturating_sub(last_alert) < cooldown_ms
		{
			return None;
		}

		let percent_change_window = percent_change(anchor.last_price, price_now);
		if percent_change_window.abs() < self.cfg.min_abs_percent_change {
			return None;
		}

		let quote_volume_window = (quote_volume_24h - anchor.quote_volume_24h).max(0.0);
		if quote_volume_window < self.cfg.min_quote_volume_in_window {
			return None;
		}

		let baseline_volume_per_window = average_volume_per_window_from_24h(quote_volume_24h, self.cfg.lookback_minutes);
		let volume_multiplier =
			if baseline_volume_per_window > 0.0 { quote_volume_window / baseline_volume_per_window } else { 0.0 };

		let volume_tier = highest_met_tier(&self.cfg.volume_multipliers, volume_multiplier)?;

		let direction = if percent_change_window >= 0.0 { "PUMP" } else { "DUMP" }.to_string();

		state.last_alert_time_ms = Some(now_ms);

		Some(MarketTickerAlertCandidate {
			symbol: ticker.symbol.clone(),
			direction,
			window_minutes: self.cfg.lookback_minutes,
			percent_change_window,
			price_now,
			quote_volume_window,
			quote_volume_24h,
			volume_multiplier,
			volume_tier,
		})
	}
}

fn percent_change(old: f64, new: f64) -> f64 {
	if old == 0.0 {
		return 0.0;
	}
	((new - old) / old) * 100.0
}

fn average_volume_per_window_from_24h(quote_volume_24h: f64, window_minutes: u64) -> f64 {
	if quote_volume_24h <= 0.0 || window_minutes == 0 {
		return 0.0;
	}
	let windows_per_day = 1440.0 / window_minutes as f64;
	if windows_per_day <= 0.0 {
		return 0.0;
	}
	quote_volume_24h / windows_per_day
}

fn highest_met_tier(tiers: &[f64], value: f64) -> Option<f64> {
	let mut best: Option<f64> = None;
	for &t in tiers {
		if value >= t {
			best = Some(best.map_or(t, |b| b.max(t)));
		}
	}
	best
}

#[cfg(test)]
mod tests {
	use super::*;

	fn ticker(symbol: &str, close_time_ms: u64, price: f64, quote_vol_24h: f64) -> TickerInfo {
		TickerInfo {
			symbol: symbol.to_string(),
			price_change: "0".to_string(),
			price_change_percent: "0".to_string(),
			weighted_average_price: "0".to_string(),
			last_price: price.to_string(),
			last_quantity: "0".to_string(),
			open_price: "0".to_string(),
			high_price: "0".to_string(),
			low_price: "0".to_string(),
			total_traded_base_asset_volume: "0".to_string(),
			total_traded_quote_asset_volume: quote_vol_24h.to_string(),
			statistics_open_time: close_time_ms.saturating_sub(86_400_000),
			statistics_close_time: close_time_ms,
			total_number_of_trades: 0,
		}
	}

	#[test]
	fn percent_change_works() {
		assert_eq!(percent_change(100.0, 110.0), 10.0);
		assert_eq!(percent_change(100.0, 90.0), -10.0);
		assert_eq!(percent_change(0.0, 10.0), 0.0);
	}

	#[test]
	fn triggers_on_price_and_volume_tier() {
		let mut cfg = TickerAlertsConfig::default();
		cfg.min_abs_percent_change = 5.0;
		cfg.min_quote_volume_in_window = 10_000.0;
		cfg.volume_multipliers = vec![5.0, 10.0];
		cfg.alert_cooldown_minutes = 0;
		cfg.sample_every_seconds = 1;
		cfg.lookback_minutes = 15;
		cfg.symbol_suffix = "USDT".to_string();

		let mut scanner = MarketTickerScanner::new(cfg);

		let t0 = ticker("ABCUSDT", 1_000_000, 100.0, 1_000_000.0);
		let t1 = ticker("ABCUSDT", 1_000_000 + 15 * 60_000, 110.0, 1_200_000.0);

		assert!(scanner.on_ticker(&t0).is_none());
		let alert = scanner.on_ticker(&t1).expect("should alert");
		assert_eq!(alert.direction, "PUMP");
		assert!(alert.percent_change_window >= 10.0 - 1e-9);
		assert!(alert.quote_volume_window >= 200_000.0 - 1e-9);
		assert!(alert.volume_tier == 10.0 || alert.volume_tier == 5.0);
	}

	#[test]
	fn respects_cooldown() {
		let mut cfg = TickerAlertsConfig::default();
		cfg.min_abs_percent_change = 1.0;
		cfg.min_quote_volume_in_window = 1.0;
		cfg.volume_multipliers = vec![1.0];
		cfg.alert_cooldown_minutes = 30;
		cfg.sample_every_seconds = 1;
		cfg.lookback_minutes = 15;
		cfg.symbol_suffix = "USDT".to_string();

		let mut scanner = MarketTickerScanner::new(cfg);

		let base = 10_000_000;
		let t0 = ticker("XYZUSDT", base, 100.0, 1_000_000.0);
		let t1 = ticker("XYZUSDT", base + 15 * 60_000, 101.5, 1_020_000.0);
		let t2 = ticker("XYZUSDT", base + 16 * 60_000, 103.0, 1_020_000.0);

		assert!(scanner.on_ticker(&t0).is_none());
		assert!(scanner.on_ticker(&t1).is_some());
		assert!(scanner.on_ticker(&t2).is_none());
	}
}
