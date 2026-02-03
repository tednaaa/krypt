use exchanges::{BinanceExchange, Exchange};

use crate::mfi::calculate_mfi;

mod mfi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let binance = BinanceExchange::new();

	let usdt_pairs = binance.get_all_usdt_pairs().await?;

	println!("{:?}", usdt_pairs.len());

	// let test_symbol = "TRADOORUSDT";
	// let candles = binance.get_klines(test_symbol, "4h", 100).await?;

	// if let Some(mfi_signal) = calculate_mfi(&candles, 14) {
	// 	println!("{mfi_signal:?}");
	// }

	Ok(())
}
