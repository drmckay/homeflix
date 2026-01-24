// Domain Event Interface
//
// This module defines the base trait for all domain events.
// Domain events represent significant state changes in the system.
//
// This interface enables:
// - Event-driven architecture
// - Decoupled communication
// - Event sourcing and replay

use serde::{Deserialize, Serialize};

/// Base trait for all domain events
/// 
/// All domain events must implement this trait to be compatible
/// with the event bus and event handlers.
/// 
/// # Requirements
/// - Must be `Send + Sync` for thread safety
/// - Must be `Serialize + Deserialize` for persistence and transmission
/// - Must have a unique event type identifier
/// - Must have a `'static` lifetime
pub trait DomainEvent: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static {
    /// Get the event type identifier
    /// 
    /// This should be a unique string that identifies the event type.
    /// Used for event routing and serialization.
    /// 
    /// # Example
    /// ```rust,ignore
    /// impl DomainEvent for MyEvent {
    ///     fn event_type(&self) -> &'static str {
    ///         "MediaIdentified"
    ///     }
    /// }
    /// ```
    fn event_type(&self) -> &'static str;
    
    /// Get the event version
    /// 
    /// Returns the version of the event schema.
    /// Used for event versioning and migration.
    /// 
    /// # Default
    /// Returns "1.0" by default.
    fn version(&self) -> &'static str {
        "1.0"
    }
    
    /// Get the event correlation ID
    /// 
    /// Returns a correlation ID for tracking related events.
    /// Used for tracing event flows across the system.
    /// 
    /// # Default
    /// Returns `None` by default.
    fn correlation_id(&self) -> Option<&str> {
        None
    }
    
    /// Get the event causation ID
    /// 
    /// Returns the ID of the event that caused this event.
    /// Used for building event causation chains.
    /// 
    /// # Default
    /// Returns `None` by default.
    fn causation_id(&self) -> Option<&str> {
        None
    }
}

/// Helper trait for events with timestamps
pub trait TimestampedEvent: DomainEvent {
    /// Get the event timestamp as ISO 8601 string
    fn timestamp(&self) -> &str;
}

/// Helper trait for events with aggregate IDs
pub trait AggregateEvent: DomainEvent {
    /// Get the aggregate ID
    /// 
    /// Returns the ID of the aggregate (entity) that generated this event.
    fn aggregate_id(&self) -> &str;
    
    /// Get the aggregate type
    /// 
    /// Returns the type name of the aggregate.
    fn aggregate_type(&self) -> &str;
}
