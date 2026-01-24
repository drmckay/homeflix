//! ValidationService trait
//!
//! Service for validating domain entities and TMDB cross-validation

use async_trait::async_trait;
use std::sync::Arc;
use crate::domain::entities::{Media, Series, Collection, Season};
use crate::interfaces::external_services::TmdbFetcher;

/// Service for validating domain entities
#[async_trait]
pub trait ValidationService: Send + Sync {
    /// Validates a media entity
    async fn validate_media(&self, media: &Media) -> Result<(), crate::shared::error::DomainError>;

    /// Validates a series entity
    async fn validate_series(&self, series: &Series) -> Result<(), crate::shared::error::DomainError>;

    /// Validates a collection entity
    async fn validate_collection(&self, collection: &Collection) -> Result<(), crate::shared::error::DomainError>;

    /// Validates a season entity
    async fn validate_season(&self, season: &Season) -> Result<(), crate::shared::error::DomainError>;
}

/// Default implementation of validation service
pub struct DefaultValidationService;

#[async_trait]
impl ValidationService for DefaultValidationService {
    async fn validate_media(&self, media: &Media) -> Result<(), crate::shared::error::DomainError> {
        // Validate required fields
        if media.file_path.is_empty() {
            return Err(crate::shared::error::DomainError::ValidationError(
                "Media file path cannot be empty".into(),
            ));
        }
        if media.title.is_empty() {
            return Err(crate::shared::error::DomainError::ValidationError(
                "Media title cannot be empty".into(),
            ));
        }

        // Validate episode-specific fields
        if media.media_type.is_episode() {
            if media.series_id.is_none() {
                return Err(crate::shared::error::DomainError::ValidationError(
                    "Episode must have a series ID".into(),
                ));
            }
            if media.season.is_none() || media.episode.is_none() {
                return Err(crate::shared::error::DomainError::ValidationError(
                    "Episode must have season and episode numbers".into(),
                ));
            }
            if let Some(season) = media.season {
                if season < 0 {
                    return Err(crate::shared::error::DomainError::ValidationError(
                        format!("Season number must be >= 0, got {}", season),
                    ));
                }
            }
            if let Some(episode) = media.episode {
                if episode < 1 {
                    return Err(crate::shared::error::DomainError::ValidationError(
                        format!("Episode number must be >= 1, got {}", episode),
                    ));
                }
            }
        }

        // Validate confidence score
        let score = media.confidence_score.value();
        if score < 0.0 || score > 1.0 {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Confidence score must be between 0.0 and 1.0, got {}", score),
            ));
        }

        // Validate duration
        if let Some(duration) = media.duration_seconds {
            if duration < 0 {
                return Err(crate::shared::error::DomainError::ValidationError(
                    format!("Duration must be >= 0, got {}", duration),
                ));
            }
        }

        Ok(())
    }

    async fn validate_series(&self, series: &Series) -> Result<(), crate::shared::error::DomainError> {
        // Validate required fields
        if series.title.is_empty() {
            return Err(crate::shared::error::DomainError::ValidationError(
                "Series title cannot be empty".into(),
            ));
        }

        // Validate confidence score
        let score = series.confidence_score.value();
        if score < 0.0 || score > 1.0 {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Confidence score must be between 0.0 and 1.0, got {}", score),
            ));
        }

        // Validate counts
        if let Some(total_seasons) = series.total_seasons {
            if total_seasons < 0 {
                return Err(crate::shared::error::DomainError::ValidationError(
                    format!("Total seasons must be >= 0, got {}", total_seasons),
                ));
            }
        }
        if let Some(total_episodes) = series.total_episodes {
            if total_episodes < 0 {
                return Err(crate::shared::error::DomainError::ValidationError(
                    format!("Total episodes must be >= 0, got {}", total_episodes),
                ));
            }
        }

        Ok(())
    }

    async fn validate_collection(&self, collection: &Collection) -> Result<(), crate::shared::error::DomainError> {
        // Validate required fields
        if collection.name.is_empty() {
            return Err(crate::shared::error::DomainError::ValidationError(
                "Collection name cannot be empty".into(),
            ));
        }

        // Validate sort mode
        if !["timeline", "release"].contains(&collection.sort_mode.as_str()) {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Invalid sort mode: {}", collection.sort_mode),
            ));
        }

        // Validate collection type
        if !["auto", "preset", "custom"].contains(&&collection.collection_type.as_str()) {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Invalid collection type: {}", collection.collection_type),
            ));
        }

        // Validate counts
        if collection.total_items < 0 {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Total items must be >= 0, got {}", collection.total_items),
            ));
        }
        if collection.available_items < 0 {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Available items must be >= 0, got {}", collection.available_items),
            ));
        }
        if collection.available_items > collection.total_items {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!(
                    "Available items cannot exceed total items, got {} > {}",
                    collection.available_items, collection.total_items
                ),
            ));
        }

        Ok(())
    }

    async fn validate_season(&self, season: &Season) -> Result<(), crate::shared::error::DomainError> {
        // Validate season number
        if season.season_number < 0 {
            return Err(crate::shared::error::DomainError::ValidationError(
                format!("Season number must be >= 0, got {}", season.season_number),
            ));
        }

        // Validate episode count
        if let Some(episode_count) = season.episode_count {
            if episode_count < 0 {
                return Err(crate::shared::error::DomainError::ValidationError(
                    format!("Episode count must be >= 0, got {}", episode_count),
                ));
            }
        }

        // Validate rating
        if let Some(rating) = season.rating {
            if rating < 0.0 || rating > 10.0 {
                return Err(crate::shared::error::DomainError::ValidationError(
                    format!("Rating must be between 0.0 and 10.0, got {}", rating),
                ));
            }
        }

        Ok(())
    }
}

// ============================================================================
// TMDB Cross-Validation
// ============================================================================

/// Result of TMDB cross-validation
#[derive(Debug, Clone)]
pub struct TmdbValidationResult {
    /// Whether the entity exists in TMDB
    pub exists: bool,
    /// Episode count for season validation
    pub episode_count: Option<i32>,
    /// Additional notes about the validation
    pub notes: Option<String>,
}

impl TmdbValidationResult {
    /// Creates a successful validation result
    pub fn found(episode_count: Option<i32>, notes: Option<String>) -> Self {
        Self {
            exists: true,
            episode_count,
            notes,
        }
    }

    /// Creates a not-found validation result
    pub fn not_found(notes: String) -> Self {
        Self {
            exists: false,
            episode_count: None,
            notes: Some(notes),
        }
    }
}

/// TMDB cross-validation service
///
/// Validates that seasons and episodes exist in TMDB.
/// This is used during scanning to verify detected content.
#[async_trait]
pub trait TmdbCrossValidator: Send + Sync {
    /// Validates that a season exists for a series in TMDB
    ///
    /// # Arguments
    /// * `series_tmdb_id` - TMDB ID of the series
    /// * `season_number` - Season number to validate
    ///
    /// # Returns
    /// * `TmdbValidationResult` - Validation result with episode count if found
    async fn validate_season(
        &self,
        series_tmdb_id: i64,
        season_number: i32,
    ) -> TmdbValidationResult;

    /// Validates that an episode exists in TMDB
    ///
    /// # Arguments
    /// * `series_tmdb_id` - TMDB ID of the series
    /// * `season_number` - Season number
    /// * `episode_number` - Episode number to validate
    ///
    /// # Returns
    /// * `TmdbValidationResult` - Validation result
    async fn validate_episode(
        &self,
        series_tmdb_id: i64,
        season_number: i32,
        episode_number: i32,
    ) -> TmdbValidationResult;

    /// Disambiguate between multiple TMDB candidates using local episode structure
    ///
    /// When multiple TV series candidates match a title, this method compares
    /// the local filesystem's season/episode structure against each candidate's
    /// TMDB data to find the best match.
    ///
    /// # Arguments
    /// * `candidate_ids` - List of TMDB IDs to compare
    /// * `local_structure` - Local episode structure from filesystem scan
    ///
    /// # Returns
    /// * Best matching TMDB ID and its structure match score
    async fn disambiguate_by_structure(
        &self,
        candidate_ids: &[i64],
        local_structure: &LocalSeriesStructure,
    ) -> Option<(i64, f32)>;

    /// Disambiguate between multiple TMDB candidates using a single episode's season/episode number
    ///
    /// Filters candidates by checking if the given season exists and has enough episodes.
    /// For example, if the file is S03E05, only candidates with season 3 having 5+ episodes pass.
    /// When multiple candidates pass, prefers the one with more seasons (more established series).
    ///
    /// # Arguments
    /// * `candidate_ids` - List of TMDB IDs to compare
    /// * `season_number` - Season number from the filename
    /// * `episode_number` - Episode number from the filename
    ///
    /// # Returns
    /// * List of TMDB IDs that can contain this episode, sorted by preference (most seasons first)
    async fn filter_candidates_by_episode(
        &self,
        candidate_ids: &[i64],
        season_number: i32,
        episode_number: i32,
    ) -> Vec<i64>;
}

/// TMDB cross-validator implementation using TmdbFetcher
pub struct TmdbCrossValidatorImpl<F: TmdbFetcher> {
    tmdb_fetcher: Arc<F>,
}

impl<F: TmdbFetcher> TmdbCrossValidatorImpl<F> {
    /// Creates a new TMDB cross-validator
    pub fn new(tmdb_fetcher: Arc<F>) -> Self {
        Self { tmdb_fetcher }
    }
}

#[async_trait]
impl<F: TmdbFetcher + 'static> TmdbCrossValidator for TmdbCrossValidatorImpl<F> {
    async fn validate_season(
        &self,
        series_tmdb_id: i64,
        season_number: i32,
    ) -> TmdbValidationResult {
        match self.tmdb_fetcher.fetch_season(series_tmdb_id, season_number).await {
            Ok(Some(season)) => TmdbValidationResult::found(
                Some(season.episode_count),
                Some(format!(
                    "Season '{}' found (Air Date: {:?})",
                    season.overview,
                    season.air_date
                )),
            ),
            Ok(None) => TmdbValidationResult::not_found(
                format!("Season {} not found in TMDB for series {}", season_number, series_tmdb_id),
            ),
            Err(e) => TmdbValidationResult::not_found(
                format!("Failed to validate season {}: {}", season_number, e),
            ),
        }
    }

    async fn validate_episode(
        &self,
        series_tmdb_id: i64,
        season_number: i32,
        episode_number: i32,
    ) -> TmdbValidationResult {
        match self.tmdb_fetcher.fetch_episode(series_tmdb_id, season_number, episode_number).await {
            Ok(Some(episode)) => TmdbValidationResult::found(
                None,
                Some(format!("Episode '{}' found", episode.name)),
            ),
            Ok(None) => TmdbValidationResult::not_found(
                format!(
                    "Episode S{:02}E{:02} not found in TMDB for series {}",
                    season_number, episode_number, series_tmdb_id
                ),
            ),
            Err(e) => TmdbValidationResult::not_found(
                format!(
                    "Failed to validate episode S{:02}E{:02}: {}",
                    season_number, episode_number, e
                ),
            ),
        }
    }

    async fn disambiguate_by_structure(
        &self,
        candidate_ids: &[i64],
        local_structure: &LocalSeriesStructure,
    ) -> Option<(i64, f32)> {
        if candidate_ids.is_empty() || local_structure.season_count == 0 {
            return None;
        }

        // If only one candidate, return it with max score
        if candidate_ids.len() == 1 {
            return Some((candidate_ids[0], 1.0));
        }

        let mut best_match: Option<(i64, f32)> = None;

        // Compare each candidate's structure against local
        for &tmdb_id in candidate_ids {
            if let Some(tmdb_structure) = self.fetch_series_structure(tmdb_id).await {
                let result = Self::compare_structures(local_structure, &tmdb_structure);

                tracing::debug!(
                    "Structure match for TMDB {} ({}): score={:.2} - {}",
                    tmdb_id, tmdb_structure.title, result.score, result.details
                );

                // Update best match if this is better
                if best_match.map(|(_, s)| result.score > s).unwrap_or(true) {
                    best_match = Some((tmdb_id, result.score));
                }
            }
        }

        // Only return if the best match has a reasonable score
        best_match.filter(|(_, score)| *score >= 0.3)
    }

    async fn filter_candidates_by_episode(
        &self,
        candidate_ids: &[i64],
        season_number: i32,
        episode_number: i32,
    ) -> Vec<i64> {
        if candidate_ids.is_empty() {
            return vec![];
        }

        // If only one candidate, return it (no filtering needed)
        if candidate_ids.len() == 1 {
            return candidate_ids.to_vec();
        }

        // Track valid candidates with their total season count for sorting
        let mut valid_candidates: Vec<(i64, i32)> = Vec::new(); // (tmdb_id, total_seasons)

        for &tmdb_id in candidate_ids {
            // First fetch TV details to get total season count
            let total_seasons = match self.tmdb_fetcher.fetch_tv_details(tmdb_id).await {
                Ok(Some(details)) => details.number_of_seasons,
                _ => 0,
            };

            // Check if this candidate has the required season with enough episodes
            match self.tmdb_fetcher.fetch_season(tmdb_id, season_number).await {
                Ok(Some(season)) => {
                    // Get episode count - use episode_count field or fall back to episodes array length
                    let ep_count = if season.episode_count > 0 {
                        season.episode_count
                    } else {
                        season.episodes.len() as i32
                    };

                    // Season exists - check if it has enough episodes
                    if ep_count >= episode_number {
                        tracing::debug!(
                            "TMDB {} has {} total seasons, season {} with {} episodes (need ep {}): VALID",
                            tmdb_id, total_seasons, season_number, ep_count, episode_number
                        );
                        valid_candidates.push((tmdb_id, total_seasons));
                    } else {
                        tracing::debug!(
                            "TMDB {} has season {} with only {} episodes (need ep {}): SKIP",
                            tmdb_id, season_number, ep_count, episode_number
                        );
                    }
                }
                Ok(None) => {
                    tracing::debug!(
                        "TMDB {} does not have season {}: SKIP",
                        tmdb_id, season_number
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch season {} for TMDB {}: {}",
                        season_number, tmdb_id, e
                    );
                }
            }
        }

        // Sort by total seasons descending (prefer series with more seasons)
        valid_candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // Return just the IDs, now sorted by preference
        valid_candidates.into_iter().map(|(id, _)| id).collect()
    }
}

// ============================================================================
// Series Structure Cross-Validation
// ============================================================================

/// Local series structure detected from filesystem
#[derive(Debug, Clone, Default)]
pub struct LocalSeriesStructure {
    /// Season numbers and their episode counts
    pub seasons: std::collections::BTreeMap<i32, Vec<i32>>,
    /// Total number of seasons detected
    pub season_count: i32,
    /// Total number of episodes detected
    pub episode_count: i32,
    /// Maximum episode number per season
    pub max_episodes_per_season: std::collections::BTreeMap<i32, i32>,
}

impl LocalSeriesStructure {
    /// Create a new empty structure
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an episode to the structure
    pub fn add_episode(&mut self, season: i32, episode: i32) {
        self.seasons
            .entry(season)
            .or_default()
            .push(episode);
        self.episode_count += 1;

        // Track max episode number per season
        let max = self.max_episodes_per_season.entry(season).or_insert(0);
        if episode > *max {
            *max = episode;
        }
    }

    /// Finalize the structure (compute season count)
    pub fn finalize(&mut self) {
        self.season_count = self.seasons.len() as i32;
    }
}

/// TMDB series structure for comparison
#[derive(Debug, Clone)]
pub struct TmdbSeriesStructure {
    /// TMDB series ID
    pub tmdb_id: i64,
    /// Series title
    pub title: String,
    /// Total seasons according to TMDB
    pub total_seasons: i32,
    /// Total episodes according to TMDB
    pub total_episodes: i32,
    /// Episode counts per season (season_number -> episode_count)
    pub season_episode_counts: std::collections::BTreeMap<i32, i32>,
}

/// Result of structure comparison
#[derive(Debug, Clone)]
pub struct StructureMatchResult {
    /// TMDB ID of the candidate
    pub tmdb_id: i64,
    /// Match score (0.0 to 1.0)
    pub score: f32,
    /// Detailed breakdown
    pub details: String,
}

impl<F: TmdbFetcher + 'static> TmdbCrossValidatorImpl<F> {
    /// Fetch TMDB series structure for comparison
    pub async fn fetch_series_structure(&self, tmdb_id: i64) -> Option<TmdbSeriesStructure> {
        let details = self.tmdb_fetcher.fetch_tv_details(tmdb_id).await.ok()??;

        let mut season_episode_counts = std::collections::BTreeMap::new();

        // Fetch episode counts for each season
        for season_num in 1..=details.number_of_seasons {
            if let Ok(Some(season)) = self.tmdb_fetcher.fetch_season(tmdb_id, season_num).await {
                season_episode_counts.insert(season_num, season.episode_count);
            }
        }

        Some(TmdbSeriesStructure {
            tmdb_id,
            title: details.name,
            total_seasons: details.number_of_seasons,
            total_episodes: details.number_of_episodes,
            season_episode_counts,
        })
    }

    /// Compare local structure against a TMDB series
    ///
    /// Returns a match score from 0.0 to 1.0 based on:
    /// - Season count match
    /// - Episode count per season match
    /// - Whether local episodes exist in TMDB
    pub fn compare_structures(
        local: &LocalSeriesStructure,
        tmdb: &TmdbSeriesStructure,
    ) -> StructureMatchResult {
        let mut score = 0.0_f32;
        let mut matched_seasons = 0;
        let mut matched_episodes = 0;
        let mut total_local_episodes = 0;

        // Check if local seasons exist in TMDB
        for (local_season, local_episodes) in &local.seasons {
            total_local_episodes += local_episodes.len();

            if let Some(&tmdb_episode_count) = tmdb.season_episode_counts.get(local_season) {
                matched_seasons += 1;

                // Check how many local episodes fit within TMDB's episode count
                for &ep in local_episodes {
                    if ep <= tmdb_episode_count {
                        matched_episodes += 1;
                    }
                }
            }
        }

        // Calculate match score
        if local.season_count > 0 {
            // Season match ratio (weight: 40%)
            let season_ratio = matched_seasons as f32 / local.season_count as f32;
            score += season_ratio * 0.4;

            // Episode match ratio (weight: 60%)
            if total_local_episodes > 0 {
                let episode_ratio = matched_episodes as f32 / total_local_episodes as f32;
                score += episode_ratio * 0.6;
            }
        }

        // Bonus for exact season count match
        if local.season_count == tmdb.total_seasons {
            score = (score + 0.1).min(1.0);
        }

        let details = format!(
            "Matched {}/{} seasons, {}/{} episodes fit. TMDB has {} seasons, {} total eps",
            matched_seasons, local.season_count,
            matched_episodes, total_local_episodes,
            tmdb.total_seasons, tmdb.total_episodes
        );

        StructureMatchResult {
            tmdb_id: tmdb.tmdb_id,
            score,
            details,
        }
    }
}
