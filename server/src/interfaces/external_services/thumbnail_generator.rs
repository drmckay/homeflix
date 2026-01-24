// Thumbnail Generator Interface
//
// This module defines interface for generating thumbnails from video files.
// Typically implemented using FFmpeg.
//
// This interface enables:
// - Testing with mock implementations
// - Swapping FFmpeg implementations
// - Adding caching layers

use async_trait::async_trait;
use crate::shared::error::ThumbnailError;

/// Thumbnail generation options
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThumbnailOptions {
    /// Thumbnail width in pixels
    pub width: Option<u32>,
    /// Thumbnail height in pixels
    pub height: Option<u32>,
    /// Timestamp in seconds to capture thumbnail from
    pub timestamp: Option<f64>,
    /// Image quality (0-100)
    pub quality: Option<u8>,
    /// Output format ("jpg", "png", "webp")
    pub format: Option<String>,
    /// Whether to preserve aspect ratio
    pub preserve_aspect_ratio: bool,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            width: Some(320),
            height: None,
            timestamp: Some(0.0), // Start of video
            quality: Some(85),
            format: Some("jpg".to_string()),
            preserve_aspect_ratio: true,
        }
    }
}

/// Thumbnail generation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThumbnailResult {
    /// Thumbnail image data (bytes)
    pub data: Vec<u8>,
    /// Image format ("jpg", "png", "webp")
    pub format: String,
    /// Actual width in pixels
    pub width: u32,
    /// Actual height in pixels
    pub height: u32,
    /// Timestamp used for capture (seconds)
    pub timestamp: f64,
    /// MIME type
    pub mime_type: String,
}

/// Thumbnail generator interface
/// 
/// Provides methods for generating thumbnails from video files.
#[async_trait]
pub trait ThumbnailGenerator: Send + Sync {
    /// Generate a thumbnail from a video file
    /// 
    /// # Arguments
    /// * `file_path` - Path to video file
    /// * `options` - Thumbnail generation options
    /// 
    /// # Returns
    /// * `Result<ThumbnailResult, ThumbnailError>` - Generated thumbnail
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - File is not a valid video file
    /// - FFmpeg execution fails
    /// - Timestamp is beyond video duration
    async fn generate(
        &self,
        file_path: &str,
        options: ThumbnailOptions,
    ) -> Result<ThumbnailResult, ThumbnailError>;
    
    /// Generate a thumbnail at a specific timestamp
    /// 
    /// # Arguments
    /// * `file_path` - Path to video file
    /// * `timestamp` - Timestamp in seconds
    /// * `width` - Optional width in pixels
    /// * `height` - Optional height in pixels
    /// 
    /// # Returns
    /// * `Result<ThumbnailResult, ThumbnailError>` - Generated thumbnail
    async fn generate_at(
        &self,
        file_path: &str,
        timestamp: f64,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<ThumbnailResult, ThumbnailError> {
        let options = ThumbnailOptions {
            timestamp: Some(timestamp),
            width,
            height,
            ..Default::default()
        };
        self.generate(file_path, options).await
    }
    
    /// Generate multiple thumbnails at different timestamps
    /// 
    /// # Arguments
    /// * `file_path` - Path to video file
    /// * `timestamps` - List of timestamps in seconds
    /// * `width` - Optional width in pixels
    /// * `height` - Optional height in pixels
    /// 
    /// # Returns
    /// * `Result<Vec<ThumbnailResult>, ThumbnailError>` - List of generated thumbnails
    async fn generate_multiple(
        &self,
        file_path: &str,
        timestamps: Vec<f64>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<Vec<ThumbnailResult>, ThumbnailError> {
        let mut results = Vec::with_capacity(timestamps.len());
        for timestamp in timestamps {
            let thumbnail = self.generate_at(file_path, timestamp, width, height).await?;
            results.push(thumbnail);
        }
        Ok(results)
    }
    
    /// Generate a poster (large thumbnail from middle of video)
    /// 
    /// # Arguments
    /// * `file_path` - Path to video file
    /// * `width` - Optional width in pixels
    /// * `height` - Optional height in pixels
    /// 
    /// # Returns
    /// * `Result<ThumbnailResult, ThumbnailError>` - Generated poster
    async fn generate_poster(
        &self,
        file_path: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<ThumbnailResult, ThumbnailError> {
        // First get duration to find middle
        // This requires VideoAnalyzer, so we'll use a default timestamp
        // Implementations can override for better behavior
        self.generate_at(file_path, 0.0, width, height).await
    }
}
