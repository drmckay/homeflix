//! Episode entity
//!
//! Represents an episode of a TV series season

use serde::{Deserialize, Serialize};

/// Episode entity - represents an episode of a TV series
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Episode {
    /// Unique identifier (None for new entities)
    pub id: Option<i64>,
    /// Media ID (links to media table)
    pub media_id: Option<i64>,
    /// Series ID
    pub series_id: i64,
    /// Season number
    pub season_number: i32,
    /// Episode number
    pub episode_number: i32,
    /// Name of the episode
    pub name: Option<String>,
    /// Short description/overview
    pub overview: Option<String>,
    /// Air date
    pub air_date: Option<String>,
    /// Still image URL
    pub still_path: Option<String>,
    /// Rating (0.0 to 10.0)
    pub rating: Option<f32>,
    /// Runtime in minutes
    pub runtime: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
}

impl Episode {
    /// Creates a new episode entity
    pub fn new(series_id: i64, season_number: i32, episode_number: i32) -> Self {
        Self {
            id: None,
            media_id: None,
            series_id,
            season_number,
            episode_number,
            name: None,
            overview: None,
            air_date: None,
            still_path: None,
            rating: None,
            runtime: None,
            tmdb_id: None,
        }
    }

    /// Sets the media ID
    pub fn with_media_id(mut self, media_id: Option<i64>) -> Self {
        self.media_id = media_id;
        self
    }

    /// Sets the name
    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// Sets the overview
    pub fn with_overview(mut self, overview: Option<String>) -> Self {
        self.overview = overview;
        self
    }

    /// Sets the air date
    pub fn with_air_date(mut self, air_date: Option<String>) -> Self {
        self.air_date = air_date;
        self
    }

    /// Sets the still image path
    pub fn with_still_path(mut self, still_path: Option<String>) -> Self {
        self.still_path = still_path;
        self
    }

    /// Sets the rating
    pub fn with_rating(mut self, rating: Option<f32>) -> Self {
        self.rating = rating;
        self
    }

    /// Sets the runtime
    pub fn with_runtime(mut self, runtime: Option<i32>) -> Self {
        self.runtime = runtime;
        self
    }

    /// Sets the TMDB ID
    pub fn with_tmdb_id(mut self, tmdb_id: Option<i64>) -> Self {
        self.tmdb_id = tmdb_id;
        self
    }

    /// Gets the display name
    pub fn display_name(&self) -> Option<&String> {
        self.name.as_ref().filter(|n| !n.is_empty())
    }

    /// Checks if this episode has aired
    pub fn has_aired(&self) -> bool {
        if let Some(air_date) = &self.air_date {
            // Parse the date and check if it's in the past
            if let Ok(parsed_date) = chrono::DateTime::parse_from_rfc3339(air_date) {
                return parsed_date <= chrono::Utc::now();
            }
        }
        false
    }

    /// Gets the season and episode display string
    pub fn season_episode_display(&self) -> String {
        format!("S{:02}E{:02}", self.season_number, self.episode_number)
    }
}
