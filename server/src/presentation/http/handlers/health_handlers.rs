//! Health Check Handlers
//!
//! HTTP handlers for health check endpoints.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

/// Health check endpoint
///
/// Returns a simple JSON response indicating the server is running.
/// Used by Docker HEALTHCHECK and monitoring tools.
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(json!({
        "status": "ok",
        "service": "homeflix-server"
    })))
}
