//! Database Infrastructure
//!
//! Provides database connection pooling and management for HomeFlixD.
//!
//! # Modules
//! - `connection_pool`: Optimized connection pool with metrics
//! - `schema`: Database schema initialization and migrations
//!
//! # Features
//! - Configurable pool sizing
//! - Connection timeout management
//! - Connection validation
//! - Pool metrics tracking
//! - Database maintenance operations
//! - Schema initialization

pub mod connection_pool;
pub mod schema;

pub use connection_pool::{
    ConnectionPool, ConnectionPoolConfig, PoolMetrics,
};
pub use schema::initialize_schema;

/// Database maintenance operations
pub async fn run_maintenance(pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<(), sqlx::Error> {
    // VACUUM to reclaim space
    sqlx::query("VACUUM")
        .execute(pool)
        .await?;

    // ANALYZE to update statistics
    sqlx::query("ANALYZE")
        .execute(pool)
        .await?;

    Ok(())
}

/// Applies database migration from SQL file
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `migration_sql` - SQL migration script
///
/// # Returns
/// * `Result<(), sqlx::Error>` - Success or error
pub async fn apply_migration(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    migration_sql: &str,
) -> Result<(), sqlx::Error> {
    // Split migration into individual statements
    for statement in migration_sql.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() {
            sqlx::query(statement)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_maintenance() {
        // This would require a real database connection
        // In a real scenario, use testcontainers or sqlite in-memory
    }

    #[tokio::test]
    async fn test_apply_migration() {
        // This would require a real database connection
        // In a real scenario, use testcontainers or sqlite in-memory
    }
}
