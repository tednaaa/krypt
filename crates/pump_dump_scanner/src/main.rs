use exchanges::{BinanceExchange, Exchange};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	info!("Starting pump/dump scanner");

	let binance = BinanceExchange::new();

	let test_symbol = "ANIMEUSDT";
	info!("{:?}", binance.get_funding_rate_info(test_symbol).await?);
	info!("{:?}", binance.get_open_interest_info(test_symbol).await?);

	Ok(())
}
