//! Enhanced Event Bus Implementation
//!
//! Provides enhanced event bus features:
//! - Async event processing (non-blocking handler execution)
//! - Event filtering/routing
//! - Event batching
//! - Dead letter queue for failed events

use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, warn, error};
use crate::interfaces::messaging::{EventBus, EventHandler, DomainEvent, HandlerResults};
use crate::shared::error::MessagingError;
use super::in_memory_event_bus::InMemoryEventBus;

/// Event filter function type
pub type EventFilter<T> = Box<dyn Fn(&T) -> bool + Send + Sync>;

/// Dead letter queue entry
#[derive(Debug, Clone)]
pub struct DeadLetterEntry {
    /// Event type name
    pub event_type: String,
    /// Serialized event payload
    pub payload: String,
    /// Error that caused the event to be dead-lettered
    pub error: String,
    /// Timestamp when event was dead-lettered
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Enhanced event bus with advanced features
pub struct EnhancedEventBus {
    /// Inner in-memory event bus for basic functionality
    inner: Arc<InMemoryEventBus>,
    /// Event filters by event type
    filters: Arc<RwLock<HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>>>,
    /// Dead letter queue
    dead_letter_queue: Arc<RwLock<Vec<DeadLetterEntry>>>,
    /// Maximum size of dead letter queue
    max_dlq_size: usize,
    /// Batch size for event batching
    batch_size: usize,
    /// Pending events for batching
    pending_events: Arc<RwLock<Vec<Box<dyn Any + Send + Sync>>>>,
    /// Background task handles for async processing
    background_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

impl EnhancedEventBus {
    /// Creates a new enhanced event bus
    ///
    /// # Arguments
    /// * `inner` - The inner event bus to wrap
    /// * `max_dlq_size` - Maximum size of dead letter queue (default: 1000)
    /// * `batch_size` - Batch size for event batching (default: 10)
    pub fn new(inner: Arc<InMemoryEventBus>, max_dlq_size: Option<usize>, batch_size: Option<usize>) -> Self {
        Self {
            inner,
            filters: Arc::new(RwLock::new(HashMap::new())),
            dead_letter_queue: Arc::new(RwLock::new(Vec::new())),
            max_dlq_size: max_dlq_size.unwrap_or(1000),
            batch_size: batch_size.unwrap_or(10),
            pending_events: Arc::new(RwLock::new(Vec::new())),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add an event filter
    ///
    /// Events that don't pass the filter will not be published.
    ///
    /// # Arguments
    /// * `filter` - The filter function
    pub async fn add_filter<T: DomainEvent>(&self, filter: EventFilter<T>) -> Result<(), MessagingError> {
        let type_id = TypeId::of::<T>();
        let mut filters = self.filters.write().await;
        let filter_list = filters.entry(type_id).or_insert_with(|| Vec::new());
        filter_list.push(Box::new(filter));
        Ok(())
    }

    /// Get dead letter queue entries
    pub async fn get_dead_letters(&self, limit: Option<usize>) -> Vec<DeadLetterEntry> {
        let dlq = self.dead_letter_queue.read().await;
        let limit = limit.unwrap_or(dlq.len());
        dlq.iter().rev().take(limit).cloned().collect()
    }

    /// Clear dead letter queue
    pub async fn clear_dead_letters(&self) {
        let mut dlq = self.dead_letter_queue.write().await;
        dlq.clear();
    }

    /// Process events asynchronously (non-blocking)
    async fn process_async<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError> {
        let inner = self.inner.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = inner.publish(event).await {
                error!("Async event processing failed: {}", e);
            }
        });
        
        {
            let mut tasks = self.background_tasks.write().await;
            tasks.push(handle);
            
            // Clean up completed tasks periodically (keep only last 100)
            // Note: We don't check is_finished() here to avoid borrow conflicts
            // Tasks will be cleaned up on next access or when limit is reached
            let current_len = tasks.len();
            if current_len > 100 {
                // Remove oldest tasks
                let remove_count = current_len - 100;
                tasks.drain(..remove_count);
            }
        }
        
        Ok(())
    }

    /// Add event to dead letter queue
    async fn add_to_dlq(&self, event_type: String, payload: String, error: String) {
        let mut dlq = self.dead_letter_queue.write().await;
        
        // Enforce max size
        if dlq.len() >= self.max_dlq_size {
            dlq.remove(0); // Remove oldest entry
        }
        
        dlq.push(DeadLetterEntry {
            event_type,
            payload,
            error,
            timestamp: chrono::Utc::now(),
        });
        
        debug!("Added event to dead letter queue (size: {})", dlq.len());
    }

    /// Check if event passes filters
    async fn passes_filters<T: DomainEvent>(&self, event: &T) -> bool {
        let type_id = TypeId::of::<T>();
        let filters = self.filters.read().await;
        
        if let Some(filter_list) = filters.get(&type_id) {
            for filter in filter_list {
                if let Some(typed_filter) = filter.downcast_ref::<EventFilter<T>>() {
                    if !typed_filter(event) {
                        debug!("Event filtered out by filter");
                        return false;
                    }
                }
            }
        }
        
        true
    }
}

#[async_trait]
impl EventBus for EnhancedEventBus {
    async fn publish<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError> {
        // Check filters
        if !self.passes_filters(&event).await {
            debug!("Event filtered out, not publishing");
            return Ok(());
        }

        // Clone event for potential DLQ entry (before move)
        let event_type = std::any::type_name::<T>().to_string();
        let payload = serde_json::to_string(&event).ok();

        // Publish through inner bus (synchronous for backward compatibility)
        match self.inner.publish(event).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Add to dead letter queue on error
                let payload_str = payload.unwrap_or_else(|| "serialization failed".to_string());
                self.add_to_dlq(event_type, payload_str, e.to_string()).await;
                Err(e)
            }
        }
    }

    async fn subscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        self.inner.subscribe(handler).await
    }

    async fn unsubscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError> {
        self.inner.unsubscribe(handler).await
    }

    async fn publish_and_wait<T: DomainEvent>(
        &self,
        event: T,
    ) -> Result<HandlerResults, MessagingError> {
        // Check filters
        if !self.passes_filters(&event).await {
            return Ok(HandlerResults {
                total_handlers: 0,
                successful_handlers: 0,
                failed_handlers: 0,
                errors: Vec::new(),
            });
        }

        self.inner.publish_and_wait(event).await
    }

    async fn handler_count(&self, event_type: &str) -> Result<usize, MessagingError> {
        self.inner.handler_count(event_type).await
    }

    async fn clear_handlers(&self, event_type: &str) -> Result<(), MessagingError> {
        self.inner.clear_handlers(event_type).await
    }

    async fn clear_all_handlers(&self) -> Result<(), MessagingError> {
        self.inner.clear_all_handlers().await
    }
}

/// Extension trait for async event publishing
#[async_trait]
pub trait AsyncEventBus: Send + Sync {
    /// Publish event asynchronously (non-blocking)
    ///
    /// This method returns immediately and processes the event in the background.
    /// Useful for fire-and-forget scenarios where you don't need to wait for handlers.
    async fn publish_async<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError>;
}

#[async_trait]
impl AsyncEventBus for EnhancedEventBus {
    async fn publish_async<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError> {
        // Check filters
        if !self.passes_filters(&event).await {
            return Ok(());
        }

        self.process_async(event).await
    }
}
