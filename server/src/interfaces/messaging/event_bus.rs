// Event Bus Interface
//
// This module defines interface for event-driven communication.
// Event bus enables decoupled communication between components.
//
// This interface enables:
// - Publish-subscribe pattern
// - Multiple handlers per event type
// - Testing with mock implementations

use async_trait::async_trait;
use std::sync::Arc;
use crate::interfaces::messaging::domain_event::DomainEvent;
use crate::interfaces::messaging::event_handler::EventHandler;
use crate::shared::error::MessagingError;

/// Event bus interface
/// 
/// Provides publish-subscribe pattern for domain events.
/// Components publish events without knowing who handles them.
/// Handlers subscribe to events they care about.
/// 
/// # Thread Safety
/// Event bus implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish a domain event to all subscribers
    /// 
    /// All handlers registered for this event type will be called.
    /// Handlers are called in the order they were subscribed.
    /// 
    /// # Arguments
    /// * `event` - The domain event to publish
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Event serialization fails
    /// - Handler execution fails (depending on error handling strategy)
    /// - Event bus is in invalid state
    /// 
    /// # Example
    /// ```rust,ignore
    /// let event = MediaIdentifiedEvent::new(1, "path/to/file", "movie");
    /// event_bus.publish(event).await?;
    /// ```
    async fn publish<T: DomainEvent>(&self, event: T) -> Result<(), MessagingError>;
    
    /// Subscribe a handler to a specific event type
    /// 
    /// The handler will be called whenever an event of type `T` is published.
    /// Multiple handlers can subscribe to the same event type.
    /// 
    /// # Arguments
    /// * `handler` - The handler to subscribe
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Handler is already subscribed
    /// - Event bus is in invalid state
    /// 
    /// # Example
    /// ```rust,ignore
    /// let handler = Arc::new(MediaIdentifiedHandler::new(repo));
    /// event_bus.subscribe(handler).await?;
    /// ```
    async fn subscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError>;
    
    /// Unsubscribe a handler from a specific event type
    /// 
    /// The handler will no longer receive events of type `T`.
    /// 
    /// # Arguments
    /// * `handler` - The handler to unsubscribe
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Handler was not subscribed
    /// - Event bus is in invalid state
    async fn unsubscribe<T: DomainEvent>(
        &self,
        handler: Arc<dyn EventHandler<T>>,
    ) -> Result<(), MessagingError>;
    
    /// Publish event and wait for all handlers to complete
    /// 
    /// Unlike `publish`, this method ensures all handlers complete
    /// before returning. Useful for critical events where
    /// you need to know all side effects completed.
    /// 
    /// # Arguments
    /// * `event` - The domain event to publish
    /// 
    /// # Returns
    /// * `Result<HandlerResults, MessagingError>` - Results from all handlers
    /// 
    /// # Errors
    /// Returns error if:
    /// - Event serialization fails
    /// - Any handler fails
    /// - Event bus is in invalid state
    async fn publish_and_wait<T: DomainEvent>(
        &self,
        event: T,
    ) -> Result<HandlerResults, MessagingError>;
    
    /// Get number of handlers subscribed to an event type
    /// 
    /// # Arguments
    /// * `event_type` - The event type to check
    /// 
    /// # Returns
    /// * `Result<usize, MessagingError>` - Number of handlers
    async fn handler_count(&self, event_type: &str) -> Result<usize, MessagingError>;
    
    /// Clear all handlers for a specific event type
    /// 
    /// # Arguments
    /// * `event_type` - The event type to clear
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    async fn clear_handlers(&self, event_type: &str) -> Result<(), MessagingError>;
    
    /// Clear all handlers for all event types
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    async fn clear_all_handlers(&self) -> Result<(), MessagingError>;
}

/// Results from event handler execution
#[derive(Debug, Clone)]
pub struct HandlerResults {
    /// Total number of handlers called
    pub total_handlers: usize,
    /// Number of handlers that succeeded
    pub successful_handlers: usize,
    /// Number of handlers that failed
    pub failed_handlers: usize,
    /// Errors from failed handlers
    pub errors: Vec<HandlerError>,
}

/// Error from a single handler execution
#[derive(Debug, Clone)]
pub struct HandlerError {
    /// Name of the handler that failed
    pub handler_name: String,
    /// The error that occurred
    pub error: MessagingError,
}

/// Event bus configuration options
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Whether to continue processing handlers if one fails
    pub continue_on_error: bool,
    /// Maximum number of handlers per event type
    pub max_handlers_per_event: Option<usize>,
    /// Whether to log all events
    pub log_events: bool,
    /// Whether to log handler execution time
    pub log_handler_timing: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            continue_on_error: true,
            max_handlers_per_event: None,
            log_events: true,
            log_handler_timing: false,
        }
    }
}

/// Event bus statistics
#[derive(Debug, Clone, Default)]
pub struct EventBusStats {
    /// Total number of events published
    pub events_published: u64,
    /// Total number of handler executions
    pub handler_executions: u64,
    /// Number of successful handler executions
    pub successful_executions: u64,
    /// Number of failed handler executions
    pub failed_executions: u64,
    /// Number of active handlers
    pub active_handlers: usize,
    /// Number of event types with handlers
    pub event_types: usize,
}

impl EventBusStats {
    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.handler_executions == 0 {
            return 1.0;
        }
        self.successful_executions as f64 / self.handler_executions as f64
    }
    
    /// Calculate failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}
