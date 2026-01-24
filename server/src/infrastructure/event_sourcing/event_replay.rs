//! Event Replay
//!
//! Provides functionality to replay events from the event store.

use std::sync::Arc;
use crate::infrastructure::event_sourcing::event_store::EventStore;
use crate::interfaces::messaging::{EventBus, DomainEvent};
use crate::shared::error::EventSourcingError;

/// Event replay service
pub struct EventReplay<E: EventBus + ?Sized> {
    event_store: Arc<EventStore>,
    event_bus: Arc<E>,
}

impl<E: EventBus + ?Sized> EventReplay<E> {
    /// Creates a new event replay service
    pub fn new(event_store: Arc<EventStore>, event_bus: Arc<E>) -> Self {
        Self {
            event_store,
            event_bus,
        }
    }

    /// Replays all events from the beginning
    pub async fn replay_all(&self) -> Result<(), EventSourcingError> {
        self.replay_from(0).await
    }

    /// Replays events from a specific version
    pub async fn replay_from(&self, version: u64) -> Result<(), EventSourcingError> {
        let mut current_version = version;
        let batch_size = 100;

        loop {
            let events = self.event_store.read(current_version, batch_size).await?;
            if events.is_empty() {
                break;
            }

            for stored_event in &events {
                // In a real implementation, we would need to deserialize the payload
                // back into a concrete DomainEvent and publish it.
                // Since DomainEvent trait object doesn't support deserialization directly
                // without knowing the type, this part is tricky in a generic way.
                
                // For now, we assume the EventBus implementation can handle raw JSON or
                // we would need a registry of event types to deserialize.
                
                // Placeholder: Log that we would replay this event
                tracing::debug!("Replaying event: {} (v{})", stored_event.event_type, stored_event.version);
                current_version = stored_event.version;
            }
        }

        Ok(())
    }
}
