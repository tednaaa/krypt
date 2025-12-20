use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::config::TelegramConfig;
use crate::types::{current_timestamp, Alert, AlertType, Timestamp};

#[derive(Serialize)]
struct SendMessageRequest {
	chat_id: String,
	text: String,
	parse_mode: String,
}

pub struct TelegramDispatcher {
	config: TelegramConfig,
	client: Client,
	last_alert_times: HashMap<(String, AlertType), Timestamp>,
	alerts_this_minute: Vec<Timestamp>,
}

impl TelegramDispatcher {
	pub fn new(config: TelegramConfig) -> Self {
		Self { config, client: Client::new(), last_alert_times: HashMap::new(), alerts_this_minute: Vec::new() }
	}

	/// Process and send an alert, applying cooldown and rate limits
	pub async fn send_alert(&mut self, alert: Alert) -> Result<()> {
		// Check cooldown for this symbol and alert type
		if !self.check_cooldown(&alert) {
			debug!("Alert for {} ({:?}) is in cooldown, skipping", alert.symbol, alert.alert_type);
			return Ok(());
		}

		// Check rate limit
		if !self.check_rate_limit() {
			warn!("Rate limit exceeded, skipping alert for {}", alert.symbol);
			return Ok(());
		}

		// Send the alert
		match self.send_telegram_message(&alert).await {
			Ok(_) => {
				info!("Sent alert: {:?} for {}", alert.alert_type, alert.symbol);

				// Update cooldown tracker
				self.last_alert_times.insert((alert.symbol.clone(), alert.alert_type.clone()), alert.timestamp);

				// Update rate limit tracker
				self.alerts_this_minute.push(alert.timestamp);

				Ok(())
			},
			Err(e) => {
				error!("Failed to send Telegram alert: {}", e);
				Err(e)
			},
		}
	}

	/// Check if alert is within cooldown period
	fn check_cooldown(&self, alert: &Alert) -> bool {
		let key = (alert.symbol.clone(), alert.alert_type.clone());

		if let Some(&last_time) = self.last_alert_times.get(&key) {
			let elapsed = alert.timestamp.saturating_sub(last_time);
			elapsed >= self.config.alert_cooldown_secs
		} else {
			true
		}
	}

	/// Check if we're within rate limit
	fn check_rate_limit(&mut self) -> bool {
		let now = current_timestamp();
		let one_minute_ago = now.saturating_sub(60);

		// Clean up old entries
		self.alerts_this_minute.retain(|&ts| ts >= one_minute_ago);

		self.alerts_this_minute.len() < self.config.max_alerts_per_minute
	}

	/// Send message to Telegram
	async fn send_telegram_message(&self, alert: &Alert) -> Result<()> {
		let url = format!("https://api.telegram.org/bot{}/sendMessage", self.config.bot_token);

		let message = alert.format_telegram();

		let request =
			SendMessageRequest { chat_id: self.config.chat_id.clone(), text: message, parse_mode: "HTML".to_string() };

		let response = self
			.client
			.post(&url)
			.json(&request)
			.timeout(Duration::from_secs(10))
			.send()
			.await
			.context("Failed to send Telegram request")?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Telegram API error: {} - {}", status, body);
		}

		Ok(())
	}

	/// Get statistics about alerts
	pub fn get_stats(&self) -> AlertStats {
		AlertStats { total_cooldowns: self.last_alert_times.len(), alerts_this_minute: self.alerts_this_minute.len() }
	}
}

#[derive(Debug, Clone)]
pub struct AlertStats {
	pub total_cooldowns: usize,
	pub alerts_this_minute: usize,
}

/// Alert dispatcher task
pub async fn alert_dispatcher_task(config: TelegramConfig, mut alert_rx: tokio::sync::mpsc::Receiver<Alert>) {
	info!("Starting alert dispatcher task");

	let mut dispatcher = TelegramDispatcher::new(config);

	while let Some(alert) = alert_rx.recv().await {
		if let Err(e) = dispatcher.send_alert(alert).await {
			error!("Error dispatching alert: {}", e);
			// Continue processing other alerts
		}
	}

	info!("Alert dispatcher task ended");
}

/// Alert priority queue - ensures high-priority alerts are sent first
pub struct AlertPriorityQueue {
	queue: Vec<Alert>,
}

impl AlertPriorityQueue {
	pub fn new() -> Self {
		Self { queue: Vec::new() }
	}

	pub fn push(&mut self, alert: Alert) {
		self.queue.push(alert);
		self.sort();
	}

	pub fn pop(&mut self) -> Option<Alert> {
		if self.queue.is_empty() {
			None
		} else {
			Some(self.queue.remove(0))
		}
	}

	pub fn len(&self) -> usize {
		self.queue.len()
	}

	pub fn is_empty(&self) -> bool {
		self.queue.is_empty()
	}

	fn sort(&mut self) {
		self.queue.sort_by(|a, b| {
			let a_priority = Self::get_priority(&a.alert_type);
			let b_priority = Self::get_priority(&b.alert_type);
			b_priority.cmp(&a_priority) // Higher priority first
		});
	}

	fn get_priority(alert_type: &AlertType) -> u8 {
		match alert_type {
			AlertType::LongSetupConfirmed => 10,
			AlertType::ShortSetupConfirmed => 10,
			AlertType::AccumulationDetected => 7,
			AlertType::DistributionDetected => 7,
			AlertType::PumpDetected => 5,
			AlertType::DumpDetected => 5,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::types::AlertDetails;

	#[test]
	fn test_priority_queue() {
		let mut queue = AlertPriorityQueue::new();

		let pump_alert = Alert {
			alert_type: AlertType::PumpDetected,
			symbol: "BTCUSDT".to_string(),
			price: 50000.0,
			details: AlertDetails { price_change_pct: Some(5.0), volume_ratio: None, cvd_change: None, timeframe: None },
			timestamp: 1000,
		};

		let long_alert = Alert {
			alert_type: AlertType::LongSetupConfirmed,
			symbol: "ETHUSDT".to_string(),
			price: 3000.0,
			details: AlertDetails { price_change_pct: Some(2.0), volume_ratio: None, cvd_change: None, timeframe: None },
			timestamp: 1001,
		};

		queue.push(pump_alert.clone());
		queue.push(long_alert.clone());

		// Long setup should come first (higher priority)
		let first = queue.pop().unwrap();
		assert_eq!(first.alert_type, AlertType::LongSetupConfirmed);

		let second = queue.pop().unwrap();
		assert_eq!(second.alert_type, AlertType::PumpDetected);
	}

	#[test]
	fn test_cooldown() {
		let config = TelegramConfig {
			bot_token: "test".to_string(),
			chat_id: "test".to_string(),
			alert_cooldown_secs: 300,
			max_alerts_per_minute: 10,
		};

		let mut dispatcher = TelegramDispatcher::new(config);

		let alert1 = Alert {
			alert_type: AlertType::PumpDetected,
			symbol: "BTCUSDT".to_string(),
			price: 50000.0,
			details: AlertDetails { price_change_pct: Some(5.0), volume_ratio: None, cvd_change: None, timeframe: None },
			timestamp: 1000,
		};

		// First alert should pass cooldown check
		assert!(dispatcher.check_cooldown(&alert1));

		// Simulate sent alert
		dispatcher.last_alert_times.insert((alert1.symbol.clone(), alert1.alert_type.clone()), alert1.timestamp);

		// Second alert with same type/symbol should fail if within cooldown
		let alert2 = Alert {
			timestamp: 1200, // 200 seconds later, within 300s cooldown
			..alert1.clone()
		};
		assert!(!dispatcher.check_cooldown(&alert2));

		// After cooldown period, should pass
		let alert3 = Alert {
			timestamp: 1301, // 301 seconds later, outside cooldown
			..alert1.clone()
		};
		assert!(dispatcher.check_cooldown(&alert3));
	}

	#[test]
	fn test_rate_limit() {
		let config = TelegramConfig {
			bot_token: "test".to_string(),
			chat_id: "test".to_string(),
			alert_cooldown_secs: 300,
			max_alerts_per_minute: 3,
		};

		let mut dispatcher = TelegramDispatcher::new(config);

		let now = current_timestamp();

		// Add 3 alerts in the last minute
		for i in 0..3 {
			dispatcher.alerts_this_minute.push(now - 30 + i);
		}

		// Should fail rate limit
		assert!(!dispatcher.check_rate_limit());

		// Clean up old alerts (simulate 2 minutes passing)
		dispatcher.alerts_this_minute.clear();

		// Should pass now
		assert!(dispatcher.check_rate_limit());
	}
}
