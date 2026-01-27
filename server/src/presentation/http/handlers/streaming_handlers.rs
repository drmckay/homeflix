//! Streaming Handlers
//!
//! HTTP handlers for media streaming.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::application::use_cases::stream_media::StreamMediaUseCase;
use crate::interfaces::external_services::VideoAnalyzer;
use crate::shared::error::ApplicationError;
use crate::infrastructure::subtitle::{SubtitleDetector, read_and_convert_srt_with_offset};
use crate::domain::repositories::MediaRepository;
use crate::domain::events::{
    StreamStartedEvent,
    StreamEndedEvent,
    StreamErrorEvent,
    ThumbnailGeneratedEvent,
};
use crate::infrastructure::messaging::InMemoryEventBus;
use crate::interfaces::messaging::EventBus;
use std::ops::Deref;
use tokio::io::{AsyncSeekExt, AsyncReadExt};
use tokio::process::Command;
use tokio_util::io::ReaderStream;

/// Query parameters for web streaming
#[derive(Debug, Deserialize)]
pub struct WebStreamQuery {
    /// Start position in seconds (accepts float, converted to integer)
    #[serde(default)]
    pub start: f64,
    /// Audio track index (optional)
    pub audio: Option<i32>,
}

/// Helper function to publish streaming events
async fn publish_stream_event<T: crate::interfaces::messaging::DomainEvent>(
    bus: &Option<Arc<InMemoryEventBus>>,
    event: T,
) {
    if let Some(bus) = bus {
        use std::ops::Deref;
        if let Err(e) = bus.deref().publish(event).await {
            tracing::warn!("Failed to publish streaming event: {}", e);
        }
    }
}

/// Stream media by ID
pub async fn stream_media(
    State(use_case): State<Arc<StreamMediaUseCase>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    // Check for Range header
    let range_header = headers.get(header::RANGE)
        .and_then(|h| h.to_str().ok());

    if let Some(range_str) = range_header {
        // Parse range header (e.g., "bytes=0-1023")
        if let Some(range) = parse_range_header(range_str) {
            let (start, end) = range;

            // Prepare stream and get file handle from use case (delegates file I/O)
            return match use_case.prepare_stream(id).await {
                Ok((media, result)) => {
                    // Publish stream started event
                    let client_ip = headers.get(header::FORWARDED)
                        .or_else(|| headers.get("x-forwarded-for"))
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());
                    let user_agent = headers.get(header::USER_AGENT)
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());
                    let event = StreamStartedEvent::new(
                        id,
                        client_ip,
                        user_agent,
                        result.needs_transcoding,
                    );
                    publish_stream_event(&event_bus, event).await;

                    let file_size = result.content_length;
                    let end = end.unwrap_or(file_size - 1);

                    if start >= file_size {
                        return Err((StatusCode::RANGE_NOT_SATISFIABLE, "Range not satisfiable".to_string()));
                    }

                    // Get file handle from use case (delegates file I/O)
                    let file = use_case.get_file_handle(id).await
                        .map_err(|e| map_error(e))?;

                    // Seek to start position
                    let length = end - start + 1;
                    let mut file = file;
                    file.seek(std::io::SeekFrom::Start(start)).await
                        .map_err(|e| map_error(ApplicationError::Filesystem(
                            crate::shared::error::FilesystemError::Io(e)
                        )))?;

                    // Create stream limited to range length
                    let stream = ReaderStream::new(file.take(length));
                    let body = Body::from_stream(stream);

                    // Build partial content response
                    let mut response = Response::new(body);
                    *response.status_mut() = StatusCode::PARTIAL_CONTENT;
                    response.headers_mut().insert(header::CONTENT_TYPE, result.content_type.parse().unwrap());
                    response.headers_mut().insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
                    response.headers_mut().insert(header::CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, file_size).parse().unwrap());
                    response.headers_mut().insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());

                    Ok(response)
                }
                Err(e) => Err(map_error(e)),
            };
        }
    }

    // No range header - stream full file
    match use_case.prepare_stream(id).await {
        Ok((media, result)) => {
            // Publish stream started event
            let client_ip = headers.get(header::FORWARDED)
                .or_else(|| headers.get("x-forwarded-for"))
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let user_agent = headers.get(header::USER_AGENT)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let event = StreamStartedEvent::new(
                id,
                client_ip,
                user_agent,
                result.needs_transcoding,
            );
            publish_stream_event(&event_bus, event).await;

            // Get file handle from use case (delegates file I/O)
            let file = use_case.get_file_handle(id).await
                .map_err(|e| map_error(e))?;
            
            // Create stream from file
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            
            // Build response
            let mut response = Response::new(body);
            response.headers_mut().insert(header::CONTENT_TYPE, result.content_type.parse().unwrap());
            response.headers_mut().insert(header::CONTENT_LENGTH, result.content_length.to_string().parse().unwrap());
            response.headers_mut().insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());
            
            Ok(response)
        }
        Err(e) => Err(map_error(e)),
    }
}

/// Parse HTTP Range header
/// 
/// # Arguments
/// * `header` - Range header value (e.g., "bytes=0-1023")
///
/// # Returns
/// * `Option<(u64, Option<u64>)>` - Start and optional end byte positions
fn parse_range_header(header: &str) -> Option<(u64, Option<u64>)> {
    if !header.starts_with("bytes=") {
        return None;
    }
    let range = &header[6..];
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parts[0].parse::<u64>().ok()?;
    let end = if parts[1].is_empty() {
        None
    } else {
        parts[1].parse::<u64>().ok()
    };
    Some((start, end))
}

/// Stream diagnostic response
#[derive(Debug, Serialize)]
pub struct StreamDiagnostic {
    pub media_id: i64,
    pub file_path: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
    pub width: u32,
    pub height: u32,
    pub duration_seconds: f64,
    pub audio_tracks: usize,
    pub needs_video_transcode: bool,
    pub browser_compatible: bool,
}

/// Check if video codec is browser-compatible
fn is_browser_compatible_codec(codec: &str) -> bool {
    let codec_lower = codec.to_lowercase();
    matches!(codec_lower.as_str(),
        "h264" | "avc" | "avc1" |
        "vp8" | "vp9" |
        "av1" | "av01"
    )
}

/// Diagnostic endpoint to check stream compatibility
pub async fn stream_diagnostic(
    State(use_case): State<Arc<StreamMediaUseCase>>,
    State(video_analyzer): State<Arc<dyn VideoAnalyzer>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get media info
    let (media, _result) = use_case.prepare_stream(id).await
        .map_err(|e| map_error(e))?;

    let file_path = &media.file_path;

    // Analyze video file
    let analysis = video_analyzer.analyze(file_path).await
        .map_err(|e| {
            tracing::error!("Failed to analyze video {}: {}", file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to analyze video: {}", e))
        })?;

    let video_codec = analysis.video_codec.clone();
    let needs_transcode = video_codec.as_ref()
        .map(|c| !is_browser_compatible_codec(c))
        .unwrap_or(false);

    let browser_compatible = video_codec.as_ref()
        .map(|c| is_browser_compatible_codec(c))
        .unwrap_or(false);

    Ok(Json(StreamDiagnostic {
        media_id: id,
        file_path: file_path.clone(),
        video_codec: analysis.video_codec,
        audio_codec: analysis.audio_codec,
        container: analysis.container,
        width: analysis.width,
        height: analysis.height,
        duration_seconds: analysis.duration_seconds,
        audio_tracks: analysis.audio_tracks.len(),
        needs_video_transcode: needs_transcode,
        browser_compatible,
    }))
}

/// Web streaming with FFmpeg transcoding
///
/// Transcodes media to fragmented MP4 for web playback, starting from a specified position.
/// Video is transcoded to H.264 if needed (HEVC etc), audio is transcoded to AAC for compatibility.
pub async fn stream_web(
    State(use_case): State<Arc<StreamMediaUseCase>>,
    State(video_analyzer): State<Arc<dyn VideoAnalyzer>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(id): Path<i64>,
    Query(query): Query<WebStreamQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Get media info
    let (media, result) = use_case.prepare_stream(id).await
        .map_err(|e| map_error(e))?;

    // Publish stream started event
    let event = StreamStartedEvent::new(
        id,
        None, // Client IP not available in this context
        None, // User agent not available in this context
        result.needs_transcoding,
    );
    publish_stream_event(&event_bus, event).await;

    let file_path = &media.file_path;
    let start_seconds = query.start.floor() as i64; // Convert float to integer seconds
    let audio_track = query.audio.unwrap_or(0);

    // Analyze video to check codec compatibility
    let analysis = video_analyzer.analyze(file_path).await
        .map_err(|e| {
            tracing::error!("Failed to analyze video {}: {}", file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to analyze video".to_string())
        })?;

    let video_codec = analysis.video_codec.as_deref().unwrap_or("unknown");
    let audio_codec = analysis.audio_codec.as_deref().unwrap_or("unknown");
    let needs_video_transcode = !is_browser_compatible_codec(video_codec);
    
    // Check if audio is already AAC (case-insensitive)
    let audio_is_aac = audio_codec.to_lowercase() == "aac";
    let needs_audio_transcode = !audio_is_aac;

    tracing::info!(
        "Web stream: id={}, file={}, start={}s, audio_track={}, video_codec={}, audio_codec={}, video_transcode={}, audio_transcode={}",
        id, file_path, start_seconds, audio_track, video_codec, audio_codec, needs_video_transcode, needs_audio_transcode
    );

    // Build FFmpeg command - transcode video if needed
    let video_codec_args: Vec<&str> = if needs_video_transcode {
        // Transcode to H.264 for browser compatibility
        vec!["-c:v", "libx264", "-preset", "fast", "-crf", "23"]
    } else {
        // Copy video stream (no re-encoding)
        vec!["-c:v", "copy"]
    };

    // Build audio codec args - only transcode if not already AAC
    let audio_codec_args: Vec<&str> = if needs_audio_transcode {
        // Transcode audio to AAC
        vec!["-c:a", "aac", "-b:a", "192k", "-ac", "2"]
    } else {
        // Copy audio stream (already AAC, no re-encoding)
        vec!["-c:a", "copy"]
    };

    let mut cmd = Command::new("ffmpeg");

    // Seeking AFTER input for accurate frame-level sync (slower but precise)
    // This ensures both video and audio start at exactly the same point
    cmd.args(["-i", file_path])
        .args(["-ss", &start_seconds.to_string()])  // Seek after input (accurate)
        .args(["-map", "0:v:0"])                    // First video stream
        .args(["-map", &format!("0:a:{}", audio_track)]); // Selected audio track

    // Add video codec args
    for arg in &video_codec_args {
        cmd.arg(arg);
    }

    // Add audio codec args
    for arg in &audio_codec_args {
        cmd.arg(arg);
    }

    // Add output args with proper A/V sync
    if needs_video_transcode {
        // Transcoding video: We can regenerate timestamps and enforce CFR
        cmd.args([
            "-vsync", "cfr",                       // Constant frame rate - regenerates timestamps for A/V sync
            "-async_depth", "1",                   // Audio sync depth
            "-fflags", "+genpts",                  // Generate presentation timestamps
            "-avoid_negative_ts", "make_zero",      // Normalize negative timestamps
            "-start_at_zero",                     // Start output timestamps at 0
            "-movflags", "frag_keyframe+empty_moov+default_base_moof", // Fragmented MP4
            "-f", "mp4",                          // Output format
            "-",                                  // Output to stdout
        ]);
    } else {
        // Copying video: Preserves original frame timing. 
        // Cannot use -vsync/fps_mode with stream copy.
        // We use -copyts to maintain synchronization between copied video and (potentially) transcoded audio.
        cmd.args([
            "-copyts",                              // Copy timestamps from input
            "-avoid_negative_ts", "make_zero",      // Normalize negative timestamps
            "-movflags", "frag_keyframe+empty_moov+default_base_moof", // Fragmented MP4
            "-f", "mp4",                           // Output format
            "-",                                   // Output to stdout
        ]);
    }

    let mut ffmpeg = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())      // Suppress FFmpeg logs
        .spawn()
        .map_err(|e| {
            tracing::error!("Failed to spawn FFmpeg: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to start transcoding".to_string())
        })?;

    let stdout = ffmpeg.stdout.take()
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get FFmpeg stdout".to_string()))?;

    // Stream FFmpeg output directly to client
    let stream = ReaderStream::new(stdout);
    let body = Body::from_stream(stream);

    // Build response
    let mut response = Response::new(body);
    response.headers_mut().insert(header::CONTENT_TYPE, "video/mp4".parse().unwrap());
    response.headers_mut().insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());

    Ok(response)
}

/// Query parameters for thumbnail generation
#[derive(Debug, Deserialize)]
pub struct ThumbnailQuery {
    /// Timestamp in seconds (default: 10% into video)
    pub timestamp: Option<f64>,
    /// Width in pixels (default: 320)
    pub width: Option<u32>,
}

/// Generate thumbnail from video
///
/// Returns a JPEG image extracted from the video at the specified timestamp.
/// Used for media items without TMDB poster images.
pub async fn generate_thumbnail(
    State(use_case): State<Arc<StreamMediaUseCase>>,
    State(video_analyzer): State<Arc<dyn VideoAnalyzer>>,
    State(event_bus): State<Option<Arc<InMemoryEventBus>>>,
    Path(id): Path<i64>,
    Query(query): Query<ThumbnailQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Get media info
    let (media, _result) = use_case.prepare_stream(id).await
        .map_err(|e| map_error(e))?;

    let file_path = &media.file_path;
    let width = query.width.unwrap_or(320);

    // Get video duration to calculate default timestamp
    let analysis = video_analyzer.analyze(file_path).await
        .map_err(|e| {
            tracing::error!("Failed to analyze video {}: {}", file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to analyze video".to_string())
        })?;

    // Default to 10% into video if no timestamp specified
    let timestamp = query.timestamp.unwrap_or(analysis.duration_seconds * 0.1);

    tracing::info!("Generating thumbnail: id={}, timestamp={}s, width={}", id, timestamp, width);

    // Build FFmpeg command to extract frame
    let output = Command::new("ffmpeg")
        .args(["-ss", &timestamp.to_string()])
        .args(["-i", file_path])
        .args(["-vframes", "1"])
        .args(["-vf", &format!("scale={}:-1", width)])
        .args(["-f", "mjpeg"])
        .args(["-q:v", "3"])  // Quality (lower = better)
        .arg("-")
        .output()
        .await
        .map_err(|e| {
            tracing::error!("Failed to run FFmpeg: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate thumbnail".to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("FFmpeg failed: {}", stderr);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate thumbnail".to_string()));
    }

    let body = Body::from(output.stdout);

    // Calculate height from width (maintain aspect ratio)
    // Use width and height from analysis if available
    let height = if analysis.width > 0 && analysis.height > 0 {
        (width as f64 * (analysis.height as f64 / analysis.width as f64)) as u32
    } else {
        // Default to 16:9 aspect ratio
        (width as f64 * 0.5625) as u32
    };

    // Publish thumbnail generated event
    // Note: We don't have a file path since thumbnail is streamed directly
    // We'll use a placeholder path or just the media ID
    let thumbnail_path = format!("thumbnail://media/{}", id);
    let event = ThumbnailGeneratedEvent::new(id, thumbnail_path, width, height);
    publish_stream_event(&event_bus, event).await;

    let mut response = Response::new(body);
    response.headers_mut().insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
    response.headers_mut().insert(header::CACHE_CONTROL, "max-age=86400".parse().unwrap()); // Cache for 1 day

    Ok(response)
}

/// Query parameters for subtitle endpoint
#[derive(Debug, Deserialize)]
pub struct SubtitleQuery {
    /// Offset in seconds to subtract from all timestamps.
    /// Used when streaming starts from a position other than 0.
    #[serde(default)]
    pub offset: f64,
}

/// Get subtitle by media ID and track index
///
/// Returns subtitle content in WebVTT format for HTML5 video compatibility.
/// Converts SRT subtitles to WebVTT on-the-fly.
///
/// # Path Parameters
/// - `media_id` - Media item ID
/// - `index` - Subtitle track index (from /v2/media/{id}/tracks response)
///
/// # Query Parameters
/// - `offset` - (optional) Seconds to subtract from timestamps for sync with seeked video
///
/// # Response
/// - 200: WebVTT subtitle content
/// - 404: Media or subtitle not found
/// - 500: Internal error
pub async fn get_subtitle(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    Path((media_id, index)): Path<(i64, usize)>,
    Query(query): Query<SubtitleQuery>,
) -> Result<Response, (StatusCode, String)> {
    // Get media to find file path
    let media = media_repo
        .find_by_id(media_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Media {} not found", media_id)))?;

    // Discover external subtitles
    let subtitle_detector = SubtitleDetector::new();
    let video_path = std::path::Path::new(&media.file_path);
    let external_subtitles = subtitle_detector.discover(video_path);

    // Check if requested index is valid
    if index >= external_subtitles.len() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Subtitle track {} not found (only {} external subtitles available)", index, external_subtitles.len())
        ));
    }

    // Get the requested subtitle
    let subtitle = &external_subtitles[index];

    // Read and convert SRT to WebVTT (with optional offset for seek sync)
    let vtt_content = read_and_convert_srt_with_offset(&subtitle.file_path, query.offset)
        .map_err(|e| {
            tracing::error!("Failed to convert subtitle {}: {}", subtitle.file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to convert subtitle: {}", e))
        })?;

    // Build response with WebVTT content
    let body = Body::from(vtt_content);
    let mut response = Response::new(body);
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/vtt; charset=utf-8".parse().unwrap()
    );

    Ok(response)
}

/// Map ApplicationError to HTTP response
fn map_error(e: ApplicationError) -> (StatusCode, String) {
    match e {
        ApplicationError::Domain(crate::shared::error::DomainError::NotFound(msg)) => (StatusCode::NOT_FOUND, msg),
        ApplicationError::Filesystem(crate::shared::error::FilesystemError::PathNotFound(msg)) => (StatusCode::NOT_FOUND, format!("File not found: {}", msg)),
        _ => {
            tracing::error!("Streaming error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        }
    }
}
