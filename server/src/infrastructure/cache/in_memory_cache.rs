//! In-Memory Cache Implementation
//!
//! Provides an in-memory implementation of CacheRepository interface

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::domain::repositories::{CacheRepository, CacheStats};
use crate::shared::error::RepositoryError;

/// Cache entry with metadata
struct CacheEntry {
    value: String,
    expires_at: Option<i64>,
    created_at: Instant,
    access_count: u64,
}

/// In-memory cache implementation
pub struct InMemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
}

impl InMemoryCache {
    /// Creates a new in-memory cache
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of entries
    /// * `ttl` - Default time to live for entries
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl,
        }
    }

    /// Checks if an entry is expired
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        if let Some(expires_at) = entry.expires_at {
            let now = chrono::Utc::now().timestamp();
            now > expires_at
        } else {
            false
        }
    }

    /// Cleans up expired entries
    async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        let now = chrono::Utc::now().timestamp();

        entries.retain(|_, entry| !self.is_expired(entry));

        if entries.len() < self.entries.read().await.len() {
            // Some entries were removed
            drop(entries);
        }
    }

    /// Evicts entries if cache is full (LRU policy)
    async fn evict_if_needed(&self) {
        let mut entries = self.entries.write().await;

        if entries.len() >= self.max_size {
            // Find oldest entry (least recently accessed)
            let oldest_key = entries
                .iter()
                .min_by_key(|(_, v)| v.access_count)
                .map(|(k, _)| k.clone());

            if let Some(key) = oldest_key {
                entries.remove(&key);
            }
        }
    }
}

#[async_trait]
impl CacheRepository for InMemoryCache {
    async fn get(&self, key: &str) -> Result<Option<String>, RepositoryError> {
        // Read pass
        let value = {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(key) {
                if self.is_expired(entry) {
                    None
                } else {
                    Some(entry.value.clone())
                }
            } else {
                None
            }
        };

        if let Some(v) = value {
            // Update access count (requires write lock)
            let mut entries = self.entries.write().await;
            if let Some(e) = entries.get_mut(key) {
                e.access_count += 1;
            }
            Ok(Some(v))
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: &str, ttl: u64) -> Result<(), RepositoryError> {
        let now = chrono::Utc::now().timestamp();
        let expires_at = Some(now + ttl as i64);

        let mut entries = self.entries.write().await;

        // Evict if needed
        self.evict_if_needed().await;

        entries.insert(key.to_string(), CacheEntry {
            value: value.to_string(),
            expires_at,
            created_at: Instant::now(),
            access_count: 0,
        });

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), RepositoryError> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, RepositoryError> {
        let entries = self.entries.read().await;
        Ok(entries.contains_key(key))
    }

    async fn clear(&self) -> Result<(), RepositoryError> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }

    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<String>>, RepositoryError> {
        let entries = self.entries.read().await;

        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let entry = entries.get(*key);

            if let Some(entry) = entry {
                if self.is_expired(entry) {
                    results.push(None);
                } else {
                    results.push(Some(entry.value.clone()));
                }
            } else {
                results.push(None);
            }
        }

        Ok(results)
    }

    async fn set_many(&self, entries: &[(&str, &str)], ttl: u64) -> Result<(), RepositoryError> {
        let now = chrono::Utc::now().timestamp();
        let expires_at = Some(now + ttl as i64);

        let mut cache_entries = self.entries.write().await;

        // Evict if needed
        self.evict_if_needed().await;

        for (key, value) in entries {
            cache_entries.insert(key.to_string(), CacheEntry {
                value: value.to_string(),
                expires_at,
                created_at: Instant::now(),
                access_count: 0,
            });
        }

        Ok(())
    }

    async fn delete_many(&self, keys: &[&str]) -> Result<(), RepositoryError> {
        let mut entries = self.entries.write().await;

        for key in keys {
            entries.remove(*key);
        }

        Ok(())
    }

    async fn find_keys(&self, pattern: &str) -> Result<Vec<String>, RepositoryError> {
        let entries = self.entries.read().await;

        let mut keys = Vec::new();
        let pattern_lower = pattern.to_lowercase();

        for key in entries.keys() {
            if key.to_lowercase().contains(&pattern_lower) {
                keys.push(key.clone());
            }
        }

        Ok(keys)
    }

    async fn count(&self) -> Result<i64, RepositoryError> {
        let entries = self.entries.read().await;
        Ok(entries.len() as i64)
    }

    async fn get_stats(&self) -> Result<CacheStats, RepositoryError> {
        let entries = self.entries.read().await;
        let now = chrono::Utc::now().timestamp();

        let mut total_size_bytes = 0i64;
        let mut expired_entries = 0i64;

        for entry in entries.values() {
            total_size_bytes += entry.value.len() as i64;

            if self.is_expired(entry) {
                expired_entries += 1;
            }
        }

        Ok(CacheStats {
            total_entries: entries.len() as i64,
            expired_entries,
            total_size_bytes,
            hit_rate: 0.0, // Would need tracking for real implementation
        })
    }

    async fn clear_expired(&self) -> Result<usize, RepositoryError> {
        let mut entries = self.entries.write().await;
        let now = chrono::Utc::now().timestamp();

        let initial_count = entries.len();

        entries.retain(|_, entry| !self.is_expired(entry));

        let removed_count = initial_count - entries.len();

        if removed_count > 0 {
            // Some entries were removed
            drop(entries);
        }

        Ok(removed_count)
    }
}
