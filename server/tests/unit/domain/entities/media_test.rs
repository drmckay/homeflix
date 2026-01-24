//! Unit tests for Media entity
//!
//! Tests cover:
//! - Entity creation and validation
//! - Business logic methods
//! - Builder pattern methods
//! - Edge cases and error handling

use homeflixd::domain::entities::Media;
use homeflixd::domain::value_objects::{MediaType, ConfidenceScore, VerificationStatus};
use homeflixd::shared::error::DomainError;

#[test]
fn test_media_creation_success() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media successfully");

    assert_eq!(media.file_path, "/path/to/movie.mkv");
    assert_eq!(media.title, "Test Movie");
    assert_eq!(media.media_type, MediaType::Movie);
    assert_eq!(media.id, None);
    assert_eq!(media.confidence_score, ConfidenceScore::default());
    assert_eq!(media.verification_status, VerificationStatus::Unverified);
}

#[test]
fn test_media_creation_empty_file_path() {
    let result = Media::new(
        "".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    );

    assert!(matches!(result, Err(DomainError::InvalidInput(_))));
}

#[test]
fn test_media_creation_empty_title() {
    let result = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "".to_string(),
    );

    assert!(matches!(result, Err(DomainError::InvalidInput(_))));
}

#[test]
fn test_is_movie() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    assert!(media.is_movie());
    assert!(!media.is_episode());
}

#[test]
fn test_is_episode() {
    let media = Media::new(
        "/path/to/episode.mkv".to_string(),
        MediaType::Episode,
        "Test Episode".to_string(),
    ).expect("Should create media");

    assert!(!media.is_movie());
    assert!(media.is_episode());
}

#[test]
fn test_mark_verified() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    assert_eq!(media.verification_status, VerificationStatus::Unverified);

    media.mark_verified();

    assert_eq!(media.verification_status, VerificationStatus::Verified);
    assert!(media.updated_at > media.created_at);
}

#[test]
fn test_mark_failed() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    media.mark_failed("Test failure reason".to_string());

    assert_eq!(media.verification_status, VerificationStatus::Failed);
    assert_eq!(media.error_notes, Some("Test failure reason".to_string()));
    assert!(media.updated_at > media.created_at);
}

#[test]
fn test_update_confidence_high() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let high_score = ConfidenceScore::new(0.90).expect("Should create score");
    media.update_confidence(high_score);

    assert_eq!(media.confidence_score, high_score);
    assert_eq!(media.verification_status, VerificationStatus::Verified);
}

#[test]
fn test_update_confidence_medium() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let medium_score = ConfidenceScore::new(0.75).expect("Should create score");
    media.update_confidence(medium_score);

    assert_eq!(media.confidence_score, medium_score);
    assert_eq!(media.verification_status, VerificationStatus::Unverified);
}

#[test]
fn test_update_confidence_low() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let low_score = ConfidenceScore::new(0.50).expect("Should create score");
    media.update_confidence(low_score);

    assert_eq!(media.confidence_score, low_score);
    assert_eq!(media.verification_status, VerificationStatus::Failed);
}

#[test]
fn test_with_series_id() {
    let media = Media::new(
        "/path/to/episode.mkv".to_string(),
        MediaType::Episode,
        "Test Episode".to_string(),
    ).expect("Should create media")
        .with_series_id(Some(123));

    assert_eq!(media.series_id, Some(123));
}

#[test]
fn test_with_season() {
    let media = Media::new(
        "/path/to/episode.mkv".to_string(),
        MediaType::Episode,
        "Test Episode".to_string(),
    ).expect("Should create media")
        .with_season(Some(1));

    assert_eq!(media.season, Some(1));
}

#[test]
fn test_with_episode() {
    let media = Media::new(
        "/path/to/episode.mkv".to_string(),
        MediaType::Episode,
        "Test Episode".to_string(),
    ).expect("Should create media")
        .with_episode(Some(5));

    assert_eq!(media.episode, Some(5));
}

#[test]
fn test_with_tmdb_id() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_tmdb_id(Some(45678));

    assert_eq!(media.tmdb_id, Some(45678));
}

#[test]
fn test_with_overview() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_overview(Some("Test overview".to_string()));

    assert_eq!(media.overview, Some("Test overview".to_string()));
}

#[test]
fn test_with_poster_url() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_poster_url(Some("http://example.com/poster.jpg".to_string()));

    assert_eq!(media.poster_url, Some("http://example.com/poster.jpg".to_string()));
}

#[test]
fn test_with_backdrop_url() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_backdrop_url(Some("http://example.com/backdrop.jpg".to_string()));

    assert_eq!(media.backdrop_url, Some("http://example.com/backdrop.jpg".to_string()));
}

#[test]
fn test_with_duration() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_duration(Some(7200));

    assert_eq!(media.duration_seconds, Some(7200));
}

#[test]
fn test_with_resolution() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_resolution(Some("1080p".to_string()));

    assert_eq!(media.resolution, Some("1080p".to_string()));
}

#[test]
fn test_with_release_date() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_release_date(Some("2024-01-01".to_string()));

    assert_eq!(media.release_date, Some("2024-01-01".to_string()));
}

#[test]
fn test_with_genres() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_genres(Some("Action, Drama".to_string()));

    assert_eq!(media.genres, Some("Action, Drama".to_string()));
}

#[test]
fn test_with_rating() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_rating(Some(8.5));

    assert_eq!(media.rating, Some(8.5));
}

#[test]
fn test_with_content_rating() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_content_rating(Some("PG-13".to_string()));

    assert_eq!(media.content_rating, Some("PG-13".to_string()));
}

#[test]
fn test_with_content_warnings() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_content_warnings(Some("violence, language".to_string()));

    assert_eq!(media.content_warnings, Some("violence, language".to_string()));
}

#[test]
fn test_update_progress() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    assert_eq!(media.current_position, 0);
    assert!(!media.is_watched);

    media.update_progress(3600, false);

    assert_eq!(media.current_position, 3600);
    assert!(!media.is_watched);
    assert!(media.updated_at > media.created_at);
}

#[test]
fn test_update_progress_watched() {
    let mut media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    media.update_progress(7200, true);

    assert_eq!(media.current_position, 7200);
    assert!(media.is_watched);
}

#[test]
fn test_display_title_with_original() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_original_title(Some("Original Title".to_string()));

    assert_eq!(media.display_title(), "Original Title");
}

#[test]
fn test_display_title_without_original() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    assert_eq!(media.display_title(), "Test Movie");
}

#[test]
fn test_builder_chain() {
    let media = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
        .with_tmdb_id(Some(12345))
        .with_overview(Some("Test overview".to_string()))
        .with_poster_url(Some("http://example.com/poster.jpg".to_string()))
        .with_duration(Some(7200))
        .with_rating(Some(8.5));

    assert_eq!(media.tmdb_id, Some(12345));
    assert_eq!(media.overview, Some("Test overview".to_string()));
    assert_eq!(media.poster_url, Some("http://example.com/poster.jpg".to_string()));
    assert_eq!(media.duration_seconds, Some(7200));
    assert_eq!(media.rating, Some(8.5));
}

#[test]
fn test_equality() {
    let media1 = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let media2 = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    assert_eq!(media1, media2);
}

#[test]
fn test_inequality_different_title() {
    let media1 = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media");

    let media2 = Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Different Movie".to_string(),
    ).expect("Should create media");

    assert_ne!(media1, media2);
}
