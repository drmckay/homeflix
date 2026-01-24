//! Application Layer
//!
//! This layer contains use cases, application services, and event handlers.
//! It orchestrates business logic and coordinates between domain and infrastructure layers.
//!
//! ## Structure
//! - **Use Cases**: Encapsulate application-specific business logic
//! - **Application Services**: Coordinate multiple use cases and workflows
//! - **Event Handlers**: React to domain events and trigger side effects

pub mod use_cases;
pub mod services;
pub mod handlers;

pub use use_cases::scan_library::ScanLibraryUseCase;
pub use use_cases::identify_media::IdentifyMediaUseCase;
pub use use_cases::stream_media::StreamMediaUseCase;
pub use use_cases::manage_series::ManageSeriesUseCase;

pub use services::scanner_orchestrator::ScannerOrchestrator;
pub use services::metadata_enricher::MetadataEnricher;
pub use services::collection_manager::CollectionManager;
