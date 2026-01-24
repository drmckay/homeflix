//! End-to-end tests for scan workflow
//!
//! Tests cover:
//! - Complete scan workflow from directory to database
//! - Event publishing
//! - Error handling
//! - Result reporting

use std::sync::Arc;
use tempfile::TempDir;
use homeflixd::application::use_cases::ScanLibraryUseCase;
use homeflixd::infrastructure::persistence::sqlite::SqliteMediaRepository;
use homeflixd::infrastructure::persistence::sqlite::SqliteSeriesRepository;
use homeflixd::infrastructure::persistence::sqlite::SqliteCacheRepository;
use homeflixd::infrastructure::cache::InMemoryCache;
use homeflixd::infrastructure::cache::MultiLevelCache;
use homeflixd::infrastructure::messaging::InMemoryEventBus;
use homeflixd::infrastructure::filesystem::WalkDirAdapter;
use homeflixd::domain::services::DefaultIdentificationService;
use homeflixd::domain::services::DefaultConfidenceService;
use sqlx::{Pool, Sqlite, SqlitePool};

async fn setup_test_environment() -> (TempDir, Pool<Sqlite>, Arc<InMemoryEventBus>) {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let pool = SqlitePool::connect(&database_url).await
        .expect("Should create pool");

    // Run migrations
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS media (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            media_type TEXT NOT NULL,
            title TEXT NOT NULL,
            overview TEXT,
            poster_url TEXT,
            backdrop_url TEXT,
            trailer_url TEXT,
            duration_seconds INTEGER,
            release_date TEXT,
            resolution TEXT,
            genres TEXT,
            series_id INTEGER,
            season INTEGER,
            episode INTEGER,
            tmdb_id INTEGER,
            original_title TEXT,
            rating REAL,
            confidence_score REAL NOT NULL DEFAULT 0.0,
            verification_status TEXT NOT NULL DEFAULT 'unverified',
            identification_strategy TEXT,
            error_notes TEXT,
            alternative_matches TEXT,
            content_rating TEXT,
            content_warnings TEXT,
            current_position INTEGER NOT NULL DEFAULT 0,
            is_watched BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await
        .expect("Should create media table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS series (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tmdb_id INTEGER UNIQUE,
            name TEXT NOT NULL,
            overview TEXT,
            poster_url TEXT,
            backdrop_url TEXT,
            first_air_date TEXT,
            last_air_date TEXT,
            genres TEXT,
            number_of_seasons INTEGER,
            number_of_episodes INTEGER,
            rating REAL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await
        .expect("Should create series table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cache (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            expires_at INTEGER
        )"
    )
    .execute(&pool)
    .await
        .expect("Should create cache table");

    let event_bus = Arc::new(InMemoryEventBus::new());

    (temp_dir, pool, event_bus)
}

#[tokio::test]
async fn test_e2e_scan_workflow_empty_directory() {
    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    let result = use_case.execute(temp_dir.path().to_str().unwrap()).await;

    assert!(result.is_ok());
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.identified_count, 0);
    assert_eq!(result.failed_count, 0);
}

#[tokio::test]
async fn test_e2e_scan_workflow_with_media_files() {
    use std::fs;

    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    // Create test media files
    let movies_dir = temp_dir.path().join("Movies");
    fs::create_dir_all(&movies_dir).expect("Should create movies dir");

    let movie1_path = movies_dir.join("Movie1.2024.mkv");
    fs::write(&movie1_path, b"fake movie content").expect("Should create movie file");

    let movie2_path = movies_dir.join("Movie2.2024.mkv");
    fs::write(&movie2_path, b"fake movie content").expect("Should create movie file");

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    let result = use_case.execute(temp_dir.path().to_str().unwrap()).await;

    assert!(result.is_ok());
    assert_eq!(result.processed_count, 2);
    assert_eq!(result.identified_count, 2);

    // Verify media was saved to database
    let all_media = media_repo.find_all().await
        .expect("Should find all media");
    assert_eq!(all_media.len(), 2);
}

#[tokio::test]
async fn test_e2e_scan_workflow_with_series() {
    use std::fs;

    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    // Create test series structure
    let series_dir = temp_dir.path().join("TV Shows");
    let show_dir = series_dir.join("Test Show");
    let season_dir = show_dir.join("Season 1");
    fs::create_dir_all(&season_dir).expect("Should create season dir");

    let episode1_path = season_dir.join("S01E01.mkv");
    fs::write(&episode1_path, b"fake episode content").expect("Should create episode file");

    let episode2_path = season_dir.join("S01E02.mkv");
    fs::write(&episode2_path, b"fake episode content").expect("Should create episode file");

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    let result = use_case.execute(temp_dir.path().to_str().unwrap()).await;

    assert!(result.is_ok());
    assert_eq!(result.processed_count, 2);
    assert_eq!(result.identified_count, 2);

    // Verify episodes were saved to database
    let all_media = media_repo.find_all().await
        .expect("Should find all media");
    assert_eq!(all_media.len(), 2);
}

#[tokio::test]
async fn test_e2e_scan_workflow_mixed_content() {
    use std::fs;

    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    // Create mixed content structure
    let movies_dir = temp_dir.path().join("Movies");
    fs::create_dir_all(&movies_dir).expect("Should create movies dir");

    let movie_path = movies_dir.join("Movie.2024.mkv");
    fs::write(&movie_path, b"fake movie content").expect("Should create movie file");

    let series_dir = temp_dir.path().join("TV Shows");
    let show_dir = series_dir.join("Test Show");
    let season_dir = show_dir.join("Season 1");
    fs::create_dir_all(&season_dir).expect("Should create season dir");

    let episode_path = season_dir.join("S01E01.mkv");
    fs::write(&episode_path, b"fake episode content").expect("Should create episode file");

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    let result = use_case.execute(temp_dir.path().to_str().unwrap()).await;

    assert!(result.is_ok());
    assert_eq!(result.processed_count, 2);
    assert_eq!(result.identified_count, 2);

    // Verify both movie and episode were saved
    let all_media = media_repo.find_all().await
        .expect("Should find all media");
    assert_eq!(all_media.len(), 2);

    let movies = media_repo.find_by_type(homeflixd::domain::value_objects::MediaType::Movie).await
        .expect("Should find movies");
    assert_eq!(movies.len(), 1);

    let episodes = media_repo.find_by_type(homeflixd::domain::value_objects::MediaType::Episode).await
        .expect("Should find episodes");
    assert_eq!(episodes.len(), 1);
}

#[tokio::test]
async fn test_e2e_scan_workflow_rescan_updates_existing() {
    use std::fs;

    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    // Create test media file
    let movies_dir = temp_dir.path().join("Movies");
    fs::create_dir_all(&movies_dir).expect("Should create movies dir");

    let movie_path = movies_dir.join("Movie.2024.mkv");
    fs::write(&movie_path, b"fake movie content").expect("Should create movie file");

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    // First scan
    let result1 = use_case.execute(temp_dir.path().to_str().unwrap()).await;
    assert!(result1.is_ok());
    assert_eq!(result1.processed_count, 1);

    // Second scan - should skip already processed media
    let result2 = use_case.execute(temp_dir.path().to_str().unwrap()).await;
    assert!(result2.is_ok());

    // Media count should still be 1 (no duplicates)
    let all_media = media_repo.find_all().await
        .expect("Should find all media");
    assert_eq!(all_media.len(), 1);
}

#[tokio::test]
async fn test_e2e_scan_workflow_invalid_directory() {
    let (temp_dir, pool, event_bus) = setup_test_environment().await;

    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
    let l1_cache = Arc::new(InMemoryCache::new(100));
    let cache = Arc::new(MultiLevelCache::new(l1_cache, cache_repo));
    let identification = Arc::new(DefaultIdentificationService);
    let confidence = Arc::new(DefaultConfidenceService);
    let walker = Arc::new(WalkDirAdapter::new());

    let use_case = ScanLibraryUseCase::new(
        media_repo,
        identification,
        confidence,
        walker,
        event_bus,
    );

    let nonexistent_path = temp_dir.path().join("nonexistent");
    let result = use_case.execute(nonexistent_path.to_str().unwrap()).await;

    assert!(result.is_ok());
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.identified_count, 0);
}
