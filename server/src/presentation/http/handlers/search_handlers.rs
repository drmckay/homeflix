//! Search Handlers
//!
//! HTTP handlers for search operations using repository pattern.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::domain::repositories::{MediaRepository, SeriesRepository};

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,
    /// Optional media type filter ("movie" or "tv")
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    /// Maximum results (default: 20)
    pub limit: Option<i32>,
}

/// Search result item
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub media_type: String,
    pub year: Option<String>,
    pub poster_url: Option<String>,
    pub overview: Option<String>,
    pub tmdb_id: Option<i64>,
}

/// Search response
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub query: String,
}

/// Search media by title
pub async fn search_media(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Searching for: {}", query.q);

    let limit = query.limit.unwrap_or(20).min(100) as usize;
    let media_type = query.media_type.as_deref();

    let media_list = media_repo
        .search(&query.q, media_type, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let results: Vec<SearchResult> = media_list
        .into_iter()
        .map(|m| {
            let year = m.release_date.as_ref().and_then(|d| d.get(..4).map(String::from));
            SearchResult {
                id: m.id.unwrap_or(0),
                title: m.title,
                media_type: m.media_type.as_str().to_string(),
                year,
                poster_url: m.poster_url,
                overview: m.overview,
                tmdb_id: m.tmdb_id,
            }
        })
        .collect();

    let total = results.len();
    let response = SearchResponse {
        results,
        total,
        query: query.q,
    };

    Ok(Json(response))
}

/// Search series by title
pub async fn search_series(
    State(series_repo): State<Arc<dyn SeriesRepository>>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Searching series for: {}", query.q);

    let limit = query.limit.unwrap_or(20).min(100) as usize;

    let series_list = series_repo
        .search(&query.q, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let results: Vec<SearchResult> = series_list
        .into_iter()
        .map(|s| {
            SearchResult {
                id: s.id.unwrap_or(0),
                title: s.title,
                media_type: "tv".to_string(),
                year: None,
                poster_url: s.poster_url,
                overview: s.overview,
                tmdb_id: s.tmdb_id,
            }
        })
        .collect();

    let total = results.len();
    let response = SearchResponse {
        results,
        total,
        query: query.q,
    };

    Ok(Json(response))
}
