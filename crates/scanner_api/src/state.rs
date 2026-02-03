use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::models::{PairSnapshot, PairUpdate};

#[derive(Clone, Default)]
pub struct AppState {
	pairs: Arc<RwLock<HashMap<String, PairSnapshot>>>,
}

impl AppState {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	pub async fn list_pairs(&self) -> Vec<PairSnapshot> {
		let pairs = self.pairs.read().await;
		pairs.values().cloned().collect()
	}

	pub async fn apply_update(
		&self,
		pair: String,
		icon: String,
		update: PairUpdate,
		updated_at: DateTime<Utc>,
	) {
		let mut pairs = self.pairs.write().await;
		let entry = pairs
			.entry(pair.clone())
			.or_insert_with(|| PairSnapshot::new(pair, icon.clone(), updated_at));

		entry.icon = icon;
		if let Some(value) = update.mfi_1h {
			entry.mfi_1h = value;
		}
		if let Some(value) = update.mfi_4h {
			entry.mfi_4h = value;
		}
		if let Some(value) = update.mfi_1d {
			entry.mfi_1d = value;
		}
		if let Some(value) = update.mfi_1w {
			entry.mfi_1w = value;
		}
		entry.updated_at = updated_at;
	}
}
