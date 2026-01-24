//! Thumbnail Generation Events
//!
//! Events emitted during thumbnail generation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a thumbnail is generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThumbnailGeneratedEvent {
    /// Media ID
    pub media_id: i64,
    /// Path to generated thumbnail
    pub thumbnail_path: String,
    /// Thumbnail width
    pub width: u32,
    /// Thumbnail height
    pub height: u32,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl ThumbnailGeneratedEvent {
    /// Creates a new thumbnail generated event
    pub fn new(media_id: i64, thumbnail_path: String, width: u32, height: u32) -> Self {
        Self {
            media_id,
            thumbnail_path,
            width,
            height,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for ThumbnailGeneratedEvent {
    fn event_type(&self) -> &'static str {
        "thumbnail_generated"
    }
}
