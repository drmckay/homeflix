//! FFmpeg Adapter Implementation
//!
//! Provides FFmpeg-based implementation of ThumbnailGenerator interface

use async_trait::async_trait;
use tokio::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use crate::interfaces::external_services::{
    ThumbnailGenerator, ThumbnailOptions, ThumbnailResult,
};
use crate::shared::error::ThumbnailError;

/// FFmpeg adapter for thumbnail generation
pub struct FFmpegAdapter {
    timeout: Duration,
}

impl FFmpegAdapter {
    /// Creates a new FFmpeg adapter
    ///
    /// # Arguments
    /// * `timeout` - Timeout for FFmpeg execution
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Executes FFmpeg command and returns output
    async fn execute_ffmpeg(&self, args: &[&str]) -> Result<Vec<u8>, ThumbnailError> {
        let output = timeout(self.timeout, async {
            let output = Command::new("ffmpeg")
                .args(args)
                .output()
                .await;

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = output.stdout;
                        Ok(stdout.to_vec())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        Err(ThumbnailError::ExecutionFailed(stderr))
                    }
                }
                Err(e) => Err(ThumbnailError::Io(e)),
            }
        }).await;

        match output {
            Ok(result) => result,
            Err(_) => Err(ThumbnailError::Timeout("FFmpeg execution timed out".into())),
        }
    }

    /// Builds FFmpeg arguments for thumbnail generation
    fn build_thumbnail_args(
        file_path: &str,
        options: &ThumbnailOptions,
        output_format: &str,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // Input file
        args.push("-i".to_string());
        args.push(file_path.to_string());

        // Timestamp
        if let Some(timestamp) = options.timestamp {
            args.push("-ss".to_string());
            args.push(format!("{:.3}", timestamp));
        }

        // Duration for single frame (1 frame)
        args.push("-vframes".to_string());
        args.push("1".to_string());

        // Dimensions
        if let Some(width) = options.width {
            args.push("-vf".to_string());
            if let Some(height) = options.height {
                args.push(format!("scale={}:{}", width, height));
            } else {
                args.push(format!("scale={}:-1", width)); // -1 for auto height (preserve aspect ratio)
            }
        } else if let Some(height) = options.height {
            args.push("-vf".to_string());
            args.push(format!("scale=-1:{}", height)); // -1 for auto width (preserve aspect ratio)
        }

        // Quality
        if let Some(quality) = options.quality {
            args.push("-q:v".to_string());
            args.push(quality.to_string());
        }

        // Output format
        args.push("-f".to_string());
        args.push(output_format.to_string());

        // Output to stdout (pipe)
        args.push("-".to_string());

        args
    }

    /// Determines output format from format option
    fn get_output_format(format: &str) -> &'static str {
        match format.to_lowercase().as_str() {
            "jpg" => "mjpeg",
            "jpeg" => "mjpeg",
            "png" => "png",
            "webp" => "libwebp",
            _ => "mjpeg", // Default to JPEG
        }
    }

    /// Determines MIME type from format
    fn get_mime_type(format: &str) -> String {
        match format.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "webp" => "image/webp".to_string(),
            _ => "image/jpeg".to_string(), // Default to JPEG
        }
    }
}

impl Default for FFmpegAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait]
impl ThumbnailGenerator for FFmpegAdapter {
    async fn generate(
        &self,
        file_path: &str,
        options: ThumbnailOptions,
    ) -> Result<ThumbnailResult, ThumbnailError> {
        let output_format = Self::get_output_format(options.format.as_deref().unwrap_or("jpg"));
        let args = Self::build_thumbnail_args(file_path, &options, output_format);
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let data = self.execute_ffmpeg(&args_refs).await?;

        // Determine actual dimensions (preserve aspect ratio if only one dimension specified)
        let (width, height) = if options.width.is_some() && options.height.is_some() {
            (options.width.unwrap_or(320), options.height.unwrap_or(240))
        } else if options.width.is_some() {
            (options.width.unwrap(), 320) // Default height
        } else if options.height.is_some() {
            (320, options.height.unwrap()) // Default width
        } else {
            (320, 240) // Default dimensions
        };

        let format = options.format.as_deref().unwrap_or("jpg").to_string();
        let mime_type = Self::get_mime_type(&format);

        Ok(ThumbnailResult {
            data,
            format,
            width,
            height,
            timestamp: options.timestamp.unwrap_or(0.0),
            mime_type,
        })
    }

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

    async fn generate_multiple(
        &self,
        file_path: &str,
        timestamps: Vec<f64>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<Vec<ThumbnailResult>, ThumbnailError> {
        let mut results = Vec::with_capacity(timestamps.len());

        for timestamp in timestamps {
            let result = self.generate_at(file_path, timestamp, width, height).await?;
            results.push(result);
        }

        Ok(results)
    }

    async fn generate_poster(
        &self,
        file_path: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<ThumbnailResult, ThumbnailError> {
        // For poster, use 10% of video duration as timestamp
        // This is a simplified approach - real implementation would get duration first
        let options = ThumbnailOptions {
            timestamp: Some(0.0), // Start of video
            width,
            height,
            ..Default::default()
        };

        self.generate(file_path, options).await
    }
}
