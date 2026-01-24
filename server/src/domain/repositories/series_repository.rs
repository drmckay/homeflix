//! SeriesRepository trait
//!
//! Repository interface for series data access

use async_trait::async_trait;
use crate::domain::entities::Series;
use crate::domain::value_objects::{ConfidenceScore, VerificationStatus};

/// Repository for series data access
#[async_trait]
pub trait SeriesRepository: Send + Sync {
    /// Finds series by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Series>, crate::shared::error::RepositoryError>;

    /// Finds series by TMDB ID
    async fn find_by_tmdb_id(&self, tmdb_id: i64) -> Result<Option<Series>, crate::shared::error::RepositoryError>;

    /// Finds series by title
    async fn find_by_title(&self, title: &str) -> Result<Option<Series>, crate::shared::error::RepositoryError>;

    /// Finds all series
    async fn find_all(&self) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Finds series by verification status
    async fn find_by_verification_status(
        &self,
        status: VerificationStatus,
    ) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Finds series by minimum confidence score
    async fn find_by_confidence(
        &self,
        min_score: ConfidenceScore,
    ) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Saves series (returns new ID)
    async fn save(&self, series: &Series) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Updates series
    async fn update(&self, series: &Series) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes series by ID
    async fn delete(&self, id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Counts total series
    async fn count(&self) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Checks if series exists by title
    async fn exists_by_title(&self, title: &str) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Checks if series exists by TMDB ID
    async fn exists_by_tmdb_id(&self, tmdb_id: i64) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Finds recently added series
    async fn find_recent(&self, limit: usize) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Finds series with low confidence
    async fn find_low_confidence(&self) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Finds series requiring manual review
    async fn find_requires_review(&self) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Search series by title
    ///
    /// # Arguments
    /// * `query` - Search query (partial match)
    /// * `limit` - Maximum results to return
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Series>, crate::shared::error::RepositoryError>;

    /// Finds series ordered by most recent episode date
    ///
    /// Returns series ranked by the created_at of their most recently added episode.
    /// This is useful for "Recently Added" features where a series with a new episode
    /// should appear higher than one without recent activity.
    ///
    /// # Arguments
    /// * `limit` - Maximum results to return
    ///
    /// # Returns
    /// * Vec of (Series, most_recent_episode_date) tuples
    async fn find_recent_by_episode(
        &self,
        limit: usize,
    ) -> Result<Vec<(Series, String)>, crate::shared::error::RepositoryError>;
}
