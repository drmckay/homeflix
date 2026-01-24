//! Metrics Handler
//!
//! Handles events to update system metrics.

use tracing::info;
use crate::interfaces::messaging::EventHandler;
use crate::domain::events::{MediaIdentifiedEvent, ScanCompletedEvent};
use crate::shared::error::MessagingError;

/// Metrics Handler
pub struct MetricsHandler;

impl MetricsHandler {
    /// Creates a new metrics handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<MediaIdentifiedEvent> for MetricsHandler {
    async fn handle(&self, event: MediaIdentifiedEvent) -> Result<(), MessagingError> {
        info!("Updating metrics for media identified: confidence={}", event.confidence_score);
        // Update prometheus metrics, etc.
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<ScanCompletedEvent> for MetricsHandler {
    async fn handle(&self, event: ScanCompletedEvent) -> Result<(), MessagingError> {
        info!("Updating metrics for scan completion");
        // Update prometheus metrics
        Ok(())
    }
}
