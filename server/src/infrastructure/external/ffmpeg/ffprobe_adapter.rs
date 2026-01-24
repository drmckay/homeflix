//! FFprobe Adapter Implementation
//!
//! Provides FFprobe-based implementation of VideoAnalyzer interface

use async_trait::async_trait;
use tokio::process::Command;
use std::time::Duration;
use tokio::time::timeout;
use crate::interfaces::external_services::{
    VideoAnalyzer, VideoAnalysis, AudioTrack, SubtitleTrack,
};
use crate::shared::error::VideoAnalyzerError;

/// FFprobe adapter for video analysis
pub struct FFprobeAdapter {
    timeout: Duration,
}

impl FFprobeAdapter {
    /// Creates a new FFprobe adapter
    ///
    /// # Arguments
    /// * `timeout` - Timeout for FFprobe execution
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Executes FFprobe command and returns output
    async fn execute_ffprobe(&self, args: &[&str]) -> Result<String, VideoAnalyzerError> {
        let output = timeout(self.timeout, async {
            let output = Command::new("ffprobe")
                .args(args)
                .output()
                .await;

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(stdout)
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        Err(VideoAnalyzerError::ExecutionFailed(stderr))
                    }
                }
                Err(e) => Err(VideoAnalyzerError::Io(e)),
            }
        }).await;

        match output {
            Ok(result) => result,
            Err(_) => Err(VideoAnalyzerError::Timeout("FFprobe execution timed out".into())),
        }
    }

    /// Parses FFprobe JSON output
    fn parse_ffprobe_json(json: &str) -> Result<serde_json::Value, VideoAnalyzerError> {
        serde_json::from_str(json)
            .map_err(|e| VideoAnalyzerError::ParseError(e.to_string()))
    }

    /// Extracts duration from FFprobe output
    fn extract_duration(json: &serde_json::Value) -> Result<f64, VideoAnalyzerError> {
        json.get("format")
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| VideoAnalyzerError::ParseError("Could not parse duration".into()))
    }

    /// Extracts video resolution from FFprobe output
    fn extract_resolution(json: &serde_json::Value) -> Result<(u32, u32), VideoAnalyzerError> {
        let width = json.get("streams")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.iter().find(|s| {
                s.get("codec_type")
                    .and_then(|ct| ct.as_str())
                    .map(|t| t == "video")
                    .unwrap_or(false)
            }))
            .and_then(|v| v.get("width"))
            .and_then(|w| w.as_u64())
            .map(|w| w as u32)
            .unwrap_or(0);

        let height = json.get("streams")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.iter().find(|s| {
                s.get("codec_type")
                    .and_then(|ct| ct.as_str())
                    .map(|t| t == "video")
                    .unwrap_or(false)
            }))
            .and_then(|v| v.get("height"))
            .and_then(|h| h.as_u64())
            .map(|h| h as u32)
            .unwrap_or(0);

        Ok((width, height))
    }

    /// Extracts audio tracks from FFprobe output
    ///
    /// Note: The `index` field uses audio-relative indexing (0, 1, 2...)
    /// which corresponds to FFmpeg's `-map 0:a:N` syntax, NOT overall stream index.
    fn extract_audio_tracks(json: &serde_json::Value) -> Result<Vec<AudioTrack>, VideoAnalyzerError> {
        let streams = json.get("streams")
            .and_then(|s| s.as_array())
            .ok_or_else(|| VideoAnalyzerError::ParseError("Could not find streams".into()))?;

        let mut audio_tracks = Vec::new();
        let mut audio_index = 0usize;  // Audio-relative index for FFmpeg -map 0:a:N
        for stream in streams.iter() {
            if let Some(codec_type) = stream.get("codec_type").and_then(|ct| ct.as_str()) {
                if codec_type == "audio" {
                    let track = AudioTrack {
                        index: audio_index,
                        language: stream.get("tags")
                            .and_then(|t| t.get("language"))
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string()),
                        codec: stream.get("codec_name")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string()),
                        sample_rate: stream.get("sample_rate")
                            .and_then(|sr| sr.as_u64())
                            .map(|sr| sr as u32),
                        channels: stream.get("channels")
                            .and_then(|ch| ch.as_u64())
                            .map(|ch| ch as u32),
                        bitrate: stream.get("bit_rate")
                            .and_then(|br| br.as_u64()),
                        title: stream.get("tags")
                            .and_then(|t| t.get("title"))
                            .and_then(|title| title.as_str())
                            .map(|s| s.to_string()),
                        is_default: stream.get("disposition")
                            .and_then(|d| d.get("default"))
                            .and_then(|df| df.as_i64())
                            .map(|df| df != 0)
                            .unwrap_or(false),
                    };
                    audio_tracks.push(track);
                    audio_index += 1;
                }
            }
        }

        Ok(audio_tracks)
    }

    /// Extracts subtitle tracks from FFprobe output
    ///
    /// Note: The `index` field uses subtitle-relative indexing (0, 1, 2...)
    /// which corresponds to FFmpeg's `-map 0:s:N` syntax, NOT overall stream index.
    fn extract_subtitle_tracks(json: &serde_json::Value) -> Result<Vec<SubtitleTrack>, VideoAnalyzerError> {
        let streams = json.get("streams")
            .and_then(|s| s.as_array())
            .ok_or_else(|| VideoAnalyzerError::ParseError("Could not find streams".into()))?;

        let mut subtitle_tracks = Vec::new();
        let mut subtitle_index = 0usize;  // Subtitle-relative index for FFmpeg -map 0:s:N
        for stream in streams.iter() {
            if let Some(codec_type) = stream.get("codec_type").and_then(|ct| ct.as_str()) {
                if codec_type == "subtitle" {
                    let track = SubtitleTrack {
                        index: subtitle_index,
                        language: stream.get("tags")
                            .and_then(|t| t.get("language"))
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string()),
                        codec: stream.get("codec_name")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string()),
                        title: stream.get("tags")
                            .and_then(|t| t.get("title"))
                            .and_then(|title| title.as_str())
                            .map(|s| s.to_string()),
                        is_default: stream.get("disposition")
                            .and_then(|d| d.get("default"))
                            .and_then(|df| df.as_i64())
                            .map(|df| df != 0)
                            .unwrap_or(false),
                    };
                    subtitle_tracks.push(track);
                    subtitle_index += 1;
                }
            }
        }

        Ok(subtitle_tracks)
    }
}

impl Default for FFprobeAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait]
impl VideoAnalyzer for FFprobeAdapter {
    async fn analyze(&self, file_path: &str) -> Result<VideoAnalysis, VideoAnalyzerError> {
        let args = &[
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            file_path,
        ];

        let json_str = self.execute_ffprobe(args).await?;
        let json = Self::parse_ffprobe_json(&json_str)?;

        let duration = Self::extract_duration(&json)?;
        let (width, height) = Self::extract_resolution(&json)?;
        let audio_tracks = Self::extract_audio_tracks(&json)?;
        let subtitle_tracks = Self::extract_subtitle_tracks(&json)?;

        // Extract video codec
        let video_stream = json.get("streams")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.iter().find(|s| {
                s.get("codec_type")
                    .and_then(|ct| ct.as_str())
                    .map(|t| t == "video")
                    .unwrap_or(false)
            }));

        let video_codec = video_stream
            .and_then(|v| v.get("codec_name"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let video_bitrate = json.get("format")
            .and_then(|f| f.get("bit_rate"))
            .and_then(|br| br.as_str())
            .and_then(|s| s.parse::<u64>().ok());

        let audio_bitrate = audio_tracks.first()
            .and_then(|t| t.bitrate);

        let frame_rate = video_stream
            .and_then(|v| v.get("r_frame_rate"))
            .and_then(|fr| fr.as_str())
            .and_then(|s| s.parse::<f64>().ok());

        let pixel_format = video_stream
            .and_then(|v| v.get("pix_fmt"))
            .and_then(|pf| pf.as_str())
            .map(|s| s.to_string());

        let rotation = video_stream
            .and_then(|v| v.get("tags"))
            .and_then(|t| t.get("rotate"))
            .and_then(|r| r.as_str())
            .and_then(|s| s.parse::<u32>().ok());

        let container = json.get("format")
            .and_then(|f| f.get("format_name"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Ok(VideoAnalysis {
            duration_seconds: duration,
            width,
            height,
            video_codec,
            audio_codec: audio_tracks.first().and_then(|t| t.codec.clone()),
            video_bitrate,
            audio_bitrate,
            frame_rate,
            pixel_format,
            rotation,
            container,
            audio_tracks,
            subtitle_tracks,
        })
    }

    async fn get_duration(&self, file_path: &str) -> Result<f64, VideoAnalyzerError> {
        let args = &[
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            file_path,
        ];

        let json_str = self.execute_ffprobe(args).await?;
        let json = Self::parse_ffprobe_json(&json_str)?;
        Self::extract_duration(&json)
    }

    async fn get_resolution(&self, file_path: &str) -> Result<(u32, u32), VideoAnalyzerError> {
        let args = &[
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            file_path,
        ];

        let json_str = self.execute_ffprobe(args).await?;
        let json = Self::parse_ffprobe_json(&json_str)?;
        Self::extract_resolution(&json)
    }

    async fn get_audio_tracks(&self, file_path: &str) -> Result<Vec<AudioTrack>, VideoAnalyzerError> {
        let args = &[
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            file_path,
        ];

        let json_str = self.execute_ffprobe(args).await?;
        let json = Self::parse_ffprobe_json(&json_str)?;
        Self::extract_audio_tracks(&json)
    }

    async fn get_subtitle_tracks(&self, file_path: &str) -> Result<Vec<SubtitleTrack>, VideoAnalyzerError> {
        let args = &[
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            file_path,
        ];

        let json_str = self.execute_ffprobe(args).await?;
        let json = Self::parse_ffprobe_json(&json_str)?;
        Self::extract_subtitle_tracks(&json)
    }

    async fn is_valid_video(&self, file_path: &str) -> Result<bool, VideoAnalyzerError> {
        match self.analyze(file_path).await {
            Ok(_) => Ok(true),
            Err(VideoAnalyzerError::ExecutionFailed(_)) => Ok(false),
            Err(VideoAnalyzerError::FfprobeNotFound) => Ok(false),
            Err(VideoAnalyzerError::Timeout(_)) => Ok(false),
            Err(_) => Ok(false),
        }
    }
}
