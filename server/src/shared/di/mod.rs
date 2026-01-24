//! Dependency Injection Module
//!
//! This module provides a dependency injection framework with support for:
//! - Service registration and resolution
//! - Service lifetime management (transient, singleton)
//! - Dependency graph validation
//! - Circular dependency detection

mod container;
mod error;
mod registry;
mod service_lifetime;

pub use container::{ServiceContainer, ServiceFactory, AsyncServiceFactory};
pub use error::{DIError, DIResult};
pub use registry::ServiceRegistry;
pub use service_lifetime::ServiceLifetime;

/// Re-export commonly used types for convenience
pub use crate::shared::di::ServiceContainer as Container;
pub use crate::shared::di::ServiceRegistry as Registry;
