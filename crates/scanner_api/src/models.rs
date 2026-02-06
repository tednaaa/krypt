use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct PairSnapshot {
	pub icon: String,
	pub pair: String,
	pub mfi_1h: f64,
	pub mfi_4h: f64,
	pub mfi_1d: f64,
	pub mfi_1w: f64,
	pub is_favorite: bool,
	pub comments: Vec<String>,
	pub updated_at: DateTime<Utc>,
}

impl PairSnapshot {
	#[must_use]
	pub fn new(pair: String, icon: String, updated_at: DateTime<Utc>) -> Self {
		Self {
			icon,
			pair,
			mfi_1h: 0.0,
			mfi_4h: 0.0,
			mfi_1d: 0.0,
			mfi_1w: 0.0,
			is_favorite: false,
			comments: Vec::new(),
			updated_at,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct PairUpdate {
	pub mfi_1h: Option<f64>,
	pub mfi_4h: Option<f64>,
	pub mfi_1d: Option<f64>,
	pub mfi_1w: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PairResponse {
	pub icon: String,
	pub pair: String,
	pub mfi_1h: f64,
	pub mfi_4h: f64,
	pub mfi_1d: f64,
	pub mfi_1w: f64,
	pub is_favorite: bool,
	pub comments: Vec<String>,
}

impl From<&PairSnapshot> for PairResponse {
	fn from(snapshot: &PairSnapshot) -> Self {
		let format_mfi = |value: f64| (value * 100.0).round() / 100.0;

		Self {
			icon: snapshot.icon.clone(),
			pair: snapshot.pair.clone(),
			mfi_1h: format_mfi(snapshot.mfi_1h),
			mfi_4h: format_mfi(snapshot.mfi_4h),
			mfi_1d: format_mfi(snapshot.mfi_1d),
			mfi_1w: format_mfi(snapshot.mfi_1w),
			is_favorite: snapshot.is_favorite,
			comments: snapshot.comments.clone(),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
	Mfi1h,
	Mfi4h,
	Mfi1d,
	Mfi1w,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
	Asc,
	Desc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortField {
	pub key: SortKey,
	pub direction: SortDirection,
}

#[must_use]
pub fn icon_url(pair: &str) -> String {
	let base = pair.strip_suffix("USDT").unwrap_or(pair);
	format!("https://static-app.bb-os.com/icon/{base}.png")
}
