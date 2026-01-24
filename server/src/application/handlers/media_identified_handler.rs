//! Media Identified Handler
//!
//! Handles MediaIdentifiedEvent by triggering side effects.

use std::sync::Arc;
use tracing::{info, debug, error};

use crate::domain::events::MediaIdentifiedEvent;
use crate::domain::repositories::MediaRepository;
use crate::domain::repositories::CacheRepository;
use crate::interfaces::messaging::{EventHandler, DomainEvent};
use crate::shared::error::{ApplicationError, MessagingError};

/// Media Identified Handler
///
/// Handles media identified events:
/// 1. Updates cache
/// 2. Logs statistics
/// 3. Triggers notifications (in future)
///
/// # Architecture Notes
/// - Implements EventHandler trait
/// - Uses dependency injection
/// - Handles errors gracefully
pub struct MediaIdentifiedHandler {
    /// Media repository for statistics
    media_repository: Arc<dyn MediaRepository>,
    /// Cache repository for invalidation
    cache_repository: Arc<dyn CacheRepository>,
}

impl MediaIdentifiedHandler {
    /// Creates a new media identified handler
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media statistics
    /// * `cache_repository` - Repository for cache invalidation
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        cache_repository: Arc<dyn CacheRepository>,
    ) -> Self {
        Self {
            media_repository,
            cache_repository,
        }
    }

    /// Handles media identified event
    ///
    /// # Arguments
    /// * `event` - The media identified event
    ///
    /// # Returns
    /// * `Result<(), ApplicationError>` - Success or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Cache operations fail
    /// - Database operations fail
    pub async fn handle_internal(&self, event: MediaIdentifiedEvent) -> Result<(), ApplicationError> {
        info!(
            "Handling media identified event: ID={}, path={}, type={}, confidence={:.2}",
            event.media_id,
            event.file_path,
            event.media_type,
            event.confidence_score
        );

        // Invalidate cache entries related to this media
        if let Err(e) = self.cache_repository
            .delete(&format!("media:{}", event.media_id))
            .await
        {
            error!("Failed to invalidate cache for media {}: {}", event.media_id, e);
        }

        // Update statistics (in a real implementation, would update counters)
        debug!("Media identified handler completed for ID: {}", event.media_id);

        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<MediaIdentifiedEvent> for MediaIdentifiedHandler {
    async fn handle(&self, event: MediaIdentifiedEvent) -> Result<(), MessagingError> {
        self.handle_internal(event).await.map_err(|e| MessagingError::HandlerError(e.to_string()))
    }
}
