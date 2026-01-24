//! Shared types and utilities used across the application

pub mod di;
pub mod error;
pub mod text;

pub use di::{DIError, DIResult, ServiceContainer, ServiceLifetime, ServiceRegistry};
pub use error::{
    ApplicationError,
    DomainError,
    FilesystemError,
    MessagingError,
    RepositoryError,
    TmdbError,
    VideoAnalyzerError,
    ThumbnailError,
};
pub use text::{FuzzyMatch, FuzzyMatchConfig, FuzzyMatcher, RomanNumeralConverter, TitleNormalizer};
