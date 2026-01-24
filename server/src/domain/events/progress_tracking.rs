//! Progress Tracking Events
//!
//! Events emitted when watch progress is updated

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when watch progress is updated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressUpdatedEvent {
    /// Media ID
    pub media_id: i64,
    /// Current position in seconds
    pub current_position_seconds: i64,
    /// Whether the media is watched
    pub is_watched: bool,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl ProgressUpdatedEvent {
    /// Creates a new progress updated event
    pub fn new(media_id: i64, current_position_seconds: i64, is_watched: bool) -> Self {
        Self {
            media_id,
            current_position_seconds,
            is_watched,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for ProgressUpdatedEvent {
    fn event_type(&self) -> &'static str {
        "progress_updated"
    }
}

/// Event emitted when media is marked as watched
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaWatchedEvent {
    /// Media ID
    pub media_id: i64,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl MediaWatchedEvent {
    /// Creates a new media watched event
    pub fn new(media_id: i64) -> Self {
        Self {
            media_id,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for MediaWatchedEvent {
    fn event_type(&self) -> &'static str {
        "media_watched"
    }
}

/// Event emitted when media is marked as unwatched
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaUnwatchedEvent {
    /// Media ID
    pub media_id: i64,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl MediaUnwatchedEvent {
    /// Creates a new media unwatched event
    pub fn new(media_id: i64) -> Self {
        Self {
            media_id,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for MediaUnwatchedEvent {
    fn event_type(&self) -> &'static str {
        "media_unwatched"
    }
}
