//! CacheRepository trait
//!
//! Repository interface for cache data access

use async_trait::async_trait;

/// Repository for cache data access
#[async_trait]
pub trait CacheRepository: Send + Sync {
    /// Gets a value from cache
    async fn get(&self, key: &str) -> Result<Option<String>, crate::shared::error::RepositoryError>;

    /// Sets a value in cache with TTL (time to live in seconds)
    async fn set(&self, key: &str, value: &str, ttl: u64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes a value from cache
    async fn delete(&self, key: &str) -> Result<(), crate::shared::error::RepositoryError>;

    /// Checks if a key exists in cache
    async fn exists(&self, key: &str) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Clears all cache entries
    async fn clear(&self) -> Result<(), crate::shared::error::RepositoryError>;

    /// Gets multiple values from cache
    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<String>>, crate::shared::error::RepositoryError>;

    /// Sets multiple values in cache
    async fn set_many(&self, entries: &[(&str, &str)], ttl: u64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes multiple keys from cache
    async fn delete_many(&self, keys: &[&str]) -> Result<(), crate::shared::error::RepositoryError>;

    /// Gets all keys matching a pattern
    async fn find_keys(&self, pattern: &str) -> Result<Vec<String>, crate::shared::error::RepositoryError>;

    /// Counts total cache entries
    async fn count(&self) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Gets cache statistics
    async fn get_stats(&self) -> Result<CacheStats, crate::shared::error::RepositoryError>;

    /// Clears expired entries
    async fn clear_expired(&self) -> Result<usize, crate::shared::error::RepositoryError>;
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of entries
    pub total_entries: i64,
    /// Number of expired entries
    pub expired_entries: i64,
    /// Total cache size in bytes
    pub total_size_bytes: i64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f32,
}
