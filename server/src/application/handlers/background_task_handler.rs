//! Background Task Handler
//!
//! Handles background task events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::{
    BackgroundScanScheduledEvent,
    BackgroundScanStartedEvent,
    BackgroundTaskCompletedEvent,
};
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Background Task Handler
///
/// Handles background task events:
/// 1. Resource management
/// 2. Notifications (in future)
/// 3. Statistics tracking (in future)
pub struct BackgroundTaskHandler;

impl BackgroundTaskHandler {
    /// Creates a new background task handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<BackgroundScanScheduledEvent> for BackgroundTaskHandler {
    async fn handle(&self, event: BackgroundScanScheduledEvent) -> Result<(), MessagingError> {
        info!(
            "Background scan scheduled: path={}, scheduled_at={}, interval={}s",
            event.scan_path,
            event.scheduled_at,
            event.scan_interval_secs
        );

        // Future: Resource management, notifications
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<BackgroundScanStartedEvent> for BackgroundTaskHandler {
    async fn handle(&self, event: BackgroundScanStartedEvent) -> Result<(), MessagingError> {
        info!("Background scan started: path={}", event.scan_path);

        // Future: Resource management, notifications
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<BackgroundTaskCompletedEvent> for BackgroundTaskHandler {
    async fn handle(&self, event: BackgroundTaskCompletedEvent) -> Result<(), MessagingError> {
        info!(
            "Background task completed: type={}, id={:?}, success={}",
            event.task_type,
            event.task_id,
            event.success
        );

        // Future: Resource cleanup, notifications, statistics
        Ok(())
    }
}
