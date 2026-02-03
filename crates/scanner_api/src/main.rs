use actix_web::{App, HttpServer, web};
use exchanges::BinanceExchange;

use crate::api::{add_comment, favorite_pair, get_pairs, remove_comment, unfavorite_pair};
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
			.route("/favorites/{pair}", web::post().to(favorite_pair))
			.route("/favorites/{pair}", web::delete().to(unfavorite_pair))
			.route("/comments/{pair}", web::post().to(add_comment))
			.route("/comments/{pair}", web::delete().to(remove_comment))
	})
	.bind(("0.0.0.0", 8080))?
	.run()
	.await?;

	Ok(())
}
