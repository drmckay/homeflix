//! Media DTOs
//!
//! Data Transfer Objects for Media-related operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::domain::entities::Media;

/// Media response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaResponse {
    /// Media ID
    pub id: i64,
    /// File path
    pub file_path: String,
    /// Media type
    pub media_type: String,
    /// Title
    pub title: String,
    /// Release year
    pub year: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Season number
    pub season: Option<i32>,
    /// Episode number
    pub episode: Option<i32>,
    /// Duration in seconds
    pub duration_seconds: Option<i32>,
    /// Resolution
    pub resolution: Option<String>,
    /// Confidence score
    pub confidence_score: f32,
    /// Verification status
    pub verification_status: String,
    /// Overview
    pub overview: Option<String>,
    /// Poster URL
    pub poster_url: Option<String>,
    /// Backdrop URL
    pub backdrop_url: Option<String>,
    /// Rating
    pub rating: Option<f32>,
    /// Is watched
    pub is_watched: bool,
    /// Current position
    pub current_position: i64,
    /// Created at timestamp (ISO 8601)
    pub created_at: String,
    /// Updated at timestamp (ISO 8601)
    pub updated_at: String,
}

impl From<Media> for MediaResponse {
    fn from(media: Media) -> Self {
        Self {
            id: media.id.unwrap_or(0),
            file_path: media.file_path,
            media_type: media.media_type.as_str().to_string(),
            title: media.title,
            year: media.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
            tmdb_id: media.tmdb_id,
            season: media.season,
            episode: media.episode,
            duration_seconds: media.duration_seconds,
            resolution: media.resolution,
            confidence_score: media.confidence_score.value(),
            verification_status: media.verification_status.as_str().to_string(),
            overview: media.overview,
            poster_url: media.poster_url,
            backdrop_url: media.backdrop_url,
            rating: media.rating,
            is_watched: media.is_watched,
            current_position: media.current_position,
            created_at: media.created_at.to_rfc3339(),
            updated_at: media.updated_at.to_rfc3339(),
        }
    }
}

/// Library media response DTO for grouped library views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMediaResponse {
    pub id: i64,
    pub file_path: String,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub trailer_url: Option<String>,
    pub duration: Option<i32>,
    pub release_date: Option<String>,
    pub resolution: Option<String>,
    pub genres: Option<String>,
    pub media_type: String,
    pub series_id: Option<i64>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub created_at: String,
    pub tmdb_id: Option<i64>,
    pub original_title: Option<String>,
    pub rating: Option<f32>,
    pub content_rating: Option<String>,
    pub content_warnings: Option<String>,
    pub current_position: i64,
    pub is_watched: bool,
}

impl LibraryMediaResponse {
    pub fn from_media(media: Media) -> Self {
        Self {
            id: media.id.unwrap_or(0),
            file_path: media.file_path,
            title: media.title,
            overview: media.overview,
            poster_url: media.poster_url,
            backdrop_url: media.backdrop_url,
            trailer_url: media.trailer_url,
            duration: media.duration_seconds,
            release_date: media.release_date,
            resolution: media.resolution,
            genres: media.genres,
            media_type: media.media_type.as_str().to_string(),
            series_id: media.series_id,
            season_number: media.season,
            episode_number: media.episode,
            created_at: media.created_at.to_rfc3339(),
            tmdb_id: media.tmdb_id,
            original_title: media.original_title,
            rating: media.rating,
            content_rating: media.content_rating,
            content_warnings: media.content_warnings,
            current_position: media.current_position,
            is_watched: media.is_watched,
        }
    }
}

/// Grouped library response DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupedLibraryResponse {
    pub recent: Vec<LibraryMediaResponse>,
    pub continue_watching: Vec<LibraryMediaResponse>,
    pub categories: HashMap<String, Vec<LibraryMediaResponse>>,
}

/// Scan request DTO
#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// Path to scan
    pub path: String,
}

/// Scan response DTO
#[derive(Debug, Serialize)]
pub struct ScanResponse {
    /// Number of files processed
    pub processed_count: usize,
    /// Number of items identified
    pub identified_count: usize,
    /// Number of items failed
    pub failed_count: usize,
    /// Duration in seconds
    pub duration_secs: u64,
}

/// Manual identify request DTO
#[derive(Debug, Deserialize)]
pub struct ManualIdentifyRequest {
    /// TMDB ID to assign
    pub tmdb_id: i64,
}

/// Manual identify response DTO
#[derive(Debug, Serialize)]
pub struct ManualIdentifyResponse {
    /// Updated media item
    pub media: MediaResponse,
    /// Message
    pub message: String,
}
