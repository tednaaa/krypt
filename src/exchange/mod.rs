use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::Stream;
use std::pin::Pin;

pub mod binance;
pub mod bybit;
pub mod types;

pub use types::*;

pub type MessageStream = Pin<Box<dyn Stream<Item = ExchangeMessage> + Send>>;

#[async_trait]
pub trait Exchange: Send + Sync {
	fn name(&self) -> &'static str;

	async fn symbols(&self) -> Result<Vec<Symbol>>;

	async fn stream_prices(&self, symbols: &[Symbol]) -> Result<MessageStream>;

	async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics>;

	async fn fetch_historical_candles(&self, symbol: &Symbol, interval: &str, limit: u32) -> Result<Vec<Candle>>;

	fn format_interval(&self, minutes: u32) -> String {
		minutes.to_string()
	}
}

pub fn create_exchange(name: &str, config: &crate::config::Config) -> Result<Box<dyn Exchange>> {
	match name.to_lowercase().as_str() {
		"binance" => Ok(Box::new(binance::BinanceExchange::new(config.binance.clone())?)),
		"bybit" => Ok(Box::new(bybit::BybitExchange::new(config.bybit.clone())?)),
		_ => anyhow::bail!("Unsupported exchange: {name}"),
	}
}
