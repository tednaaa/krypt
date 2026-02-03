use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::models::{PairSnapshot, PairUpdate, icon_url};

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

	pub async fn apply_update(&self, pair: String, icon: String, update: PairUpdate, updated_at: DateTime<Utc>) {
		let mut pairs = self.pairs.write().await;
		let entry = pairs.entry(pair.clone()).or_insert_with(|| PairSnapshot::new(pair, icon.clone(), updated_at));

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

	pub async fn favorite_pair(&self, pair: &str) -> PairSnapshot {
		let mut pairs = self.pairs.write().await;
		let entry =
			pairs.entry(pair.to_string()).or_insert_with(|| PairSnapshot::new(pair.to_string(), icon_url(pair), Utc::now()));
		entry.is_favorite = true;
		entry.clone()
	}

	pub async fn unfavorite_pair(&self, pair: &str) -> Option<PairSnapshot> {
		let mut pairs = self.pairs.write().await;
		let entry = pairs.get_mut(pair)?;
		entry.is_favorite = false;
		Some(entry.clone())
	}

	pub async fn add_comment(&self, pair: &str, comment: String) -> PairSnapshot {
		let mut pairs = self.pairs.write().await;
		let entry =
			pairs.entry(pair.to_string()).or_insert_with(|| PairSnapshot::new(pair.to_string(), icon_url(pair), Utc::now()));
		entry.comments.push(comment);
		entry.clone()
	}

	pub async fn remove_comment(&self, pair: &str, comment: &str) -> Option<PairSnapshot> {
		let mut pairs = self.pairs.write().await;
		let entry = pairs.get_mut(pair)?;
		if let Some(index) = entry.comments.iter().position(|value| value == comment) {
			entry.comments.remove(index);
			return Some(entry.clone());
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn favorite_and_unfavorite() {
		let state = AppState::new();

		let favored = state.favorite_pair("XTZUSDT").await;
		assert!(favored.is_favorite);

		let unfavored = state.unfavorite_pair("XTZUSDT").await;
		assert!(unfavored.is_some());
		assert!(!unfavored.unwrap().is_favorite);
	}

	#[tokio::test]
	async fn add_and_remove_comment() {
		let state = AppState::new();

		let updated = state.add_comment("XTZUSDT", "hello".to_string()).await;
		assert_eq!(updated.comments, vec!["hello"]);

		let removed = state.remove_comment("XTZUSDT", "hello").await;
		assert!(removed.is_some());
		assert!(removed.unwrap().comments.is_empty());
	}
}
