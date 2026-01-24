//! Streaming Events
//!
//! Events emitted during media streaming

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when streaming starts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamStartedEvent {
    /// Media ID
    pub media_id: i64,
    /// Client IP address (if available)
    pub client_ip: Option<String>,
    /// User agent (if available)
    pub user_agent: Option<String>,
    /// Whether transcoding is needed
    pub needs_transcoding: bool,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl StreamStartedEvent {
    /// Creates a new stream started event
    pub fn new(
        media_id: i64,
        client_ip: Option<String>,
        user_agent: Option<String>,
        needs_transcoding: bool,
    ) -> Self {
        Self {
            media_id,
            client_ip,
            user_agent,
            needs_transcoding,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for StreamStartedEvent {
    fn event_type(&self) -> &'static str {
        "stream_started"
    }
}

/// Event emitted when streaming ends
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamEndedEvent {
    /// Media ID
    pub media_id: i64,
    /// Duration of stream in seconds
    pub duration_seconds: Option<f64>,
    /// Bytes streamed
    pub bytes_streamed: Option<u64>,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl StreamEndedEvent {
    /// Creates a new stream ended event
    pub fn new(
        media_id: i64,
        duration_seconds: Option<f64>,
        bytes_streamed: Option<u64>,
    ) -> Self {
        Self {
            media_id,
            duration_seconds,
            bytes_streamed,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for StreamEndedEvent {
    fn event_type(&self) -> &'static str {
        "stream_ended"
    }
}

/// Event emitted when streaming encounters an error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamErrorEvent {
    /// Media ID
    pub media_id: i64,
    /// Error message
    pub error_message: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl StreamErrorEvent {
    /// Creates a new stream error event
    pub fn new(media_id: i64, error_message: String) -> Self {
        Self {
            media_id,
            error_message,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for StreamErrorEvent {
    fn event_type(&self) -> &'static str {
        "stream_error"
    }
}
