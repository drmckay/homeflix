//! Whisper.cpp Speech-to-Text Module
//!
//! Provides audio transcription using the whisper.cpp CLI tool.
//! Generates SRT subtitles with timestamps from video audio tracks.

mod adapter;

pub use adapter::*;
