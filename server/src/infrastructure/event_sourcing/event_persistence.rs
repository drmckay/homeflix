//! Event Persistence Interface
//!
//! Defines the interface for persisting domain events.

use async_trait::async_trait;
use crate::interfaces::messaging::DomainEvent;
use crate::infrastructure::event_sourcing::event_store::StoredEvent;
use crate::shared::error::EventSourcingError;

/// Interface for event persistence
#[async_trait]
pub trait EventPersistence: Send + Sync {
    /// Save an event to persistence
    /// 
    /// # Arguments
    /// * `event` - The stored event to save
    /// 
    /// # Returns
    /// * `Result<u64, EventSourcingError>` - The version/sequence number of the saved event
    async fn save(&self, event: &StoredEvent) -> Result<u64, EventSourcingError>;

    /// Load events from persistence
    /// 
    /// # Arguments
    /// * `from_version` - The version to start loading from (exclusive)
    /// * `limit` - Maximum number of events to load
    /// 
    /// # Returns
    /// * `Result<Vec<StoredEvent>, EventSourcingError>` - List of stored events
    async fn load(&self, from_version: u64, limit: usize) -> Result<Vec<StoredEvent>, EventSourcingError>;
}
