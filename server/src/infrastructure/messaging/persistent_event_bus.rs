//! Persistent Event Bus Implementation
//!
//! Provides an event bus wrapper that persists events to an event store
//! while maintaining backward compatibility with InMemoryEventBus.

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::infrastructure::event_sourcing::event_store::EventStore;
use crate::infrastructure::messaging::in_memory_event_bus::InMemoryEventBus;
use crate::interfaces::messaging::{EventBus, DomainEvent};
use crate::shared::error::MessagingError;

/// Persistent event bus that wraps InMemoryEventBus and adds event persistence
///
/// This implementation:
/// 1. Saves events to the event store (for event sourcing)
/// 2. Publishes events to the in-memory bus (for immediate handler execution)
///
/// # Backward Compatibility
/// This is a wrapper around InMemoryEventBus, so all existing handlers
/// continue to work without modification.
pub struct PersistentEventBus {
    /// Inner in-memory event bus for immediate event handling
    inner: Arc<InMemoryEventBus>,
    /// Event store for persistence
    event_store: Arc<EventStore>,
}

impl PersistentEventBus {
    /// Creates a new persistent event bus
    ///
    /// # Arguments
    /// * `inner` - The in-memory event bus to wrap
    /// * `event_store` - The event store for persistence
    pub fn new(inner: Arc<InMemoryEventBus>, event_store: Arc<EventStore>) -> Self {
        Self {
            inner,
            event_store,
        }
    }
}

#[async_trait]
impl EventBus for PersistentEventBus {
    async fn publish<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError> {
        // 1. Save to event store first (for event sourcing)
        if let Err(e) = self.event_store.append(&event).await {
            // Log error but don't fail - event bus should be resilient
            warn!("Failed to persist event {}: {}", event.event_type(), e);
            // Continue with in-memory publishing even if persistence fails
        } else {
            debug!("Persisted event {} to event store", event.event_type());
        }

        // 2. Publish to in-memory bus (for immediate handler execution)
        self.inner.publish(event).await
    }

    async fn subscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn crate::interfaces::messaging::EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        // Delegate to inner bus
        self.inner.subscribe(handler).await
    }

    async fn unsubscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn crate::interfaces::messaging::EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        // Delegate to inner bus
        self.inner.unsubscribe(handler).await
    }

    async fn publish_and_wait<T: DomainEvent>(
        &self,
        event: T,
    ) -> Result<crate::interfaces::messaging::HandlerResults, MessagingError> {
        // Save to event store first
        if let Err(e) = self.event_store.append(&event).await {
            warn!("Failed to persist event {}: {}", event.event_type(), e);
        }

        // Delegate to inner bus
        self.inner.publish_and_wait(event).await
    }

    async fn handler_count(&self, event_type: &str) -> Result<usize, MessagingError> {
        // Delegate to inner bus
        self.inner.handler_count(event_type).await
    }

    async fn clear_handlers(&self, event_type: &str) -> Result<(), MessagingError> {
        // Delegate to inner bus
        self.inner.clear_handlers(event_type).await
    }

    async fn clear_all_handlers(&self) -> Result<(), MessagingError> {
        // Delegate to inner bus
        self.inner.clear_all_handlers().await
    }
}
