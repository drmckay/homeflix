//! TMDB Cache Implementation
//!
//! Provides specialized caching for TMDB lookups including IMDB-to-TMDB mappings.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::domain::repositories::CacheRepository;
use crate::shared::error::RepositoryError;

/// Cached TMDB entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCacheEntry {
    /// TMDB ID
    pub tmdb_id: i64,
    /// Media type (movie or tv)
    pub media_type: String,
}

/// TMDB cache for external ID lookups
///
/// This cache wraps a CacheRepository to provide type-safe
/// IMDB-to-TMDB and other external ID lookups.
pub struct TmdbCache<C: CacheRepository> {
    cache: Arc<C>,
    /// Default TTL in seconds (7 days)
    default_ttl: u64,
}

impl<C: CacheRepository> TmdbCache<C> {
    /// Creates a new TMDB cache
    ///
    /// # Arguments
    /// * `cache` - Underlying cache repository
    pub fn new(cache: Arc<C>) -> Self {
        Self {
            cache,
            default_ttl: 7 * 24 * 60 * 60, // 7 days
        }
    }

    /// Creates a new TMDB cache with custom TTL
    ///
    /// # Arguments
    /// * `cache` - Underlying cache repository
    /// * `ttl_days` - TTL in days
    pub fn with_ttl(cache: Arc<C>, ttl_days: u32) -> Self {
        Self {
            cache,
            default_ttl: u64::from(ttl_days) * 24 * 60 * 60,
        }
    }

    /// Generates cache key for IMDB lookup
    fn imdb_key(imdb_id: &str) -> String {
        format!("tmdb:imdb:{}", imdb_id)
    }

    /// Generates cache key for TVDB lookup
    fn tvdb_key(tvdb_id: i64) -> String {
        format!("tmdb:tvdb:{}", tvdb_id)
    }

    /// Lookup IMDB ID in cache
    ///
    /// # Arguments
    /// * `imdb_id` - IMDB ID (e.g., "tt1234567")
    ///
    /// # Returns
    /// * `Result<Option<TmdbCacheEntry>, RepositoryError>` - Cached entry if found
    pub async fn lookup_imdb(&self, imdb_id: &str) -> Result<Option<TmdbCacheEntry>, RepositoryError> {
        let key = Self::imdb_key(imdb_id);
        match self.cache.get(&key).await? {
            Some(value) => {
                let entry: TmdbCacheEntry = serde_json::from_str(&value)
                    .map_err(|e| RepositoryError::Deserialization(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Store IMDB to TMDB mapping
    ///
    /// # Arguments
    /// * `imdb_id` - IMDB ID (e.g., "tt1234567")
    /// * `tmdb_id` - TMDB ID
    /// * `media_type` - Media type ("movie" or "tv")
    pub async fn store_imdb_mapping(
        &self,
        imdb_id: &str,
        tmdb_id: i64,
        media_type: &str,
    ) -> Result<(), RepositoryError> {
        let key = Self::imdb_key(imdb_id);
        let entry = TmdbCacheEntry {
            tmdb_id,
            media_type: media_type.to_string(),
        };
        let value = serde_json::to_string(&entry)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.cache.set(&key, &value, self.default_ttl).await
    }

    /// Store IMDB to TMDB mapping with custom TTL
    ///
    /// # Arguments
    /// * `imdb_id` - IMDB ID (e.g., "tt1234567")
    /// * `tmdb_id` - TMDB ID
    /// * `media_type` - Media type ("movie" or "tv")
    /// * `ttl_days` - TTL in days
    pub async fn store_imdb_mapping_with_ttl(
        &self,
        imdb_id: &str,
        tmdb_id: i64,
        media_type: &str,
        ttl_days: u32,
    ) -> Result<(), RepositoryError> {
        let key = Self::imdb_key(imdb_id);
        let entry = TmdbCacheEntry {
            tmdb_id,
            media_type: media_type.to_string(),
        };
        let value = serde_json::to_string(&entry)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        let ttl = u64::from(ttl_days) * 24 * 60 * 60;
        self.cache.set(&key, &value, ttl).await
    }

    /// Lookup TVDB ID in cache
    ///
    /// # Arguments
    /// * `tvdb_id` - TVDB ID
    ///
    /// # Returns
    /// * `Result<Option<TmdbCacheEntry>, RepositoryError>` - Cached entry if found
    pub async fn lookup_tvdb(&self, tvdb_id: i64) -> Result<Option<TmdbCacheEntry>, RepositoryError> {
        let key = Self::tvdb_key(tvdb_id);
        match self.cache.get(&key).await? {
            Some(value) => {
                let entry: TmdbCacheEntry = serde_json::from_str(&value)
                    .map_err(|e| RepositoryError::Deserialization(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Store TVDB to TMDB mapping
    ///
    /// # Arguments
    /// * `tvdb_id` - TVDB ID
    /// * `tmdb_id` - TMDB ID
    /// * `media_type` - Media type ("movie" or "tv")
    pub async fn store_tvdb_mapping(
        &self,
        tvdb_id: i64,
        tmdb_id: i64,
        media_type: &str,
    ) -> Result<(), RepositoryError> {
        let key = Self::tvdb_key(tvdb_id);
        let entry = TmdbCacheEntry {
            tmdb_id,
            media_type: media_type.to_string(),
        };
        let value = serde_json::to_string(&entry)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.cache.set(&key, &value, self.default_ttl).await
    }

    /// Clear all TMDB cache entries
    pub async fn clear_tmdb_cache(&self) -> Result<(), RepositoryError> {
        // Find all TMDB-related keys
        let keys = self.cache.find_keys("tmdb:").await?;
        if !keys.is_empty() {
            let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
            self.cache.delete_many(&key_refs).await?;
        }
        Ok(())
    }

    /// Get count of TMDB cache entries
    pub async fn tmdb_cache_count(&self) -> Result<usize, RepositoryError> {
        let keys = self.cache.find_keys("tmdb:").await?;
        Ok(keys.len())
    }
}
