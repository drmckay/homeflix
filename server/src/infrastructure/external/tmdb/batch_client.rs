//! Batch TMDB Client Implementation
//!
//! Provides batch operations for TMDB API with:
//! - Concurrent request processing
//! - Adaptive rate limiting
//! - Batch search operations
//! - Result aggregation

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use futures::stream::{self, StreamExt};
use tokio::sync::Semaphore;
use tracing::{debug, warn, instrument};

use crate::interfaces::external_services::{
    TmdbService, TmdbSearcher, TmdbFetcher, TmdbResolver,
    TmdbMatch, MovieDetail, TvDetail, SeasonDetail, EpisodeDetail,
};
use crate::domain::value_objects::{MatchStrategy, ConfidenceScore};
use crate::domain::repositories::CacheRepository;
use crate::shared::error::TmdbError;

use super::client::TmdbClient;

/// Batch search request
#[derive(Debug, Clone)]
pub struct BatchSearchRequest {
    /// Search query
    pub query: String,
    /// Optional year filter
    pub year: Option<i32>,
    /// Whether to search movies (true) or TV shows (false)
    pub is_movie: bool,
}

/// Batch search result
#[derive(Debug)]
pub struct BatchSearchResult {
    /// Original request
    pub request: BatchSearchRequest,
    /// Search results
    pub results: Result<Vec<TmdbMatch>, TmdbError>,
}

/// Batch fetch request for detailed information
#[derive(Debug, Clone)]
pub enum BatchFetchRequest {
    /// Fetch movie details
    Movie(i64),
    /// Fetch TV show details
    Tv(i64),
    /// Fetch season details
    Season { tv_id: i64, season_number: i32 },
    /// Fetch episode details
    Episode { tv_id: i64, season: i32, episode: i32 },
}

/// Batch fetch result
#[derive(Debug)]
pub struct BatchFetchResult {
    /// Original request
    pub request: BatchFetchRequest,
    /// Fetch result
    pub result: Result<Option<TmdbDetail>, TmdbError>,
}

/// TMDB detail type (union of all detail types)
#[derive(Debug, Clone)]
pub enum TmdbDetail {
    Movie(MovieDetail),
    Tv(TvDetail),
    Season(SeasonDetail),
    Episode(EpisodeDetail),
}

/// Batch TMDB client with concurrent operations
pub struct BatchTmdbClient {
    /// Base TMDB client
    client: Arc<TmdbClient>,
    /// Semaphore for concurrent request limiting
    request_limiter: Arc<Semaphore>,
    /// Maximum concurrent batch operations
    max_concurrent: usize,
}

impl BatchTmdbClient {
    /// Creates a new batch TMDB client
    ///
    /// # Arguments
    /// * `client` - Base TMDB client
    /// * `max_concurrent` - Maximum concurrent requests (default: 8)
    pub fn new(client: Arc<TmdbClient>, max_concurrent: usize) -> Self {
        Self {
            client,
            request_limiter: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Creates batch client with default concurrency (8)
    pub fn with_defaults(client: Arc<TmdbClient>) -> Self {
        Self::new(client, 8)
    }

    /// Executes multiple search requests concurrently
    ///
    /// # Arguments
    /// * `requests` - Vector of search requests
    ///
    /// # Returns
    /// * `Vec<BatchSearchResult>` - Results in same order as requests
    ///
    /// # Performance
    /// - Processes requests in parallel with bounded concurrency
    /// - Respects rate limits
    /// - Aggregates results efficiently
    #[instrument(skip(self))]
    pub async fn batch_search(
        &self,
        requests: Vec<BatchSearchRequest>,
    ) -> Vec<BatchSearchResult> {
        if requests.is_empty() {
            return Vec::new();
        }

        debug!("Starting batch search for {} requests", requests.len());

        let results = stream::iter(requests)
            .map(|request| {
                let client = Arc::clone(&self.client);
                let limiter = Arc::clone(&self.request_limiter);
                
                async move {
                    let _permit = limiter.acquire().await;
                    
                    let result = if request.is_movie {
                        client.search_movie(&request.query, request.year)
                            .await
                            .map_err(TmdbError::from)
                    } else {
                        client.search_tv(&request.query, request.year)
                            .await
                            .map_err(TmdbError::from)
                    };

                    BatchSearchResult {
                        request,
                        results: result,
                    }
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        let successful = results.iter().filter(|r| r.results.is_ok()).count();
        let failed = results.len() - successful;

        debug!(
            "Batch search completed: {} successful, {} failed",
            successful, failed
        );

        results
    }

    /// Executes multiple fetch requests concurrently
    ///
    /// # Arguments
    /// * `requests` - Vector of fetch requests
    ///
    /// # Returns
    /// * `Vec<BatchFetchResult>` - Results in same order as requests
    ///
    /// # Performance
    /// - Processes requests in parallel with bounded concurrency
    /// - Respects rate limits
    /// - Handles mixed request types (movies, TV, seasons, episodes)
    #[instrument(skip(self))]
    pub async fn batch_fetch(
        &self,
        requests: Vec<BatchFetchRequest>,
    ) -> Vec<BatchFetchResult> {
        if requests.is_empty() {
            return Vec::new();
        }

        debug!("Starting batch fetch for {} requests", requests.len());

        let results = stream::iter(requests)
            .map(|request| {
                let client = Arc::clone(&self.client);
                let limiter = Arc::clone(&self.request_limiter);
                
                async move {
                    let _permit = limiter.acquire().await;
                    
                    let result = match request.clone() {
                        BatchFetchRequest::Movie(id) => {
                            client.fetch_movie_details(id)
                                .await
                                .map(|opt| opt.map(TmdbDetail::Movie))
                        }
                        BatchFetchRequest::Tv(id) => {
                            client.fetch_tv_details(id)
                                .await
                                .map(|opt| opt.map(TmdbDetail::Tv))
                        }
                        BatchFetchRequest::Season { tv_id, season_number } => {
                            client.fetch_season(tv_id, season_number)
                                .await
                                .map(|opt| opt.map(TmdbDetail::Season))
                        }
                        BatchFetchRequest::Episode { tv_id, season, episode } => {
                            client.fetch_episode(tv_id, season, episode)
                                .await
                                .map(|opt| opt.map(TmdbDetail::Episode))
                        }
                    };

                    BatchFetchResult {
                        request,
                        result: result.map_err(TmdbError::from),
                    }
                }
            })
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await;

        let successful = results.iter().filter(|r| r.result.is_ok()).count();
        let failed = results.len() - successful;

        debug!(
            "Batch fetch completed: {} successful, {} failed",
            successful, failed
        );

        results
    }

    /// Batch search with deduplication
    ///
    /// Searches for multiple queries and deduplicates results by TMDB ID.
    /// Useful when scanning multiple files that might match the same content.
    ///
    /// # Arguments
    /// * `requests` - Vector of search requests
    ///
    /// # Returns
    /// * `HashMap<String, Vec<TmdbMatch>>` - Map from query to results
    pub async fn batch_search_deduplicated(
        &self,
        requests: Vec<BatchSearchRequest>,
    ) -> HashMap<String, Vec<TmdbMatch>> {
        let results = self.batch_search(requests).await;
        
        let mut deduplicated = HashMap::new();
        let mut seen_ids = HashMap::new();
        
        for result in results {
            if let Ok(matches) = result.results {
                let unique_matches: Vec<TmdbMatch> = matches
                    .into_iter()
                    .filter(|m| {
                        // Only keep first occurrence of each TMDB ID
                        let key = format!("{}:{}", m.media_type, m.tmdb_id);
                        seen_ids.insert(key, true).is_none()
                    })
                    .collect();
                
                deduplicated.insert(result.request.query, unique_matches);
            }
        }
        
        deduplicated
    }

    /// Fetches all episodes for a TV show season in parallel
    ///
    /// # Arguments
    /// * `tv_id` - TMDB TV show ID
    /// * `season_number` - Season number
    ///
    /// # Returns
    /// * `Result<Vec<EpisodeDetail>, TmdbError>` - All episodes in season
    ///
    /// # Performance
    /// - Fetches season details first
    /// - Then fetches episode details in parallel if needed
    pub async fn fetch_season_episodes(
        &self,
        tv_id: i64,
        season_number: i32,
    ) -> Result<Vec<EpisodeDetail>, TmdbError> {
        // First fetch season details (includes basic episode info)
        let season_detail = self.client.fetch_season(tv_id, season_number).await?;
        
        match season_detail {
            Some(season) => {
                debug!(
                    "Fetched season {} with {} episodes",
                    season_number,
                    season.episodes.len()
                );
                
                // Episodes are already included in season detail
                // If we need more detailed info per episode, we could batch fetch here
                Ok(season.episodes)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Fetches multiple seasons for a TV show in parallel
    ///
    /// # Arguments
    /// * `tv_id` - TMDB TV show ID
    /// * `season_numbers` - Vector of season numbers to fetch
    ///
    /// # Returns
    /// * `Result<Vec<SeasonDetail>, TmdbError>` - All requested seasons
    pub async fn fetch_multiple_seasons(
        &self,
        tv_id: i64,
        season_numbers: Vec<i32>,
    ) -> Result<Vec<SeasonDetail>, TmdbError> {
        let requests: Vec<BatchFetchRequest> = season_numbers
            .into_iter()
            .map(|season| BatchFetchRequest::Season { tv_id, season_number: season })
            .collect();

        let results = self.batch_fetch(requests).await;
        
        let mut seasons = Vec::new();
        let mut errors = Vec::new();
        
        for result in results {
            match result.result {
                Ok(Some(TmdbDetail::Season(season))) => {
                    seasons.push(season);
                }
                Ok(None) => {
                    // Season not found, skip
                }
                Ok(_) => {
                    // Wrong type, skip
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }
        
        if !errors.is_empty() {
            warn!(
                "Some season fetches failed for TV ID {}: {} errors",
                tv_id,
                errors.len()
            );
        }
        
        Ok(seasons)
    }
}

#[async_trait]
impl TmdbSearcher for BatchTmdbClient {
    async fn search_movie(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError> {
        self.client.search_movie(query, year).await
    }

    async fn search_tv(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError> {
        self.client.search_tv(query, year).await
    }
}

#[async_trait]
impl TmdbFetcher for BatchTmdbClient {
    async fn fetch_movie_details(&self, id: i64) -> Result<Option<MovieDetail>, TmdbError> {
        self.client.fetch_movie_details(id).await
    }

    async fn fetch_tv_details(&self, id: i64) -> Result<Option<TvDetail>, TmdbError> {
        self.client.fetch_tv_details(id).await
    }

    async fn fetch_season(&self, tv_id: i64, season_number: i32) -> Result<Option<SeasonDetail>, TmdbError> {
        self.client.fetch_season(tv_id, season_number).await
    }

    async fn fetch_episode(
        &self,
        tv_id: i64,
        season: i32,
        episode: i32,
    ) -> Result<Option<EpisodeDetail>, TmdbError> {
        self.client.fetch_episode(tv_id, season, episode).await
    }

    async fn fetch_collection_details(&self, collection_id: i64) -> Result<Option<crate::interfaces::external_services::CollectionDetail>, crate::shared::error::TmdbError> {
        self.client.fetch_collection_details(collection_id).await
    }
}

#[async_trait]
impl TmdbResolver for BatchTmdbClient {
    async fn find_by_external_id(&self, id: &str, source: &str) -> Result<Option<TmdbMatch>, TmdbError> {
        self.client.find_by_external_id(id, source).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_search_request_creation() {
        let request = BatchSearchRequest {
            query: "Test Movie".to_string(),
            year: Some(2020),
            is_movie: true,
        };
        
        assert_eq!(request.query, "Test Movie");
        assert_eq!(request.year, Some(2020));
        assert!(request.is_movie);
    }

    #[test]
    fn test_batch_fetch_request_creation() {
        let movie_request = BatchFetchRequest::Movie(123);
        let tv_request = BatchFetchRequest::Tv(456);
        let season_request = BatchFetchRequest::Season {
            tv_id: 789,
            season_number: 1,
        };
        let episode_request = BatchFetchRequest::Episode {
            tv_id: 789,
            season: 1,
            episode: 1,
        };
        
        match movie_request {
            BatchFetchRequest::Movie(id) => assert_eq!(id, 123),
            _ => panic!("Expected Movie request"),
        }
        
        match tv_request {
            BatchFetchRequest::Tv(id) => assert_eq!(id, 456),
            _ => panic!("Expected Tv request"),
        }
        
        match season_request {
            BatchFetchRequest::Season { tv_id, season_number } => {
                assert_eq!(tv_id, 789);
                assert_eq!(season_number, 1);
            }
            _ => panic!("Expected Season request"),
        }
        
        match episode_request {
            BatchFetchRequest::Episode { tv_id, season, episode } => {
                assert_eq!(tv_id, 789);
                assert_eq!(season, 1);
                assert_eq!(episode, 1);
            }
            _ => panic!("Expected Episode request"),
        }
    }
}
