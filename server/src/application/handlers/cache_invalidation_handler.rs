//! Cache Invalidation Handler
//!
//! Handles events that require cache invalidation.

use std::sync::Arc;
use tracing::{info, error};
use crate::domain::repositories::CacheRepository;
use crate::interfaces::messaging::EventHandler;
use crate::domain::events::MediaVerifiedEvent;
use crate::shared::error::MessagingError;

/// Cache Invalidation Handler
pub struct CacheInvalidationHandler {
    cache_repository: Arc<dyn CacheRepository>,
}

impl CacheInvalidationHandler {
    /// Creates a new cache invalidation handler
    pub fn new(cache_repository: Arc<dyn CacheRepository>) -> Self {
        Self { cache_repository }
    }
}

#[async_trait::async_trait]
impl EventHandler<MediaVerifiedEvent> for CacheInvalidationHandler {
    async fn handle(&self, event: MediaVerifiedEvent) -> Result<(), MessagingError> {
        info!("Invalidating cache for verified media: {}", event.media_id);
        
        let key = format!("media:{}", event.media_id);
        if let Err(e) = self.cache_repository.delete(&key).await {
            error!("Failed to invalidate cache: {}", e);
            // We don't return error here to avoid failing the event processing
            // as cache invalidation failure is non-critical
        }
        
        Ok(())
    }
}
