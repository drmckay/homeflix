//! Series entity
//!
//! Represents a TV series

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::domain::value_objects::{ConfidenceScore, VerificationStatus};

/// Series entity - represents a TV series
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Series {
    /// Unique identifier (None for new entities)
    pub id: Option<i64>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Title of the series
    pub title: String,
    /// Short description/overview
    pub overview: Option<String>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Backdrop image URL
    pub backdrop_url: Option<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence_score: ConfidenceScore,
    /// Verification status
    pub verification_status: VerificationStatus,
    /// First air date
    pub first_air_date: Option<String>,
    /// Last air date
    pub last_air_date: Option<String>,
    /// Series status (e.g., "Returning Series", "Ended")
    pub status: Option<String>,
    /// Total number of seasons
    pub total_seasons: Option<i32>,
    /// Total number of episodes
    pub total_episodes: Option<i32>,
    /// Original title (for non-English content)
    pub original_title: Option<String>,
    /// Genres (comma-separated)
    pub genres: Option<String>,
    /// Rating (0.0 to 10.0)
    pub rating: Option<f32>,
    /// Alternative matches (JSON string)
    pub alternative_matches: Option<String>,
    /// Notes about any errors
    pub error_notes: Option<String>,
    /// When this series was last verified
    pub last_verified: Option<DateTime<Utc>>,
    /// When this series was created in the database
    pub created_at: DateTime<Utc>,
    /// When this series was last updated
    pub updated_at: DateTime<Utc>,
}

impl Series {
    /// Creates a new series entity
    ///
    /// # Errors
    /// Returns error if title is empty
    pub fn new(title: String) -> Result<Self, crate::shared::error::DomainError> {
        if title.is_empty() {
            return Err(crate::shared::error::DomainError::InvalidInput(
                "Title cannot be empty".into(),
            ));
        }

        Ok(Self {
            id: None,
            tmdb_id: None,
            title,
            overview: None,
            poster_url: None,
            backdrop_url: None,
            confidence_score: ConfidenceScore::default(),
            verification_status: VerificationStatus::Unverified,
            first_air_date: None,
            last_air_date: None,
            status: None,
            total_seasons: None,
            total_episodes: None,
            original_title: None,
            genres: None,
            rating: None,
            alternative_matches: None,
            error_notes: None,
            last_verified: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Marks this series as verified
    pub fn mark_verified(&mut self) {
        self.verification_status = VerificationStatus::Verified;
        self.last_verified = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Marks this series as failed
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

    /// Sets the first air date
    pub fn with_first_air_date(mut self, first_air_date: Option<String>) -> Self {
        self.first_air_date = first_air_date;
        self
    }

    /// Sets the last air date
    pub fn with_last_air_date(mut self, last_air_date: Option<String>) -> Self {
        self.last_air_date = last_air_date;
        self
    }

    /// Sets the status
    pub fn with_status(mut self, status: Option<String>) -> Self {
        self.status = status;
        self
    }

    /// Sets the total seasons
    pub fn with_total_seasons(mut self, total_seasons: Option<i32>) -> Self {
        self.total_seasons = total_seasons;
        self
    }

    /// Sets the total episodes
    pub fn with_total_episodes(mut self, total_episodes: Option<i32>) -> Self {
        self.total_episodes = total_episodes;
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

    /// Gets the display title (original title if available, otherwise title)
    pub fn display_title(&self) -> &str {
        self.original_title.as_ref().unwrap_or(&self.title)
    }

    /// Checks if the series is currently airing
    pub fn is_currently_airing(&self) -> bool {
        matches!(self.status.as_deref(), Some("Returning Series") | Some("Planned"))
    }

    /// Checks if the series has ended
    pub fn has_ended(&self) -> bool {
        matches!(self.status.as_deref(), Some("Ended") | Some("Canceled"))
    }
}
