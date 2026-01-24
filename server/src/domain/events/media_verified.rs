//! MediaVerified event
//!
//! Emitted when media is verified

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when media is verified
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaVerifiedEvent {
    /// Media ID
    pub media_id: i64,
    /// File path
    pub file_path: String,
    /// Verification status
    pub verification_status: String,
    /// Confidence score before verification
    pub confidence_before: f32,
    /// Confidence score after verification
    pub confidence_after: f32,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl MediaVerifiedEvent {
    /// Creates a new media verified event
    pub fn new(
        media_id: i64,
        file_path: String,
        verification_status: String,
        confidence_before: f32,
        confidence_after: f32,
    ) -> Self {
        Self {
            media_id,
            file_path,
            verification_status,
            confidence_before,
            confidence_after,
            timestamp: Utc::now(),
        }
    }

    /// Gets the event type name
    pub fn event_type() -> &'static str {
        "media_verified"
    }

    /// Calculates confidence improvement
    pub fn confidence_improvement(&self) -> f32 {
        self.confidence_after - self.confidence_before
    }
}

impl crate::interfaces::messaging::DomainEvent for MediaVerifiedEvent {
    fn event_type(&self) -> &'static str {
        Self::event_type()
    }
}
