//! Collection Handlers
//!
//! HTTP handlers for collection operations using repository pattern.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

use crate::domain::repositories::CollectionRepository;

/// Collection summary for list view
#[derive(Debug, Serialize)]
pub struct CollectionSummary {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub total_items: i32,
    pub available_items: i32,
    pub collection_type: String,
    pub sort_mode: String,
}

/// Collection detail with items
#[derive(Debug, Serialize)]
pub struct CollectionDetail {
    #[serde(flatten)]
    pub summary: CollectionSummary,
    pub items: Vec<CollectionItemResponse>,
}

/// Collection item response
#[derive(Debug, Serialize)]
pub struct CollectionItemResponse {
    pub id: i64,
    pub tmdb_id: i64,
    pub media_type: String,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub release_date: Option<String>,
    pub timeline_order: i32,
    pub timeline_year: Option<i32>,
    pub timeline_notes: Option<String>,
    pub is_available: bool,
    pub media_id: Option<i64>,
}

/// List all collections
pub async fn list_collections(
    State(collection_repo): State<Arc<dyn CollectionRepository>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Listing all collections");

    let collections = collection_repo
        .find_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let summaries: Vec<CollectionSummary> = collections
        .into_iter()
        .map(|c| CollectionSummary {
            id: c.id.unwrap_or(0),
            name: c.name,
            description: c.description,
            poster_url: c.poster_url,
            backdrop_url: c.backdrop_url,
            total_items: c.total_items,
            available_items: c.available_items,
            collection_type: c.collection_type,
            sort_mode: c.sort_mode,
        })
        .collect();

    Ok(Json(summaries))
}

/// Get collection by ID with items
pub async fn get_collection(
    State(collection_repo): State<Arc<dyn CollectionRepository>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Getting collection {}", id);

    // Get collection
    let collection = collection_repo
        .find_by_id(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, format!("Collection {} not found", id)))?;

    let summary = CollectionSummary {
        id: collection.id.unwrap_or(0),
        name: collection.name,
        description: collection.description,
        poster_url: collection.poster_url,
        backdrop_url: collection.backdrop_url,
        total_items: collection.total_items,
        available_items: collection.available_items,
        collection_type: collection.collection_type,
        sort_mode: collection.sort_mode,
    };

    // Get collection items
    let collection_items = collection_repo
        .find_items(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<CollectionItemResponse> = collection_items
        .into_iter()
        .map(|item| CollectionItemResponse {
            id: item.id,
            tmdb_id: item.tmdb_id,
            media_type: item.media_type,
            title: item.title,
            overview: item.overview,
            poster_url: item.poster_url,
            release_date: item.release_date,
            timeline_order: item.timeline_order,
            timeline_year: item.timeline_year,
            timeline_notes: item.timeline_notes,
            is_available: item.is_available,
            media_id: item.media_id,
        })
        .collect();

    let detail = CollectionDetail { summary, items };

    Ok(Json(detail))
}
