//! TMDB Client Implementation
//!
//! Provides TMDB API client with caching, rate limiting, and retry logic

use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::debug;
use crate::interfaces::external_services::{
    TmdbService, TmdbSearcher, TmdbFetcher, TmdbResolver, TmdbCreditsFetcher,
    TmdbMatch, MovieDetail, TvDetail, SeasonDetail, EpisodeDetail, Genre, CollectionInfo,
    CollectionDetail, CollectionPartInfo, Credits, CastMember, CrewMember,
};
use crate::domain::value_objects::{MatchStrategy, ConfidenceScore};
use crate::domain::repositories::CacheRepository;
use crate::shared::error::TmdbError;
use crate::shared::text::{TitleNormalizer, FuzzyMatcher, FuzzyMatchConfig};

/// TMDB API client with caching and rate limiting
pub struct TmdbClient {
    api_key: String,
    http_client: Client,
    cache: Arc<dyn CacheRepository>,
    base_url: String,
    image_base_url: String,
    rate_limiter: Arc<RateLimiter>,
}

impl TmdbClient {
    /// Creates a new TMDB client
    ///
    /// # Arguments
    /// * `api_key` - TMDB API key
    /// * `cache` - Cache repository for caching responses
    ///
    /// # Errors
    /// Returns error if client creation fails
    pub fn new(api_key: &str, cache: Arc<dyn CacheRepository>) -> Result<Self, TmdbError> {
        if api_key.is_empty() {
            return Err(TmdbError::InvalidApiKey);
        }

        Ok(Self {
            api_key: api_key.to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| TmdbError::Network(e.to_string()))?,
            cache,
            base_url: "https://api.themoviedb.org/3".to_string(),
            image_base_url: "https://image.tmdb.org/t/p/w500".to_string(),
            rate_limiter: Arc::new(RateLimiter::new(4)), // 4 requests per second
        })
    }

    /// Makes a GET request to TMDB API
    async fn make_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<T, TmdbError> {
        self.rate_limiter.acquire().await;

        // Determine separator: use & if endpoint already has query params, else ?
        let separator = if endpoint.contains('?') { '&' } else { '?' };
        let url = format!("{}{}{}api_key={}", self.base_url, endpoint, separator, self.api_key);

        let response = self.http_client
            .get(&url)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            return Err(TmdbError::ApiError(status.as_u16()));
        }

        let data = response.json().await?;
        Ok(data)
    }

    /// Searches with multiple strategies
    async fn search_with_strategies(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<TmdbMatch>, TmdbError> {
        let mut all_matches = Vec::new();

        // Strategy 1: IMDB ID lookup (if query is IMDB ID)
        if query.starts_with("tt") {
            if let Some(match_result) = self.find_by_external_id(query, "imdb_id").await? {
                all_matches.push(match_result);
            }
        }

        // Strategy 2: Filename + Year
        if let Some(y) = year {
            let movie_matches = self.search_movie(query, Some(y)).await?;
            all_matches.extend(movie_matches);
        }

        // Strategy 3: Folder + Year (same as strategy 2 for TMDB)
        if let Some(y) = year {
            let tv_matches = self.search_tv(query, Some(y)).await?;
            all_matches.extend(tv_matches);
        }

        // Strategy 4: Filename only (year-agnostic)
        if all_matches.is_empty() {
            let movie_matches = self.search_movie(query, None).await?;
            all_matches.extend(movie_matches);
        }

        // Strategy 5: Alternative titles (remove articles)
        if all_matches.is_empty() {
            let cleaned_query = query
                .to_lowercase()
                .replace("the ", "")
                .replace("a ", "")
                .replace("an ", "");
            let movie_matches = self.search_movie(&cleaned_query, year).await?;
            all_matches.extend(movie_matches);
        }

        // Sort by confidence score
        all_matches.sort_by(|a, b| b.confidence.value().partial_cmp(&a.confidence.value()).unwrap());

        Ok(all_matches)
    }

    /// Search with fuzzy matching and title variants
    ///
    /// This method generates search variants for the title and uses fuzzy
    /// matching to find the best match from TMDB results.
    ///
    /// # Arguments
    /// * `title` - The title to search for
    /// * `year` - Optional year to filter results
    /// * `media_type` - "movie" or "tv"
    ///
    /// # Returns
    /// Best matching result if found with score above threshold
    pub async fn search_fuzzy(
        &self,
        title: &str,
        year: Option<i32>,
        media_type: &str,
    ) -> Result<Option<TmdbMatch>, TmdbError> {
        // Generate search variants (roman numerals, without articles, etc.)
        let search_variants = TitleNormalizer::get_search_variants(title);

        let fuzzy_config = FuzzyMatchConfig {
            min_similarity: 0.75, // Lower threshold for initial matching
            year_match_bonus: 0.10,
            close_year_bonus: 0.05,
            ..Default::default()
        };

        let mut best_match: Option<(TmdbMatch, f64)> = None;

        // Try each search variant
        for variant in &search_variants {
            let results = if media_type == "tv" {
                self.search_tv(variant, year).await?
            } else {
                self.search_movie(variant, year).await?
            };

            // Find best fuzzy match from results
            for result in results {
                let mut score = FuzzyMatcher::compare_titles(title, &result.title).score;

                // Apply year bonus
                if let (Some(expected_year), Some(result_year)) = (year, result.year) {
                    if expected_year == result_year {
                        score += fuzzy_config.year_match_bonus;
                    } else if (expected_year - result_year).abs() <= 1 {
                        score += fuzzy_config.close_year_bonus;
                    }
                }

                // Clamp score to 1.0
                score = score.min(1.0);

                if score >= fuzzy_config.min_similarity {
                    if best_match.is_none() || score > best_match.as_ref().unwrap().1 {
                        best_match = Some((result, score));
                    }
                }
            }
        }

        // Return the best match with updated confidence
        if let Some((mut tmdb_match, score)) = best_match {
            tmdb_match.confidence = ConfidenceScore::new(score as f32)
                .unwrap_or_default();
            tmdb_match.strategy = MatchStrategy::FuzzySearch;
            Ok(Some(tmdb_match))
        } else {
            Ok(None)
        }
    }

}

#[async_trait]
impl TmdbSearcher for TmdbClient {
    async fn search_movie(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError> {
        let endpoint = if let Some(y) = year {
            format!("/search/movie?query={}&year={}", urlencoding::encode(query), y)
        } else {
            format!("/search/movie?query={}", urlencoding::encode(query))
        };

        let response: TmdbSearchResponse = self.make_request(&endpoint).await?;

        let matches: Vec<TmdbMatch> = response.results
            .into_iter()
            .filter_map(|m| {
                Some(TmdbMatch {
                    tmdb_id: m.id,
                    title: m.title?,
                    year: m.release_date.and_then(|d| d.get(..4).and_then(|s| s.parse().ok())),
                    media_type: "movie".to_string(),
                    confidence: ConfidenceScore::default(),
                    strategy: MatchStrategy::FilenameOnly,
                })
            })
            .collect();

        Ok(matches)
    }

    async fn search_tv(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError> {
        let endpoint = if let Some(y) = year {
            format!("/search/tv?query={}&first_air_date_year={}", urlencoding::encode(query), y)
        } else {
            format!("/search/tv?query={}", urlencoding::encode(query))
        };

        let response: TmdbSearchResponse = self.make_request(&endpoint).await?;

        let matches: Vec<TmdbMatch> = response.results
            .into_iter()
            .filter_map(|m| {
                Some(TmdbMatch {
                    tmdb_id: m.id,
                    title: m.name?,
                    year: m.first_air_date.and_then(|d| d.get(..4).and_then(|s| s.parse().ok())),
                    media_type: "tv".to_string(),
                    confidence: ConfidenceScore::default(),
                    strategy: MatchStrategy::FilenameOnly,
                })
            })
            .collect();

        Ok(matches)
    }
}

#[async_trait]
impl TmdbFetcher for TmdbClient {
    async fn fetch_movie_details(&self, id: i64) -> Result<Option<MovieDetail>, TmdbError> {
        // Check cache first
        let cache_key = format!("movie:{}", id);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/movie/{}", id);
        let response: Option<MovieDetail> = self.make_request(&endpoint).await?;

        // Cache result
        if let Some(ref detail) = response {
            let cached_value = serde_json::to_string(detail)?;
            self.cache.set(&cache_key, &cached_value, 86400).await?; // 24 hours TTL
        }

        Ok(response)
    }

    async fn fetch_tv_details(&self, id: i64) -> Result<Option<TvDetail>, TmdbError> {
        // Check cache first
        let cache_key = format!("tv:{}", id);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/tv/{}", id);
        let response: Option<TvDetail> = self.make_request(&endpoint).await?;

        // Cache result
        if let Some(ref detail) = response {
            let cached_value = serde_json::to_string(detail)?;
            self.cache.set(&cache_key, &cached_value, 86400).await?; // 24 hours TTL
        }

        Ok(response)
    }

    async fn fetch_season(&self, tv_id: i64, season_number: i32) -> Result<Option<SeasonDetail>, TmdbError> {
        // Check cache first
        let cache_key = format!("season:{}:{}", tv_id, season_number);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/tv/{}/season/{}", tv_id, season_number);
        let response: Option<SeasonDetail> = self.make_request(&endpoint).await?;

        // Cache result
        if let Some(ref detail) = response {
            let cached_value = serde_json::to_string(detail)?;
            self.cache.set(&cache_key, &cached_value, 86400).await?; // 24 hours TTL
        }

        Ok(response)
    }

    async fn fetch_episode(
        &self,
        tv_id: i64,
        season: i32,
        episode: i32,
    ) -> Result<Option<EpisodeDetail>, TmdbError> {
        // Check cache first
        let cache_key = format!("episode:{}:{}:{}", tv_id, season, episode);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/tv/{}/season/{}/episode/{}", tv_id, season, episode);
        let response: Option<EpisodeDetail> = self.make_request(&endpoint).await?;

        // Cache result
        if let Some(ref detail) = response {
            let cached_value = serde_json::to_string(detail)?;
            self.cache.set(&cache_key, &cached_value, 86400).await?; // 24 hours TTL
        }

        Ok(response)
    }

    async fn fetch_collection_details(&self, collection_id: i64) -> Result<Option<CollectionDetail>, TmdbError> {
        use super::dto::TmdbCollectionDetailsResponse;

        // Check cache first
        let cache_key = format!("collection:{}", collection_id);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/collection/{}", collection_id);
        let response: Option<TmdbCollectionDetailsResponse> = self.make_request(&endpoint).await?;

        let details = response.map(|r| CollectionDetail {
            id: r.id,
            name: r.name,
            overview: r.overview,
            poster_path: r.poster_path,
            backdrop_path: r.backdrop_path,
            total_parts: r.parts.len() as i32,
            parts: r.parts.into_iter().map(|p| CollectionPartInfo {
                tmdb_id: p.id,
                title: p.title,
                overview: p.overview,
                poster_path: p.poster_path,
                release_date: p.release_date,
            }).collect(),
        });

        // Cache result for 7 days (collections don't change often)
        if let Some(ref d) = details {
            let cached_value = serde_json::to_string(d)?;
            self.cache.set(&cache_key, &cached_value, 86400 * 7).await?;
        }

        Ok(details)
    }
}

#[async_trait]
impl TmdbResolver for TmdbClient {
    async fn find_by_external_id(&self, id: &str, source: &str) -> Result<Option<TmdbMatch>, TmdbError> {
        // Check cache first
        let cache_key = format!("{}:{}", source, id);
        if let Some(cached) = self.cache.get(&cache_key).await? {
            return Ok(serde_json::from_str(&cached)?);
        }

        let endpoint = format!("/find/{}?external_source={}", id, source);
        let response: TmdbFindResponse = self.make_request(&endpoint).await?;

        let result = response.movie_results
            .into_iter()
            .map(|m| TmdbMatch {
                tmdb_id: m.id,
                title: m.title,
                year: m.release_date.map(|d| d[..4].parse().unwrap_or(0)),
                media_type: "movie".to_string(),
                confidence: ConfidenceScore::new(0.95).unwrap(), // High confidence for IMDB ID
                strategy: MatchStrategy::ImdbId,
            })
            .next();

        // Cache result
        if let Some(ref result) = result {
            let cached_value = serde_json::to_string(result)?;
            self.cache.set(&cache_key, &cached_value, 86400).await?; // 24 hours TTL
        }

        Ok(result)
    }
}

use crate::interfaces::external_services::{
    TmdbContentRatingFetcher, TmdbSimilarFetcher, TmdbReconciler,
    ContentRatingInfo, SimilarResult,
};
use crate::domain::services::{ConfidenceService, DefaultConfidenceService};

#[async_trait]
impl TmdbContentRatingFetcher for TmdbClient {
    async fn fetch_movie_content_rating(&self, tmdb_id: i64) -> Result<ContentRatingInfo, TmdbError> {
        let endpoint = format!("/movie/{}/release_dates", tmdb_id);
        let response: Result<TmdbReleaseDatesResponse, _> = self.make_request(&endpoint).await;

        match response {
            Ok(body) => {
                // Prefer US rating, then GB, CA, AU
                let preferred_countries = ["US", "GB", "CA", "AU"];

                for country in preferred_countries {
                    if let Some(result) = body.results.iter().find(|r| r.iso_3166_1 == country) {
                        for rd in &result.release_dates {
                            if let Some(cert) = &rd.certification {
                                if !cert.is_empty() {
                                    return Ok(ContentRatingInfo {
                                        rating: Some(cert.clone()),
                                        descriptors: rd.descriptors.clone().unwrap_or_default(),
                                    });
                                }
                            }
                        }
                    }
                }

                // Fallback: any country with certification
                for result in &body.results {
                    for rd in &result.release_dates {
                        if let Some(cert) = &rd.certification {
                            if !cert.is_empty() {
                                return Ok(ContentRatingInfo {
                                    rating: Some(cert.clone()),
                                    descriptors: rd.descriptors.clone().unwrap_or_default(),
                                });
                            }
                        }
                    }
                }

                Ok(ContentRatingInfo::default())
            }
            Err(_) => Ok(ContentRatingInfo::default()),
        }
    }

    async fn fetch_tv_content_rating(&self, tmdb_id: i64) -> Result<ContentRatingInfo, TmdbError> {
        let endpoint = format!("/tv/{}/content_ratings", tmdb_id);
        let response: Result<TmdbTvContentRatingsResponse, _> = self.make_request(&endpoint).await;

        match response {
            Ok(body) => {
                // Prefer US rating
                let preferred_countries = ["US", "GB", "CA", "AU"];

                for country in preferred_countries {
                    if let Some(result) = body.results.iter().find(|r| r.iso_3166_1 == country) {
                        if !result.rating.is_empty() {
                            return Ok(ContentRatingInfo {
                                rating: Some(result.rating.clone()),
                                descriptors: Vec::new(),
                            });
                        }
                    }
                }

                // Fallback: first available
                if let Some(result) = body.results.first() {
                    if !result.rating.is_empty() {
                        return Ok(ContentRatingInfo {
                            rating: Some(result.rating.clone()),
                            descriptors: Vec::new(),
                        });
                    }
                }

                Ok(ContentRatingInfo::default())
            }
            Err(_) => Ok(ContentRatingInfo::default()),
        }
    }
}

#[async_trait]
impl TmdbSimilarFetcher for TmdbClient {
    async fn fetch_similar(&self, tmdb_id: i64, media_type: &str) -> Result<Vec<SimilarResult>, TmdbError> {
        let endpoint_type = if media_type == "tv" { "tv" } else { "movie" };
        let endpoint = format!("/{}/{}/similar", endpoint_type, tmdb_id);

        let response: Result<TmdbSimilarResponse, _> = self.make_request(&endpoint).await;

        match response {
            Ok(body) => {
                let results = body.results.into_iter().map(|r| {
                    SimilarResult {
                        id: r.id,
                        title: r.title.or(r.name).unwrap_or_default(),
                        poster_path: r.poster_path,
                        backdrop_path: r.backdrop_path,
                        release_date: r.release_date.or(r.first_air_date),
                        vote_average: r.vote_average.unwrap_or(0.0),
                        media_type: media_type.to_string(),
                    }
                }).collect();
                Ok(results)
            }
            Err(_) => Ok(Vec::new()),
        }
    }
}

#[async_trait]
impl TmdbCreditsFetcher for TmdbClient {
    async fn fetch_movie_credits(&self, tmdb_id: i64) -> Result<Credits, TmdbError> {
        let endpoint = format!("/movie/{}/credits", tmdb_id);
        let response: Result<TmdbCreditsResponse, _> = self.make_request(&endpoint).await;

        match response {
            Ok(body) => Ok(Credits {
                cast: body.cast.into_iter()
                    .take(15) // Limit to top 15 cast members
                    .map(|c| CastMember {
                        id: c.id,
                        name: c.name,
                        character: c.character.unwrap_or_default(),
                        profile_path: c.profile_path,
                        order: c.order.unwrap_or(999),
                    })
                    .collect(),
                crew: body.crew.into_iter()
                    .filter(|c| matches!(c.job.as_str(), "Director" | "Writer" | "Screenplay" | "Producer" | "Executive Producer"))
                    .take(10) // Limit crew
                    .map(|c| CrewMember {
                        id: c.id,
                        name: c.name,
                        job: c.job,
                        department: c.department,
                        profile_path: c.profile_path,
                    })
                    .collect(),
            }),
            Err(e) => {
                debug!("Failed to fetch movie credits for {}: {}", tmdb_id, e);
                Ok(Credits::default())
            }
        }
    }

    async fn fetch_tv_credits(&self, tmdb_id: i64) -> Result<Credits, TmdbError> {
        // For TV shows, use aggregate_credits which includes all seasons
        let endpoint = format!("/tv/{}/aggregate_credits", tmdb_id);
        let response: Result<TmdbTvAggregateCreditsResponse, _> = self.make_request(&endpoint).await;

        match response {
            Ok(body) => Ok(Credits {
                cast: body.cast.into_iter()
                    .take(15) // Limit to top 15 cast members
                    .map(|c| {
                        // Get the main character name from roles
                        let character = c.roles.first()
                            .map(|r| r.character.clone())
                            .unwrap_or_default();
                        CastMember {
                            id: c.id,
                            name: c.name,
                            character,
                            profile_path: c.profile_path,
                            order: c.order.unwrap_or(999),
                        }
                    })
                    .collect(),
                crew: body.crew.into_iter()
                    .filter(|c| {
                        c.jobs.iter().any(|j| matches!(j.job.as_str(),
                            "Director" | "Writer" | "Creator" | "Executive Producer" | "Showrunner"))
                    })
                    .take(10) // Limit crew
                    .map(|c| {
                        let job = c.jobs.first()
                            .map(|j| j.job.clone())
                            .unwrap_or_default();
                        CrewMember {
                            id: c.id,
                            name: c.name,
                            job,
                            department: c.department,
                            profile_path: c.profile_path,
                        }
                    })
                    .collect(),
            }),
            Err(e) => {
                debug!("Failed to fetch TV credits for {}: {}", tmdb_id, e);
                Ok(Credits::default())
            }
        }
    }
}

#[async_trait]
impl TmdbReconciler for TmdbClient {
    async fn reconcile(
        &self,
        target_title: &str,
        target_year: Option<i32>,
        folder_name: Option<&str>,
        detected_type: &str,
        nfo_is_xml: bool,
        detected_season: Option<i32>,
        detected_episode: Option<i32>,
    ) -> Result<(Option<TmdbMatch>, Vec<TmdbMatch>), TmdbError> {
        let mut candidates: Vec<TmdbMatch> = Vec::new();
        let confidence_service = DefaultConfidenceService::new();

        // Strategy 2: Filename + Year
        if let Some(year) = target_year {
            let mut matches = if detected_type == "movie" {
                self.search_movie(target_title, Some(year)).await?
            } else {
                self.search_tv(target_title, Some(year)).await?
            };
            for m in &mut matches {
                m.strategy = MatchStrategy::FilenameWithYear;
            }
            candidates.extend(matches);
        }

        // Strategy 3: Folder + Year
        if let Some(folder) = folder_name {
            let mut matches = if detected_type == "movie" {
                self.search_movie(folder, target_year).await?
            } else {
                self.search_tv(folder, target_year).await?
            };
            for m in &mut matches {
                m.strategy = MatchStrategy::FolderWithYear;
            }
            candidates.extend(matches);
        }

        // Strategy 4: Filename Only (Year Agnostic)
        let mut matches = if detected_type == "movie" {
            self.search_movie(target_title, None).await?
        } else {
            self.search_tv(target_title, None).await?
        };
        for m in &mut matches {
            m.strategy = MatchStrategy::FilenameOnly;
        }
        candidates.extend(matches);

        // Strategy 6: Alternative Titles (use TitleNormalizer for variants)
        if candidates.is_empty() {
            // Generate search variants (roman numerals, without articles, etc.)
            let search_variants = TitleNormalizer::get_search_variants(target_title);

            for variant in search_variants.iter().skip(1) { // Skip original, already tried
                let mut matches = if detected_type == "movie" {
                    self.search_movie(variant, target_year).await?
                } else {
                    self.search_tv(variant, target_year).await?
                };
                for m in &mut matches {
                    m.strategy = MatchStrategy::AlternativeTitle;
                }
                candidates.extend(matches);

                // Don't make too many API calls
                if candidates.len() >= 5 {
                    break;
                }
            }
        }

        // Strategy 7: Fuzzy search fallback
        if candidates.is_empty() {
            if let Some(fuzzy_match) = self.search_fuzzy(target_title, target_year, detected_type).await? {
                candidates.push(fuzzy_match);
            }
        }

        // Remove duplicates
        candidates.sort_by_key(|m| m.tmdb_id);
        candidates.dedup_by_key(|m| m.tmdb_id);

        // Strategy 5: Cross-Validation & Scoring
        for c in candidates.iter_mut() {
            let jw = strsim::jaro_winkler(&target_title.to_lowercase(), &c.title.to_lowercase());
            let lev = strsim::levenshtein(&target_title.to_lowercase(), &c.title.to_lowercase());

            let has_year_match = if let (Some(ty), Some(cy)) = (target_year, c.year) {
                (ty - cy).abs() <= 1
            } else {
                false
            };

            // Cross-validation: verify episode exists for TV
            let mut validates_episode = false;
            if c.media_type == "tv" {
                if let (Some(s), Some(e)) = (detected_season, detected_episode) {
                    if let Ok(Some(_)) = self.fetch_episode(c.tmdb_id, s, e).await {
                        validates_episode = true;
                    }
                }
            }

            let score = confidence_service.calculate_identification_score(
                &c.strategy,
                Some(lev),
                jw as f32,
                has_year_match,
                nfo_is_xml,
                validates_episode,
                false,
            );

            c.confidence = ConfidenceScore::new(score).unwrap_or_default();
        }

        // Apply multi-candidate penalty
        if candidates.len() > 1 {
            for c in candidates.iter_mut() {
                let current = c.confidence.value();
                c.confidence = ConfidenceScore::new((current - 0.15).max(0.0)).unwrap_or_default();
            }
        }

        // Sort by confidence descending
        candidates.sort_by(|a, b| {
            b.confidence.value().partial_cmp(&a.confidence.value()).unwrap_or(std::cmp::Ordering::Equal)
        });

        let best = candidates.first().cloned();
        let alternatives = if candidates.len() > 1 {
            candidates.iter().skip(1).take(3).cloned().collect()
        } else {
            Vec::new()
        };

        Ok((best, alternatives))
    }
}

/// Rate limiter for TMDB API
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    permits_per_second: usize,
}

impl RateLimiter {
    /// Creates a new rate limiter
    ///
    /// # Arguments
    /// * `permits_per_second` - Number of requests allowed per second
    pub fn new(permits_per_second: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits_per_second)),
            permits_per_second,
        }
    }

    /// Acquires a permit from the rate limiter
    pub async fn acquire(&self) {
        let _permit = self.semaphore.acquire().await;
        tokio::time::sleep(Duration::from_millis(1000) / self.permits_per_second as u32).await;
    }
}

// ============================================================================
// TMDB API Response DTOs
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct TmdbSearchResponse {
    results: Vec<TmdbSearchResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbSearchResult {
    id: i64,
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbFindResponse {
    movie_results: Vec<TmdbMovieResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbMovieResult {
    id: i64,
    title: String,
    release_date: Option<String>,
}

// Release dates response for content ratings
#[derive(Debug, serde::Deserialize)]
struct TmdbReleaseDatesResponse {
    results: Vec<TmdbReleaseDateResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbReleaseDateResult {
    iso_3166_1: String,
    release_dates: Vec<TmdbReleaseDate>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbReleaseDate {
    certification: Option<String>,
    descriptors: Option<Vec<String>>,
}

// TV content ratings response
#[derive(Debug, serde::Deserialize)]
struct TmdbTvContentRatingsResponse {
    results: Vec<TmdbTvContentRating>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbTvContentRating {
    iso_3166_1: String,
    rating: String,
}

// Similar content response
#[derive(Debug, serde::Deserialize)]
struct TmdbSimilarResponse {
    results: Vec<TmdbSimilarResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbSimilarResult {
    id: i64,
    title: Option<String>,
    name: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f32>,
}

// Credits response (for movies)
#[derive(Debug, serde::Deserialize)]
struct TmdbCreditsResponse {
    cast: Vec<TmdbCastMember>,
    crew: Vec<TmdbCrewMember>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbCastMember {
    id: i64,
    name: String,
    character: Option<String>,
    profile_path: Option<String>,
    order: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbCrewMember {
    id: i64,
    name: String,
    job: String,
    department: String,
    profile_path: Option<String>,
}

// TV aggregate credits response
#[derive(Debug, serde::Deserialize)]
struct TmdbTvAggregateCreditsResponse {
    cast: Vec<TmdbTvAggregateCastMember>,
    crew: Vec<TmdbTvAggregateCrewMember>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbTvAggregateCastMember {
    id: i64,
    name: String,
    profile_path: Option<String>,
    order: Option<i32>,
    roles: Vec<TmdbTvRole>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbTvRole {
    character: String,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbTvAggregateCrewMember {
    id: i64,
    name: String,
    department: String,
    profile_path: Option<String>,
    jobs: Vec<TmdbTvJob>,
}

#[derive(Debug, serde::Deserialize)]
struct TmdbTvJob {
    job: String,
}
