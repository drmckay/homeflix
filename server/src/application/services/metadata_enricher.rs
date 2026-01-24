//! Metadata Enricher
//!
//! Service for enriching media metadata from external sources.
//! Handles batch enrichment and caching.

use std::sync::Arc;
use tracing::{info, debug, warn};

use crate::domain::entities::Media;
use crate::domain::repositories::MediaRepository;
use crate::domain::repositories::SeriesRepository;
use crate::domain::repositories::CollectionRepository;
use crate::interfaces::external_services::TmdbService;
use crate::shared::error::ApplicationError;

/// Metadata Enricher
///
/// Enriches media metadata from external sources:
/// 1. Fetches detailed TMDB metadata
/// 2. Updates media with posters, backdrops, genres
/// 3. Links series to episodes
/// 4. Detects and creates collections
///
/// # Architecture Notes
/// - Uses dependency injection for all services
/// - Batch processing for efficiency
/// - Error handling for individual items
pub struct MetadataEnricher {
    /// Media repository for persistence
    media_repository: Arc<dyn MediaRepository>,
    /// Series repository for linking episodes
    series_repository: Arc<dyn SeriesRepository>,
    /// Collection repository for movie collections
    collection_repository: Arc<dyn CollectionRepository>,
    /// TMDB service for metadata lookup
    tmdb_service: Arc<dyn TmdbService>,
}

impl MetadataEnricher {
    /// Creates a new metadata enricher
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media persistence
    /// * `series_repository` - Repository for series data
    /// * `collection_repository` - Repository for collection data
    /// * `tmdb_service` - TMDB service for metadata lookup
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        series_repository: Arc<dyn SeriesRepository>,
        collection_repository: Arc<dyn CollectionRepository>,
        tmdb_service: Arc<dyn TmdbService>,
    ) -> Self {
        Self {
            media_repository,
            series_repository,
            collection_repository,
            tmdb_service,
        }
    }

    /// Enriches a single media item with TMDB metadata
    ///
    /// # Arguments
    /// * `media_id` - ID of media to enrich
    ///
    /// # Returns
    /// * `Result<(), ApplicationError>` - Success or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Media not found
    /// - TMDB lookup fails
    /// - Database update fails
    pub async fn enrich_media(&self, media_id: i64) -> Result<(), ApplicationError> {
        // Fetch media
        let mut media = self.media_repository
            .find_by_id(media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Media with ID {} not found", media_id))
            ))?;

        info!("Enriching media: {} (ID: {})", media.file_path, media_id);

        // Skip if already enriched (has poster or overview)
        if media.poster_url.is_some() && media.overview.is_some() {
            debug!("Media already enriched, skipping");
            return Ok(());
        }

        // Fetch detailed metadata from TMDB
        if let Some(tmdb_id) = media.tmdb_id {
            if media.is_movie() {
                if let Some(details) = self.tmdb_service.fetch_movie_details(tmdb_id).await? {
                    media = media
                        .with_overview(Some(details.overview))
                        .with_poster_url(details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)))
                        .with_backdrop_url(details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)))
                                                .with_genres(Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", ")))
                        .with_rating(Some(details.vote_average))
                        .with_release_date(Some(details.release_date));

                    // Check for collection
                    if let Some(collection_info) = details.belongs_to_collection {
                        self.create_or_link_collection(&collection_info).await?;
                    }
                }
            } else if media.is_episode() {
                // Fetch TV series details
                if let Some(series_id) = media.series_id {
                    if let Some(series) = self.series_repository.find_by_id(series_id).await? {
                        if let Some(series_tmdb_id) = series.tmdb_id {
                            if let Some(details) = self.tmdb_service.fetch_tv_details(series_tmdb_id).await? {
                                // Update series metadata
                                let mut updated_series = series.clone()
                                    .with_overview(Some(details.overview))
                                    .with_poster_url(details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)))
                                    .with_backdrop_url(details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)))
                                                            .with_genres(Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", ")))
                                    .with_rating(Some(details.vote_average))
                                    .with_first_air_date(Some(details.first_air_date))
                                    .with_last_air_date(details.last_air_date.clone())
                                    .with_status(Some(details.status))
                                    .with_total_seasons(Some(details.number_of_seasons))
                                    .with_total_episodes(Some(details.number_of_episodes));

                                self.series_repository.update(&updated_series).await?;
                            }
                        }
                    }
                }
            }
        }

        // Save updated media
        self.media_repository.update(&media).await?;

        debug!("Media enriched: {} (ID: {})", media.title, media_id);
        Ok(())
    }

    /// Batch enriches multiple media items
    ///
    /// # Arguments
    /// * `media_ids` - List of media IDs to enrich
    ///
    /// # Returns
    /// * `Result<EnrichmentStats, ApplicationError>` - Enrichment statistics
    ///
    /// # Errors
    /// Returns error if:
    /// - Any media enrichment fails
    pub async fn enrich_batch(&self, media_ids: Vec<i64>) -> Result<EnrichmentStats, ApplicationError> {
        info!("Batch enriching {} media items", media_ids.len());

        let total_count = media_ids.len();
        let mut successful = 0;
        let mut failed = 0;

        for media_id in media_ids {
            match self.enrich_media(media_id).await {
                Ok(()) => {
                    successful += 1;
                }
                Err(e) => {
                    failed += 1;
                    warn!("Failed to enrich media ID {}: {}", media_id, e);
                }
            }
        }

        let stats = EnrichmentStats {
            total: total_count,
            successful,
            failed,
        };

        info!(
            "Batch enrichment complete: {}/{}/{} successful",
            stats.successful,
            stats.total,
            stats.successful
        );

        Ok(stats)
    }

    /// Enriches all unverified media
    ///
    /// # Returns
    /// * `Result<EnrichmentStats, ApplicationError>` - Enrichment statistics
    pub async fn enrich_unverified(&self) -> Result<EnrichmentStats, ApplicationError> {
        info!("Starting enrichment of unverified media");

        let unverified = self.media_repository.find_unverified().await?;

        let media_ids: Vec<i64> = unverified
            .into_iter()
            .filter_map(|m| m.id)
            .collect();

        self.enrich_batch(media_ids).await
    }

    /// Creates or links a collection
    async fn create_or_link_collection(
        &self,
        collection_info: &crate::interfaces::external_services::CollectionInfo,
    ) -> Result<(), ApplicationError> {
        // Check if collection already exists
        let existing = self.collection_repository
            .find_by_tmdb_id(collection_info.id)
            .await?;

        if existing.is_none() {
            // Create new collection
            use crate::domain::entities::Collection;
            let collection = Collection::new(collection_info.name.clone())?
                .with_tmdb_collection_id(Some(collection_info.id));

            let collection_id = self.collection_repository.save(&collection).await?;

            info!("Created collection: {} (ID: {})", collection_info.name, collection_id);
        } else {
            debug!("Collection already exists: {}", collection_info.name);
        }

        Ok(())
    }

    /// Refreshes metadata for all media in a series
    ///
    /// # Arguments
    /// * `series_id` - ID of series to refresh
    pub async fn refresh_series_metadata(&self, series_id: i64) -> Result<(), ApplicationError> {
        info!("Refreshing metadata for series ID: {}", series_id);

        // Fetch series
        let series = self.series_repository
            .find_by_id(series_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Series with ID {} not found", series_id))
            ))?;

        // Fetch all media in series
        let media_list = self.media_repository.find_by_series(series_id).await?;

        // Enrich all media
        let media_ids: Vec<i64> = media_list
            .into_iter()
            .filter_map(|m| m.id)
            .collect();

        self.enrich_batch(media_ids).await?;

        // Refresh series metadata
        if let Some(tmdb_id) = series.tmdb_id {
            if let Some(details) = self.tmdb_service.fetch_tv_details(tmdb_id).await? {
                let mut updated_series = series.clone()
                    .with_overview(Some(details.overview))
                                                        .with_poster_url(details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)))                                    .with_backdrop_url(details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)))
                                            .with_genres(Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", ")))
                    .with_rating(Some(details.vote_average))
                    .with_first_air_date(Some(details.first_air_date))
                    .with_last_air_date(details.last_air_date.clone())
                    .with_status(Some(details.status))
                    .with_total_seasons(Some(details.number_of_seasons))
                    .with_total_episodes(Some(details.number_of_episodes));

                self.series_repository.update(&updated_series).await?;
            }
        }

        info!("Series metadata refreshed: {}", series.title);
        Ok(())
    }
}

/// Statistics from batch enrichment operation
#[derive(Debug, Clone)]
pub struct EnrichmentStats {
    /// Total items processed
    pub total: usize,
    /// Number successfully enriched
    pub successful: usize,
    /// Number that failed
    pub failed: usize,
}

impl EnrichmentStats {
    /// Calculates success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.successful as f64 / self.total as f64
    }
}
