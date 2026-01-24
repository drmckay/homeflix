//! Dependency Injection error types
//!
//! This module defines error types specific to the DI container and service registry.

use thiserror::Error;

/// Dependency Injection errors
#[derive(Debug, Error)]
pub enum DIError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Downcast error")]
    DowncastError,

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Service registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Service resolution failed: {0}")]
    ResolutionFailed(String),

    #[error("Factory error: {0}")]
    FactoryError(String),

    #[error("Invalid service lifetime")]
    InvalidLifetime,
}

/// DI-specific result type
pub type DIResult<T> = Result<T, DIError>;
