//! Service lifetime for dependency injection
//!
//! Defines the lifetime scope of services in the DI container.

/// Service lifetime determines how service instances are managed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceLifetime {
    /// Transient services are created each time they are requested
    /// Each call to resolve returns a new instance
    Transient,

    /// Singleton services are created once and reused for all requests
    /// The same instance is returned for all calls to resolve
    Singleton,
}

impl ServiceLifetime {
    /// Returns true if this is a singleton service
    pub fn is_singleton(&self) -> bool {
        matches!(self, ServiceLifetime::Singleton)
    }

    /// Returns true if this is a transient service
    pub fn is_transient(&self) -> bool {
        matches!(self, ServiceLifetime::Transient)
    }
}

impl std::fmt::Display for ServiceLifetime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceLifetime::Transient => write!(f, "Transient"),
            ServiceLifetime::Singleton => write!(f, "Singleton"),
        }
    }
}
