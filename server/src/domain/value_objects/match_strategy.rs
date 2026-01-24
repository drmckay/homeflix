//! MatchStrategy value object
//!
//! Represents the strategy used to identify media content

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Identification strategy used for matching media
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStrategy {
    /// Match by IMDB ID from filename or NFO
    ImdbId,
    /// Match by TMDB ID from NFO file
    TmdbId,
    /// Match by filename with year
    FilenameWithYear,
    /// Match by folder name with year
    FolderWithYear,
    /// Match by filename only (year-agnostic)
    FilenameOnly,
    /// Match by alternative title (with articles removed)
    AlternativeTitle,
    /// Match by NFO file metadata
    NfoMetadata,
    /// Match by fuzzy search
    FuzzySearch,
    /// Match by manual user input
    Manual,
}

impl MatchStrategy {
    /// Returns string representation of match strategy
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchStrategy::ImdbId => "imdb_id",
            MatchStrategy::TmdbId => "tmdb_id",
            MatchStrategy::FilenameWithYear => "filename_with_year",
            MatchStrategy::FolderWithYear => "folder_with_year",
            MatchStrategy::FilenameOnly => "filename_only",
            MatchStrategy::AlternativeTitle => "alternative_title",
            MatchStrategy::NfoMetadata => "nfo_metadata",
            MatchStrategy::FuzzySearch => "fuzzy_search",
            MatchStrategy::Manual => "manual",
        }
    }

    /// Returns the confidence weight for this strategy
    /// Higher weight = higher confidence when match succeeds
    pub fn confidence_weight(&self) -> f32 {
        match self {
            MatchStrategy::ImdbId => 0.95,      // Highest confidence
            MatchStrategy::TmdbId => 0.90,      // Very high
            MatchStrategy::NfoMetadata => 0.85,    // High
            MatchStrategy::FilenameWithYear => 0.75, // Medium-high
            MatchStrategy::FolderWithYear => 0.70,  // Medium
            MatchStrategy::FilenameOnly => 0.60,    // Medium-low
            MatchStrategy::AlternativeTitle => 0.55,  // Low
            MatchStrategy::FuzzySearch => 0.50,       // Very low
            MatchStrategy::Manual => 1.00,           // User is always right
        }
    }
}

impl fmt::Display for MatchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MatchStrategy {
    type Err = crate::shared::error::DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "imdb_id" => Ok(MatchStrategy::ImdbId),
            "tmdb_id" => Ok(MatchStrategy::TmdbId),
            "filename_with_year" => Ok(MatchStrategy::FilenameWithYear),
            "folder_with_year" => Ok(MatchStrategy::FolderWithYear),
            "filename_only" => Ok(MatchStrategy::FilenameOnly),
            "alternative_title" => Ok(MatchStrategy::AlternativeTitle),
            "nfo_metadata" => Ok(MatchStrategy::NfoMetadata),
            "fuzzy_search" => Ok(MatchStrategy::FuzzySearch),
            "manual" => Ok(MatchStrategy::Manual),
            _ => Err(crate::shared::error::DomainError::InvalidInput(format!(
                "Invalid match strategy: {}",
                s
            ))),
        }
    }
}
