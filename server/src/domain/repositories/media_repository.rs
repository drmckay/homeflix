//! MediaRepository trait
//!
//! Repository interface for media data access

use async_trait::async_trait;
use crate::domain::entities::Media;
use crate::domain::value_objects::{MediaType, ConfidenceScore, VerificationStatus};

/// Repository for media data access
#[async_trait]
pub trait MediaRepository: Send + Sync {
    /// Finds media by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Media>, crate::shared::error::RepositoryError>;

    /// Finds media by file path
    async fn find_by_path(&self, path: &str) -> Result<Option<Media>, crate::shared::error::RepositoryError>;

    /// Finds all media
    async fn find_all(&self) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds media by type
    async fn find_by_type(&self, media_type: MediaType) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds media by series ID
    async fn find_by_series(&self, series_id: i64) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds media by series and season
    async fn find_by_season(&self, series_id: i64, season: i32) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds unverified media
    async fn find_unverified(&self) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds media by minimum confidence score
    async fn find_by_confidence(&self, min_score: ConfidenceScore) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Saves media (returns new ID)
    async fn save(&self, media: &Media) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Updates media
    async fn update(&self, media: &Media) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes media by ID
    async fn delete(&self, id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Counts total media
    async fn count(&self) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Counts media by type
    async fn count_by_type(&self, media_type: MediaType) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Checks if media exists by path
    async fn exists_by_path(&self, path: &str) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Updates watch progress for media
    async fn update_progress(&self, media_id: i64, position: i64, watched: bool) -> Result<(), crate::shared::error::RepositoryError>;

    /// Finds recently added media
    async fn find_recent(&self, limit: usize) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds watched media
    async fn find_watched(&self) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds unwatched media
    async fn find_unwatched(&self) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Search media by title
    ///
    /// # Arguments
    /// * `query` - Search query (partial match)
    /// * `media_type` - Optional filter by media type ("movie" or "tv")
    /// * `limit` - Maximum results to return
    async fn search(
        &self,
        query: &str,
        media_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Gets watch progress for media
    ///
    /// # Arguments
    /// * `media_id` - Media ID
    ///
    /// # Returns
    /// * `(position_seconds, is_watched, last_updated)` tuple
    async fn get_progress(&self, media_id: i64) -> Result<Option<(i64, bool, String)>, crate::shared::error::RepositoryError>;

    /// Marks media as watched
    async fn mark_watched(&self, media_id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Marks media as unwatched
    async fn mark_unwatched(&self, media_id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Finds in-progress media (started but not finished)
    ///
    /// Returns media where current_position > 0 and is_watched = false,
    /// ordered by updated_at descending (most recently watched first)
    async fn find_in_progress(&self, limit: usize) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;

    /// Finds recently added movies only (no episodes/series)
    ///
    /// Returns movies ordered by created_at descending
    async fn find_recent_movies(&self, limit: usize) -> Result<Vec<Media>, crate::shared::error::RepositoryError>;
}
