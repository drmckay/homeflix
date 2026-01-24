// FFmpeg/FFprobe Adapters
//
// This module provides implementations for video analysis and thumbnail generation
// using FFmpeg and FFprobe.

pub mod ffprobe_adapter;
pub mod ffmpeg_adapter;

pub use ffprobe_adapter::FFprobeAdapter;
pub use ffmpeg_adapter::FFmpegAdapter;
