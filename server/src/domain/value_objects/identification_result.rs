//! IdentificationResult value object
//!
//! Represents the result of identifying media content from a file

use serde::{Deserialize, Serialize};
use crate::domain::value_objects::{MediaType, MatchStrategy};

/// Result of media identification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentificationResult {
    /// Identified media type
    pub media_type: MediaType,
    /// Cleaned title
    pub title: String,
    /// Extracted year (if found)
    pub year: Option<i32>,
    /// Season number (for episodes)
    pub season: Option<i32>,
    /// Episode number (for episodes)
    pub episode: Option<i32>,
    /// Multiple episodes (for multi-part files like S01E01-E03)
    pub multi_episode: Option<Vec<i32>>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Strategy used for identification
    pub strategy: MatchStrategy,
    /// TMDB ID (if resolved)
    pub tmdb_id: Option<i64>,
    /// IMDB ID (if found)
    pub imdb_id: Option<String>,
    /// Series name (extracted from folder/filename)
    pub series_name: Option<String>,
    /// Alternative matches (lower confidence)
    pub alternative_matches: Vec<AlternativeMatch>,
}

/// Alternative match with lower confidence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlternativeMatch {
    /// TMDB ID
    pub tmdb_id: i64,
    /// Title
    pub title: String,
    /// Year
    pub year: Option<i32>,
    /// Confidence score
    pub confidence: f32,
}

impl IdentificationResult {
    /// Creates a new identification result
    pub fn new(
        media_type: MediaType,
        title: String,
        strategy: MatchStrategy,
    ) -> Self {
        Self {
            media_type,
            title,
            year: None,
            season: None,
            episode: None,
            multi_episode: None,
            confidence: strategy.confidence_weight(),
            strategy,
            tmdb_id: None,
            imdb_id: None,
            series_name: None,
            alternative_matches: Vec::new(),
        }
    }

    /// Sets the year
    pub fn with_year(mut self, year: Option<i32>) -> Self {
        self.year = year;
        self
    }

    /// Sets the season number
    pub fn with_season(mut self, season: Option<i32>) -> Self {
        self.season = season;
        self
    }

    /// Sets the episode number
    pub fn with_episode(mut self, episode: Option<i32>) -> Self {
        self.episode = episode;
        self
    }

    /// Sets multiple episodes (for multi-part files)
    pub fn with_multi_episode(mut self, episodes: Vec<i32>) -> Self {
        self.multi_episode = Some(episodes);
        self
    }

    /// Sets the series name
    pub fn with_series_name(mut self, name: Option<String>) -> Self {
        self.series_name = name;
        self
    }

    /// Sets the TMDB ID
    pub fn with_tmdb_id(mut self, tmdb_id: Option<i64>) -> Self {
        self.tmdb_id = tmdb_id;
        self
    }

    /// Sets the IMDB ID
    pub fn with_imdb_id(mut self, imdb_id: Option<String>) -> Self {
        self.imdb_id = imdb_id;
        self
    }

    /// Sets the confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Adds an alternative match
    pub fn add_alternative_match(mut self, match_result: AlternativeMatch) -> Self {
        self.alternative_matches.push(match_result);
        self
    }

    /// Checks if this is a movie
    pub fn is_movie(&self) -> bool {
        self.media_type.is_movie()
    }

    /// Checks if this is an episode
    pub fn is_episode(&self) -> bool {
        self.media_type.is_episode()
    }

    /// Checks if confidence is high
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.85
    }

    /// Checks if confidence is low
    pub fn is_low_confidence(&self) -> bool {
        self.confidence < 0.70
    }
}

impl AlternativeMatch {
    /// Creates a new alternative match
    pub fn new(tmdb_id: i64, title: String, confidence: f32) -> Self {
        Self {
            tmdb_id,
            title,
            year: None,
            confidence,
        }
    }

    /// Sets the year
    pub fn with_year(mut self, year: Option<i32>) -> Self {
        self.year = year;
        self
    }
}
