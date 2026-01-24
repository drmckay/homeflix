//! Scan Completed Handler
//!
//! Handles ScanCompletedEvent by triggering side effects.

use std::sync::Arc;
use tracing::{info, debug, error};

use crate::domain::events::ScanCompletedEvent;
use crate::domain::repositories::MediaRepository;
use crate::interfaces::messaging::{EventHandler, DomainEvent};
use crate::shared::error::{ApplicationError, MessagingError};

/// Scan Completed Handler
///
/// Handles scan completed events:
/// 1. Logs scan statistics
/// 2. Updates system metrics
/// 3. Triggers notifications (in future)
/// 4. Schedules follow-up tasks (in future)
///
/// # Architecture Notes
/// - Implements EventHandler trait
/// - Uses dependency injection
/// - Handles errors gracefully
pub struct ScanCompletedHandler {
    /// Media repository for statistics
    media_repository: Arc<dyn MediaRepository>,
}

impl ScanCompletedHandler {
    /// Creates a new scan completed handler
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media statistics
    pub fn new(media_repository: Arc<dyn MediaRepository>) -> Self {
        Self { media_repository }
    }

    /// Handles scan completed event
    ///
    /// # Arguments
    /// * `event` - The scan completed event
    ///
    /// # Returns
    /// * `Result<(), ApplicationError>` - Success or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Database operations fail
    pub async fn handle_internal(&self, event: ScanCompletedEvent) -> Result<(), ApplicationError> {
        info!(
            "Handling scan completed event: {} processed, {} identified, {} failed in {:.2}s",
            event.processed_count,
            event.identified_count,
            event.failed_count,
            event.duration_secs
        );

        // Calculate success rate
        let success_rate = event.success_rate();

        // Log scan performance
        if event.duration_secs > 0 {
            let items_per_second = event.items_per_second();
            debug!(
                "Scan performance: {:.2} items/second, success rate: {:.1}%",
                items_per_second,
                success_rate * 100.0
            );
        }

        // Update system metrics (in a real implementation, would update metrics store)
        self.update_metrics(&event).await?;

        // Log summary
        info!(
            "Scan completed handler finished: path={}, {} items, {}s duration",
            event.scan_path,
            event.processed_count,
            event.duration_secs
        );

        Ok(())
    }

    /// Updates system metrics based on scan results
    async fn update_metrics(&self, event: &ScanCompletedEvent) -> Result<(), ApplicationError> {
        // Get total media count
        let total_count = self.media_repository.count().await?;

        // Calculate scan coverage
        let coverage = if total_count > 0 {
            (event.processed_count as f64 / total_count as f64) * 100.0
        } else {
            0.0
        };

        debug!(
            "Updated metrics: total_media={}, scan_coverage={:.1}%",
            total_count,
            coverage
        );

        // In a real implementation, would store metrics in a metrics store
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<ScanCompletedEvent> for ScanCompletedHandler {
    async fn handle(&self, event: ScanCompletedEvent) -> Result<(), MessagingError> {
        self.handle_internal(event).await.map_err(|e| MessagingError::HandlerError(e.to_string()))
    }
}
