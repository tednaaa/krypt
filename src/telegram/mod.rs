use crate::config::TelegramConfig;
use crate::pump_scanner::{PumpCandidate, QualificationResult};
use anyhow::{Context, Result};
use teloxide::{
	prelude::*,
	types::{MessageId, ParseMode, ThreadId},
};
use tracing::{error, info};

/// Telegram bot for posting pump alerts to a channel
pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

impl TelegramBot {
	/// Creates a new Telegram bot instance
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);
		Self { bot, config }
	}

	/// Posts a pump alert to the configured channel
	pub async fn post_alert(&self, candidate: &PumpCandidate, qualification: &QualificationResult) -> Result<()> {
		let message = self.format_alert_message(candidate, qualification);

		let chat_id = self.config.chat_id.parse::<i64>().context("Invalid chat_id format")?;

		let mut request = self.bot.send_message(ChatId(chat_id), message).parse_mode(ParseMode::Html);

		// Add topic ID if configured
		if let Some(ref topic_id) = self.config.pump_screener_topic_id {
			if !topic_id.is_empty() {
				if let Ok(thread_id) = topic_id.parse::<i32>() {
					request = request.message_thread_id(ThreadId(MessageId(thread_id)));
				}
			}
		}

		match request.await {
			Ok(_) => {
				info!(
					symbol = %candidate.symbol,
					"Alert posted to Telegram"
				);
				Ok(())
			},
			Err(e) => {
				error!(
					symbol = %candidate.symbol,
					error = %e,
					"Failed to post alert to Telegram"
				);
				Err(e.into())
			},
		}
	}

	/// Formats the alert message according to specification
	fn format_alert_message(&self, candidate: &PumpCandidate, qualification: &QualificationResult) -> String {
		let symbol_display = format!("{}/{}", candidate.symbol.base, candidate.symbol.quote);
		let price = candidate.current_price;
		let change_pct = candidate.price_change.change_pct;
		let time_mins = candidate.price_change.time_elapsed_mins;
		let volume_ratio = candidate.volume_ratio;

		// Format derivatives data
		let derivatives = &qualification.derivatives_details;
		let oi_str = derivatives
			.oi_increase_pct
			.map_or_else(|| "Open Interest: N/A".to_string(), |v| format!("Open Interest: +{v:.1}%"));

		let funding_str =
			derivatives.funding_rate.map_or_else(|| "Funding: N/A".to_string(), |v| format!("Funding: {:.3}%", v * 100.0));

		let ls_ratio_str = derivatives.long_ratio.map_or_else(
			|| "Long / Short: N/A".to_string(),
			|r| {
				let long_pct = r * 100.0;
				let short_pct = (1.0 - r) * 100.0;
				format!("Long / Short: {long_pct:.0}% / {short_pct:.0}%")
			},
		);

		// Format technical context
		let technical_context = qualification.technical_context();
		let technical_str = if technical_context.is_empty() {
			"• No specific technical signals".to_string()
		} else {
			technical_context.join("\n")
		};

		// Build Coinglass URL
		let coinglass_url = format!("https://www.coinglass.com/tv/{}{}", candidate.symbol.base, candidate.symbol.quote);

		// Format the complete message
		format!(
			"🚨 <b>PUMP DETECTED — {symbol_display}</b>\n\
			\n\
			<b>Price:</b> {price:.2} USDT (+{change_pct:.1}% in {time_mins}m)\n\
			<b>Volume:</b> x{volume_ratio:.1} vs average\n\
			\n\
			{oi_str}\n\
			{funding_str}\n\
			{ls_ratio_str}\n\
			\n\
			📍 <b>Technical context:</b>\n\
			{technical_str}\n\
			\n\
			🔗 <a href=\"{coinglass_url}\">Coinglass</a>"
		)
	}

	/// Tests the bot connection by sending a test message
	pub async fn test_connection(&self) -> Result<()> {
		let chat_id = self.config.chat_id.parse::<i64>().context("Invalid chat_id format")?;

		let mut request = self.bot.send_message(ChatId(chat_id), "🤖 Pump Scanner Bot initialized");

		// Add topic ID if configured
		if let Some(ref topic_id) = self.config.pump_screener_topic_id {
			if !topic_id.is_empty() {
				if let Ok(thread_id) = topic_id.parse::<i32>() {
					request = request.message_thread_id(ThreadId(MessageId(thread_id)));
				}
			}
		}

		request.await.context("Failed to send test message")?;

		info!("Telegram bot connection verified");
		Ok(())
	}
}

/// Formats price with appropriate precision
#[cfg(test)]
fn format_price(price: f64) -> String {
	if price >= 1000.0 {
		format!("{price:.2}")
	} else if price >= 1.0 {
		format!("{price:.3}")
	} else if price >= 0.01 {
		format!("{price:.4}")
	} else {
		format!("{price:.6}")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::exchange::Symbol;
	use crate::pump_scanner::qualifier::{DerivativesResult, MomentumStatus, TechnicalResult};
	use crate::pump_scanner::tracker::PriceChange;
	use crate::pump_scanner::QualificationResult;
	use chrono::Utc;

	#[test]
	fn test_format_price() {
		assert_eq!(format_price(50000.0), "50000.00");
		assert_eq!(format_price(100.0), "100.000");
		assert_eq!(format_price(1.5), "1.500");
		assert_eq!(format_price(0.05), "0.0500");
		assert_eq!(format_price(0.0001), "0.000100");
	}

	#[test]
	fn test_alert_message_format() {
		let config = TelegramConfig {
			bot_token: "test_token".to_string(),
			chat_id: "123456".to_string(),
			pump_screener_topic_id: None,
			alert_cooldown_secs: 300,
			max_alerts_per_minute: 10,
		};

		let bot = TelegramBot::new(config);

		let candidate = PumpCandidate {
			symbol: Symbol::new("BTC", "USDT", "binance"),
			price_change: PriceChange {
				start_price: 50000.0,
				end_price: 52500.0,
				change_pct: 5.0,
				time_elapsed_mins: 10,
				start_time: Utc::now(),
				end_time: Utc::now(),
			},
			volume_ratio: 3.1,
			current_price: 52500.0,
		};

		let qualification = QualificationResult {
			qualified: true,
			score: 3,
			conditions_met: vec!["OI increased 11%".to_string(), "Funding rate 0.0310".to_string()],
			conditions_failed: vec![],
			derivatives_details: DerivativesResult {
				conditions_met: vec![],
				conditions_failed: vec![],
				oi_increase_pct: Some(11.0),
				funding_rate: Some(0.031),
				long_ratio: Some(0.71),
			},
			technical_details: TechnicalResult {
				conditions_met: vec!["Price above EMA50".to_string()],
				conditions_failed: vec![],
				ema_extended: true,
				near_pivot_resistance: Some("R1".to_string()),
				momentum_status: MomentumStatus::Slowing("deceleration detected".to_string()),
			},
		};

		let message = bot.format_alert_message(&candidate, &qualification);

		// Verify key components are in the message
		assert!(message.contains("PUMP DETECTED — BTC/USDT"));
		assert!(message.contains("52500.00 USDT"));
		assert!(message.contains("+5.0% in 10m"));
		assert!(message.contains("x3.1 vs average"));
		assert!(message.contains("Open Interest: +11.0%"));
		assert!(message.contains("Funding: 3.100%"));
		assert!(message.contains("Long / Short: 71% / 29%"));
		assert!(message.contains("Technical context:"));
		assert!(message.contains("Coinglass"));
	}
}
