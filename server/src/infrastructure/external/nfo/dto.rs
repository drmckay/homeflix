//! NFO Data Transfer Objects
//!
//! Structures for NFO file metadata parsing

use serde::Deserialize;

/// Parsed NFO metadata from Kodi/XBMC format files
#[derive(Debug, Clone)]
pub struct NfoMetadata {
    /// Title from NFO
    pub title: Option<String>,
    /// Original title (for foreign films)
    pub original_title: Option<String>,
    /// Plot/synopsis
    pub plot: Option<String>,
    /// Release year
    pub year: Option<i32>,
    /// Duration in minutes
    pub duration_min: Option<i64>,
    /// IMDB ID (tt1234567)
    pub imdb_id: Option<String>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Genre list
    pub genres: Vec<String>,
    /// Season number (for episodes)
    pub season: Option<i32>,
    /// Episode number (for episodes)
    pub episode: Option<i32>,
    /// Whether this was parsed from valid XML
    pub is_xml: bool,
    /// Credibility score (0.0 to 1.0)
    pub credibility_score: f32,
    /// Extraction method ("xml_movie", "xml_episode", "xml_tvshow", "semantic")
    pub extraction_method: String,
}

impl Default for NfoMetadata {
    fn default() -> Self {
        Self {
            title: None,
            original_title: None,
            plot: None,
            year: None,
            duration_min: None,
            imdb_id: None,
            tmdb_id: None,
            genres: Vec::new(),
            season: None,
            episode: None,
            is_xml: false,
            credibility_score: 0.0,
            extraction_method: "unknown".to_string(),
        }
    }
}

impl NfoMetadata {
    /// Creates new empty NfoMetadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if this metadata has any useful identification info
    pub fn has_id(&self) -> bool {
        self.imdb_id.is_some() || self.tmdb_id.is_some()
    }

    /// Returns the best available ID for lookups
    pub fn best_id(&self) -> Option<String> {
        if let Some(ref imdb) = self.imdb_id {
            return Some(imdb.clone());
        }
        if let Some(tmdb) = self.tmdb_id {
            return Some(format!("tmdb:{}", tmdb));
        }
        None
    }
}

/// Internal: Movie NFO root element
#[derive(Debug, Deserialize)]
#[serde(rename = "movie")]
pub(crate) struct MovieNfoRoot {
    pub title: String,
    pub originaltitle: Option<String>,
    pub plot: Option<String>,
    pub runtime: Option<i64>,
    pub id: Option<String>, // Can be IMDB
    pub tmdbid: Option<i64>,
    pub year: Option<i32>,
    #[serde(default)]
    pub genre: Vec<String>,
}

/// Internal: Episode XML structure
#[derive(Debug, Deserialize)]
#[serde(rename = "episodedetails")]
pub(crate) struct EpisodeNfoRoot {
    pub title: String,
    pub plot: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub id: Option<String>, // IMDB
    pub tmdbid: Option<i64>,
    pub aired: Option<String>,
}

/// Internal: TV Show XML structure
#[derive(Debug, Deserialize)]
#[serde(rename = "tvshow")]
pub(crate) struct TvShowNfoRoot {
    pub title: String,
    pub plot: Option<String>,
    pub id: Option<String>,
    pub tmdbid: Option<i64>,
}
