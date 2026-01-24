//! # Media Identifier
//!
//! A guessit-inspired media filename parser implemented in pure Rust.
//! Uses a Rebulk-style pattern matching algorithm to identify media files
//! from their filenames.
//!
//! ## Quick Start
//!
//! ```rust
//! use media_identifier::parse;
//!
//! let result = parse("Dark.Matter.S01E05.720p.HDTV.x264-KILLERS.mkv");
//!
//! assert_eq!(result.title, Some("Dark Matter".to_string()));
//! assert_eq!(result.episode_info.season, Some(1));
//! assert_eq!(result.episode_info.episode, Some(5));
//! ```
//!
//! ## Architecture
//!
//! The parser uses a multi-stage pipeline inspired by Python's Rebulk library:
//!
//! 1. **Pattern Matching**: Find all known patterns (year, season/episode, quality markers, etc.)
//! 2. **Conflict Resolution**: Handle overlapping matches using priority-based rules
//! 3. **Hole Detection**: Identify unmatched regions in the input
//! 4. **Title Extraction**: Determine the title from holes and match positions
//! 5. **Assembly**: Build the final structured result
//!
//! ## Supported Patterns
//!
//! - **Season/Episode**: S01E05, S1E5, 1x05, S01E01E02, S01E01-E02
//! - **Year**: 1990-2039
//! - **Quality**: 720p, 1080p, 2160p, 4K
//! - **Source**: BluRay, WEB-DL, WEBRip, HDTV, DVDRip
//! - **Codec**: x264, x265, HEVC, XviD
//! - **Audio**: DTS, AC3, AAC, DD5.1, Atmos
//! - **Language**: Hun, Eng, Ger, Fre, etc.
//! - **Release Group**: -SPARKS, -YIFY, etc.

pub mod markers;
pub mod parser;
pub mod patterns;
pub mod tokenizer;
pub mod types;

// Re-export main types and functions for convenience
pub use parser::{parse, parse_debug, AnalysisResult, MediaParser, ParserConfig};
pub use types::{EpisodeInfo, Hole, Match, MatchCategory, MediaType, ParsedMedia, QualityInfo};
