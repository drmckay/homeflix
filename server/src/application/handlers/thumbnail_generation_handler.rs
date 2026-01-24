//! Thumbnail Generation Handler
//!
//! Handles thumbnail generation events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::ThumbnailGeneratedEvent;
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Thumbnail Generation Handler
///
/// Handles thumbnail generation events:
/// 1. Updates cache
/// 2. Logs statistics
pub struct ThumbnailGenerationHandler;

impl ThumbnailGenerationHandler {
    /// Creates a new thumbnail generation handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<ThumbnailGeneratedEvent> for ThumbnailGenerationHandler {
    async fn handle(&self, event: ThumbnailGeneratedEvent) -> Result<(), MessagingError> {
        info!(
            "Thumbnail generated: media_id={}, path={}, size={}x{}",
            event.media_id,
            event.thumbnail_path,
            event.width,
            event.height
        );

        // Future: Update cache, update statistics
        Ok(())
    }
}
