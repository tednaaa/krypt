use futures::stream::{self, StreamExt};

use exchanges::{BinanceExchange, Exchange};

use crate::mfi::calculate_mfi;

mod mfi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let binance = BinanceExchange::new();

	let usdt_pairs = binance.get_all_usdt_pairs().await?;

	stream::iter(usdt_pairs)
		.for_each_concurrent(10, |pair| {
			let binance = binance.clone();
			async move {
				match binance.get_klines(&pair, "1d", 100).await {
					Ok(candles) => {
						if let Some(mfi_signal) = calculate_mfi(&candles, 14) {
							println!("{pair} | {mfi_signal:?}");
						}
					},
					Err(e) => eprintln!("Error fetching {pair}: {e}"),
				}
			}
		})
		.await;

	Ok(())
}
