//! Subtitle Generation Handler
//!
//! Handles subtitle generation events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::{
    SubtitleGenerationCompletedEvent,
    SubtitleGenerationFailedEvent,
};
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Subtitle Generation Handler
///
/// Handles subtitle generation events:
/// 1. Logs completion/failure
/// 2. Triggers cache invalidation (in future)
/// 3. Sends notifications (in future)
pub struct SubtitleGenerationHandler;

impl SubtitleGenerationHandler {
    /// Creates a new subtitle generation handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<SubtitleGenerationCompletedEvent> for SubtitleGenerationHandler {
    async fn handle(&self, event: SubtitleGenerationCompletedEvent) -> Result<(), MessagingError> {
        info!(
            "Subtitle generation completed: media_id={}, job_id={}, path={}, language={}, translated={}",
            event.media_id,
            event.job_id,
            event.subtitle_path,
            event.language,
            event.was_translated
        );

        // Future: Invalidate cache, send notifications, update statistics
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<SubtitleGenerationFailedEvent> for SubtitleGenerationHandler {
    async fn handle(&self, event: SubtitleGenerationFailedEvent) -> Result<(), MessagingError> {
        tracing::warn!(
            "Subtitle generation failed: media_id={}, job_id={}, error={}",
            event.media_id,
            event.job_id,
            event.error_message
        );

        // Future: Send error notifications, update statistics
        Ok(())
    }
}
