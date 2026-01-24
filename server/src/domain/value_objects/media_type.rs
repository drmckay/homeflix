//! MediaType value object
//!
//! Represents the type of media content (movie, episode, etc.)

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Media type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    /// Full-length movie
    Movie,
    /// TV series episode
    Episode,
    /// Unknown type (not yet identified)
    Unknown,
}

impl MediaType {
    /// Returns the string representation of the media type
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Movie => "movie",
            MediaType::Episode => "episode",
            MediaType::Unknown => "unknown",
        }
    }

    /// Checks if this is a movie
    pub fn is_movie(&self) -> bool {
        matches!(self, MediaType::Movie)
    }

    /// Checks if this is an episode
    pub fn is_episode(&self) -> bool {
        matches!(self, MediaType::Episode)
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MediaType {
    type Err = crate::shared::error::DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "movie" => Ok(MediaType::Movie),
            "episode" => Ok(MediaType::Episode),
            "unknown" => Ok(MediaType::Unknown),
            _ => Err(crate::shared::error::DomainError::InvalidInput(format!(
                "Invalid media type: {}",
                s
            ))),
        }
    }
}
