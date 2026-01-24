//! Domain Layer - Pure business logic and domain models
//!
//! This layer contains:
//! - Entities: Core business objects with identity
//! - Value Objects: Immutable objects defined by their attributes
//! - Domain Services: Stateless services containing business logic
//! - Repository Interfaces: Abstractions for data access
//! - Domain Events: Events that occur within the domain

pub mod entities;
pub mod value_objects;
pub mod events;
pub mod services;
pub mod repositories;
pub mod presets;

pub use entities::{Collection, Episode, Media, Season, Series};
pub use value_objects::{
    ConfidenceScore, IdentificationResult, MatchStrategy, MediaType, VerificationStatus,
};
pub use events::{
    CollectionDetectedEvent, MediaIdentifiedEvent, MediaVerifiedEvent, ScanCompletedEvent, ScanFailedEvent,
};
