use exchanges::{BinanceExchange, Exchange};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let binance = BinanceExchange::new();
	println!("{:?}", binance.get_funding_rate_info("MINA").await?);

	Ok(())
}
