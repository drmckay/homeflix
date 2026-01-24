//! Subtitle Generation Events
//!
//! Events emitted during subtitle generation process

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when subtitle generation starts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubtitleGenerationStartedEvent {
    /// Media ID
    pub media_id: i64,
    /// Job ID for tracking
    pub job_id: String,
    /// Audio track index
    pub audio_track_index: usize,
    /// Source language (if specified)
    pub source_language: Option<String>,
    /// Target language (if translation requested)
    pub target_language: Option<String>,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl SubtitleGenerationStartedEvent {
    /// Creates a new subtitle generation started event
    pub fn new(
        media_id: i64,
        job_id: String,
        audio_track_index: usize,
        source_language: Option<String>,
        target_language: Option<String>,
    ) -> Self {
        Self {
            media_id,
            job_id,
            audio_track_index,
            source_language,
            target_language,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for SubtitleGenerationStartedEvent {
    fn event_type(&self) -> &'static str {
        "subtitle_generation_started"
    }
}

/// Event emitted when subtitle generation completes successfully
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubtitleGenerationCompletedEvent {
    /// Media ID
    pub media_id: i64,
    /// Job ID for tracking
    pub job_id: String,
    /// Path to generated subtitle file
    pub subtitle_path: String,
    /// Language code of the subtitle
    pub language: String,
    /// Whether translation was applied
    pub was_translated: bool,
    /// Audio fingerprint (hex string)
    pub audio_fingerprint: String,
    /// Duration in seconds
    pub duration_seconds: f64,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl SubtitleGenerationCompletedEvent {
    /// Creates a new subtitle generation completed event
    pub fn new(
        media_id: i64,
        job_id: String,
        subtitle_path: String,
        language: String,
        was_translated: bool,
        audio_fingerprint: String,
        duration_seconds: f64,
    ) -> Self {
        Self {
            media_id,
            job_id,
            subtitle_path,
            language,
            was_translated,
            audio_fingerprint,
            duration_seconds,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for SubtitleGenerationCompletedEvent {
    fn event_type(&self) -> &'static str {
        "subtitle_generation_completed"
    }
}

/// Event emitted when subtitle generation fails
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubtitleGenerationFailedEvent {
    /// Media ID
    pub media_id: i64,
    /// Job ID for tracking
    pub job_id: String,
    /// Error message
    pub error_message: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl SubtitleGenerationFailedEvent {
    /// Creates a new subtitle generation failed event
    pub fn new(media_id: i64, job_id: String, error_message: String) -> Self {
        Self {
            media_id,
            job_id,
            error_message,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for SubtitleGenerationFailedEvent {
    fn event_type(&self) -> &'static str {
        "subtitle_generation_failed"
    }
}
