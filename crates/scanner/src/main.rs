use exchanges::{BinanceExchange, Exchange};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let binance = BinanceExchange::new();

	let result = binance.get_klines("ZORAUSDT", "1h", 100).await?;
	println!("{result:?}");

	Ok(())
}
