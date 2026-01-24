//! Domain Events - Events that occur within the domain
//!
//! Domain events represent significant occurrences that other parts of the
//! application may need to react to.

pub mod collection_detected;
pub mod media_identified;
pub mod media_verified;
pub mod scan_completed;
pub mod scan_failed;
pub mod subtitle_generation;
pub mod progress_tracking;
pub mod streaming;
pub mod collection_management;
pub mod thumbnail_generation;
pub mod background_tasks;

pub use collection_detected::CollectionDetectedEvent;
pub use media_identified::MediaIdentifiedEvent;
pub use media_verified::MediaVerifiedEvent;
pub use scan_completed::ScanCompletedEvent;
pub use scan_failed::ScanFailedEvent;

// Subtitle Generation Events
pub use subtitle_generation::{
    SubtitleGenerationStartedEvent,
    SubtitleGenerationCompletedEvent,
    SubtitleGenerationFailedEvent,
};

// Progress Tracking Events
pub use progress_tracking::{
    ProgressUpdatedEvent,
    MediaWatchedEvent,
    MediaUnwatchedEvent,
};

// Streaming Events
pub use streaming::{
    StreamStartedEvent,
    StreamEndedEvent,
    StreamErrorEvent,
};

// Collection Management Events
pub use collection_management::{
    CollectionCreatedEvent,
    CollectionUpdatedEvent,
    CollectionItemAddedEvent,
};

// Thumbnail Generation Events
pub use thumbnail_generation::ThumbnailGeneratedEvent;

// Background Task Events
pub use background_tasks::{
    BackgroundScanScheduledEvent,
    BackgroundScanStartedEvent,
    BackgroundTaskCompletedEvent,
};
