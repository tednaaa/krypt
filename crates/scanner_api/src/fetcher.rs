use std::time::Duration;

use anyhow::{Context, anyhow};
use chrono::Utc;
use exchanges::{BinanceExchange, Exchange};
use futures::stream::{self, StreamExt};

use crate::mfi::calculate_mfi;
use crate::models::{PairUpdate, icon_url};
use crate::state::AppState;

const KLINE_LIMIT: u32 = 100;
const MFI_LENGTH: usize = 14;
const PAIR_CONCURRENCY: usize = 10;
const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60);

struct MfiSnapshot {
	value: f64,
	price: f64,
}

pub fn spawn_refresh_loop(state: AppState, binance: BinanceExchange) {
	tokio::spawn(async move {
		let mut interval = tokio::time::interval(REFRESH_INTERVAL);
		interval.tick().await;

		loop {
			if let Err(err) = refresh_pairs(&state, &binance).await {
				eprintln!("Failed to refresh pairs: {err}");
			}

			interval.tick().await;
		}
	});
}

async fn refresh_pairs(state: &AppState, binance: &BinanceExchange) -> anyhow::Result<()> {
	let pairs = binance.get_all_usdt_pairs().await.context("Failed to fetch USDT pairs")?;

	stream::iter(pairs)
		.for_each_concurrent(PAIR_CONCURRENCY, |pair| async move {
			if let Err(err) = refresh_pair(state, binance, &pair).await {
				eprintln!("Failed to refresh {pair}: {err}");
			}
		})
		.await;

	state.persist().await.context("Failed to persist refreshed state")?;

	Ok(())
}

async fn refresh_pair(state: &AppState, binance: &BinanceExchange, pair: &str) -> anyhow::Result<()> {
	let icon = icon_url(pair);
	let updated_at = Utc::now();

	let mut update = PairUpdate::default();

	match fetch_mfi(binance, pair, "1h").await {
		Ok(snapshot) => {
			update.mfi_1h = Some(snapshot.value);
			update.price = Some(snapshot.price);
		},
		Err(err) => eprintln!("MFI 1h fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "4h").await {
		Ok(snapshot) => update.mfi_4h = Some(snapshot.value),
		Err(err) => eprintln!("MFI 4h fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "1d").await {
		Ok(snapshot) => update.mfi_1d = Some(snapshot.value),
		Err(err) => eprintln!("MFI 1d fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "1w").await {
		Ok(snapshot) => update.mfi_1w = Some(snapshot.value),
		Err(err) => eprintln!("MFI 1w fetch failed for {pair}: {err}"),
	}

	state.apply_update(pair.to_string(), icon, update, updated_at).await;
	Ok(())
}

async fn fetch_mfi(binance: &BinanceExchange, pair: &str, interval: &str) -> anyhow::Result<MfiSnapshot> {
	let candles = binance
		.get_klines(pair, interval, KLINE_LIMIT)
		.await
		.with_context(|| format!("Failed to fetch klines for {pair} ({interval})"))?;

	let price = candles
		.last()
		.map(|candle| candle.open)
		.ok_or_else(|| anyhow!("No candle data for {pair} ({interval})"))?;
	let value = calculate_mfi(&candles, MFI_LENGTH).ok_or_else(|| anyhow!("Insufficient candle data for {pair} ({interval})"))?;

	Ok(MfiSnapshot { value, price })
}
