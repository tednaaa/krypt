use actix_web::{App, HttpServer, web};
use exchanges::BinanceExchange;

use crate::api::get_pairs;
use crate::fetcher::spawn_refresh_loop;
use crate::state::AppState;

mod api;
mod fetcher;
mod mfi;
mod models;
mod state;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
	let state = AppState::new();
	let binance = BinanceExchange::new();

	spawn_refresh_loop(state.clone(), binance);

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(state.clone()))
			.route("/pairs", web::get().to(get_pairs))
	})
	.bind(("0.0.0.0", 8080))?
	.run()
	.await?;

	Ok(())
}
