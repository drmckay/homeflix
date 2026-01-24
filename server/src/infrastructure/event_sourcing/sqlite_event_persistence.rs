//! SQLite Event Persistence Implementation
//!
//! Provides SQLite-based persistence for domain events.

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use std::sync::Arc;
use tracing::debug;

use crate::infrastructure::event_sourcing::event_persistence::EventPersistence;
use crate::infrastructure::event_sourcing::event_store::StoredEvent;
use crate::shared::error::EventSourcingError;

/// SQLite-based event persistence implementation
pub struct SqliteEventPersistence {
    pool: Arc<Pool<Sqlite>>,
}

impl SqliteEventPersistence {
    /// Creates a new SQLite event persistence
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventPersistence for SqliteEventPersistence {
    async fn save(&self, event: &StoredEvent) -> Result<u64, EventSourcingError> {
        let created_at_str = event.created_at.to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO events (
                event_type,
                aggregate_id,
                aggregate_type,
                payload,
                version,
                correlation_id,
                causation_id,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&event.event_type)
        .bind(&event.aggregate_id)
        .bind(&event.aggregate_type)
        .bind(&event.payload)
        .bind(event.version as i64)
        .bind(&event.correlation_id)
        .bind(&event.causation_id)
        .bind(&created_at_str)
        .execute(&*self.pool)
        .await
        .map_err(|e| EventSourcingError::Persistence(format!("Failed to save event: {}", e)))?;

        let event_id = result.last_insert_rowid() as u64;
        debug!("Saved event {} with ID {}", event.event_type, event_id);
        
        Ok(event_id)
    }

    async fn load(&self, from_version: u64, limit: usize) -> Result<Vec<StoredEvent>, EventSourcingError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id,
                event_type,
                aggregate_id,
                aggregate_type,
                payload,
                correlation_id,
                causation_id,
                created_at
            FROM events
            WHERE id > ?
            ORDER BY id ASC
            LIMIT ?
            "#,
        )
        .bind(from_version as i64)
        .bind(limit as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| EventSourcingError::Persistence(format!("Failed to load events: {}", e)))?;

        let stored_events: Vec<StoredEvent> = rows
            .into_iter()
            .map(|row| -> Result<StoredEvent, EventSourcingError> {
                let version: i64 = row.try_get("id")?;
                let event_type: String = row.try_get("event_type")?;
                let aggregate_id: Option<String> = row.try_get("aggregate_id").ok();
                let aggregate_type: Option<String> = row.try_get("aggregate_type").ok();
                let payload: String = row.try_get("payload")?;
                let correlation_id: Option<String> = row.try_get("correlation_id").ok();
                let causation_id: Option<String> = row.try_get("causation_id").ok();
                let created_at_str: String = row.try_get("created_at")?;
                
                // Parse created_at from string (RFC3339 format)
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| EventSourcingError::Deserialization(format!("Failed to parse created_at: {}", e)))?;
                
                Ok(StoredEvent {
                    version: version as u64,
                    event_type,
                    aggregate_id,
                    aggregate_type,
                    payload,
                    correlation_id,
                    causation_id,
                    created_at,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        debug!("Loaded {} events from version {}", stored_events.len(), from_version);
        Ok(stored_events)
    }
}

