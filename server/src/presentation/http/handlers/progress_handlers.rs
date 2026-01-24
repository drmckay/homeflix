//! Progress Handlers
//!
//! HTTP handlers for watch progress operations using repository pattern.

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::domain::repositories::MediaRepository;
use crate::domain::events::{
    ProgressUpdatedEvent,
    MediaWatchedEvent,
    MediaUnwatchedEvent,
};
use crate::infrastructure::messaging::InMemoryEventBus;
use crate::interfaces::messaging::EventBus;

/// Helper function to publish events through Arc<InMemoryEventBus>
async fn publish_event<T: crate::interfaces::messaging::DomainEvent>(
    bus: &Arc<InMemoryEventBus>,
    event: T,
) -> Result<(), crate::shared::error::MessagingError> {
    // Arc<InMemoryEventBus> derefs to InMemoryEventBus, which implements EventBus
    // Use the deref to get &InMemoryEventBus and call publish
    use std::ops::Deref;
    bus.deref().publish(event).await
}

/// Request to update watch progress
#[derive(Debug, Deserialize)]
pub struct UpdateProgressRequest {
    /// Current position in seconds
    pub current_position_seconds: i64,
    /// Whether the media is watched
    #[serde(default)]
    pub is_watched: Option<bool>,
}

/// Response for progress operations
#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    /// Media ID
    pub media_id: i64,
    /// Current position in seconds
    pub current_position_seconds: i64,
    /// Whether the media is watched
    pub is_watched: bool,
    /// Last updated timestamp
    pub last_updated: String,
}

/// Update watch progress for a media item
pub async fn update_progress(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(media_id): Path<i64>,
    Json(request): Json<UpdateProgressRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!("Updating progress for media {}: {}s", media_id, request.current_position_seconds);

    // Check if media exists
    let media = media_repo
        .find_by_id(media_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if media.is_none() {
        return Err((StatusCode::NOT_FOUND, format!("Media {} not found", media_id)));
    }

    // Update progress in media table
    let watched = request.is_watched.unwrap_or(false);
    media_repo
        .update_progress(media_id, request.current_position_seconds, watched)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Publish progress updated event
    if let Some(bus) = &event_bus {
        let event = ProgressUpdatedEvent::new(
            media_id,
            request.current_position_seconds,
            watched,
        );
        if let Err(e) = publish_event(bus, event).await {
            tracing::warn!("Failed to publish progress updated event: {}", e);
        }
    }

    // Fetch updated progress
    let progress = get_progress_from_repo(&media_repo, media_id).await?;

    Ok(Json(progress))
}

/// Get watch progress for a media item
pub async fn get_progress(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    Path(media_id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let progress = get_progress_from_repo(&media_repo, media_id).await?;
    Ok(Json(progress))
}

/// Internal helper to get progress from repository
async fn get_progress_from_repo(
    media_repo: &Arc<dyn MediaRepository>,
    media_id: i64,
) -> Result<ProgressResponse, (StatusCode, String)> {
    let result = media_repo
        .get_progress(media_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match result {
        Some((position, is_watched, last_updated)) => Ok(ProgressResponse {
            media_id,
            current_position_seconds: position,
            is_watched,
            last_updated,
        }),
        None => Ok(ProgressResponse {
            media_id,
            current_position_seconds: 0,
            is_watched: false,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }),
    }
}

/// Mark media as watched
pub async fn mark_watched(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(media_id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    media_repo
        .mark_watched(media_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Publish media watched event
    if let Some(bus) = &event_bus {
        let event = MediaWatchedEvent::new(media_id);
        if let Err(e) = publish_event(bus, event).await {
            tracing::warn!("Failed to publish media watched event: {}", e);
        }
    }

    Ok(StatusCode::OK)
}

/// Mark media as unwatched
pub async fn mark_unwatched(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(media_id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    media_repo
        .mark_unwatched(media_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Publish media unwatched event
    if let Some(bus) = &event_bus {
        let event = MediaUnwatchedEvent::new(media_id);
        if let Err(e) = publish_event(bus, event).await {
            tracing::warn!("Failed to publish media unwatched event: {}", e);
        }
    }

    Ok(StatusCode::OK)
}
