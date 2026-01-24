//! Event Handlers
//!
//! Event handlers react to domain events and trigger side effects.
//! They provide loose coupling through the event bus.

pub mod media_identified_handler;
pub mod scan_completed_handler;
pub mod collection_detected_handler;

pub mod cache_invalidation_handler;
pub mod notification_handler;
pub mod metrics_handler;

pub mod subtitle_generation_handler;
pub mod progress_tracking_handler;
pub mod streaming_handler;
pub mod collection_management_handler;
pub mod thumbnail_generation_handler;
pub mod background_task_handler;

pub use media_identified_handler::MediaIdentifiedHandler;
pub use scan_completed_handler::ScanCompletedHandler;
pub use collection_detected_handler::CollectionDetectedHandler;
pub use cache_invalidation_handler::CacheInvalidationHandler;
pub use notification_handler::NotificationHandler;
pub use metrics_handler::MetricsHandler;

pub use subtitle_generation_handler::SubtitleGenerationHandler;
pub use progress_tracking_handler::ProgressTrackingHandler;
pub use streaming_handler::StreamingHandler;
pub use collection_management_handler::CollectionManagementHandler;
pub use thumbnail_generation_handler::ThumbnailGenerationHandler;
pub use background_task_handler::BackgroundTaskHandler;
