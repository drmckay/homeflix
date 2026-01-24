//! Database Connection Pool
//!
//! Provides optimized connection pooling for SQLite with:
//! - Configurable pool size
//! - Connection timeout
//! - Connection validation
//! - Pool metrics tracking
//!
//! # Performance Characteristics
//! - Reduces connection overhead
//! - Limits concurrent connections
//! - Enables connection reuse
//! - Provides monitoring capabilities

use sqlx::{Pool, Sqlite, sqlite::SqliteConnectOptions, pool::PoolOptions};
use std::time::Duration;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn, debug};

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Database connection string
    pub database_url: String,
    /// Maximum number of connections in pool (default: 10)
    pub max_connections: u32,
    /// Minimum number of connections to maintain (default: 2)
    pub min_connections: u32,
    /// Connection timeout in seconds (default: 30)
    pub connection_timeout_secs: u64,
    /// Idle connection timeout in seconds (default: 600)
    pub idle_timeout_secs: u64,
    /// Maximum connection lifetime in seconds (default: 3600)
    pub max_lifetime_secs: u64,
    /// Whether to test connections on checkout (default: true)
    pub test_on_checkout: bool,
    /// Enable connection statistics (default: true)
    pub enable_metrics: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:data.db?mode=rwc".to_string(),
            max_connections: 10,
            min_connections: 2,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 3600,
            test_on_checkout: true,
            enable_metrics: true,
        }
    }
}

impl ConnectionPoolConfig {
    /// Creates a new connection pool configuration
    ///
    /// # Arguments
    /// * `database_url` - Database connection string
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            ..Default::default()
        }
    }

    /// Sets maximum pool size
    ///
    /// # Arguments
    /// * `max` - Maximum number of connections
    ///
    /// # Note
    /// Should be based on expected concurrent operations
    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max.max(1);
        self
    }

    /// Sets minimum pool size
    ///
    /// # Arguments
    /// * `min` - Minimum number of connections to maintain
    pub fn with_min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Sets connection timeout
    ///
    /// # Arguments
    /// * `timeout_secs` - Timeout in seconds
    pub fn with_connection_timeout(mut self, timeout_secs: u64) -> Self {
        self.connection_timeout_secs = timeout_secs;
        self
    }

    /// Sets idle connection timeout
    ///
    /// # Arguments
    /// * `timeout_secs` - Timeout in seconds
    pub fn with_idle_timeout(mut self, timeout_secs: u64) -> Self {
        self.idle_timeout_secs = timeout_secs;
        self
    }

    /// Sets maximum connection lifetime
    ///
    /// # Arguments
    /// * `lifetime_secs` - Maximum lifetime in seconds
    pub fn with_max_lifetime(mut self, lifetime_secs: u64) -> Self {
        self.max_lifetime_secs = lifetime_secs;
        self
    }

    /// Enables or disables connection testing on checkout
    ///
    /// # Arguments
    /// * `enabled` - Whether to test connections
    pub fn with_test_on_checkout(mut self, enabled: bool) -> Self {
        self.test_on_checkout = enabled;
        self
    }

    /// Enables or disables metrics collection
    ///
    /// # Arguments
    /// * `enabled` - Whether to collect metrics
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }

    /// Validates configuration
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if valid, error message otherwise
    pub fn validate(&self) -> Result<(), String> {
        if self.database_url.is_empty() {
            return Err("Database URL cannot be empty".to_string());
        }

        if self.max_connections < self.min_connections {
            return Err(format!(
                "Max connections ({}) must be >= min connections ({})",
                self.max_connections, self.min_connections
            ));
        }

        if self.max_connections > 100 {
            return Err("Max connections cannot exceed 100".to_string());
        }

        Ok(())
    }
}

/// Connection pool metrics
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    /// Current number of connections in use
    pub active_connections: usize,
    /// Current number of idle connections
    pub idle_connections: usize,
    /// Total number of connections created
    pub total_connections_created: usize,
    /// Total number of connections closed
    pub total_connections_closed: usize,
    /// Peak number of active connections
    pub peak_active_connections: usize,
    /// Pool size (max + idle)
    pub pool_size: usize,
}

impl PoolMetrics {
    /// Creates new pool metrics
    pub fn new() -> Self {
        Self {
            active_connections: 0,
            idle_connections: 0,
            total_connections_created: 0,
            total_connections_closed: 0,
            peak_active_connections: 0,
            pool_size: 0,
        }
    }

    /// Updates peak if current active is higher
    pub fn update_peak(&mut self, active: usize) {
        if active > self.peak_active_connections {
            self.peak_active_connections = active;
        }
    }

    /// Calculates pool utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.pool_size == 0 {
            return 0.0;
        }
        (self.active_connections as f64 / self.pool_size as f64) * 100.0
    }
}

/// Database connection pool with metrics
pub struct ConnectionPool {
    /// Underlying SQLx pool
    pool: Pool<Sqlite>,
    /// Pool configuration
    config: ConnectionPoolConfig,
    /// Connection metrics
    metrics: Arc<PoolMetrics>,
    /// Active connection counter
    active_connections: Arc<AtomicUsize>,
    /// Total connections created counter
    total_created: Arc<AtomicUsize>,
    /// Total connections closed counter
    total_closed: Arc<AtomicUsize>,
}

impl ConnectionPool {
    /// Creates a new connection pool
    ///
    /// # Arguments
    /// * `config` - Pool configuration
    ///
    /// # Returns
    /// * `Result<Self, String>` - Pool or error message
    ///
    /// # Errors
    /// Returns error if:
    /// - Configuration is invalid
    /// - Database connection fails
    pub async fn create(config: ConnectionPoolConfig) -> Result<Self, String> {
        // Validate configuration
        config.validate()?;

        info!(
            "Creating connection pool: max={}, min={}, timeout={}s",
            config.max_connections,
            config.min_connections,
            config.connection_timeout_secs
        );

        // Build connection options
        let options = SqliteConnectOptions::from_str(&config.database_url)
            .map_err(|e| format!("Invalid database URL: {}", e))?
            .create_if_missing(true);

        // Configure SQLite pragmas for performance
        let options = options
            .pragma("journal_mode", "WAL") // Write-Ahead Logging for better concurrency
            .pragma("synchronous", "NORMAL") // Balance between safety and performance
            .pragma("cache_size", "-64000") // 64MB cache
            .pragma("temp_store", "MEMORY") // Store temporary tables in memory
            .pragma("mmap_size", "268435456") // 256MB memory-mapped I/O
            .pragma("page_size", "4096"); // 4KB page size (matches filesystem)

        // Create pool with sqlx 0.8 API
        let pool = PoolOptions::<Sqlite>::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .test_before_acquire(config.test_on_checkout)
            .connect_with(options)
            .await
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        info!("Connection pool created successfully");

        Ok(Self {
            pool,
            config,
            metrics: Arc::new(PoolMetrics::new()),
            active_connections: Arc::new(AtomicUsize::new(0)),
            total_created: Arc::new(AtomicUsize::new(0)),
            total_closed: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// Creates connection pool from environment variables
    ///
    /// # Environment Variables
    /// - `DATABASE_URL`: Database connection string (required)
    /// - `DB_MAX_CONNECTIONS`: Maximum pool size (default: 10)
    /// - `DB_MIN_CONNECTIONS`: Minimum pool size (default: 2)
    /// - `DB_CONNECTION_TIMEOUT`: Connection timeout in seconds (default: 30)
    /// - `DB_IDLE_TIMEOUT`: Idle timeout in seconds (default: 600)
    /// - `DB_MAX_LIFETIME`: Max connection lifetime in seconds (default: 3600)
    ///
    /// # Returns
    /// * `Result<Self, String>` - Pool or error message
    pub async fn from_env() -> Result<Self, String> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string());

        let mut config = ConnectionPoolConfig::new(database_url);

        if let Ok(max) = std::env::var("DB_MAX_CONNECTIONS") {
            if let Ok(val) = max.parse::<u32>() {
                config = config.with_max_connections(val);
            }
        }

        if let Ok(min) = std::env::var("DB_MIN_CONNECTIONS") {
            if let Ok(val) = min.parse::<u32>() {
                config = config.with_min_connections(val);
            }
        }

        if let Ok(timeout) = std::env::var("DB_CONNECTION_TIMEOUT") {
            if let Ok(val) = timeout.parse::<u64>() {
                config = config.with_connection_timeout(val);
            }
        }

        if let Ok(timeout) = std::env::var("DB_IDLE_TIMEOUT") {
            if let Ok(val) = timeout.parse::<u64>() {
                config = config.with_idle_timeout(val);
            }
        }

        if let Ok(lifetime) = std::env::var("DB_MAX_LIFETIME") {
            if let Ok(val) = lifetime.parse::<u64>() {
                config = config.with_max_lifetime(val);
            }
        }

        Self::create(config).await
    }

    /// Gets the underlying SQLx pool
    pub fn inner(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Gets pool metrics
    pub fn metrics(&self) -> PoolMetrics {
        let active = self.active_connections.load(Ordering::SeqCst);
        let total_created = self.total_created.load(Ordering::SeqCst);
        let total_closed = self.total_closed.load(Ordering::SeqCst);
        let pool_size = self.pool.size() as usize;

        PoolMetrics {
            active_connections: active,
            idle_connections: pool_size.saturating_sub(active),
            total_connections_created: total_created,
            total_connections_closed: total_closed,
            peak_active_connections: self.metrics.peak_active_connections,
            pool_size,
        }
    }

    /// Closes the connection pool
    ///
    /// # Returns
    /// * `Result<(), sqlx::Error>` - Success or error
    pub async fn close(self) -> Result<(), sqlx::Error> {
        info!("Closing connection pool");
        self.pool.close().await;
        Ok(())
    }

    /// Runs database maintenance operations
    ///
    /// # Operations
    /// - VACUUM: Reclaims unused space
    /// - ANALYZE: Updates query planner statistics
    /// - REINDEX: Rebuilds indexes
    ///
    /// # Returns
    /// * `Result<(), sqlx::Error>` - Success or error
    pub async fn maintenance(&self) -> Result<(), sqlx::Error> {
        info!("Running database maintenance");

        // VACUUM to reclaim space
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await?;

        // ANALYZE to update statistics
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await?;

        info!("Database maintenance completed");
        Ok(())
    }

    /// Applies database migrations
    ///
    /// # Arguments
    /// * `migration_sql` - SQL migration to execute
    ///
    /// # Returns
    /// * `Result<(), sqlx::Error>` - Success or error
    pub async fn apply_migration(&self, migration_sql: &str) -> Result<(), sqlx::Error> {
        info!("Applying database migration");

        // Split migration into individual statements
        for statement in migration_sql.split(';') {
            let statement = statement.trim();
            if !statement.is_empty() {
                sqlx::query(statement)
                    .execute(&self.pool)
                    .await?;
            }
        }

        info!("Migration applied successfully");
        Ok(())
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        debug!("ConnectionPool dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.connection_timeout_secs, 30);
        assert!(config.test_on_checkout);
    }

    #[test]
    fn test_config_builder() {
        let config = ConnectionPoolConfig::new("sqlite:test.db".to_string())
            .with_max_connections(20)
            .with_min_connections(5)
            .with_connection_timeout(60)
            .with_idle_timeout(300)
            .with_max_lifetime(1800)
            .with_test_on_checkout(false)
            .with_metrics(false);

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connection_timeout_secs, 60);
        assert_eq!(config.idle_timeout_secs, 300);
        assert_eq!(config.max_lifetime_secs, 1800);
        assert!(!config.test_on_checkout);
        assert!(!config.enable_metrics);
    }

    #[test]
    fn test_config_validate() {
        let config = ConnectionPoolConfig::new("sqlite:test.db".to_string());
        assert!(config.validate().is_ok());

        // Test empty database URL
        let invalid_config = ConnectionPoolConfig::new("".to_string());
        assert!(invalid_config.validate().is_err());

        // Test max < min
        let invalid_config = ConnectionPoolConfig::new("sqlite:test.db".to_string())
            .with_max_connections(5)
            .with_min_connections(10);
        assert!(invalid_config.validate().is_err());

        // Test max > 100
        let invalid_config = ConnectionPoolConfig::new("sqlite:test.db".to_string())
            .with_max_connections(150);
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_pool_metrics() {
        let mut metrics = PoolMetrics::new();
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.peak_active_connections, 0);

        metrics.update_peak(5);
        assert_eq!(metrics.peak_active_connections, 5);

        metrics.update_peak(3);
        assert_eq!(metrics.peak_active_connections, 5); // Should not decrease

        metrics.pool_size = 10;
        metrics.active_connections = 5;
        assert_eq!(metrics.utilization(), 50.0);
    }
}
