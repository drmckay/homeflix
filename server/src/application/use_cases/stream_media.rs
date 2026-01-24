//! Stream Media Use Case
//!
//! Orchestrates media streaming process including:
//! - Direct streaming (range requests)
//! - Transcoding (when needed)
//! - Audio track switching
//! - Progress tracking

use std::sync::Arc;
use std::path::Path;
use tracing::{info, debug, warn, error};

use crate::domain::entities::Media;
use crate::domain::repositories::MediaRepository;
use crate::interfaces::external_services::VideoAnalyzer;
use crate::shared::error::ApplicationError;

/// Streaming configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Whether to transcode audio to AAC
    pub transcode_audio: bool,
    /// Video bitrate (bps)
    pub video_bitrate: Option<u32>,
    /// Audio bitrate (bps)
    pub audio_bitrate: Option<u32>,
    /// Maximum segment duration (seconds)
    pub max_segment_duration: Option<u32>,
    /// Whether to use hardware acceleration
    pub hardware_acceleration: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            transcode_audio: true,
            video_bitrate: None,
            audio_bitrate: None,
            max_segment_duration: Some(10),
            hardware_acceleration: false,
        }
    }
}

/// Result of stream request
#[derive(Debug, Clone)]
pub struct StreamResult {
    /// Content type
    pub content_type: String,
    /// Content length in bytes
    pub content_length: u64,
    /// Whether this is a range request
    pub is_range_request: bool,
    /// Range start (if applicable)
    pub range_start: Option<u64>,
    /// Range end (if applicable)
    pub range_end: Option<u64>,
    /// Whether transcoding is needed
    pub needs_transcoding: bool,
}

/// Stream Media Use Case
///
/// Orchestrates complete media streaming workflow:
/// 1. Validates media exists and is accessible
/// 2. Determines if transcoding is needed
/// 3. Handles HTTP range requests
/// 4. Manages audio track switching
/// 5. Tracks playback progress
///
/// # Architecture Notes
/// - Supports direct play for compatible formats
/// - Supports smart transcoding for web compatibility
/// - Uses dependency injection for all services
pub struct StreamMediaUseCase {
    /// Media repository for media lookup
    media_repository: Arc<dyn MediaRepository>,
    /// Video analyzer for codec detection
    video_analyzer: Arc<dyn VideoAnalyzer>,
    /// Default streaming configuration
    default_config: StreamConfig,
}

impl StreamMediaUseCase {
    /// Creates a new stream media use case
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media lookup
    /// * `video_analyzer` - Video analyzer for codec detection
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        video_analyzer: Arc<dyn VideoAnalyzer>,
    ) -> Self {
        Self {
            media_repository,
            video_analyzer,
            default_config: StreamConfig::default(),
        }
    }

    /// Sets the default streaming configuration
    pub fn with_config(mut self, config: StreamConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Prepares media for streaming
    ///
    /// # Arguments
    /// * `media_id` - ID of media to stream
    ///
    /// # Returns
    /// * `Result<(Media, StreamResult), ApplicationError>` - Media and stream info
    ///
    /// # Errors
    /// Returns error if:
    /// - Media not found
    /// - File not accessible
    /// - Video analysis fails
    pub async fn prepare_stream(
        &self,
        media_id: i64,
    ) -> Result<(Media, StreamResult), ApplicationError> {
        // Fetch media from database
        let media = self.media_repository
            .find_by_id(media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Media with ID {} not found", media_id))
            ))?;

        info!("Preparing stream for media: {} (ID: {})", media.file_path, media_id);

        // Check if file exists and is accessible
        let path = Path::new(&media.file_path);
        if !path.exists() {
            return Err(ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::PathNotFound(media.file_path.clone())
            ));
        }

        // Get file metadata
        let metadata = std::fs::metadata(&path).map_err(|e| {
            ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::Io(e)
            )
        })?;

        let file_size = metadata.len();
        let content_type = self.determine_content_type(&path)?;

        // Analyze video to determine if transcoding is needed
        let needs_transcoding = self.check_transcoding_needed(&media, &path).await?;

        debug!(
            "Stream prepared: size={}, content_type={}, needs_transcoding={}",
            file_size,
            content_type,
            needs_transcoding
        );

        let stream_result = StreamResult {
            content_type,
            content_length: file_size,
            is_range_request: false,
            range_start: None,
            range_end: None,
            needs_transcoding,
        };

        Ok((media, stream_result))
    }

    /// Prepares a range request for streaming
    ///
    /// # Arguments
    /// * `media_id` - ID of media to stream
    /// * `range_start` - Start byte position
    /// * `range_end` - End byte position
    ///
    /// # Returns
    /// * `Result<(Media, StreamResult), ApplicationError>` - Media and stream info
    pub async fn prepare_range_stream(
        &self,
        media_id: i64,
        range_start: u64,
        range_end: u64,
    ) -> Result<(Media, StreamResult), ApplicationError> {
        // Fetch media from database
        let media = self.media_repository
            .find_by_id(media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Media with ID {} not found", media_id))
            ))?;

        info!(
            "Preparing range stream for media: {} (ID: {}) range={}-{}",
            media.file_path,
            media_id,
            range_start,
            range_end
        );

        // Check if file exists and is accessible
        let path = Path::new(&media.file_path);
        if !path.exists() {
            return Err(ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::PathNotFound(media.file_path.clone())
            ));
        }

        // Get file metadata
        let metadata = std::fs::metadata(&path).map_err(|e| {
            ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::Io(e)
            )
        })?;

        let file_size = metadata.len();
        let content_type = self.determine_content_type(&path)?;

        // Validate range
        if range_start >= file_size {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::InvalidInput(format!(
                    "Range start {} exceeds file size {}",
                    range_start, file_size
                ))
            ));
        }

        if range_end > file_size {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::InvalidInput(format!(
                    "Range end {} exceeds file size {}",
                    range_end, file_size
                ))
            ));
        }

        // Analyze video to determine if transcoding is needed
        let needs_transcoding = self.check_transcoding_needed(&media, &path).await?;

        debug!(
            "Range stream prepared: size={}, content_type={}, needs_transcoding={}, range={}-{}",
            file_size,
            content_type,
            needs_transcoding,
            range_start,
            range_end
        );

        let stream_result = StreamResult {
            content_type,
            content_length: file_size,
            is_range_request: true,
            range_start: Some(range_start),
            range_end: Some(range_end),
            needs_transcoding,
        };

        Ok((media, stream_result))
    }

    /// Updates playback progress for media
    ///
    /// # Arguments
    /// * `media_id` - ID of media
    /// * `position` - Current playback position in seconds
    /// * `watched` - Whether media is fully watched
    ///
    /// # Returns
    /// * `Result<(), ApplicationError>` - Success or error
    pub async fn update_progress(
        &self,
        media_id: i64,
        position: i64,
        watched: bool,
    ) -> Result<(), ApplicationError> {
        info!(
            "Updating progress for media ID {}: position={}, watched={}",
            media_id,
            position,
            watched
        );

        self.media_repository
            .update_progress(media_id, position, watched)
            .await?;

        debug!("Progress updated for media ID {}", media_id);
        Ok(())
    }

    /// Determines the content type for streaming
    fn determine_content_type(&self, path: &Path) -> Result<String, ApplicationError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| ApplicationError::Internal("Invalid file extension".to_string()))?;

        let content_type = match extension.to_lowercase().as_str() {
            "mp4" | "m4v" => "video/mp4",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "wmv" => "video/x-ms-wmv",
            "flv" => "video/x-flv",
            "ts" | "m2ts" => "video/mp2t",
            "ogv" => "video/ogg",
            "3gp" => "video/3gpp",
            _ => "application/octet-stream",
        };

        Ok(content_type.to_string())
    }

    /// Checks if transcoding is needed for the media
    async fn check_transcoding_needed(
        &self,
        media: &Media,
        path: &Path,
    ) -> Result<bool, ApplicationError> {
        // Analyze video codec
        let path_str = path.to_str().ok_or_else(|| ApplicationError::Filesystem(crate::shared::error::FilesystemError::InvalidPath(path.to_string_lossy().to_string())))?;
        let analysis = self.video_analyzer.analyze(path_str).await?;

        // Check if video codec is web-compatible
        let web_compatible_codecs = ["h264", "hevc", "vp8", "vp9", "av1"];

        let video_codec = analysis.video_codec.clone().unwrap_or("unknown".to_string()).to_lowercase();
        let is_web_compatible = web_compatible_codecs.contains(&video_codec.as_str());

        // Check if audio codec is web-compatible
        let web_compatible_audio = ["aac", "opus", "vorbis"];
        let audio_codec = analysis.audio_codec.clone().unwrap_or("unknown".to_string()).to_lowercase();
        let is_audio_compatible = web_compatible_audio.contains(&audio_codec.as_str());

        // Check if container is web-compatible
        let web_compatible_containers = ["mp4", "webm"];
        let container = analysis.container.clone().unwrap_or("unknown".to_string()).to_lowercase();
        let is_container_compatible = web_compatible_containers.contains(&container.as_str());

        let needs_transcoding = !is_web_compatible || !is_audio_compatible || !is_container_compatible;

        debug!(
            "Transcoding check: video_codec={}, audio_codec={}, container={}, needs_transcoding={}",
            video_codec,
            audio_codec,
            container,
            needs_transcoding
        );

        Ok(needs_transcoding)
    }

    /// Generates FFmpeg command for transcoding
    ///
    /// # Arguments
    /// * `input_path` - Input file path
    /// * `config` - Streaming configuration
    ///
    /// # Returns
    /// * `Vec<String>` - FFmpeg command arguments
    pub fn generate_transcode_command(&self, input_path: &str, config: &StreamConfig) -> Vec<String> {
        let mut args = vec![
            "-i".to_string(),
            input_path.to_string(),
            "-c:v".to_string(),
            "libx264".to_string(),
            "-preset".to_string(),
            "fast".to_string(),
            "-crf".to_string(),
            "23".to_string(),
            "-maxrate".to_string(),
            format!("{}k", config.video_bitrate.unwrap_or(5000) / 1000),
            "-bufsize".to_string(),
            format!("{}k", config.video_bitrate.unwrap_or(5000) / 100),
            "-movflags".to_string(),
            "faststart".to_string(),
        ];

        if config.transcode_audio {
            args.extend(vec![
                "-c:a".to_string(),
                "aac".to_string(),
                "-b:a".to_string(),
                format!("{}k", config.audio_bitrate.unwrap_or(128)),
                "-ar".to_string(),
                "48000".to_string(),
            ]);
        }

        if let Some(max_duration) = config.max_segment_duration {
            args.extend(vec![
                "-f".to_string(),
                "segment".to_string(),
                "-segment_time".to_string(),
                format!("{}", max_duration),
                "-segment_format".to_string(),
                "m4a".to_string(),
            ]);
        }

        if config.hardware_acceleration {
            args.insert(1, "-hwaccel".to_string());
            args.insert(2, "auto".to_string());
        }

        args
    }

    /// Gets a file handle for streaming
    ///
    /// # Arguments
    /// * `media_id` - ID of media to stream
    ///
    /// # Returns
    /// * `Result<tokio::fs::File, ApplicationError>` - File handle for streaming
    ///
    /// # Errors
    /// Returns error if:
    /// - Media not found
    /// - File not accessible
    pub async fn get_file_handle(
        &self,
        media_id: i64,
    ) -> Result<tokio::fs::File, ApplicationError> {
        // Fetch media from database
        let media = self.media_repository
            .find_by_id(media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Media with ID {} not found", media_id))
            ))?;

        // Open file for streaming
        let path = Path::new(&media.file_path);
        tokio::fs::File::open(&path).await.map_err(|e| {
            ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::Io(e)
            )
        })
    }

    /// Validates if a file is streamable
    ///
    /// # Arguments
    /// * `path` - File path to check
    ///
    /// # Returns
    /// * `Result<bool, ApplicationError>` - Whether file is streamable
    pub fn is_streamable(path: &Path) -> Result<bool, ApplicationError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| ApplicationError::Internal("Invalid file extension".to_string()))?;

        let streamable_extensions = [
            "mp4", "mkv", "avi", "mov", "webm", "flv", "ts", "m2ts", "ogv", "3gp"
        ];

        Ok(streamable_extensions.contains(&extension.to_lowercase().as_str()))
    }
}
