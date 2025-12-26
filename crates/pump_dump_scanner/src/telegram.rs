use exchanges::{FundingRateInfo, OpenInterestInfo};
use teloxide::{
	prelude::*,
	types::{InputFile, MediaPhoto, MessageId, ParseMode, ThreadId},
};

use crate::config::TelegramConfig;

pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

pub struct TokenAlert {
	pub symbol: String,
	pub open_interest_info: OpenInterestInfo,
}

impl TelegramBot {
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);
		Self { bot, config }
	}

	pub async fn send_alert(&self, token: &TokenAlert, chart_screenshot: Vec<u8>) -> anyhow::Result<()> {
		let chat_id = self.config.chat_id.clone();
		let caption = self.format_alert_message(token);

		let mut request =
			self.bot.send_photo(chat_id, InputFile::memory(chart_screenshot)).caption(caption).parse_mode(ParseMode::Html);

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await.map_err(|error| anyhow::anyhow!("Failed to send alert: {error}"))?;

		Ok(())
	}

	fn format_alert_message(&self, token: &TokenAlert) -> String {
		let sections = [self.format_header(token), self.format_market_stats(token), self.format_footer(&token.symbol)];

		sections.join("\n\n")
	}

	fn format_header(&self, token: &TokenAlert) -> String {
		format!("🔔 Токен <code>{}</code>", token.symbol)
	}

	fn format_market_stats(&self, token: &TokenAlert) -> String {
		let oi = &token.open_interest_info;

		let format = |label: &str, value: f64| {
			let emoji = if value >= 0.0 { "🟩" } else { "🟥" };
			let sign = if value >= 0.0 { "+" } else { "" };
			format!("{emoji}  <code>{sign}{value:.2}% ({label})</code>")
		};

		format!(
			"📈 Open Interest\n\
			{}\n\
			{}\n\
			{}\n\
			{}",
			format("15m", oi.percent_change_15_minutes),
			format("1h", oi.percent_change_1_hour),
			format("4h", oi.percent_change_4_hours),
			format("24h", oi.percent_change_1_day),
		)
	}

	fn format_footer(&self, symbol: &str) -> String {
		format!(r#"📊 <a href="https://www.coinglass.com/tv/Binance_{symbol}">Coinglass</a>"#)
	}
}
