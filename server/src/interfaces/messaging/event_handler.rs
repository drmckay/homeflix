// Event Handler Interface
//
// This module defines interface for handling domain events.
// Event handlers implement business logic in response to events.
//
// This interface enables:
// - Decoupled event processing
// - Multiple handlers per event type
// - Testing with mock implementations

use async_trait::async_trait;
use crate::interfaces::messaging::domain_event::DomainEvent;
use crate::shared::error::MessagingError;

/// Event handler interface
/// 
/// Implementations of this trait handle specific types of domain events.
/// Handlers are registered with the event bus and called when events are published.
/// 
/// # Type Parameters
/// * `T` - The specific domain event type this handler processes
/// 
/// # Example
/// ```rust,no_run
/// # use async_trait::async_trait;
/// # use std::sync::Arc;
/// # use homeflixd::interfaces::messaging::EventHandler;
/// # use homeflixd::domain::events::MediaIdentifiedEvent;
/// # use homeflixd::shared::error::MessagingError;
/// # struct MediaRepository;
/// struct MediaIdentifiedHandler {
///     // ...
/// }
/// 
/// #[async_trait]
/// impl EventHandler<MediaIdentifiedEvent> for MediaIdentifiedHandler {
///     async fn handle(&self, event: MediaIdentifiedEvent) -> Result<(), MessagingError> {
///         // Handle event: update database, send notifications, etc.
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait EventHandler<T: DomainEvent>: Send + Sync {
    /// Handle a domain event
    /// 
    /// This method is called by the event bus when an event of type `T` is published.
    /// 
    /// # Arguments
    /// * `event` - The domain event to handle
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    /// 
    /// # Errors
    /// Returns error if:
    /// - Handler encounters an error during processing
    /// - External dependencies fail
    /// 
    /// # Error Handling
    /// The event bus should handle errors from handlers gracefully:
    /// - Log the error
    /// - Continue processing other handlers
    /// - Optionally retry or move to dead letter queue
    async fn handle(&self, event: T) -> Result<(), MessagingError>;
    
    /// Get handler name
    /// 
    /// Returns a descriptive name for this handler.
    /// Used for logging and debugging.
    /// 
    /// # Default
    /// Returns type name by default.
    fn name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
    
    /// Check if handler can handle the event
    /// 
    /// Returns true if this handler should process the given event.
    /// Allows for conditional handling based on event content.
    /// 
    /// # Arguments
    /// * `event` - The domain event to check
    /// 
    /// # Returns
    /// * `bool` - True if handler should process this event
    /// 
    /// # Default
    /// Returns true by default (handles all events of type `T`).
    fn can_handle(&self, event: &T) -> bool {
        let _ = event;
        true
    }
}

/// Async event handler with timeout support
/// 
/// Extends EventHandler with timeout configuration.
/// Useful for handlers that may hang or take too long.
#[async_trait]
pub trait AsyncEventHandler<T: DomainEvent>: EventHandler<T> {
    /// Get timeout for handler execution
    /// 
    /// Returns the maximum time the handler should take to process an event.
    /// 
    /// # Returns
    /// * `Option<std::time::Duration>` - Timeout duration, None for unlimited
    /// 
    /// # Default
    /// Returns None (no timeout) by default.
    fn timeout(&self) -> Option<std::time::Duration> {
        None
    }
    
    /// Handle event with timeout
    /// 
    /// Wraps the handle method with timeout enforcement.
    /// Implementations can override for custom timeout behavior.
    /// 
    /// # Arguments
    /// * `event` - The domain event to handle
    /// 
    /// # Returns
    /// * `Result<(), MessagingError>` - Success or error
    async fn handle_with_timeout(&self, event: T) -> Result<(), MessagingError> {
        if let Some(timeout) = self.timeout() {
            match tokio::time::timeout(timeout, self.handle(event)).await {
                Ok(result) => result,
                Err(_) => Err(MessagingError::HandlerTimeout(self.name())),
            }
        } else {
            self.handle(event).await
        }
    }
}

/// Blanket implementation: AsyncEventHandler is also an EventHandler
#[async_trait]
impl<T: DomainEvent, H> EventHandler<T> for H where H: AsyncEventHandler<T> + Send + Sync {
    async fn handle(&self, event: T) -> Result<(), MessagingError> {
        self.handle_with_timeout(event).await
    }
}
