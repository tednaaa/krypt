use actix_cors::Cors;
use actix_web::http::header;

pub fn build_cors() -> Cors {
	Cors::default()
		.allowed_origin("http://localhost:5173")
		.allowed_origin("http://127.0.0.1:5173")
		.allowed_origin("http://localhost:5174")
		.allowed_origin("http://127.0.0.1:5174")
		.allowed_methods(vec!["GET", "POST", "DELETE"])
		.allowed_headers(vec![header::CONTENT_TYPE, header::ACCEPT])
		.max_age(3600)
}

#[cfg(test)]
mod tests {
	use super::build_cors;
	use actix_web::{
		App, HttpResponse,
		http::{Method, header},
		test, web,
	};

	async fn health() -> HttpResponse {
		HttpResponse::Ok().finish()
	}

	#[actix_web::test]
	async fn allows_localhost_origin_preflight() {
		let app = test::init_service(App::new().wrap(build_cors()).route("/health", web::get().to(health))).await;

		let req = test::TestRequest::default()
			.method(Method::OPTIONS)
			.uri("/health")
			.insert_header((header::ORIGIN, "http://localhost:5173"))
			.insert_header((header::ACCESS_CONTROL_REQUEST_METHOD, "GET"))
			.to_request();

		let resp = test::call_service(&app, req).await;
		assert!(resp.status().is_success());
		let allow_origin = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).expect("missing allow origin header");
		assert_eq!(allow_origin.to_str().unwrap(), "http://localhost:5173");
	}
}
