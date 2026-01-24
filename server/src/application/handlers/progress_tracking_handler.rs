//! Progress Tracking Handler
//!
//! Handles progress tracking events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::{
    ProgressUpdatedEvent,
    MediaWatchedEvent,
    MediaUnwatchedEvent,
};
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Progress Tracking Handler
///
/// Handles progress tracking events:
/// 1. Updates analytics
/// 2. Triggers recommendation engine updates (in future)
/// 3. Updates "continue watching" lists (in future)
pub struct ProgressTrackingHandler;

impl ProgressTrackingHandler {
    /// Creates a new progress tracking handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<ProgressUpdatedEvent> for ProgressTrackingHandler {
    async fn handle(&self, event: ProgressUpdatedEvent) -> Result<(), MessagingError> {
        info!(
            "Progress updated: media_id={}, position={}s, watched={}",
            event.media_id,
            event.current_position_seconds,
            event.is_watched
        );

        // Future: Update analytics, recommendation engine, continue watching lists
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<MediaWatchedEvent> for ProgressTrackingHandler {
    async fn handle(&self, event: MediaWatchedEvent) -> Result<(), MessagingError> {
        info!("Media watched: media_id={}", event.media_id);

        // Future: Update statistics, recommendation engine, user preferences
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<MediaUnwatchedEvent> for ProgressTrackingHandler {
    async fn handle(&self, event: MediaUnwatchedEvent) -> Result<(), MessagingError> {
        info!("Media unwatched: media_id={}", event.media_id);

        // Future: Update statistics, recommendation engine
        Ok(())
    }
}
