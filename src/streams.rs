use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::time::{interval, sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::config::WebSocketConfig;
use crate::types::{StreamMessage, TickerData, TradeData};

pub struct BinanceStreamManager {
	base_url: String,
	ws_config: WebSocketConfig,
}

impl BinanceStreamManager {
	pub fn new(base_url: String, ws_config: WebSocketConfig) -> Self {
		Self { base_url, ws_config }
	}

	/// Connect to the all-market ticker stream
	pub async fn connect_ticker_stream(&self, tx: tokio::sync::mpsc::Sender<StreamMessage>) -> Result<()> {
		let url = format!("{}/!ticker@arr", self.base_url);
		info!("Connecting to ticker stream: {}", url);

		let mut reconnect_delay = self.ws_config.reconnect_base_delay_secs;

		loop {
			match self.run_ticker_stream(&url, tx.clone()).await {
				Ok(_) => {
					info!("Ticker stream ended normally");
					break;
				},
				Err(e) => {
					error!("Ticker stream error: {}. Reconnecting in {}s...", e, reconnect_delay);
					sleep(Duration::from_secs(reconnect_delay)).await;

					// Exponential backoff
					reconnect_delay = (reconnect_delay * 2).min(self.ws_config.reconnect_max_delay_secs);
				},
			}
		}

		Ok(())
	}

	async fn run_ticker_stream(&self, url: &str, tx: tokio::sync::mpsc::Sender<StreamMessage>) -> Result<()> {
		let (ws_stream, _) = connect_async(url).await.context("Failed to connect to ticker stream")?;

		info!("Connected to ticker stream");

		let (mut write, mut read) = ws_stream.split();

		// Spawn ping task
		let ping_interval = self.ws_config.ping_interval_secs;
		tokio::spawn(async move {
			let mut interval = interval(Duration::from_secs(ping_interval));
			loop {
				interval.tick().await;
				if write.send(Message::Ping(vec![])).await.is_err() {
					break;
				}
				debug!("Sent ping to ticker stream");
			}
		});

		// Read messages
		while let Some(msg) = read.next().await {
			let msg = msg.context("Error reading message from ticker stream")?;

			match msg {
				Message::Text(text) => match self.parse_ticker_message(&text) {
					Ok(stream_msg) => {
						if tx.send(stream_msg).await.is_err() {
							warn!("Ticker channel closed, stopping stream");
							break;
						}
					},
					Err(e) => {
						warn!("Failed to parse ticker message: {}", e);
					},
				},
				Message::Ping(payload) => {
					debug!("Received ping, sending pong");
					// Pong is automatically sent by the library
				},
				Message::Pong(_) => {
					debug!("Received pong");
				},
				Message::Close(_) => {
					info!("Received close message from ticker stream");
					break;
				},
				_ => {},
			}
		}

		Ok(())
	}

	fn parse_ticker_message(&self, text: &str) -> Result<StreamMessage> {
		let tickers: Vec<TickerData> = serde_json::from_str(text).context("Failed to parse ticker array")?;

		Ok(StreamMessage::Ticker(tickers))
	}

	/// Connect to a specific symbol's trade stream
	pub async fn connect_trade_stream(&self, symbol: String, tx: tokio::sync::mpsc::Sender<StreamMessage>) -> Result<()> {
		let symbol_lower = symbol.to_lowercase();
		let url = format!("{}/{}@trade", self.base_url, symbol_lower);
		info!("Connecting to trade stream: {}", url);

		let mut reconnect_delay = self.ws_config.reconnect_base_delay_secs;

		loop {
			match self.run_trade_stream(&url, tx.clone()).await {
				Ok(_) => {
					info!("Trade stream for {} ended normally", symbol);
					break;
				},
				Err(e) => {
					error!("Trade stream error for {}: {}. Reconnecting in {}s...", symbol, e, reconnect_delay);
					sleep(Duration::from_secs(reconnect_delay)).await;

					// Exponential backoff
					reconnect_delay = (reconnect_delay * 2).min(self.ws_config.reconnect_max_delay_secs);
				},
			}
		}

		Ok(())
	}

	async fn run_trade_stream(&self, url: &str, tx: tokio::sync::mpsc::Sender<StreamMessage>) -> Result<()> {
		let (ws_stream, _) = connect_async(url).await.context("Failed to connect to trade stream")?;

		debug!("Connected to trade stream");

		let (mut write, mut read) = ws_stream.split();

		// Spawn ping task
		let ping_interval = self.ws_config.ping_interval_secs;
		tokio::spawn(async move {
			let mut interval = interval(Duration::from_secs(ping_interval));
			loop {
				interval.tick().await;
				if write.send(Message::Ping(vec![])).await.is_err() {
					break;
				}
			}
		});

		// Read messages
		while let Some(msg) = read.next().await {
			let msg = msg.context("Error reading message from trade stream")?;

			match msg {
				Message::Text(text) => match self.parse_trade_message(&text) {
					Ok(stream_msg) => {
						if tx.send(stream_msg).await.is_err() {
							warn!("Trade channel closed, stopping stream");
							break;
						}
					},
					Err(e) => {
						warn!("Failed to parse trade message: {}", e);
					},
				},
				Message::Ping(_) => {
					debug!("Received ping on trade stream");
				},
				Message::Pong(_) => {
					debug!("Received pong on trade stream");
				},
				Message::Close(_) => {
					info!("Received close message from trade stream");
					break;
				},
				_ => {},
			}
		}

		Ok(())
	}

	fn parse_trade_message(&self, text: &str) -> Result<StreamMessage> {
		let trade: TradeData = serde_json::from_str(text).context("Failed to parse trade data")?;

		Ok(StreamMessage::Trade(trade))
	}
}

/// Manages dynamic subscriptions to trade streams
pub struct TradeStreamSubscriptionManager {
	base_url: String,
	ws_config: WebSocketConfig,
	active_streams: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl TradeStreamSubscriptionManager {
	pub fn new(base_url: String, ws_config: WebSocketConfig) -> Self {
		Self {
			base_url,
			ws_config,
			active_streams: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
		}
	}

	/// Subscribe to a symbol's trade stream
	pub async fn subscribe(&self, symbol: String, tx: tokio::sync::mpsc::Sender<StreamMessage>) {
		let mut streams = self.active_streams.write().await;

		// Check if already subscribed
		if streams.contains_key(&symbol) {
			debug!("Already subscribed to {}", symbol);
			return;
		}

		info!("Subscribing to trade stream: {}", symbol);

		let manager = BinanceStreamManager::new(self.base_url.clone(), self.ws_config.clone());

		let symbol_clone = symbol.clone();
		let handle = tokio::spawn(async move {
			if let Err(e) = manager.connect_trade_stream(symbol_clone.clone(), tx).await {
				error!("Trade stream task failed for {}: {}", symbol_clone, e);
			}
		});

		streams.insert(symbol, handle);
	}

	/// Unsubscribe from a symbol's trade stream
	pub async fn unsubscribe(&self, symbol: &str) {
		let mut streams = self.active_streams.write().await;

		if let Some(handle) = streams.remove(symbol) {
			info!("Unsubscribing from trade stream: {}", symbol);
			handle.abort();
		}
	}

	/// Update subscriptions based on current Tier 1 symbols
	pub async fn update_subscriptions(&self, tier1_symbols: Vec<String>, tx: tokio::sync::mpsc::Sender<StreamMessage>) {
		let current_symbols: std::collections::HashSet<String> = {
			let streams = self.active_streams.read().await;
			streams.keys().cloned().collect()
		};

		let target_symbols: std::collections::HashSet<String> = tier1_symbols.into_iter().collect();

		// Unsubscribe from symbols no longer in Tier 1
		for symbol in current_symbols.difference(&target_symbols) {
			self.unsubscribe(symbol).await;
		}

		// Subscribe to new Tier 1 symbols
		for symbol in target_symbols.difference(&current_symbols) {
			self.subscribe(symbol.clone(), tx.clone()).await;
		}
	}

	/// Get count of active streams
	pub async fn active_count(&self) -> usize {
		let streams = self.active_streams.read().await;
		streams.len()
	}
}
