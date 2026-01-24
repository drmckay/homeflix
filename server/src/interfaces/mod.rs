// Interface Abstraction Layer
//
// This module contains all interface definitions that establish the Dependency Inversion Principle.
// These interfaces define contracts between layers without specifying implementations.
//
// Layer Structure:
// - external_services: Interfaces for external APIs (TMDB, FFmpeg, etc.)
// - filesystem: Interfaces for file system operations
// - messaging: Interfaces for event-driven communication
// - repositories: Repository interfaces (defined in domain layer, re-exported here)

pub mod external_services;
pub mod filesystem;
pub mod messaging;

// Re-export domain repository interfaces for convenience
pub use crate::domain::repositories::{
    MediaRepository,
    SeriesRepository,
    CollectionRepository,
    CacheRepository,
};
