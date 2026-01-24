//! Unit tests for IdentificationService
//!
//! Tests cover:
//! - Media type identification
//! - Season/episode extraction
//! - Title cleaning
//! - Content identification

use homeflixd::domain::services::IdentificationService;
use homeflixd::domain::value_objects::MediaType;
use homeflixd::domain::services::DefaultIdentificationService;

#[tokio::test]
async fn test_identify_media_type_movie() {
    let service = DefaultIdentificationService;
    let result = service.identify_media_type("/path/to/Movie.2024.mkv").await
        .expect("Should identify media type");

    assert_eq!(result, MediaType::Movie);
}

#[tokio::test]
async fn test_identify_media_type_episode_with_se() {
    let service = DefaultIdentificationService;
    let result = service.identify_media_type("/path/to/S01E05.mkv").await
        .expect("Should identify media type");

    assert_eq!(result, MediaType::Episode);
}

#[tokio::test]
async fn test_identify_media_type_episode_with_season_folder() {
    let service = DefaultIdentificationService;
    let result = service.identify_media_type("/Season 1/Episode 1.mkv").await
        .expect("Should identify media type");

    assert_eq!(result, MediaType::Episode);
}

#[tokio::test]
async fn test_extract_season_episode_se_format() {
    let service = DefaultIdentificationService;
    let result = service.extract_season_episode("Show.S02E05.1080p.mkv").await
        .expect("Should extract season/episode");

    assert_eq!(result, Some((2, 5)));
}

#[tokio::test]
async fn test_extract_season_episode_space_format() {
    let service = DefaultIdentificationService;
    let result = service.extract_season_episode("Show S03 E10.mkv").await
        .expect("Should extract season/episode");

    assert_eq!(result, Some((3, 10)));
}

#[tokio::test]
async fn test_extract_season_episode_no_match() {
    let service = DefaultIdentificationService;
    let result = service.extract_season_episode("Movie.2024.mkv").await
        .expect("Should return None");

    assert_eq!(result, None);
}

#[tokio::test]
async fn test_extract_season_episode_invalid_format() {
    let service = DefaultIdentificationService;
    let result = service.extract_season_episode("Show.S99E99.mkv").await
        .expect("Should return None for invalid season/episode");

    assert_eq!(result, None);
}

#[tokio::test]
async fn test_clean_title_removes_brackets() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie.[Group].2024.1080p.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("["));
    assert!(!result.contains("]"));
    assert!(!result.contains("Group"));
}

#[tokio::test]
async fn test_clean_title_removes_trailing_group() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie-REPACK.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("-REPACK"));
}

#[tokio::test]
async fn test_clean_title_replaces_dots() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie.Name.2024.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("."));
}

#[tokio::test]
async fn test_clean_title_replaces_underscores() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie_Name_2024.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("_"));
}

#[tokio::test]
async fn test_clean_title_removes_quality_tags() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie.Name.1080p.x264.BluRay.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("1080p"));
    assert!(!result.contains("x264"));
    assert!(!result.contains("BluRay"));
}

#[tokio::test]
async fn test_clean_title_removes_year() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie.Name.2024.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("2024"));
}

#[tokio::test]
async fn test_clean_title_removes_articles() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("The Movie Name.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("the "));
}

#[tokio::test]
async fn test_clean_title_title_case() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("MOVIE.NAME.2024.mkv").await
        .expect("Should clean title");

    assert!(result.chars().all(|c| c.is_uppercase() || c.is_whitespace()));
}

#[tokio::test]
async fn test_clean_title_removes_multiple_spaces() {
    let service = DefaultIdentificationService;
    let result = service.clean_title("Movie  Name  .2024.mkv").await
        .expect("Should clean title");

    assert!(!result.contains("  "));
}

#[tokio::test]
async fn test_identify_content_movie() {
    let service = DefaultIdentificationService;
    let result = service.identify_content("/path/to/Movie.2024.mkv").await
        .expect("Should identify content");

    assert_eq!(result.media_type, MediaType::Movie);
    assert!(result.title.contains("Movie"));
    assert_eq!(result.season, None);
    assert_eq!(result.episode, None);
}

#[tokio::test]
async fn test_identify_content_episode() {
    let service = DefaultIdentificationService;
    let result = service.identify_content("/path/to/Show.S02E05.mkv").await
        .expect("Should identify content");

    assert_eq!(result.media_type, MediaType::Episode);
    assert!(result.title.contains("Show"));
    assert_eq!(result.season, Some(2));
    assert_eq!(result.episode, Some(5));
}

#[tokio::test]
async fn test_identify_content_with_year() {
    let service = DefaultIdentificationService;
    let result = service.identify_content("/path/to/Movie.2024.mkv").await
        .expect("Should identify content");

    // Year should be removed from title
    assert!(!result.title.contains("2024"));
}

#[tokio::test]
async fn test_identify_content_complex_filename() {
    let service = DefaultIdentificationService;
    let result = service.identify_content(
        "/path/to/Show.Name.S03E10.1080p.BluRay.x264-Group.mkv"
    ).await
        .expect("Should identify content");

    assert_eq!(result.media_type, MediaType::Episode);
    assert_eq!(result.season, Some(3));
    assert_eq!(result.episode, Some(10));
    assert!(!result.title.contains("1080p"));
    assert!(!result.title.contains("BluRay"));
    assert!(!result.title.contains("Group"));
}
