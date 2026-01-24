//! Domain Services - Stateless services containing business logic
//!
//! Domain services contain business logic that doesn't naturally fit
//! within a single entity or value object.

pub mod confidence_service;
pub mod identification_service;
pub mod validation_service;
pub mod metadata_service;

pub use confidence_service::{ConfidenceService, DefaultConfidenceService, ConfidenceLevel};
pub use identification_service::{IdentificationService, DefaultIdentificationService, FolderPattern};
pub use validation_service::{
    ValidationService, DefaultValidationService,
    TmdbCrossValidator, TmdbCrossValidatorImpl, TmdbValidationResult
};
pub use metadata_service::MetadataService;
