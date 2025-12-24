use exchanges::{FundingRateInfo, OpenInterestInfo};
use teloxide::{
	prelude::*,
	types::{MessageId, ParseMode, ThreadId},
};

use crate::config::TelegramConfig;

pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

pub struct TokenAlert {
	pub symbol: String,
	pub funding_rate_info: FundingRateInfo,
	pub open_interest_info: OpenInterestInfo,
}

impl TelegramBot {
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);
		Self { bot, config }
	}

	pub async fn send_alert(&self, token: &TokenAlert) -> anyhow::Result<()> {
		let chat_id = self.config.chat_id.clone();
		let mut request = self.bot.send_message(chat_id, self.format_alert_message(token)).parse_mode(ParseMode::Html);

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await.map_err(|error| anyhow::anyhow!("Failed to send alert: {error}"))?;

		Ok(())
	}

	fn format_alert_message(&self, token: &TokenAlert) -> String {
		let funding = &token.funding_rate_info;
		let oi = &token.open_interest_info;

		let sections = [
			self.format_header(token),
			self.format_funding_info(funding),
			self.format_open_interest_info(oi),
			self.format_footer(&token.symbol),
		];

		sections.join("\n\n")
	}

	fn format_header(&self, token: &TokenAlert) -> String {
		format!("<code>{}</code>", token.symbol)
	}

	fn format_funding_info(&self, funding: &FundingRateInfo) -> String {
		format!(
			"📊 <b>Funding Rate:</b> <code>{:.8}</code> (avg: <code>{:.8}</code>)",
			funding.funding_rate, funding.average_funding_rate
		)
	}

	fn format_open_interest_info(&self, oi: &OpenInterestInfo) -> String {
		let format_value = |value: f64| {
			if value >= 0.0 { format!("+{value:.2}%") } else { format!("{value:.2}%") }
		};

		format!(
			"📈 <b>Open Interest:</b>\n\
         <code>15m: {}</code> \t•\t <code>1h: {}</code> \t•\t <code>4h: {}</code>\n\
         <code>1d: {}</code> \t•\t <code>7d: {}</code> \t•\t <code>30d: {}</code>",
			format_value(oi.open_interest_percent_change_15_minutes),
			format_value(oi.open_interest_percent_change_1_hour),
			format_value(oi.open_interest_percent_change_4_hours),
			format_value(oi.open_interest_percent_change_1_day),
			format_value(oi.open_interest_percent_change_7_days),
			format_value(oi.open_interest_percent_change_30_days)
		)
	}

	fn format_footer(&self, symbol: &str) -> String {
		format!(r#"📊 <a href="https://www.coinglass.com/tv/Binance_{symbol}">Coinglass</a>"#)
	}
}
