// External Service Adapters
//
// This module contains implementations for external services including:
// - TMDB client
// - FFmpeg/FFprobe adapters
// - NFO file parser
// - Chromaprint (fpcalc) audio fingerprinting
// - Whisper.cpp speech-to-text
// - Ollama LLM translation

pub mod tmdb;
pub mod ffmpeg;
pub mod nfo;
pub mod chromaprint;
pub mod whisper;
pub mod ollama;

pub use tmdb::*;
pub use ffmpeg::*;
pub use nfo::{NfoParser, NfoMetadata};
pub use chromaprint::*;
pub use whisper::*;
pub use ollama::*;
