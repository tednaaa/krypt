use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use tokio::fs;
use tokio::sync::RwLock;

use crate::models::{PairSnapshot, PairUpdate, icon_url};

#[derive(Clone)]
pub struct AppState {
	pairs: Arc<RwLock<HashMap<String, PairSnapshot>>>,
	storage_path: Arc<PathBuf>,
}

impl AppState {
	#[must_use]
	pub async fn load(storage_path: impl Into<PathBuf>) -> Result<Self> {
		let storage_path = storage_path.into();
		let pairs = match fs::read_to_string(&storage_path).await {
			Ok(contents) => {
				if contents.trim().is_empty() {
					HashMap::new()
				} else {
					serde_json::from_str(&contents).context("Failed to parse state.json")?
				}
			},
			Err(err) if err.kind() == ErrorKind::NotFound => HashMap::new(),
			Err(err) => {
				return Err(err).with_context(|| format!("Failed to read state from {}", storage_path.display()));
			},
		};

		Ok(Self { pairs: Arc::new(RwLock::new(pairs)), storage_path: Arc::new(storage_path) })
	}

	pub async fn persist(&self) -> Result<()> {
		let payload = {
			let pairs = self.pairs.read().await;
			serde_json::to_string_pretty(&*pairs).context("Failed to serialize state")?
		};

		fs::write(&*self.storage_path, payload)
			.await
			.with_context(|| format!("Failed to write state to {}", self.storage_path.display()))?;

		Ok(())
	}

	pub async fn list_pairs(&self) -> Vec<PairSnapshot> {
		let pairs = self.pairs.read().await;
		pairs.values().cloned().collect()
	}

	pub async fn apply_update(&self, pair: String, icon: String, update: PairUpdate, updated_at: DateTime<Utc>) {
		let mut pairs = self.pairs.write().await;
		let entry = pairs.entry(pair.clone()).or_insert_with(|| PairSnapshot::new(pair, icon.clone(), updated_at));

		entry.icon = icon;
		if let Some(value) = update.price {
			entry.price = value;
		}
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

	pub async fn favorite_pair(&self, pair: &str) -> Result<PairSnapshot> {
		let snapshot = {
			let mut pairs = self.pairs.write().await;
			let entry = pairs
				.entry(pair.to_string())
				.or_insert_with(|| PairSnapshot::new(pair.to_string(), icon_url(pair), Utc::now()));
			entry.is_favorite = true;
			entry.clone()
		};

		self.persist().await?;

		Ok(snapshot)
	}

	pub async fn unfavorite_pair(&self, pair: &str) -> Result<Option<PairSnapshot>> {
		let snapshot = {
			let mut pairs = self.pairs.write().await;
			let Some(entry) = pairs.get_mut(pair) else {
				return Ok(None);
			};
			entry.is_favorite = false;
			Some(entry.clone())
		};

		if snapshot.is_some() {
			self.persist().await?;
		}

		Ok(snapshot)
	}

	pub async fn add_comment(&self, pair: &str, comment: String) -> Result<PairSnapshot> {
		let snapshot = {
			let mut pairs = self.pairs.write().await;
			let entry = pairs
				.entry(pair.to_string())
				.or_insert_with(|| PairSnapshot::new(pair.to_string(), icon_url(pair), Utc::now()));
			entry.comments.push(comment);
			entry.clone()
		};

		self.persist().await?;

		Ok(snapshot)
	}

	pub async fn remove_comment(&self, pair: &str, comment: &str) -> Result<Option<PairSnapshot>> {
		let snapshot = {
			let mut pairs = self.pairs.write().await;
			let Some(entry) = pairs.get_mut(pair) else {
				return Ok(None);
			};
			if let Some(index) = entry.comments.iter().position(|value| value == comment) {
				entry.comments.remove(index);
				Some(entry.clone())
			} else {
				None
			}
		};

		if snapshot.is_some() {
			self.persist().await?;
		}

		Ok(snapshot)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	fn temp_state_path() -> PathBuf {
		let mut path = std::env::temp_dir();
		let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_nanos()).unwrap_or(0);
		path.push(format!("scanner_api_state_{nanos}.json"));
		path
	}

	#[tokio::test]
	async fn favorite_and_unfavorite() {
		let path = temp_state_path();
		let state = AppState::load(&path).await.unwrap();

		let favored = state.favorite_pair("XTZUSDT").await.unwrap();
		assert!(favored.is_favorite);

		let unfavored = state.unfavorite_pair("XTZUSDT").await.unwrap();
		assert!(unfavored.is_some());
		assert!(!unfavored.unwrap().is_favorite);

		let _ = std::fs::remove_file(path);
	}

	#[tokio::test]
	async fn add_and_remove_comment() {
		let path = temp_state_path();
		let state = AppState::load(&path).await.unwrap();

		let updated = state.add_comment("XTZUSDT", "hello".to_string()).await.unwrap();
		assert_eq!(updated.comments, vec!["hello"]);

		let removed = state.remove_comment("XTZUSDT", "hello").await.unwrap();
		assert!(removed.is_some());
		assert!(removed.unwrap().comments.is_empty());

		let _ = std::fs::remove_file(path);
	}

	#[tokio::test]
	async fn persists_state_to_disk() {
		let path = temp_state_path();
		let state = AppState::load(&path).await.unwrap();

		state.favorite_pair("ETHUSDT").await.unwrap();
		state.add_comment("ETHUSDT", "watch".to_string()).await.unwrap();

		let restored = AppState::load(&path).await.unwrap();
		let pairs = restored.list_pairs().await;
		let snapshot = pairs.iter().find(|pair| pair.pair == "ETHUSDT").unwrap();
		assert!(snapshot.is_favorite);
		assert_eq!(snapshot.comments, vec!["watch"]);

		let _ = std::fs::remove_file(path);
	}
}
