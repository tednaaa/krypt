use exchanges::{MarketLiquidationsInfo, OpenInterestInfo};
use teloxide::{
	prelude::*,
	types::{InputFile, MessageId, ParseMode, ThreadId},
};

use crate::config::TelegramConfig;

#[derive(Clone)]
pub struct TelegramBot {
	bot: Bot,
	config: TelegramConfig,
}

pub struct TokenAlert {
	pub symbol: String,
	pub open_interest_info: OpenInterestInfo,
	pub liquidation_info: MarketLiquidationsInfo,
	pub liquidation_heatmap_screenshot: Vec<u8>,
}

pub struct MarketTickerAlert {
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

impl TelegramBot {
	pub fn new(config: TelegramConfig) -> Self {
		let bot = Bot::new(&config.bot_token);

		Self { bot, config }
	}

	#[allow(dead_code)]
	pub async fn hello(&self) -> anyhow::Result<()> {
		let mut request = self.bot.send_message(self.config.chat_id.clone(), "Hello!");

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await?;

		Ok(())
	}

	pub async fn send_alert(&self, token: &TokenAlert) -> anyhow::Result<()> {
		let chat_id = self.config.chat_id.clone();
		let caption = self.format_alert_message(token);

		let mut request = self
			.bot
			.send_photo(chat_id, InputFile::memory(token.liquidation_heatmap_screenshot.clone()))
			.parse_mode(ParseMode::Html)
			.caption(caption);

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await.map_err(|error| anyhow::anyhow!("Failed to send alert: {error}"))?;

		Ok(())
	}

	pub async fn send_market_ticker_alert(&self, alert: &MarketTickerAlert) -> anyhow::Result<()> {
		let chat_id = self.config.chat_id.clone();
		let text = self.format_market_ticker_alert_message(alert);

		let mut request = self.bot.send_message(chat_id, text).parse_mode(ParseMode::Html);

		if let Some(thread_id) = self.config.thread_id {
			request = request.message_thread_id(ThreadId(MessageId(thread_id)));
		}

		request.await.map_err(|error| anyhow::anyhow!("Failed to send market ticker alert: {error}"))?;
		Ok(())
	}

	fn format_alert_message(&self, token: &TokenAlert) -> String {
		let sections = [
			self.format_header(token),
			self.format_liquidation_info(token),
			self.format_market_stats(token),
			self.format_footer(&token.liquidation_info.symbol),
		];

		sections.join("\n\n")
	}

	fn format_header(&self, token: &TokenAlert) -> String {
		format!("🔔 <code>{}</code> | {}$", token.symbol, token.liquidation_info.symbol_price)
	}

	fn format_liquidation_info(&self, token: &TokenAlert) -> String {
		let liquidation = &token.liquidation_info;

		let side_info = match liquidation.side.as_str() {
			"BUY" => "shorts 🔴",
			"SELL" => "longs 🟢",
			_ => "unknown",
		};

		format!("💥 Liquidated {side_info} | <code>{:.0}$</code>", liquidation.usd_price)
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

	fn format_market_ticker_alert_message(&self, alert: &MarketTickerAlert) -> String {
		let sign = if alert.percent_change_window >= 0.0 { "+" } else { "" };
		let vol_m = alert.volume_multiplier;
		let tier = alert.volume_tier;

		// Keep it HTML-safe by using only simple tags and code blocks.
		// Symbol is from exchange; still treat as plain text inside <code>.
		let header = format!(
			"🔔 <b>{}</b> <code>{}</code> | <b>{}{:.2}%</b> ({}m)",
			alert.direction, alert.symbol, sign, alert.percent_change_window, alert.window_minutes
		);

		let stats = format!(
			"💰 Price: <code>{:.6}</code>\n\
			📊 Volume ({}m est.): <code>{:.0} USDT</code> | <b>x{:.1}</b> (tier x{:.0})\n\
			🧾 Volume (24h): <code>{:.0} USDT</code>",
			alert.price_now, alert.window_minutes, alert.quote_volume_window, vol_m, tier, alert.quote_volume_24h
		);

		let link = format!(r#"🔗 <a href="https://www.binance.com/en/futures/{}">Binance Futures</a>"#, alert.symbol);

		format!("{header}\n\n{stats}\n\n{link}")
	}
}
