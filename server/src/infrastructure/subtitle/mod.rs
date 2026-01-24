//! Subtitle Infrastructure Module
//!
//! This module provides subtitle-related functionality including:
//! - Detection of external subtitle files (.srt)
//! - Language detection from filenames
//! - SRT to WebVTT conversion for HTML5 compatibility

pub mod detector;
pub mod converter;

pub use detector::*;
pub use converter::*;
