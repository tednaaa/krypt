use teloxide::{
	prelude::*,
	types::{MessageId, ParseMode, ThreadId},
};

use crate::config::TelegramConfig;

pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

impl TelegramBot {
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);
		Self { bot, config }
	}

	pub async fn send_alert(&self) -> anyhow::Result<()> {
		let chat_id = self.config.chat_id.clone();
		let mut request = self.bot.send_message(chat_id, "Привет всем кто не спит");

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await.map_err(|error| anyhow::anyhow!("Failed to send alert: {error}"))?;

		Ok(())
	}
}
