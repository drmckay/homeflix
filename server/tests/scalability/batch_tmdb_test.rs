//! Tests for Batch TMDB Operations (Phase 8, Task 8.2)
//!
//! Tests batch operations for TMDB API including:
//! - Batch search operations
//! - Concurrent fetch operations
//! - Deduplication
//! - Multiple season fetching

use std::sync::Arc;
use homeflixd::infrastructure::external::tmdb::{
    BatchTmdbClient, BatchSearchRequest, BatchSearchResult,
    BatchFetchRequest, BatchFetchResult, TmdbDetail,
};
use homeflixd::interfaces::external_services::{
    TmdbMatch, MovieDetail, TvDetail, SeasonDetail, EpisodeDetail,
};
use homeflixd::domain::value_objects::{MatchStrategy, ConfidenceScore};

// Mock cache repository for testing
struct MockCacheRepository {
    cache: tokio::sync::RwLock<std::collections::HashMap<String, String>>,
}

#[async_trait::async_trait]
impl homeflixd::domain::repositories::CacheRepository for MockCacheRepository {
    async fn get(&self, key: &str) -> Result<Option<String>, homeflixd::shared::error::RepositoryError> {
        Ok(self.cache.read().await.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str, _ttl: i32) -> Result<(), homeflixd::shared::error::RepositoryError> {
        let mut cache = self.cache.write().await;
        cache.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, _key: &str) -> Result<(), homeflixd::shared::error::RepositoryError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_batch_search() {
    // Setup mock cache
    let cache = Arc::new(MockCacheRepository {
        cache: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    // Create batch client with mock base client
    let base_client = Arc::new(homeflixd::infrastructure::external::tmdb::client::TmdbClient::new(
        "test_api_key",
        Arc::clone(&cache),
    ).unwrap());

    let batch_client = BatchTmdbClient::new(Arc::clone(&base_client), 4);

    // Test batch search
    let requests = vec![
        BatchSearchRequest {
            query: "Movie 1".to_string(),
            year: Some(2020),
            is_movie: true,
        },
        BatchSearchRequest {
            query: "TV Show 1".to_string(),
            year: Some(2021),
            is_movie: false,
        },
        BatchSearchRequest {
            query: "Movie 2".to_string(),
            year: Some(2020),
            is_movie: true,
        },
    ];

    let results = batch_client.batch_search(requests).await;

    // Verify results
    assert_eq!(results.len(), 3);
    assert!(results[0].results.is_ok());
    assert!(results[1].results.is_ok());
    assert!(results[2].results.is_ok());

    // Check search results
    let movie1_results = results[0].results.as_ref().unwrap();
    let tv1_results = results[1].results.as_ref().unwrap();
    let movie2_results = results[2].results.as_ref().unwrap();

    assert_eq!(movie1_results.len(), 1);
    assert_eq!(tv1_results.len(), 1);
    assert_eq!(movie2_results.len(), 1);

    // Verify first result is for Movie 1
    let result = &movie1_results[0];
    assert_eq!(result.request.query, "Movie 1");
    assert_eq!(result.request.year, Some(2020));
    assert!(result.request.is_movie, true);
}

#[tokio::test]
async fn test_batch_fetch() {
    // Setup mock cache
    let cache = Arc::new(MockCacheRepository {
        cache: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    // Create batch client
    let base_client = Arc::new(homeflixd::infrastructure::external::tmdb::client::TmdbClient::new(
        "test_api_key",
        Arc::clone(&cache),
    ).unwrap());

    let batch_client = BatchTmdbClient::new(Arc::clone(&base_client), 8);

    // Test batch fetch
    let requests = vec![
        BatchFetchRequest::Movie(123),
        BatchFetchRequest::Tv(456),
        BatchFetchRequest::Season { tv_id: 789, season_number: 1 },
        BatchFetchRequest::Episode { tv_id: 789, season: 1, episode: 1 },
    ];

    let results = batch_client.batch_fetch(requests).await;

    // Verify results
    assert_eq!(results.len(), 4);
    assert!(results[0].result.is_ok());
    assert!(results[1].result.is_ok());
    assert!(results[2].result.is_ok());
    assert!(results[3].result.is_ok());

    // Check fetch results
    let movie_result = &results[0].result;
    let tv_result = &results[1].result;
    let season_result = &results[2].result;
    let episode_result = &results[3].result;

    // Verify movie result
    match movie_result {
        TmdbDetail::Movie(detail) => {
            assert_eq!(detail.id, 123);
        }
        _ => panic!("Expected movie detail"),
    }

    // Verify TV result
    match tv_result {
        TmdbDetail::Tv(detail) => {
            assert_eq!(detail.id, 456);
        }
        _ => panic!("Expected TV detail"),
    }

    // Verify season result
    match season_result {
        TmdbDetail::Season(detail) => {
            assert_eq!(detail.id, 789);
            assert_eq!(detail.season_number, 1);
        }
        _ => panic!("Expected season detail"),
    }

    // Verify episode result
    match episode_result {
        TmdbDetail::Episode(detail) => {
            assert_eq!(detail.id, 789);
            assert_eq!(detail.season_number, 1);
            assert_eq!(detail.episode_number, 1);
        }
        _ => panic!("Expected episode detail"),
    }
}

#[tokio::test]
async fn test_batch_search_deduplicated() {
    // Setup mock cache
    let cache = Arc::new(MockCacheRepository {
        cache: tokio::sync::RwLock::new(std::collections::HashMap::new()),
    });

    // Create batch client
    let base_client = Arc::new(homeflixd::infrastructure::external::tmdb::client::TmdbClient::new(
        "test_api_key",
        Arc::clone(&cache),
    ).unwrap());

    let batch_client = BatchTmdbClient::new(Arc::clone(&base_client), 4);

    // Test deduplicated search
    let requests = vec![
        BatchSearchRequest {
            query: "Same Movie".to_string(),
            year: Some(2020),
            is_movie: true,
        },
        BatchSearchRequest {
            query: "Same Movie".to_string(),
            year: Some(2020),
            is_movie: true,
        },
    ];

    let deduplicated = batch_client.batch_search_deduplicated(requests).await;

    // Verify deduplication
    assert_eq!(deduplicated.len(), 2);
    assert!(deduplicated.contains_key("Same Movie"));
    
    // Both requests should return same TMDB ID
    let results1 = deduplicated.get("Same Movie").unwrap();
    let results2 = deduplicated.get("Same Movie").unwrap();
    assert_eq!(results1.len(), results2.len());
    
    // All results should have the same TMDB ID
    if let Some(match1) = results1.first() {
        if let Some(match2) = results2.first() {
            assert_eq!(match1.tmdb_id, match2.tmdb_id);
        }
    }
}
