//! Integration tests for SqliteMediaRepository
//!
//! Tests cover:
//! - CRUD operations
//! - Query operations
//! - Transaction handling
//! - Error handling

use std::sync::Arc;
use sqlx::{Pool, Sqlite, SqlitePool};
use tempfile::NamedTempFile;
use homeflixd::infrastructure::persistence::sqlite::SqliteMediaRepository;
use homeflixd::domain::entities::Media;
use homeflixd::domain::value_objects::{MediaType, ConfidenceScore, VerificationStatus};
use homeflixd::domain::repositories::MediaRepository;

async fn create_test_pool() -> Pool<Sqlite> {
    let temp_file = NamedTempFile::new().expect("Should create temp file");
    let database_url = format!("sqlite:{}", temp_file.path().display());
    SqlitePool::connect(&database_url).await
        .expect("Should create pool")
}

#[tokio::test]
async fn test_media_repository_create() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let id = repo.save(&media).await
        .expect("Should save media");

    assert!(id > 0);
}

#[tokio::test]
async fn test_media_repository_find_by_id() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let id = repo.save(&media).await
        .expect("Should save media");

    let found = repo.find_by_id(id).await
        .expect("Should find media");

    assert!(found.is_some());
    assert_eq!(found.unwrap().file_path, media.file_path);
    assert_eq!(found.unwrap().title, media.title);
}

#[tokio::test]
async fn test_media_repository_find_by_id_not_found() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let found = repo.find_by_id(99999).await
        .expect("Should execute query");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_media_repository_find_by_path() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    repo.save(&media).await
        .expect("Should save media");

    let found = repo.find_by_path("/path/to/movie.mkv").await
        .expect("Should find media");

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, media.id);
}

#[tokio::test]
async fn test_media_repository_find_by_path_not_found() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let found = repo.find_by_path("/nonexistent/path.mkv").await
        .expect("Should execute query");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_media_repository_find_all() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media1 = Media::new(
        "/path/to/movie1.mkv".to_string(),
        MediaType::Movie,
        "Test Movie 1".to_string(),
    ).expect("Should create media");

    let media2 = Media::new(
        "/path/to/movie2.mkv".to_string(),
        MediaType::Movie,
        "Test Movie 2".to_string(),
    ).expect("Should create media");

    repo.save(&media1).await
        .expect("Should save media");
    repo.save(&media2).await
        .expect("Should save media");

    let all = repo.find_all().await
        .expect("Should find all media");

    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_media_repository_find_by_type() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let movie = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let episode = Media::new(
        "/path/to/episode.mkv".to_string(),
        MediaType::Episode,
        "Test Episode".to_string(),
    ).expect("Should create media");

    repo.save(&movie).await
        .expect("Should save media");
    repo.save(&episode).await
        .expect("Should save media");

    let movies = repo.find_by_type(MediaType::Movie).await
        .expect("Should find movies");

    assert_eq!(movies.len(), 1);
    assert_eq!(movies[0].media_type, MediaType::Movie);
}

#[tokio::test]
async fn test_media_repository_update() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let id = repo.save(&media).await
        .expect("Should save media");

    media.mark_verified();
    repo.update(&media).await
        .expect("Should update media");

    let updated = repo.find_by_id(id).await
        .expect("Should find media");

    assert!(updated.is_some());
    assert_eq!(updated.unwrap().verification_status, VerificationStatus::Verified);
}

#[tokio::test]
async fn test_media_repository_delete() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let id = repo.save(&media).await
        .expect("Should save media");

    repo.delete(id).await
        .expect("Should delete media");

    let found = repo.find_by_id(id).await
        .expect("Should execute query");

    assert!(found.is_none());
}

#[tokio::test]
async fn test_media_repository_count() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let media1 = Media::new(
        "/path/to/movie1.mkv".to_string(),
        MediaType::Movie,
        "Test Movie 1".to_string(),
    ).expect("Should create media");

    let media2 = Media::new(
        "/path/to/movie2.mkv".to_string(),
        MediaType::Movie,
        "Test Movie 2".to_string(),
    ).expect("Should create media");

    let media3 = Media::new(
        "/path/to/movie3.mkv".to_string(),
        MediaType::Movie,
        "Test Movie 3".to_string(),
    ).expect("Should create media");

    repo.save(&media1).await
        .expect("Should save media");
    repo.save(&media2).await
        .expect("Should save media");
    repo.save(&media3).await
        .expect("Should save media");

    let count = repo.count().await
        .expect("Should count media");

    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_media_repository_find_unverified() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let verified_media = Media::new(
        "/path/to/verified.mkv".to_string(),
        MediaType::Movie,
        "Verified Movie".to_string(),
    ).expect("Should create media")
        .with_confidence(ConfidenceScore::new(0.90).expect("Should create score"));

    let unverified_media = Media::new(
        "/path/to/unverified.mkv".to_string(),
        MediaType::Movie,
        "Unverified Movie".to_string(),
    ).expect("Should create media")
        .with_confidence(ConfidenceScore::new(0.50).expect("Should create score"));

    repo.save(&verified_media).await
        .expect("Should save media");
    repo.save(&unverified_media).await
        .expect("Should save media");

    let unverified = repo.find_unverified().await
        .expect("Should find unverified media");

    assert_eq!(unverified.len(), 1);
    assert_eq!(unverified[0].title, "Unverified Movie");
}

#[tokio::test]
async fn test_media_repository_find_by_confidence() {
    let pool = create_test_pool().await;
    let repo = SqliteMediaRepository::new(pool.clone());

    let high_confidence = Media::new(
        "/path/to/high.mkv".to_string(),
        MediaType::Movie,
        "High Confidence Movie".to_string(),
    ).expect("Should create media")
        .with_confidence(ConfidenceScore::new(0.90).expect("Should create score"));

    let low_confidence = Media::new(
        "/path/to/low.mkv".to_string(),
        MediaType::Movie,
        "Low Confidence Movie".to_string(),
    ).expect("Should create media")
        .with_confidence(ConfidenceScore::new(0.50).expect("Should create score"));

    repo.save(&high_confidence).await
        .expect("Should save media");
    repo.save(&low_confidence).await
        .expect("Should save media");

    let min_score = ConfidenceScore::new(0.75).expect("Should create score");
    let results = repo.find_by_confidence(min_score).await
        .expect("Should find by confidence");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "High Confidence Movie");
}
