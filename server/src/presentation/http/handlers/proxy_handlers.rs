//! Proxy Handlers
//!
//! HTTP handlers for proxying external resources (CORS bypass).

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::infrastructure::cache::ImageCache;
use tracing::{debug, warn};

/// Query parameters for image proxy
#[derive(Debug, Deserialize)]
pub struct ImageProxyQuery {
    /// URL to proxy (must be TMDB image URL)
    pub url: String,
}

/// Proxy TMDB images to bypass CORS restrictions
///
/// Only allows proxying from image.tmdb.org for security.
/// Images are cached in the filesystem cache for faster subsequent requests.
pub async fn proxy_image(
    State(image_cache): State<Arc<ImageCache>>,
    Query(query): Query<ImageProxyQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Security: only allow TMDB image URLs
    if !query.url.starts_with("https://image.tmdb.org/") {
        return Err((StatusCode::BAD_REQUEST, "Invalid image URL - only TMDB images allowed".to_string()));
    }

    // Try to get from cache first
    match image_cache.get_cached_image(&query.url) {
        Ok(Some(cached_bytes)) => {
            debug!("Serving image from cache: {}", query.url);
            
            // Determine content type from URL extension
            let content_type = get_content_type_from_url(&query.url);
            
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
            headers.insert(
                header::CACHE_CONTROL,
                "public, max-age=31536000".parse().unwrap(), // 1 year cache
            );

            return Ok((headers, cached_bytes));
        }
        Ok(None) => {
            // Not in cache, will download below
            debug!("Image not in cache, downloading: {}", query.url);
        }
        Err(e) => {
            // Cache read error - log but continue to download
            warn!("Cache read error for {}: {}, falling back to download", query.url, e);
        }
    }

    // Download from TMDB
    let client = reqwest::Client::new();
    let res = client
        .get(&query.url)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = res.status();
    if !status.is_success() {
        return Err((status, "Failed to fetch image from TMDB".to_string()));
    }

    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| get_content_type_from_url(&query.url))
        .to_string();

    let bytes = res
        .bytes()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Save to cache (non-blocking - log errors but don't fail the request)
    let bytes_vec = bytes.to_vec();
    if let Err(e) = image_cache.save_cached_image(&query.url, &bytes_vec) {
        warn!("Failed to save image to cache {}: {}", query.url, e);
    }

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=31536000".parse().unwrap(), // 1 year cache
    );

    Ok((headers, bytes_vec))
}

/// Determines content type from URL extension
fn get_content_type_from_url(url: &str) -> &str {
    if url.ends_with(".jpg") || url.ends_with(".jpeg") {
        "image/jpeg"
    } else if url.ends_with(".png") {
        "image/png"
    } else if url.ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg" // Default
    }
}
