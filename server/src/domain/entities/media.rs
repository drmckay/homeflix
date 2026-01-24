//! Media entity
//!
//! Represents a piece of media content (movie or episode)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::value_objects::{MediaType, ConfidenceScore, VerificationStatus};

/// Media entity - represents a movie or TV episode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Media {
    /// Unique identifier (None for new entities)
    pub id: Option<i64>,
    /// File system path to the media file
    pub file_path: String,
    /// Media type (movie or episode)
    pub media_type: MediaType,
    /// Title of the media
    pub title: String,
    /// Short description/overview
    pub overview: Option<String>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Backdrop image URL
    pub backdrop_url: Option<String>,
    /// Trailer URL
    pub trailer_url: Option<String>,
    /// Duration in seconds
    pub duration_seconds: Option<i32>,
    /// Release date
    pub release_date: Option<String>,
    /// Resolution (e.g., "1080p", "4K")
    pub resolution: Option<String>,
    /// Genres (comma-separated)
    pub genres: Option<String>,
    /// Series ID (for episodes)
    pub series_id: Option<i64>,
    /// Season number (for episodes)
    pub season: Option<i32>,
    /// Episode number (for episodes)
    pub episode: Option<i32>,
    /// Episode end number (for multi-episode files like S01E01E02)
    pub episode_end: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Original title (for non-English content)
    pub original_title: Option<String>,
    /// Rating (0.0 to 10.0)
    pub rating: Option<f32>,
    /// Confidence score (0.0 to 1.0)
    pub confidence_score: ConfidenceScore,
    /// Verification status
    pub verification_status: VerificationStatus,
    /// Strategy used for identification
    pub identification_strategy: Option<String>,
    /// Notes about any errors
    pub error_notes: Option<String>,
    /// Alternative matches (JSON string)
    pub alternative_matches: Option<String>,
    /// Content rating (e.g., "PG-13", "R")
    pub content_rating: Option<String>,
    /// Content warnings (e.g., "violence, language")
    pub content_warnings: Option<String>,
    /// Current playback position in seconds
    pub current_position: i64,
    /// Whether the media has been watched
    pub is_watched: bool,
    /// When this media was created in the database
    pub created_at: DateTime<Utc>,
    /// When this media was last updated
    pub updated_at: DateTime<Utc>,
}

impl Media {
    /// Creates a new media entity
    ///
    /// # Errors
    /// Returns error if file_path or title is empty
    pub fn new(
        file_path: String,
        media_type: MediaType,
        title: String,
    ) -> Result<Self, crate::shared::error::DomainError> {
        if file_path.is_empty() {
            return Err(crate::shared::error::DomainError::InvalidInput(
                "File path cannot be empty".into(),
            ));
        }
        if title.is_empty() {
            return Err(crate::shared::error::DomainError::InvalidInput(
                "Title cannot be empty".into(),
            ));
        }

        Ok(Self {
            id: None,
            file_path,
            media_type,
            title,
            overview: None,
            poster_url: None,
            backdrop_url: None,
            trailer_url: None,
            duration_seconds: None,
            release_date: None,
            resolution: None,
            genres: None,
            series_id: None,
            season: None,
            episode: None,
            episode_end: None,
            tmdb_id: None,
            original_title: None,
            rating: None,
            confidence_score: ConfidenceScore::default(),
            verification_status: VerificationStatus::Unverified,
            identification_strategy: None,
            error_notes: None,
            alternative_matches: None,
            content_rating: None,
            content_warnings: None,
            current_position: 0,
            is_watched: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Checks if this is a movie
    pub fn is_movie(&self) -> bool {
        self.media_type.is_movie()
    }

    /// Checks if this is an episode
    pub fn is_episode(&self) -> bool {
        self.media_type.is_episode()
    }

    /// Marks this media as verified
    pub fn mark_verified(&mut self) {
        self.verification_status = VerificationStatus::Verified;
        self.updated_at = Utc::now();
    }

    /// Marks this media as failed
    pub fn mark_failed(&mut self, reason: String) {
        self.verification_status = VerificationStatus::Failed;
        self.error_notes = Some(reason);
        self.updated_at = Utc::now();
    }

    /// Updates the confidence score and adjusts verification status
    pub fn update_confidence(&mut self, score: ConfidenceScore) {
        self.confidence_score = score;
        if score.is_high() {
            self.verification_status = VerificationStatus::Verified;
        } else if score.is_medium() {
            self.verification_status = VerificationStatus::Unverified;
        } else {
            self.verification_status = VerificationStatus::Failed;
        }
        self.updated_at = Utc::now();
    }

    /// Sets the series ID
    pub fn with_series_id(mut self, series_id: Option<i64>) -> Self {
        self.series_id = series_id;
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

    /// Sets the episode end number (for multi-episode files)
    pub fn with_episode_end(mut self, episode_end: Option<i32>) -> Self {
        self.episode_end = episode_end;
        self
    }

    /// Sets the TMDB ID
    pub fn with_tmdb_id(mut self, tmdb_id: Option<i64>) -> Self {
        self.tmdb_id = tmdb_id;
        self
    }

    /// Sets the overview
    pub fn with_overview(mut self, overview: Option<String>) -> Self {
        self.overview = overview;
        self
    }

    /// Sets the poster URL
    pub fn with_poster_url(mut self, poster_url: Option<String>) -> Self {
        self.poster_url = poster_url;
        self
    }

    /// Sets the backdrop URL
    pub fn with_backdrop_url(mut self, backdrop_url: Option<String>) -> Self {
        self.backdrop_url = backdrop_url;
        self
    }

    /// Sets the duration
    pub fn with_duration(mut self, duration_seconds: Option<i32>) -> Self {
        self.duration_seconds = duration_seconds;
        self
    }

    /// Sets the resolution
    pub fn with_resolution(mut self, resolution: Option<String>) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the release date
    pub fn with_release_date(mut self, release_date: Option<String>) -> Self {
        self.release_date = release_date;
        self
    }

    /// Sets the genres
    pub fn with_genres(mut self, genres: Option<String>) -> Self {
        self.genres = genres;
        self
    }

    /// Sets the rating
    pub fn with_rating(mut self, rating: Option<f32>) -> Self {
        self.rating = rating;
        self
    }

    /// Sets the content rating
    pub fn with_content_rating(mut self, content_rating: Option<String>) -> Self {
        self.content_rating = content_rating;
        self
    }

    /// Sets the content warnings
    pub fn with_content_warnings(mut self, content_warnings: Option<String>) -> Self {
        self.content_warnings = content_warnings;
        self
    }

    /// Updates the current playback position
    pub fn update_progress(&mut self, position: i64, watched: bool) {
        self.current_position = position;
        self.is_watched = watched;
        self.updated_at = Utc::now();
    }

    /// Gets the display title (original title if available, otherwise title)
    pub fn display_title(&self) -> &str {
        self.original_title.as_ref().unwrap_or(&self.title)
    }
}
