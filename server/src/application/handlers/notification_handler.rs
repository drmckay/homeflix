//! Notification Handler
//!
//! Handles events that require sending notifications.

use tracing::info;
use crate::interfaces::messaging::EventHandler;
use crate::domain::events::ScanCompletedEvent;
use crate::shared::error::MessagingError;

/// Notification Handler
pub struct NotificationHandler;

impl NotificationHandler {
    /// Creates a new notification handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<ScanCompletedEvent> for NotificationHandler {
    async fn handle(&self, event: ScanCompletedEvent) -> Result<(), MessagingError> {
        info!(
            "Sending notification for scan completion: processed={}, identified={}, failed={}",
            event.processed_count, event.identified_count, event.failed_count
        );
        
        // In a real implementation, this would send an email, push notification, etc.
        // For now, we just log it.
        
        Ok(())
    }
}
