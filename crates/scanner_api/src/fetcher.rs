use std::time::Duration;

use anyhow::{Context, anyhow};
use chrono::Utc;
use futures::stream::{self, StreamExt};
use exchanges::{BinanceExchange, Exchange};

use crate::mfi::calculate_mfi;
use crate::models::PairUpdate;
use crate::state::AppState;

const KLINE_LIMIT: u32 = 100;
const MFI_LENGTH: usize = 14;
const PAIR_CONCURRENCY: usize = 10;
const REFRESH_INTERVAL: Duration = Duration::from_secs(10 * 60);

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

	Ok(())
}

async fn refresh_pair(state: &AppState, binance: &BinanceExchange, pair: &str) -> anyhow::Result<()> {
	let icon = icon_url(pair);
	let updated_at = Utc::now();

	let mut update = PairUpdate::default();

	match fetch_mfi(binance, pair, "1h").await {
		Ok(value) => update.mfi_1h = Some(value),
		Err(err) => eprintln!("MFI 1h fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "4h").await {
		Ok(value) => update.mfi_4h = Some(value),
		Err(err) => eprintln!("MFI 4h fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "1d").await {
		Ok(value) => update.mfi_1d = Some(value),
		Err(err) => eprintln!("MFI 1d fetch failed for {pair}: {err}"),
	}

	match fetch_mfi(binance, pair, "1w").await {
		Ok(value) => update.mfi_1w = Some(value),
		Err(err) => eprintln!("MFI 1w fetch failed for {pair}: {err}"),
	}

	state.apply_update(pair.to_string(), icon, update, updated_at).await;
	Ok(())
}

async fn fetch_mfi(binance: &BinanceExchange, pair: &str, interval: &str) -> anyhow::Result<f64> {
	let candles = binance
		.get_klines(pair, interval, KLINE_LIMIT)
		.await
		.with_context(|| format!("Failed to fetch klines for {pair} ({interval})"))?;

	calculate_mfi(&candles, MFI_LENGTH)
		.ok_or_else(|| anyhow!("Insufficient candle data for {pair} ({interval})"))
}

fn icon_url(pair: &str) -> String {
	let base = pair.strip_suffix("USDT").unwrap_or(pair);
	format!("https://cdn.coinglasscdn.com/static/img/coins/{base}.png")
}
