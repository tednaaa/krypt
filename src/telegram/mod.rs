use crate::config::TelegramConfig;
use crate::pump_scanner::{PumpCandidate, SignalAnalysis};
use anyhow::{Context, Result};
use teloxide::{
	prelude::*,
	types::{MessageId, ParseMode, ThreadId},
};
use tracing::{error, info};

pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

impl TelegramBot {
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);
		Self { bot, config }
	}

	pub async fn post_alert(&self, candidate: &PumpCandidate, analysis: &SignalAnalysis) -> Result<()> {
		let message = self.format_alert_message(candidate, analysis);

		let chat_id = self.config.chat_id.parse::<i64>().context("Invalid chat_id format")?;

		let mut request = self.bot.send_message(ChatId(chat_id), message).parse_mode(ParseMode::Html);

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
					score = analysis.total_score,
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

	fn format_alert_message(&self, candidate: &PumpCandidate, analysis: &SignalAnalysis) -> String {
		let symbol_display = format!("{}/{}", candidate.symbol.base, candidate.symbol.quote);
		let price = candidate.current_price;
		let change_pct = candidate.price_change.change_pct;
		let time_mins = candidate.price_change.time_elapsed_mins;

		let oi_str = analysis.open_interest.increase_pct.map_or_else(
			|| {
				analysis
					.open_interest
					.value
					.map_or_else(|| "Open Interest: N/A".to_string(), |value| format!("Open Interest: {value:.2}"))
			},
			|increase| {
				format!(
					"Open Interest: +{increase:.1}%{}",
					if analysis.open_interest.is_overheated { " ✅ +1 for short" } else { "" }
				)
			},
		);

		let funding_str = analysis.funding_rate.value.map_or_else(
			|| "Funding Rate: N/A".to_string(),
			|rate| {
				format!(
					"Funding Rate: {:.3}%{}",
					rate * 100.0,
					if analysis.funding_rate.is_overheated { " ✅ +1 for short" } else { "" }
				)
			},
		);

		let ls_str = if let (Some(long), Some(short)) = (analysis.long_short_ratio.long_pct, analysis.long_short_ratio.short_pct) {
			format!(
				"Longs: {:.0}% - Shorts: {:.0}%{}",
				long,
				short,
				if analysis.long_short_ratio.is_overheated { " ✅ +1 for short" } else { "" }
			)
		} else {
			"Longs/Shorts: N/A".to_string()
		};

		let volume_str = format!(
			"Volume: {:.1}x{}",
			analysis.volume.ratio,
			if analysis.volume.is_significant { " ✅ significant" } else { "" }
		);

		let ema_str = analysis.ema_status.ema50_distance.map_or_else(
			|| "EMA: N/A".to_string(),
			|ema50| {
				let mut parts = vec![format!("EMA50: +{ema50:.1}%")];
				if let Some(ema200) = analysis.ema_status.ema200_distance {
					parts.push(format!("EMA200: +{ema200:.1}%"));
				}
				if analysis.ema_status.is_extended {
					parts.push("✅ +1 for short".to_string());
				}
				format!("EMA: {}", parts.join(", "))
			},
		);

		let pivot_str = analysis.pivot_status.level.as_ref().map_or_else(
			|| "Pivot: N/A".to_string(),
			|level| {
				format!(
					"Pivot: {level}{}",
					if analysis.pivot_status.is_near_resistance { " ✅ +1 for short" } else { "" }
				)
			},
		);

		let coinglass_url = format!("https://www.coinglass.com/tv/{}{}", candidate.symbol.base, candidate.symbol.quote);

		format!(
			"🚨 <b>PUMP DETECTED — {symbol_display}</b>\n\
			\n\
			<b>Price:</b> {price:.2} USDT (+{change_pct:.1}% in {time_mins}m)\n\
			<b>Short Score:</b> {}/6 ⭐️\n\
			\n\
			{oi_str}\n\
			{funding_str}\n\
			{ls_str}\n\
			{volume_str}\n\
			{ema_str}\n\
			{pivot_str}\n\
			\n\
			🔗 <a href=\"{coinglass_url}\">Coinglass</a>",
			analysis.total_score
		)
	}

	pub async fn test_connection(&self) -> Result<()> {
		let chat_id = self.config.chat_id.parse::<i64>().context("Invalid chat_id format")?;

		let mut request = self.bot.send_message(ChatId(chat_id), "🤖 Pump Scanner Bot initialized");

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
	use crate::pump_scanner::detector::PumpCandidate;
	use crate::pump_scanner::tracker::PriceChange;

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
		use crate::pump_scanner::analysis::*;

		let config = TelegramConfig {
			bot_token: "test_token".to_string(),
			chat_id: "123456".to_string(),
			pump_screener_topic_id: None,
			alert_cooldown_secs: 300,
		};

		let bot = TelegramBot::new(config);

		let candidate = PumpCandidate {
			symbol: Symbol::new("BTC", "USDT", "binance"),
			price_change: PriceChange {
				start_price: 50000.0,
				change_pct: 5.0,
				time_elapsed_mins: 10,
			},
			volume_ratio: 3.1,
			current_price: 52500.0,
		};

		let analysis = SignalAnalysis {
			open_interest: OpenInterestSignal { value: Some(1_000_000.0), increase_pct: Some(11.0), is_overheated: true },
			funding_rate: FundingRateSignal { value: Some(0.031), is_overheated: true },
			long_short_ratio: LongShortSignal { long_pct: Some(71.0), short_pct: Some(29.0), is_overheated: true },
			volume: VolumeSignal { ratio: 3.1, is_significant: true },
			ema_status: EmaSignal { ema50_distance: Some(2.5), ema200_distance: Some(5.1), is_extended: true },
			pivot_status: PivotSignal { level: Some("R1".to_string()), is_near_resistance: true },
			total_score: 6,
		};

		let message = bot.format_alert_message(&candidate, &analysis);

		// Verify key components are in the message
		assert!(message.contains("PUMP DETECTED — BTC/USDT"), "Missing PUMP DETECTED header");
		assert!(message.contains("52500.00 USDT"), "Missing price");
		assert!(message.contains("+5.0% in 10m"), "Missing price change");
		assert!(message.contains("6/6"), "Missing score");
		assert!(message.contains("Open Interest: +11.0%"), "Missing OI");
		assert!(message.contains("Funding Rate: 3.100%"), "Missing funding");
		assert!(message.contains("Longs: 71% - Shorts: 29%"), "Missing L/S ratio");
		assert!(message.contains("Volume: 3.1x"), "Missing volume");
		assert!(message.contains("Coinglass"), "Missing coinglass link");
	}
}
