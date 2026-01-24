//! Season entity
//!
//! Represents a season of a TV series

use serde::{Deserialize, Serialize};

/// Season entity - represents a season of a TV series
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Season {
    /// Unique identifier (None for new entities)
    pub id: Option<i64>,
    /// Series ID
    pub series_id: i64,
    /// Season number
    pub season_number: i32,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Name of the season
    pub name: Option<String>,
    /// Short description/overview
    pub overview: Option<String>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Air date
    pub air_date: Option<String>,
    /// Number of episodes
    pub episode_count: Option<i32>,
    /// Rating (0.0 to 10.0)
    pub rating: Option<f32>,
}

impl Season {
    /// Creates a new season entity
    pub fn new(series_id: i64, season_number: i32) -> Self {
        Self {
            id: None,
            series_id,
            season_number,
            tmdb_id: None,
            name: None,
            overview: None,
            poster_url: None,
            air_date: None,
            episode_count: None,
            rating: None,
        }
    }

    /// Sets the TMDB ID
    pub fn with_tmdb_id(mut self, tmdb_id: Option<i64>) -> Self {
        self.tmdb_id = tmdb_id;
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

    /// Sets the poster URL
    pub fn with_poster_url(mut self, poster_url: Option<String>) -> Self {
        self.poster_url = poster_url;
        self
    }

    /// Sets the air date
    pub fn with_air_date(mut self, air_date: Option<String>) -> Self {
        self.air_date = air_date;
        self
    }

    /// Sets the episode count
    pub fn with_episode_count(mut self, episode_count: Option<i32>) -> Self {
        self.episode_count = episode_count;
        self
    }

    /// Sets the rating
    pub fn with_rating(mut self, rating: Option<f32>) -> Self {
        self.rating = rating;
        self
    }

    /// Checks if this is a special season (usually 0)
    pub fn is_special(&self) -> bool {
        self.season_number == 0
    }

    /// Gets the display name
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            if !name.is_empty() {
                return name.clone();
            }
        }
        if self.is_special() {
            "Specials".to_string()
        } else {
            format!("Season {}", self.season_number)
        }
    }
}
