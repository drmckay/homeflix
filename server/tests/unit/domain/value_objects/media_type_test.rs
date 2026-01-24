//! Unit tests for MediaType value object
//!
//! Tests cover:
//! - Type creation and comparison
//! - Type checking methods
//! - String conversion

use homeflixd::domain::value_objects::MediaType;

#[test]
fn test_media_type_movie() {
    let media_type = MediaType::Movie;
    assert_eq!(media_type.as_str(), "movie");
    assert!(media_type.is_movie());
    assert!(!media_type.is_episode());
}

#[test]
fn test_media_type_episode() {
    let media_type = MediaType::Episode;
    assert_eq!(media_type.as_str(), "episode");
    assert!(!media_type.is_movie());
    assert!(media_type.is_episode());
}

#[test]
fn test_media_type_from_str_movie() {
    let media_type = MediaType::from_str("movie").expect("Should parse movie");
    assert_eq!(media_type, MediaType::Movie);
}

#[test]
fn test_media_type_from_str_episode() {
    let media_type = MediaType::from_str("episode").expect("Should parse episode");
    assert_eq!(media_type, MediaType::Episode);
}

#[test]
fn test_media_type_from_str_invalid() {
    let result = MediaType::from_str("invalid");
    assert!(result.is_err());
}

#[test]
fn test_media_type_serialization() {
    let media_type = MediaType::Movie;
    let serialized = serde_json::to_string(&media_type).expect("Should serialize");
    assert_eq!(serialized, "\"movie\"");
}

#[test]
fn test_media_type_deserialization() {
    let json = "\"episode\"";
    let media_type: MediaType = serde_json::from_str(json).expect("Should deserialize");
    assert_eq!(media_type, MediaType::Episode);
}

#[test]
fn test_media_type_clone() {
    let media_type1 = MediaType::Movie;
    let media_type2 = media_type1.clone();
    assert_eq!(media_type1, media_type2);
}

#[test]
fn test_media_type_debug() {
    let media_type = MediaType::Movie;
    let debug_str = format!("{:?}", media_type);
    assert!(debug_str.contains("Movie"));
}

#[test]
fn test_media_type_partial_eq() {
    let media_type1 = MediaType::Movie;
    let media_type2 = MediaType::Movie;
    assert_eq!(media_type1, media_type2);
}

#[test]
fn test_media_type_partial_ne() {
    let media_type1 = MediaType::Movie;
    let media_type2 = MediaType::Episode;
    assert_ne!(media_type1, media_type2);
}
