use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::Stream;
use std::pin::Pin;

pub mod binance;
pub mod bybit;
pub mod types;

pub use types::*;

/// Stream type for exchange messages
pub type MessageStream = Pin<Box<dyn Stream<Item = ExchangeMessage> + Send>>;

/// Exchange trait for abstracting different cryptocurrency exchanges
#[async_trait]
pub trait Exchange: Send + Sync {
	/// Returns the exchange name
	fn name(&self) -> &'static str;

	/// Returns the list of symbols to track
	async fn symbols(&self) -> Result<Vec<Symbol>>;

	/// Connects to WebSocket and streams candles for multiple symbols
	async fn stream_candles(&self, symbols: &[Symbol], intervals: &[&str]) -> Result<MessageStream>;

	/// Fetches derivatives metrics (OI, funding, long/short ratio) via REST
	async fn fetch_derivatives_metrics(&self, symbol: &Symbol) -> Result<DerivativesMetrics>;

	/// Fetches historical candles for pivot calculation
	async fn fetch_historical_candles(&self, symbol: &Symbol, interval: &str, limit: u32) -> Result<Vec<Candle>>;

	/// Checks if exchange supports the given symbol
	fn supports_symbol(&self, symbol: &Symbol) -> bool {
		symbol.exchange == self.name()
	}
}

/// Helper function to create exchange instances
pub fn create_exchange(name: &str, config: &crate::config::Config) -> Result<Box<dyn Exchange>> {
	match name.to_lowercase().as_str() {
		"binance" => Ok(Box::new(binance::BinanceExchange::new(config.binance.clone())?)),
		"bybit" => Ok(Box::new(bybit::BybitExchange::new(config.bybit.clone())?)),
		_ => anyhow::bail!("Unsupported exchange: {}", name),
	}
}
