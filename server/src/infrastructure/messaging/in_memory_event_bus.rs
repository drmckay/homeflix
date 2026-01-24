//! In-Memory Event Bus Implementation
//!
//! Provides an in-memory implementation of the EventBus interface

use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::interfaces::messaging::{EventBus, EventHandler, DomainEvent, HandlerResults};
use crate::shared::error::MessagingError;

/// In-memory event bus implementation
pub struct InMemoryEventBus {
    subscribers: Arc<RwLock<HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>>>,
}

impl InMemoryEventBus {
    /// Creates a new in-memory event bus
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError> {
        let type_id = TypeId::of::<T>();
        let subscribers = self.subscribers.read().await;

        if let Some(handlers) = subscribers.get(&type_id) {
            for handler in handlers {
                if let Some(typed_handler) = handler.downcast_ref::<Arc<dyn EventHandler<T>>>() {
                    if let Err(e) = typed_handler.handle(event.clone()).await {
                        tracing::error!("Event handler error for {}: {}", std::any::type_name::<T>(), e);
                    }
                }
            }
        } else {
            tracing::warn!("No subscribers for event type: {}", std::any::type_name::<T>());
        }

        Ok(())
    }

    async fn subscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        let type_id = TypeId::of::<T>();
        let mut subscribers = self.subscribers.write().await;

        let handlers = subscribers.entry(type_id).or_insert_with(|| Vec::new());
        handlers.push(Box::new(handler));

        Ok(())
    }

    async fn unsubscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        let type_id = TypeId::of::<T>();
        let mut subscribers = self.subscribers.write().await;

        if let Some(handlers) = subscribers.get_mut(&type_id) {
            handlers.retain(|h| {
                if let Some(typed_handler) = h.downcast_ref::<Arc<dyn EventHandler<T>>>() {
                    !Arc::ptr_eq(typed_handler, &handler)
                } else {
                    true
                }
            });
        }

        Ok(())
    }

    async fn publish_and_wait<T: DomainEvent>(
        &self,
        event: T,
    ) -> Result<crate::interfaces::messaging::HandlerResults, MessagingError> {
        self.publish(event).await?;
        // InMemoryEventBus processes handlers sequentially/immediately in publish, so we are already done.
        Ok(crate::interfaces::messaging::HandlerResults {
            total_handlers: 0,
            successful_handlers: 0,
            failed_handlers: 0,
            errors: Vec::new(),
        })
    }

    async fn handler_count(&self, _event_type: &str) -> Result<usize, MessagingError> {
        Ok(0) // Requires mapping string name to TypeId
    }

    async fn clear_handlers(&self, _event_type: &str) -> Result<(), MessagingError> {
        Ok(())
    }

    async fn clear_all_handlers(&self) -> Result<(), MessagingError> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.clear();
        Ok(())
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}
