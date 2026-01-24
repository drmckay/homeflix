//! Identify Media Use Case
//!
//! Orchestrates media identification process including:
//! - TMDB reconciliation
//! - Confidence scoring with multiple strategies
//! - Alternative match tracking
//! - Event publishing

use std::sync::Arc;
use tracing::{info, debug, warn};

use crate::domain::entities::Media;
use crate::domain::events::{MediaIdentifiedEvent, MediaVerifiedEvent};
use crate::domain::repositories::MediaRepository;
use crate::domain::value_objects::{MediaType, MatchStrategy, ConfidenceScore};
use crate::interfaces::external_services::{TmdbService, TmdbSearcher, TmdbFetcher, TmdbResolver};
use crate::interfaces::messaging::EventBus;
use crate::shared::error::ApplicationError;
use crate::shared::text::{TitleNormalizer, FuzzyMatcher};

/// Result of media identification
#[derive(Debug, Clone)]
pub struct IdentificationResult {
    /// Identified media entity
    pub media: Media,
    /// Confidence score
    pub confidence: f32,
    /// Strategy used for best match
    pub strategy_used: MatchStrategy,
    /// Alternative matches found
    pub alternatives: Vec<AlternativeMatch>,
}

/// Alternative match with lower confidence
#[derive(Debug, Clone)]
pub struct AlternativeMatch {
    /// TMDB ID
    pub tmdb_id: i64,
    /// Title
    pub title: String,
    /// Year
    pub year: Option<i32>,
    /// Confidence score
    pub confidence: f32,
}

/// Identify Media Use Case
///
/// Orchestrates complete media identification workflow:
/// 1. Searches TMDB with multiple strategies
/// 2. Fetches detailed metadata
/// 3. Calculates confidence scores
/// 4. Selects best match
/// 5. Updates database
/// 6. Publishes events
///
/// # Architecture Notes
/// - Uses all 6 search strategies from roadmap
/// - Calculates confidence based on multiple factors
/// - Tracks alternative matches for manual review
/// - Publishes events for side effects
pub struct IdentifyMediaUseCase<E: EventBus + ?Sized> {
    /// Media repository for persistence
    media_repository: Arc<dyn MediaRepository>,
    /// TMDB service for metadata lookup
    tmdb_service: Arc<dyn TmdbService>,
    /// Event bus for publishing events
    event_bus: Arc<E>,
}

impl<E: EventBus + ?Sized> IdentifyMediaUseCase<E> {
    /// Creates a new identify media use case
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media persistence
    /// * `tmdb_service` - TMDB service for metadata lookup
    /// * `event_bus` - Event bus for publishing events
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        tmdb_service: Arc<dyn TmdbService>,
        event_bus: Arc<E>,
    ) -> Self {
        Self {
            media_repository,
            tmdb_service,
            event_bus,
        }
    }

    /// Executes media identification for a specific media item
    ///
    /// # Arguments
    /// * `media_id` - ID of media to identify
    ///
    /// # Returns
    /// * `Result<IdentificationResult, ApplicationError>` - Identification result or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Media not found
    /// - TMDB lookup fails
    /// - Database update fails
    pub async fn execute(&self, media_id: i64) -> Result<IdentificationResult, ApplicationError> {
        // Fetch existing media
        let mut media = self.media_repository
            .find_by_id(media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Media with ID {} not found", media_id))
            ))?;

        info!("Identifying media: {} (ID: {})", media.file_path, media_id);

        // Extract search parameters from media
        let title = self.clean_title(&media.title);
        let year = self.extract_year(&media.title, &media.release_date);

        debug!("Cleaned title: '{}' -> '{}', year: {:?}", media.title, title, year);

        // Perform search with all 6 strategies
        let all_matches = self.search_with_all_strategies(&title, year, &media.media_type).await?;

        if all_matches.is_empty() {
            warn!("No TMDB matches found for: {}", title);
            return Ok(IdentificationResult {
                media,
                confidence: 0.0,
                strategy_used: MatchStrategy::FilenameOnly,
                alternatives: Vec::new(),
            });
        }

        // Select best match based on confidence
        let best_match = self.select_best_match(&all_matches, &title, year);

        // Fetch detailed metadata for best match
        if best_match.tmdb_id > 0 {
            let media_type = media.media_type.clone();
            self.enrich_media(&mut media, best_match.tmdb_id, &media_type).await?;
        }

        // Capture old confidence
        let old_confidence = media.confidence_score.value();

        // Update media with TMDB ID and confidence
        media.tmdb_id = Some(best_match.tmdb_id);
        media.identification_strategy = Some(best_match.strategy.as_str().to_string());

        let confidence_score = ConfidenceScore::new(best_match.confidence.value())?;
        media.update_confidence(confidence_score);

        // Save updated media
        self.media_repository.update(&media).await?;

        // Publish media verified event
        let event = MediaVerifiedEvent::new(
            media_id,
            media.file_path.clone(),
            "verified".to_string(),
            old_confidence,
            best_match.confidence.value(),
        );

        if let Err(e) = self.event_bus.publish(event).await {
            warn!("Failed to publish media verified event: {}", e);
        }

        // Collect alternative matches
        let alternatives: Vec<AlternativeMatch> = all_matches
            .into_iter()
            .filter(|m| m.tmdb_id != best_match.tmdb_id)
            .map(|m| AlternativeMatch {
                tmdb_id: m.tmdb_id,
                title: m.title,
                year: m.year,
                confidence: m.confidence.value(),
            })
            .collect();

        info!(
            "Media identified: {} with confidence {:.2} (strategy: {})",
            media.title,
            best_match.confidence,
            best_match.strategy.as_str()
        );

        Ok(IdentificationResult {
            media,
            confidence: best_match.confidence.value(),
            strategy_used: best_match.strategy,
            alternatives,
        })
    }

    /// Executes batch identification for multiple media items
    ///
    /// # Arguments
    /// * `media_ids` - List of media IDs to identify
    ///
    /// # Returns
    /// * `Result<Vec<(i64, IdentificationResult)>, ApplicationError>` - Results or error
    pub async fn execute_batch(
        &self,
        media_ids: Vec<i64>,
    ) -> Result<Vec<(i64, IdentificationResult)>, ApplicationError> {
        let mut results = Vec::with_capacity(media_ids.len());

        for media_id in media_ids {
            match self.execute(media_id).await {
                Ok(result) => {
                    results.push((media_id, result));
                }
                Err(e) => {
                    warn!("Failed to identify media ID {}: {}", media_id, e);
                    // Continue with other items even if one fails
                }
            }
        }

        Ok(results)
    }

    /// Lists all media items
    ///
    /// # Returns
    /// * `Result<Vec<Media>, ApplicationError>` - List of media or error
    pub async fn list_all(&self) -> Result<Vec<Media>, ApplicationError> {
        Ok(self.media_repository.find_all().await?)
    }

    /// Searches TMDB using all 6 strategies from roadmap
    ///
    /// # Strategies
    /// 1. IMDB ID lookup (if filename contains IMDB ID)
    /// 2. Filename + Year
    /// 3. Folder + Year (same as 2 for TMDB)
    /// 4. Filename only (year-agnostic)
    /// 5. Alternative titles (remove articles)
    /// 6. Fuzzy search (as fallback)
    async fn search_with_all_strategies(
        &self,
        title: &str,
        year: Option<i32>,
        media_type: &MediaType,
    ) -> Result<Vec<crate::interfaces::external_services::TmdbMatch>, ApplicationError> {
        let mut all_matches = Vec::new();

        // Strategy 1: IMDB ID lookup (if title contains IMDB ID)
        if let Some(imdb_id) = self.extract_imdb_id(title) {
            if let Some(match_result) = self.tmdb_service.find_by_external_id(&imdb_id, "imdb_id").await? {
                let mut match_with_confidence = match_result;
                match_with_confidence.confidence = ConfidenceScore::new(0.95)?;
                match_with_confidence.strategy = MatchStrategy::ImdbId;
                all_matches.push(match_with_confidence);
                debug!("Strategy 1 (IMDB ID): Found match with confidence 0.95");
            }
        }

        // Strategy 2: Filename + Year (with fuzzy verification)
        if let Some(y) = year {
            let matches = self.tmdb_service.search_movie(title, Some(y)).await?;
            for m in matches {
                // Verify the result actually matches the query using fuzzy matching
                // Use 0.85 threshold to avoid accepting partial matches like
                // "Back to the Future" for "Back to the Future Part Two"
                let fuzzy_result = FuzzyMatcher::compare_titles(title, &m.title);
                if fuzzy_result.score > 0.90 {
                    // Scale confidence based on fuzzy match score
                    let confidence = (0.85 * fuzzy_result.score as f32).max(0.70);
                    let mut matched = m;
                    matched.confidence = ConfidenceScore::new(confidence)?;
                    matched.strategy = MatchStrategy::FilenameWithYear;
                    all_matches.push(matched);
                    debug!("Strategy 2 (Filename + Year): Verified match '{}' with fuzzy score {:.2}",
                        all_matches.last().unwrap().title, fuzzy_result.score);
                }
            }
        }

        // Strategy 3: Folder + Year for TV (with fuzzy verification)
        if all_matches.is_empty() && media_type.is_episode() {
            if let Some(y) = year {
                let matches = self.tmdb_service.search_tv(title, Some(y)).await?;
                for m in matches {
                    // Use 0.85 threshold to avoid partial matches
                    let fuzzy_result = FuzzyMatcher::compare_titles(title, &m.title);
                    if fuzzy_result.score > 0.90 {
                        let confidence = (0.80 * fuzzy_result.score as f32).max(0.65);
                        let mut matched = m;
                        matched.confidence = ConfidenceScore::new(confidence)?;
                        matched.strategy = MatchStrategy::FolderWithYear;
                        all_matches.push(matched);
                        debug!("Strategy 3 (Folder + Year): Verified match '{}' with fuzzy score {:.2}",
                            all_matches.last().unwrap().title, fuzzy_result.score);
                    }
                }
            }
        }

        // Strategy 4: Filename only (year-agnostic, with fuzzy verification)
        if all_matches.is_empty() {
            let matches = if media_type.is_movie() {
                self.tmdb_service.search_movie(title, None).await?
            } else {
                self.tmdb_service.search_tv(title, None).await?
            };

            for m in matches {
                // Verify the result actually matches the query using fuzzy matching
                // Use 0.85 threshold to avoid accepting partial matches like
                // "Back to the Future" for "Back to the Future Part Two"
                let fuzzy_result = FuzzyMatcher::compare_titles(title, &m.title);
                if fuzzy_result.score > 0.90 {
                    let confidence = (0.70 * fuzzy_result.score as f32).max(0.60);
                    let mut matched = m;
                    matched.confidence = ConfidenceScore::new(confidence)?;
                    matched.strategy = MatchStrategy::FilenameOnly;
                    all_matches.push(matched);
                    debug!("Strategy 4 (Filename only): Verified match '{}' with fuzzy score {:.2}",
                        all_matches.last().unwrap().title, fuzzy_result.score);
                }
            }
        }

        // Strategy 5: Alternative titles (remove articles, with fuzzy verification)
        if all_matches.is_empty() {
            let alt_title = self.remove_articles(title);
            if alt_title != title {
                let matches = if media_type.is_movie() {
                    self.tmdb_service.search_movie(&alt_title, year).await?
                } else {
                    self.tmdb_service.search_tv(&alt_title, year).await?
                };

                for m in matches {
                    // Compare against the alternative title variant
                    // Use 0.85 threshold to avoid partial matches
                    let fuzzy_result = FuzzyMatcher::compare_titles(&alt_title, &m.title);
                    if fuzzy_result.score > 0.90 {
                        let confidence = (0.65 * fuzzy_result.score as f32).max(0.55);
                        let mut matched = m;
                        matched.confidence = ConfidenceScore::new(confidence)?;
                        matched.strategy = MatchStrategy::AlternativeTitle;
                        all_matches.push(matched);
                        debug!("Strategy 5 (Alternative title): Verified match '{}' with fuzzy score {:.2}",
                            all_matches.last().unwrap().title, fuzzy_result.score);
                    }
                }
            }
        }

        // Strategy 6: Fuzzy search with title variants (as fallback)
        // Generates variants like "Part Three" -> "Part III", "Part 3"
        if all_matches.is_empty() {
            let search_variants = TitleNormalizer::get_search_variants(title);
            debug!("Strategy 6: Trying {} title variants for '{}': {:?}", search_variants.len(), title, search_variants);

            for variant in &search_variants {
                // Skip if variant is same as original title (already tried)
                if variant.to_lowercase() == title.to_lowercase() {
                    debug!("Strategy 6: Skipping variant '{}' (same as original)", variant);
                    continue;
                }

                debug!("Strategy 6: Searching TMDB with variant '{}'", variant);
                let search_results = if media_type.is_movie() {
                    self.tmdb_service.search_movie(variant, year).await?
                } else {
                    self.tmdb_service.search_tv(variant, year).await?
                };

                debug!("Strategy 6: TMDB returned {} results for '{}'", search_results.len(), variant);

                // Use fuzzy matching to find best match from results
                // Compare against the VARIANT used for search, not the original title
                // e.g., for "Back to the Future Part I" searching with "Back to the Future"
                // we compare TMDB result "Back to the Future" against variant "Back to the Future"
                for result in &search_results {
                    let fuzzy_result = FuzzyMatcher::compare_titles(variant, &result.title);
                    debug!("Strategy 6: Comparing '{}' vs TMDB '{}' -> score {:.4}",
                        variant, result.title, fuzzy_result.score);

                    // Accept if fuzzy score is good enough (>0.75)
                    if fuzzy_result.score > 0.75 {
                        let mut matched = result.clone();
                        // Scale confidence based on fuzzy score
                        let confidence = (fuzzy_result.score * 0.75).min(0.75) as f32;
                        matched.confidence = ConfidenceScore::new(confidence)?;
                        matched.strategy = MatchStrategy::FuzzySearch;
                        debug!("Strategy 6: Accepted match '{}' (TMDB ID: {}) with confidence {:.2}",
                            matched.title, matched.tmdb_id, confidence);
                        all_matches.push(matched);
                    }
                }

                // Stop if we found good matches
                if !all_matches.is_empty() {
                    debug!("Strategy 6: Found {} matches, stopping variant search", all_matches.len());
                    break;
                }
            }
        } else {
            debug!("Skipping Strategy 6: already have {} matches from earlier strategies", all_matches.len());
        }

        // Deduplicate matches by TMDB ID
        all_matches.sort_by(|a, b| b.confidence.value().partial_cmp(&a.confidence.value()).unwrap());
        all_matches.dedup_by(|a, b| a.tmdb_id == b.tmdb_id);

        Ok(all_matches)
    }

    /// Selects the best match from all found matches
    fn select_best_match(
        &self,
        matches: &[crate::interfaces::external_services::TmdbMatch],
        original_title: &str,
        year: Option<i32>,
    ) -> crate::interfaces::external_services::TmdbMatch {
        // Find best match by confidence (already sorted)
        let best = matches
            .first()
            .cloned()
            .unwrap_or_else(|| crate::interfaces::external_services::TmdbMatch {
                tmdb_id: 0,
                title: original_title.to_string(),
                year,
                media_type: "unknown".to_string(),
                confidence: ConfidenceScore::default(),
                strategy: MatchStrategy::FilenameOnly,
            });

        best
    }

    /// Enriches media with detailed TMDB metadata
    async fn enrich_media(
        &self,
        media: &mut Media,
        tmdb_id: i64,
        media_type: &MediaType,
    ) -> Result<(), ApplicationError> {
        if media_type.is_movie() {
            if let Some(details) = self.tmdb_service.fetch_movie_details(tmdb_id).await? {
                media.overview = Some(details.overview);
                media.poster_url = details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
                media.backdrop_url = details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b));
                media.rating = Some(details.vote_average);
                media.genres = Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", "));
                media.release_date = Some(details.release_date);
                media.duration_seconds = details.runtime.map(|r| r * 60);
            }
        } else {
            if let Some(details) = self.tmdb_service.fetch_tv_details(tmdb_id).await? {
                media.overview = Some(details.overview);
                media.poster_url = details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
                media.backdrop_url = details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b));
                media.rating = Some(details.vote_average);
                media.genres = Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", "));
                media.release_date = Some(details.first_air_date);
            }
        }

        Ok(())
    }

    /// Extracts IMDB ID from title if present
    fn extract_imdb_id(&self, title: &str) -> Option<String> {
        let imdb_regex = regex::Regex::new(r"\b(tt\d{7,8})\b").ok()?;
        imdb_regex.find(title).map(|m| m.as_str().to_string())
    }

    /// Extracts year from title or release date
    fn extract_year(&self, title: &str, release_date: &Option<String>) -> Option<i32> {
        // Try release date first
        if let Some(ref date) = release_date {
            if let Some(year_str) = date.split('-').next() {
                if let Ok(year) = year_str.parse::<i32>() {
                    if year >= 1900 && year <= 2100 {
                        return Some(year);
                    }
                }
            }
        }

        // Try to extract from title
        let year_regex = regex::Regex::new(r"\b(19|20)\d{2}\b").ok()?;
        year_regex.find(title).and_then(|m| m.as_str().parse().ok())
    }

    /// Cleans title by removing common patterns
    fn clean_title(&self, title: &str) -> String {
        let mut cleaned = title.to_string();

        // Remove square brackets (release groups)
        let bracket_regex = regex::Regex::new(r#"\[.*?\]"#).unwrap();
        cleaned = bracket_regex.replace_all(&cleaned, "").to_string();

        // Replace dots, dashes, underscores with spaces
        let separator_regex = regex::Regex::new(r#"[\._-]"#).unwrap();
        cleaned = separator_regex.replace_all(&cleaned, " ").to_string();

        // Remove quality tags
        let quality_regex = regex::Regex::new(r"(?i)\b(1080p|720p|480p|2160p|4k|x264|x265|h264|h265|hevc|avc|bluray|bdrip|webrip|webdl|web dl|hdtv|dvdrip|hdrip|remux|extended|directors cut|remastered)\b").unwrap();
        cleaned = quality_regex.replace_all(&cleaned, "").to_string();

        // Remove parenthetical info like (1989) or (Extended Edition)
        let paren_regex = regex::Regex::new(r"\([^)]*\)").unwrap();
        cleaned = paren_regex.replace_all(&cleaned, "").to_string();

        // Remove standalone year at end (e.g., "Back to the Future 1985" -> "Back to the Future")
        // But keep years that are part of the title like "2001 A Space Odyssey"
        let year_at_end_regex = regex::Regex::new(r"\s+(19|20)\d{2}\s*$").unwrap();
        cleaned = year_at_end_regex.replace(&cleaned, "").to_string();

        // Remove articles at start
        let article_regex = regex::Regex::new(r"(?i)^(The|A|An)\s+").unwrap();
        cleaned = article_regex.replace(&cleaned, "").to_string();

        // Clean up multiple spaces
        let space_regex = regex::Regex::new(r"\s+").unwrap();
        cleaned = space_regex.replace_all(&cleaned.trim(), " ").to_string();

        cleaned
    }

    /// Removes articles from title
    fn remove_articles(&self, title: &str) -> String {
        let article_regex = regex::Regex::new(r"(?i)^(The|A|An)\s+").unwrap();
        article_regex.replace(title, "").to_string()
    }
}
