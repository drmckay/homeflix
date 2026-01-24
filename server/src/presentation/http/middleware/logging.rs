//! Logging Middleware
//!
//! Logs HTTP requests and responses.

use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};
use tracing::{info, info_span};
use std::time::Instant;

/// Logging middleware
pub async fn logging_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let span = info_span!("request", %method, %uri);
    let _enter = span.enter();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration = ?duration,
        "Request processed"
    );

    response
}
