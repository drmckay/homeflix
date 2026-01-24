//! NFO File Parser
//!
//! Parses Kodi/XBMC .nfo metadata files in various formats:
//! - XML movie format
//! - XML episode format
//! - XML tvshow format
//! - Plain text with IMDB/TMDB IDs

mod parser;
mod dto;

pub use parser::NfoParser;
pub use dto::NfoMetadata;
