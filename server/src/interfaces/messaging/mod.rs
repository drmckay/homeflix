// Messaging Interfaces
//
// This module defines interfaces for event-driven communication.
// These interfaces enable decoupled communication between components through events.
//
// Interfaces:
// - domain_event: Base trait for all domain events
// - event_handler: Interface for event handlers
// - event_bus: Interface for publishing and subscribing to events

pub mod domain_event;
pub mod event_handler;
pub mod event_bus;

// Re-export all messaging traits
pub use domain_event::DomainEvent;
pub use event_handler::EventHandler;
pub use event_bus::{EventBus, HandlerResults};
