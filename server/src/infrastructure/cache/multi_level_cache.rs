//! Multi-Level Cache Implementation
//!
//! Provides a multi-level cache combining in-memory (L1) and database (L2) caches

use async_trait::async_trait;
use std::sync::Arc;
use crate::domain::repositories::{CacheRepository, CacheStats};
use crate::shared::error::RepositoryError;
use crate::infrastructure::cache::{InMemoryCache, DatabaseCache};

/// Multi-level cache with L1 (in-memory) and L2 (database)
pub struct MultiLevelCache {
    l1: Arc<InMemoryCache>,
    l2: Arc<DatabaseCache>,
}

impl MultiLevelCache {
    /// Creates a new multi-level cache
    ///
    /// # Arguments
    /// * `l1` - L1 in-memory cache
    /// * `l2` - L2 database cache
    pub fn new(l1: Arc<InMemoryCache>, l2: Arc<DatabaseCache>) -> Self {
        Self { l1, l2 }
    }
}

#[async_trait]
impl CacheRepository for MultiLevelCache {
    /// Try to get from L1 cache first, then L2
    async fn get(&self, key: &str) -> Result<Option<String>, RepositoryError> {
        // Try L1 first
        if let Ok(Some(value)) = self.l1.get(key).await {
            return Ok(Some(value));
        }

        // Fall back to L2
        self.l2.get(key).await
    }

    /// Set in both L1 and L2 caches
    async fn set(&self, key: &str, value: &str, ttl: u64) -> Result<(), RepositoryError> {
        // Set in L1
        self.l1.set(key, value, ttl).await?;

        // Set in L2
        self.l2.set(key, value, ttl).await?;

        Ok(())
    }

    /// Delete from both L1 and L2 caches
    async fn delete(&self, key: &str) -> Result<(), RepositoryError> {
        self.l1.delete(key).await?;
        self.l2.delete(key).await?;

        Ok(())
    }

    /// Check if key exists in either cache
    async fn exists(&self, key: &str) -> Result<bool, RepositoryError> {
        // Check L1 first
        if self.l1.exists(key).await? {
            return Ok(true);
        }

        // Fall back to L2
        self.l2.exists(key).await
    }

    /// Clear both caches
    async fn clear(&self) -> Result<(), RepositoryError> {
        self.l1.clear().await?;
        self.l2.clear().await?;

        Ok(())
    }

    /// Get multiple keys from both caches
    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<String>>, RepositoryError> {
        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            // Try L1 first
            if let Some(value) = self.l1.get(key).await? {
                results.push(Some(value));
                continue;
            }

            // Fall back to L2
            let value = self.l2.get(key).await?;
            results.push(value);
        }

        Ok(results)
    }

    /// Set multiple keys in both caches
    async fn set_many(&self, entries: &[(&str, &str)], ttl: u64) -> Result<(), RepositoryError> {
        for (key, value) in entries {
            // Set in L1
            self.l1.set(key, value, ttl).await?;

            // Set in L2
            self.l2.set(key, value, ttl).await?;
        }

        Ok(())
    }

    /// Delete multiple keys from both caches
    async fn delete_many(&self, keys: &[&str]) -> Result<(), RepositoryError> {
        for key in keys {
            self.l1.delete(key).await?;
            self.l2.delete(key).await?;
        }

        Ok(())
    }

    /// Find keys matching pattern in both caches
    async fn find_keys(&self, pattern: &str) -> Result<Vec<String>, RepositoryError> {
        let mut keys = Vec::new();

        // Get from L1
        let l1_keys = self.l1.find_keys(pattern).await?;
        keys.extend(l1_keys);

        // Get from L2
        let l2_keys = self.l2.find_keys(pattern).await?;
        keys.extend(l2_keys);

        Ok(keys)
    }

    /// Count total entries across both caches
    async fn count(&self) -> Result<i64, RepositoryError> {
        let l1_count = self.l1.count().await?;
        let l2_count = self.l2.count().await?;

        Ok(l1_count + l2_count)
    }

    /// Get combined statistics from both caches
    async fn get_stats(&self) -> Result<CacheStats, RepositoryError> {
        let l1_stats = self.l1.get_stats().await?;
        let l2_stats = self.l2.get_stats().await?;

        Ok(CacheStats {
            total_entries: l1_stats.total_entries + l2_stats.total_entries,
            expired_entries: l1_stats.expired_entries + l2_stats.expired_entries,
            total_size_bytes: l1_stats.total_size_bytes + l2_stats.total_size_bytes,
            hit_rate: (l1_stats.hit_rate + l2_stats.hit_rate) / 2.0,
        })
    }

    /// Clear expired entries from both caches
    async fn clear_expired(&self) -> Result<usize, RepositoryError> {
        let l1_cleared = self.l1.clear_expired().await?;
        let l2_cleared = self.l2.clear_expired().await?;

        Ok(l1_cleared + l2_cleared)
    }
}
