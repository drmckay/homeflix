//! Event Store
//!
//! Manages the append-only log of domain events.

use std::sync::Arc;
use crate::interfaces::messaging::DomainEvent;
use crate::infrastructure::event_sourcing::event_persistence::EventPersistence;
use crate::shared::error::EventSourcingError;

/// Event store for managing domain events
pub struct EventStore {
    persistence: Arc<dyn EventPersistence>,
}

impl EventStore {
    /// Creates a new event store
    pub fn new(persistence: Arc<dyn EventPersistence>) -> Self {
        Self { persistence }
    }

    /// Appends an event to the store
    pub async fn append<T: DomainEvent>(&self, event: &T) -> Result<u64, EventSourcingError> {
        let payload = serde_json::to_string(event)
            .map_err(|e| EventSourcingError::Serialization(e.to_string()))?;
        
        // Try to extract aggregate information using type erasure
        // This is a simplified approach - in a full implementation, we'd use trait objects
        let (aggregate_id, aggregate_type) = (None, None);
        
        let stored_event = StoredEvent {
            version: 0, // Version will be assigned by persistence/database
            event_type: event.event_type().to_string(),
            aggregate_id,
            aggregate_type,
            payload,
            correlation_id: event.correlation_id().map(|s| s.to_string()),
            causation_id: event.causation_id().map(|s| s.to_string()),
            created_at: chrono::Utc::now(),
        };
        
        self.persistence.save(&stored_event).await
    }

    /// Reads events from the store
    pub async fn read(&self, from_version: u64, limit: usize) -> Result<Vec<StoredEvent>, EventSourcingError> {
        self.persistence.load(from_version, limit).await
    }
}

/// Represents a stored event
#[derive(Debug, Clone)]
pub struct StoredEvent {
    /// Global event version/sequence number
    pub version: u64,
    /// Event type identifier
    pub event_type: String,
    /// Aggregate ID (if applicable)
    pub aggregate_id: Option<String>,
    /// Aggregate type (if applicable)
    pub aggregate_type: Option<String>,
    /// Serialized event data
    pub payload: String,
    /// Correlation ID for tracking related events
    pub correlation_id: Option<String>,
    /// Causation ID for tracking event chains
    pub causation_id: Option<String>,
    /// Event timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}
