//! Series DTOs
//!
//! Data Transfer Objects for Series-related operations

use serde::{Deserialize, Serialize};
use crate::domain::entities::{Media, Series};
use crate::presentation::http::dto::media_dto::LibraryMediaResponse;

/// Series response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesResponse {
    /// Series ID
    pub id: i64,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Title
    pub title: String,
    /// Overview
    pub overview: Option<String>,
    /// Poster URL
    pub poster_url: Option<String>,
    /// Backdrop URL
    pub backdrop_url: Option<String>,
    /// Confidence score
    pub confidence_score: f32,
    /// Verification status
    pub verification_status: String,
    /// First air date
    pub first_air_date: Option<String>,
    /// Last air date
    pub last_air_date: Option<String>,
    /// Status
    pub status: Option<String>,
    /// Total seasons
    pub total_seasons: Option<i32>,
    /// Total episodes
    pub total_episodes: Option<i32>,
    /// Rating
    pub rating: Option<f32>,
    /// Created at timestamp (ISO 8601)
    pub created_at: String,
    /// Updated at timestamp (ISO 8601)
    pub updated_at: String,
}

impl From<Series> for SeriesResponse {
    fn from(series: Series) -> Self {
        Self {
            id: series.id.unwrap_or(0),
            tmdb_id: series.tmdb_id,
            title: series.title,
            overview: series.overview,
            poster_url: series.poster_url,
            backdrop_url: series.backdrop_url,
            confidence_score: series.confidence_score.value(),
            verification_status: series.verification_status.as_str().to_string(),
            first_air_date: series.first_air_date,
            last_air_date: series.last_air_date,
            status: series.status,
            total_seasons: series.total_seasons,
            total_episodes: series.total_episodes,
            rating: series.rating,
            created_at: series.created_at.to_rfc3339(),
            updated_at: series.updated_at.to_rfc3339(),
        }
    }
}

/// Season group for series details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonGroup {
    pub season_number: i32,
    pub episodes: Vec<LibraryMediaResponse>,
}

/// Series info for series details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesInfo {
    pub id: i64,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub tmdb_id: Option<i64>,
}

impl From<&Series> for SeriesInfo {
    fn from(series: &Series) -> Self {
        Self {
            id: series.id.unwrap_or(0),
            title: series.title.clone(),
            overview: series.overview.clone(),
            poster_url: series.poster_url.clone(),
            tmdb_id: series.tmdb_id,
        }
    }
}

/// Series details response with episodes grouped by season
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesDetailsResponse {
    pub series: SeriesInfo,
    pub seasons: Vec<SeasonGroup>,
}

impl SeriesDetailsResponse {
    /// Create SeriesDetailsResponse from series and episodes
    pub fn from_series_and_episodes(series: Series, episodes: Vec<Media>) -> Self {
        use std::collections::BTreeMap;

        let series_info = SeriesInfo::from(&series);

        // Group episodes by season
        let mut season_map: BTreeMap<i32, Vec<LibraryMediaResponse>> = BTreeMap::new();
        for episode in episodes {
            let season_num = episode.season.unwrap_or(1);
            season_map
                .entry(season_num)
                .or_default()
                .push(LibraryMediaResponse::from_media(episode));
        }

        // Convert to sorted season groups
        let seasons: Vec<SeasonGroup> = season_map
            .into_iter()
            .map(|(season_number, mut episodes)| {
                // Sort episodes by episode number
                episodes.sort_by_key(|e| e.episode_number.unwrap_or(0));
                SeasonGroup { season_number, episodes }
            })
            .collect();

        Self { series: series_info, seasons }
    }
}
