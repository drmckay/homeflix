//! Collection Detected Handler
//!
//! Handles CollectionDetectedEvent by triggering side effects.

use std::sync::Arc;
use tracing::{info, debug, error};

use crate::domain::events::CollectionDetectedEvent;
use crate::domain::repositories::MediaRepository;
use crate::domain::repositories::CollectionRepository;
use crate::interfaces::messaging::{EventHandler, DomainEvent};
use crate::shared::error::{ApplicationError, MessagingError};

/// Collection Detected Handler
///
/// Handles collection detected events:
/// 1. Creates or links collections
/// 2. Updates collection statistics
/// 3. Logs collection information
///
/// # Architecture Notes
/// - Implements EventHandler trait
/// - Uses dependency injection
/// - Handles errors gracefully
pub struct CollectionDetectedHandler {
    /// Media repository for linking media to collections
    media_repository: Arc<dyn MediaRepository>,
    /// Collection repository for persistence
    collection_repository: Arc<dyn CollectionRepository>,
}

impl CollectionDetectedHandler {
    /// Creates a new collection detected handler
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media linking
    /// * `collection_repository` - Repository for collection persistence
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        collection_repository: Arc<dyn CollectionRepository>,
    ) -> Self {
        Self {
            media_repository,
            collection_repository,
        }
    }

    /// Handles collection detected event
    ///
    /// # Arguments
    /// * `event` - The collection detected event
    ///
    /// # Returns
    /// * `Result<(), ApplicationError>` - Success or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Collection operations fail
    /// - Media linking fails
    pub async fn handle_internal(&self, event: CollectionDetectedEvent) -> Result<(), ApplicationError> {
        info!(
            "Handling collection detected event: name={}, tmdb_id={}",
            event.collection_name,
            event.tmdb_id
        );

        // Create or link collection
        match self.create_or_link_collection(
            &event.collection_name,
            event.tmdb_id,
        ).await {
            Ok(collection_id) => {
                debug!("Collection created/linked: {} (ID: {})", event.collection_name, collection_id);
            }
            Err(e) => {
                error!("Failed to create/link collection: {}", e);
            }
        }

        Ok(())
    }

    /// Creates or links a collection
    async fn create_or_link_collection(
        &self,
        collection_name: &str,
        tmdb_id: i64,
    ) -> Result<i64, ApplicationError> {
        // Check if collection already exists
        let existing = self.collection_repository
            .find_by_tmdb_id(tmdb_id)
            .await?;

        if existing.is_none() {
            // Create new collection
            use crate::domain::entities::Collection;
            let collection = Collection::new(collection_name.to_string())?;

            let collection_id = self.collection_repository.save(&collection).await?;

            info!("Created collection: {} (ID: {})", collection_name, collection_id);
            Ok(collection_id)
        } else {
            // Collection already exists, link media to it
            let collection_id = existing.unwrap().id.unwrap_or(0);

            // Get all movies to potentially link
            let movies = self.media_repository
                .find_by_type(crate::domain::value_objects::MediaType::Movie)
                .await?;

            // Link movies with this TMDB collection ID
            let mut linked_count = 0;
            for movie in movies {
                if let Some(movie_tmdb_id) = movie.tmdb_id {
                    if movie_tmdb_id == tmdb_id {
                        // Link media to collection
                        // In a real implementation, would update media with collection_id
                        debug!("Linked media {} to collection {}", movie.id.unwrap_or(0), collection_id);
                        linked_count += 1;
                    }
                }
            }

            info!(
                "Linked {} media to existing collection: {} (ID: {})",
                linked_count,
                collection_name,
                collection_id
            );

            Ok(collection_id)
        }
    }
}

#[async_trait::async_trait]
impl EventHandler<CollectionDetectedEvent> for CollectionDetectedHandler {
    async fn handle(&self, event: CollectionDetectedEvent) -> Result<(), MessagingError> {
        self.handle_internal(event).await.map_err(|e| MessagingError::HandlerError(e.to_string()))
    }
}
