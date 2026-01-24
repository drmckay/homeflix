//! Repository Interfaces - Abstractions for data access
//!
//! Repository interfaces define the contract for data access implementations.
//! They use domain entities and return domain errors.

pub mod cache_repository;
pub mod collection_repository;
pub mod credits_repository;
pub mod media_repository;
pub mod series_repository;

pub use cache_repository::{CacheRepository, CacheStats};
pub use collection_repository::CollectionRepository;
pub use credits_repository::{CreditsRepository, CreditEntry, CreditType};
pub use media_repository::MediaRepository;
pub use series_repository::SeriesRepository;
