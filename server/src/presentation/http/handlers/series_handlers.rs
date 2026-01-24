//! Series Handlers
//!
//! HTTP handlers for series operations.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use crate::application::use_cases::manage_series::ManageSeriesUseCase;
use crate::domain::repositories::MediaRepository;
use crate::presentation::http::dto::series_dto::{SeriesResponse, SeriesDetailsResponse};
use crate::shared::error::ApplicationError;

/// Get series by ID with episodes grouped by season
pub async fn get_series(
    State(use_case): State<Arc<ManageSeriesUseCase>>,
    State(media_repo): State<Arc<dyn MediaRepository>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Fetch series
    let series = match use_case.get_series(id).await {
        Ok(s) => s,
        Err(ApplicationError::Domain(crate::shared::error::DomainError::NotFound(msg))) => {
            return Err((StatusCode::NOT_FOUND, msg));
        }
        Err(e) => {
            tracing::error!("Error getting series: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()));
        }
    };

    // Fetch episodes for this series
    let episodes = match media_repo.find_by_series(id).await {
        Ok(eps) => eps,
        Err(e) => {
            tracing::error!("Error fetching episodes for series {}: {}", id, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch episodes".to_string()));
        }
    };

    let response = SeriesDetailsResponse::from_series_and_episodes(series, episodes);
    Ok(Json(response))
}

/// List all series
pub async fn list_series(
    State(use_case): State<Arc<ManageSeriesUseCase>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match use_case.list_all().await {
        Ok(series_list) => {
            let response: Vec<SeriesResponse> = series_list
                .into_iter()
                .map(SeriesResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Error listing series: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))
        }
    }
}
