use std::cmp::Ordering;

use actix_web::{Error, HttpResponse, Responder, web};
use serde::Deserialize;

use crate::models::{PairResponse, SortDirection, SortField, SortKey};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PairsQuery {
	pub sort: Option<String>,
}

#[derive(Debug)]
struct SortParseError {
	message: String,
}

impl SortParseError {
	fn new(message: impl Into<String>) -> Self {
		Self { message: message.into() }
	}
}

impl std::fmt::Display for SortParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message)
	}
}

pub async fn get_pairs(state: web::Data<AppState>, query: web::Query<PairsQuery>) -> Result<impl Responder, Error> {
	let sort_fields = match query.sort.as_deref() {
		Some(value) => parse_sort_fields(value).map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?,
		None => Vec::new(),
	};

	let mut pairs: Vec<PairResponse> = state.list_pairs().await.iter().map(PairResponse::from).collect();

	if !sort_fields.is_empty() {
		sort_pairs(&mut pairs, &sort_fields);
	}

	Ok(HttpResponse::Ok().json(pairs))
}

fn parse_sort_fields(raw: &str) -> Result<Vec<SortField>, SortParseError> {
	let mut result = Vec::new();

	for chunk in raw.split(',') {
		let trimmed = chunk.trim();
		if trimmed.is_empty() {
			continue;
		}

		let mut parts = trimmed.split(':');
		let field = parts.next().unwrap_or_default();
		let direction = parts.next();

		let key = parse_sort_key(field)?;
		let direction = match direction {
			Some(value) if !value.is_empty() => parse_sort_direction(value)?,
			_ => SortDirection::Desc,
		};

		if parts.next().is_some() {
			return Err(SortParseError::new(format!("Invalid sort format: {trimmed}")));
		}

		result.push(SortField { key, direction });
	}

	Ok(result)
}

fn parse_sort_key(value: &str) -> Result<SortKey, SortParseError> {
	match value {
		"mfi_1h" => Ok(SortKey::Mfi1h),
		"mfi_4h" => Ok(SortKey::Mfi4h),
		"mfi_1d" => Ok(SortKey::Mfi1d),
		"mfi_1w" => Ok(SortKey::Mfi1w),
		_ => Err(SortParseError::new(format!(
			"Unsupported sort field: {value}. Use mfi_1h, mfi_4h, mfi_1d, or mfi_1w.",
		))),
	}
}

fn parse_sort_direction(value: &str) -> Result<SortDirection, SortParseError> {
	match value {
		"asc" => Ok(SortDirection::Asc),
		"desc" => Ok(SortDirection::Desc),
		_ => Err(SortParseError::new(format!("Unsupported sort direction: {value}. Use asc or desc."))),
	}
}

fn sort_pairs(pairs: &mut [PairResponse], sort_fields: &[SortField]) {
	pairs.sort_by(|left, right| compare_pairs(left, right, sort_fields));
}

fn compare_pairs(left: &PairResponse, right: &PairResponse, sort_fields: &[SortField]) -> Ordering {
	for field in sort_fields {
		let ordering = match field.key {
			SortKey::Mfi1h => compare_f64(left.mfi_1h, right.mfi_1h),
			SortKey::Mfi4h => compare_f64(left.mfi_4h, right.mfi_4h),
			SortKey::Mfi1d => compare_f64(left.mfi_1d, right.mfi_1d),
			SortKey::Mfi1w => compare_f64(left.mfi_1w, right.mfi_1w),
		};

		if ordering != Ordering::Equal {
			return match field.direction {
				SortDirection::Asc => ordering,
				SortDirection::Desc => ordering.reverse(),
			};
		}
	}

	Ordering::Equal
}

fn compare_f64(left: f64, right: f64) -> Ordering {
	left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::models::PairResponse;

	#[test]
	fn parse_sort_fields_multi() {
		let fields = parse_sort_fields("mfi_1h:desc,mfi_4h:asc").unwrap();
		assert_eq!(
			fields,
			vec![
				SortField { key: SortKey::Mfi1h, direction: SortDirection::Desc },
				SortField { key: SortKey::Mfi4h, direction: SortDirection::Asc },
			]
		);
	}

	#[test]
	fn parse_sort_fields_defaults_to_desc() {
		let fields = parse_sort_fields("mfi_1d").unwrap();
		assert_eq!(fields, vec![SortField { key: SortKey::Mfi1d, direction: SortDirection::Desc }]);
	}

	#[test]
	fn parse_sort_fields_rejects_unknown_field() {
		let error = parse_sort_fields("foo:asc").unwrap_err();
		assert!(error.to_string().contains("Unsupported sort field"));
	}

	#[test]
	fn sort_pairs_multi_key() {
		let mut pairs = vec![
			PairResponse {
				icon: "a".to_string(),
				pair: "AAAUSDT".to_string(),
				mfi_1h: 10.0,
				mfi_4h: 60.0,
				mfi_1d: 50.0,
				mfi_1w: 30.0,
			},
			PairResponse {
				icon: "b".to_string(),
				pair: "BBBUSDT".to_string(),
				mfi_1h: 5.0,
				mfi_4h: 60.0,
				mfi_1d: 80.0,
				mfi_1w: 30.0,
			},
			PairResponse {
				icon: "c".to_string(),
				pair: "CCCUSDT".to_string(),
				mfi_1h: 12.0,
				mfi_4h: 20.0,
				mfi_1d: 80.0,
				mfi_1w: 70.0,
			},
		];

		let sort_fields = vec![
			SortField { key: SortKey::Mfi1d, direction: SortDirection::Desc },
			SortField { key: SortKey::Mfi4h, direction: SortDirection::Desc },
			SortField { key: SortKey::Mfi1h, direction: SortDirection::Asc },
		];

		sort_pairs(&mut pairs, &sort_fields);

		assert_eq!(pairs[0].pair, "BBBUSDT");
		assert_eq!(pairs[1].pair, "CCCUSDT");
		assert_eq!(pairs[2].pair, "AAAUSDT");
	}
}
