//! Streaming Handler
//!
//! Handles streaming events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::{
    StreamStartedEvent,
    StreamEndedEvent,
    StreamErrorEvent,
};
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Streaming Handler
///
/// Handles streaming events:
/// 1. Updates analytics and bandwidth tracking
/// 2. Logs streaming statistics
/// 3. Triggers auto-pause/resume logic (in future)
pub struct StreamingHandler;

impl StreamingHandler {
    /// Creates a new streaming handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<StreamStartedEvent> for StreamingHandler {
    async fn handle(&self, event: StreamStartedEvent) -> Result<(), MessagingError> {
        info!(
            "Stream started: media_id={}, transcoding={}, client_ip={:?}",
            event.media_id,
            event.needs_transcoding,
            event.client_ip
        );

        // Future: Update analytics, bandwidth tracking, user activity
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<StreamEndedEvent> for StreamingHandler {
    async fn handle(&self, event: StreamEndedEvent) -> Result<(), MessagingError> {
        info!(
            "Stream ended: media_id={}, duration={:?}s, bytes={:?}",
            event.media_id,
            event.duration_seconds,
            event.bytes_streamed
        );

        // Future: Update analytics, bandwidth statistics, user activity
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<StreamErrorEvent> for StreamingHandler {
    async fn handle(&self, event: StreamErrorEvent) -> Result<(), MessagingError> {
        tracing::warn!(
            "Stream error: media_id={}, error={}",
            event.media_id,
            event.error_message
        );

        // Future: Log error metrics, send alerts
        Ok(())
    }
}
