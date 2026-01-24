//! Tests for Connection Pool (Phase 8, Task 8.4)
//!
//! Tests for optimized connection pooling including:
//! - Configuration validation
//! - Pool metrics tracking
//! - Connection lifecycle management

use std::time::Duration;
use homeflixd::infrastructure::database::{
    ConnectionPool, ConnectionPoolConfig, PoolMetrics,
};

#[tokio::test]
async fn test_connection_pool_config_default() {
    let config = ConnectionPoolConfig::default();
    assert_eq!(config.max_connections, 10);
    assert_eq!(config.min_connections, 2);
    assert_eq!(config.connection_timeout_secs, 30);
    assert_eq!(config.idle_timeout_secs, 600);
    assert_eq!(config.max_lifetime_secs, 3600);
    assert!(config.test_on_checkout, true);
    assert!(config.enable_metrics, true);
}

#[tokio::test]
async fn test_connection_pool_config_builder() {
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

#[tokio::test]
fn test_connection_pool_config_validate() {
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

#[tokio::test]
fn test_pool_metrics() {
    let mut metrics = PoolMetrics::new();
    assert_eq!(metrics.active_connections, 0);
    assert_eq!(metrics.peak_active_connections, 0);
    assert_eq!(metrics.total_connections_created, 0);
    assert_eq!(metrics.total_connections_closed, 0);
    assert_eq!(metrics.pool_size, 0);
    assert_eq!(metrics.utilization(), 0.0);

    metrics.update_peak(5);
    assert_eq!(metrics.peak_active_connections, 5);

    metrics.update_peak(3);
    assert_eq!(metrics.peak_active_connections, 5); // Should not decrease

    metrics.pool_size = 10;
    metrics.active_connections = 5;
    assert_eq!(metrics.utilization(), 50.0);
}

#[tokio::test]
fn test_pool_metrics_time_remaining() {
    let mut metrics = PoolMetrics::new();
    metrics.processed = 50;
    metrics.pool_size = 100;
    metrics.update_time_remaining(60); // 60 seconds for 50 files

    assert!(metrics.estimated_seconds_remaining.is_some());
    let remaining = metrics.estimated_seconds_remaining.unwrap();
    
    // Should be approximately 60 seconds for remaining 50 files
    assert!((remaining - 60.0).abs() < 5.0);
}
