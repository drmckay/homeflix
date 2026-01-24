// Video Analyzer Interface
//
// This module defines the interface for video file analysis.
// Typically implemented using FFprobe/FFmpeg.
//
// This interface enables:
// - Testing with mock implementations
// - Swapping FFmpeg implementations
// - Adding caching layers

use async_trait::async_trait;
use crate::shared::error::VideoAnalyzerError;

/// Video analysis result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoAnalysis {
    /// Video duration in seconds
    pub duration_seconds: f64,
    /// Video width in pixels
    pub width: u32,
    /// Video height in pixels
    pub height: u32,
    /// Video codec
    pub video_codec: Option<String>,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// Video bitrate in bits per second
    pub video_bitrate: Option<u64>,
    /// Audio bitrate in bits per second
    pub audio_bitrate: Option<u64>,
    /// Frame rate (frames per second)
    pub frame_rate: Option<f64>,
    /// Pixel format
    pub pixel_format: Option<String>,
    /// Video rotation (degrees)
    pub rotation: Option<u32>,
    /// Container format (e.g., "mp4", "matroska")
    pub container: Option<String>,
    /// List of audio tracks
    pub audio_tracks: Vec<AudioTrack>,
    /// List of subtitle tracks
    pub subtitle_tracks: Vec<SubtitleTrack>,
}

/// Audio track information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioTrack {
    /// Track index
    pub index: usize,
    /// Language code (e.g., "eng", "hun")
    pub language: Option<String>,
    /// Codec
    pub codec: Option<String>,
    /// Sample rate in Hz
    pub sample_rate: Option<u32>,
    /// Number of channels
    pub channels: Option<u32>,
    /// Bitrate in bits per second
    pub bitrate: Option<u64>,
    /// Track title
    pub title: Option<String>,
    /// Whether this is the default audio track
    pub is_default: bool,
}

/// Subtitle track information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubtitleTrack {
    /// Track index
    pub index: usize,
    /// Language code (e.g., "eng", "hun")
    pub language: Option<String>,
    /// Codec/format
    pub codec: Option<String>,
    /// Track title
    pub title: Option<String>,
    /// Whether this is the default subtitle track
    pub is_default: bool,
}

/// Video analyzer interface
/// 
/// Provides methods for analyzing video files to extract metadata
/// such as duration, resolution, codecs, and track information.
#[async_trait]
pub trait VideoAnalyzer: Send + Sync {
    /// Analyze a video file and extract metadata
    /// 
    /// # Arguments
    /// * `file_path` - Path to the video file
    /// 
    /// # Returns
    /// * `Result<VideoAnalysis, VideoAnalyzerError>` - Video analysis result
    /// 
    /// # Errors
    /// Returns error if:
    /// - File does not exist
    /// - File is not a valid video file
    /// - FFprobe execution fails
    async fn analyze(&self, file_path: &str) -> Result<VideoAnalysis, VideoAnalyzerError>;
    
    /// Get video duration in seconds
    /// 
    /// # Arguments
    /// * `file_path` - Path to the video file
    /// 
    /// # Returns
    /// * `Result<f64, VideoAnalyzerError>` - Duration in seconds
    async fn get_duration(&self, file_path: &str) -> Result<f64, VideoAnalyzerError>;
    
    /// Get video resolution (width, height)
    /// 
    /// # Arguments
    /// * `file_path` - Path to the video file
    /// 
    /// # Returns
    /// * `Result<(u32, u32), VideoAnalyzerError>` - Width and height in pixels
    async fn get_resolution(&self, file_path: &str) -> Result<(u32, u32), VideoAnalyzerError>;
    
    /// Get all audio tracks from a video file
    /// 
    /// # Arguments
    /// * `file_path` - Path to the video file
    /// 
    /// # Returns
    /// * `Result<Vec<AudioTrack>, VideoAnalyzerError>` - List of audio tracks
    async fn get_audio_tracks(&self, file_path: &str) -> Result<Vec<AudioTrack>, VideoAnalyzerError>;
    
    /// Get all subtitle tracks from a video file
    /// 
    /// # Arguments
    /// * `file_path` - Path to the video file
    /// 
    /// # Returns
    /// * `Result<Vec<SubtitleTrack>, VideoAnalyzerError>` - List of subtitle tracks
    async fn get_subtitle_tracks(&self, file_path: &str) -> Result<Vec<SubtitleTrack>, VideoAnalyzerError>;
    
    /// Check if a file is a valid video file
    /// 
    /// # Arguments
    /// * `file_path` - Path to the file
    /// 
    /// # Returns
    /// * `Result<bool, VideoAnalyzerError>` - True if valid video file
    async fn is_valid_video(&self, file_path: &str) -> Result<bool, VideoAnalyzerError>;
}
