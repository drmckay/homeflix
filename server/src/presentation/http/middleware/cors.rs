//! CORS Middleware
//!
//! Configures Cross-Origin Resource Sharing.

use tower_http::cors::{AllowOrigin, CorsLayer, Any};
use axum::http::{header, Method};
use std::time::Duration;

/// Creates a predefined CORS layer
pub fn cors_layer() -> CorsLayer {
    let allowed_origins = [
        "http://localhost:5174",
        "http://localhost:5173",
        "http://127.0.0.1:5174",
        "http://127.0.0.1:5173",
        "http://homeflix.home",
        "https://homeflix.home",
    ];

    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |_, _| {
            true // Allow all origins for Chromecast support
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::RANGE,
            "x-test-chromecast".parse().unwrap(),
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}
