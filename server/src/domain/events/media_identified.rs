//! MediaIdentified event
//!
//! Emitted when media is successfully identified

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when media is successfully identified
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaIdentifiedEvent {
    /// Media ID
    pub media_id: i64,
    /// File path
    pub file_path: String,
    /// Media type
    pub media_type: String,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Confidence score
    pub confidence_score: f32,
    /// Strategy used for identification
    pub strategy_used: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl MediaIdentifiedEvent {
    /// Creates a new media identified event
    pub fn new(
        media_id: i64,
        file_path: String,
        media_type: String,
        tmdb_id: Option<i64>,
        confidence_score: f32,
        strategy_used: String,
    ) -> Self {
        Self {
            media_id,
            file_path,
            media_type,
            tmdb_id,
            confidence_score,
            strategy_used,
            timestamp: Utc::now(),
        }
    }

    /// Gets the event type name
    pub fn event_type() -> &'static str {
        "media_identified"
    }
}

impl crate::interfaces::messaging::DomainEvent for MediaIdentifiedEvent {
    fn event_type(&self) -> &'static str {
        Self::event_type()
    }
}
