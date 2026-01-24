//! SQLite Cache Repository Implementation
//!
//! Provides SQLite-based implementation of CacheRepository trait

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use crate::domain::repositories::{CacheRepository, CacheStats};
use crate::shared::error::RepositoryError;

/// SQLite implementation of CacheRepository
pub struct SqliteCacheRepository {
    pool: Pool<Sqlite>,
}

impl SqliteCacheRepository {
    /// Creates a new SQLite cache repository
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Checks if a cache entry is expired
    fn is_expired(expires_at: &Option<i64>) -> bool {
        match expires_at {
            Some(timestamp) => {
                let now = chrono::Utc::now().timestamp();
                now > *timestamp
            }
            None => false,
        }
    }
}

#[async_trait]
impl CacheRepository for SqliteCacheRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, RepositoryError> {
        let result = sqlx::query(
            "SELECT value, expires_at FROM cache WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                let expires_at: Option<i64> = row.try_get("expires_at")?;
                if Self::is_expired(&expires_at) {
                    // Delete expired entry and return None
                    sqlx::query("DELETE FROM cache WHERE key = ?")
                        .bind(key)
                        .execute(&self.pool)
                        .await?;
                    Ok(None)
                } else {
                    let value: String = row.try_get("value")?;
                    Ok(Some(value))
                }
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: &str, ttl: u64) -> Result<(), RepositoryError> {
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + ttl as i64;

        sqlx::query(
            "INSERT OR REPLACE INTO cache (key, value, expires_at) VALUES (?, ?, ?)"
        )
        .bind(key)
        .bind(value)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM cache WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "SELECT COUNT(*) as count FROM cache WHERE key = ?"
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn clear(&self) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM cache")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_many(&self, keys: &[&str]) -> Result<Vec<Option<String>>, RepositoryError> {
        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let value = self.get(key).await?;
            results.push(value);
        }

        Ok(results)
    }

    async fn set_many(&self, entries: &[(&str, &str)], ttl: u64) -> Result<(), RepositoryError> {
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + ttl as i64;

        for (key, value) in entries {
            sqlx::query(
                "INSERT OR REPLACE INTO cache (key, value, expires_at) VALUES (?, ?, ?)"
            )
            .bind(key)
            .bind(value)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete_many(&self, keys: &[&str]) -> Result<(), RepositoryError> {
        for key in keys {
            self.delete(key).await?;
        }

        Ok(())
    }

    async fn find_keys(&self, pattern: &str) -> Result<Vec<String>, RepositoryError> {
        let sql_pattern = format!("%{}%", pattern);
        let rows = sqlx::query(
            "SELECT key FROM cache WHERE key LIKE ? ORDER BY key ASC"
        )
        .bind(&sql_pattern)
        .fetch_all(&self.pool)
        .await?;

        let keys: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("key").ok())
            .collect();

        Ok(keys)
    }

    async fn count(&self) -> Result<i64, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM cache")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get("count")?)
    }

    async fn get_stats(&self) -> Result<CacheStats, RepositoryError> {
        let now = chrono::Utc::now().timestamp();

        let total_result = sqlx::query("SELECT COUNT(*) as count FROM cache")
            .fetch_one(&self.pool)
            .await?;

        let expired_result = sqlx::query(
            "SELECT COUNT(*) as count FROM cache WHERE expires_at < ?"
        )
        .bind(now)
        .fetch_one(&self.pool)
            .await?;

        let size_result = sqlx::query("SELECT SUM(LENGTH(value)) as total_size FROM cache")
            .fetch_one(&self.pool)
            .await?;

        let total_entries: i64 = total_result.try_get("count")?;
        let expired_entries: i64 = expired_result.try_get("count")?;
        let total_size_bytes: i64 = size_result.try_get::<Option<i64>, _>("total_size")?.unwrap_or(0);

        // Calculate hit rate (placeholder - in real implementation, track hits/misses)
        let hit_rate = 0.0;

        Ok(CacheStats {
            total_entries,
            expired_entries,
            total_size_bytes,
            hit_rate,
        })
    }

    async fn clear_expired(&self) -> Result<usize, RepositoryError> {
        let now = chrono::Utc::now().timestamp();

        let result = sqlx::query(
            "DELETE FROM cache WHERE expires_at < ?"
        )
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}
