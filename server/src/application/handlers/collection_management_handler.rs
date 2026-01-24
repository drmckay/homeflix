//! Collection Management Handler
//!
//! Handles collection management events by triggering side effects.

use std::sync::Arc;
use tracing::info;

use crate::domain::events::{
    CollectionCreatedEvent,
    CollectionUpdatedEvent,
    CollectionItemAddedEvent,
};
use crate::interfaces::messaging::EventHandler;
use crate::shared::error::MessagingError;

/// Collection Management Handler
///
/// Handles collection management events:
/// 1. Invalidates cache
/// 2. Sends notifications (in future)
/// 3. Updates statistics (in future)
pub struct CollectionManagementHandler;

impl CollectionManagementHandler {
    /// Creates a new collection management handler
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventHandler<CollectionCreatedEvent> for CollectionManagementHandler {
    async fn handle(&self, event: CollectionCreatedEvent) -> Result<(), MessagingError> {
        info!(
            "Collection created: id={}, name={}, type={}, tmdb_id={:?}",
            event.collection_id,
            event.name,
            event.collection_type,
            event.tmdb_collection_id
        );

        // Future: Invalidate cache, send notifications
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<CollectionUpdatedEvent> for CollectionManagementHandler {
    async fn handle(&self, event: CollectionUpdatedEvent) -> Result<(), MessagingError> {
        info!(
            "Collection updated: id={}, name={}, items={}/{}",
            event.collection_id,
            event.name,
            event.available_items,
            event.total_items
        );

        // Future: Invalidate cache, update statistics
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<CollectionItemAddedEvent> for CollectionManagementHandler {
    async fn handle(&self, event: CollectionItemAddedEvent) -> Result<(), MessagingError> {
        info!(
            "Collection item added: collection_id={}, media_id={:?}, tmdb_id={}, title={}",
            event.collection_id,
            event.media_id,
            event.tmdb_id,
            event.title
        );

        // Future: Invalidate cache, update statistics
        Ok(())
    }
}
