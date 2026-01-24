//! Preset Franchise Collections
//!
//! Manually curated timelines for popular franchises that mix movies and TV series.

use serde::{Deserialize, Serialize};
use crate::shared::error::DomainError;

/// A single item in a preset collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCollectionItem {
    /// TMDB ID
    pub tmdb_id: i64,
    /// Media type ("movie" or "tv")
    pub media_type: String,
    /// Title
    pub title: String,
    /// Order in timeline (1-based)
    pub timeline_order: i32,
    /// In-universe year (optional)
    pub timeline_year: Option<i32>,
    /// Additional timeline notes
    pub timeline_notes: Option<String>,
    /// For TV series - which seasons to include (start, end) inclusive
    pub season_range: Option<SeasonRange>,
}

/// Season range for TV series (start, end) inclusive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonRange {
    pub start: i32,
    pub end: i32,
}

impl SeasonRange {
    /// Convert to tuple for backward compatibility
    pub fn as_tuple(&self) -> (i32, i32) {
        (self.start, self.end)
    }
}

impl PresetCollectionItem {
    /// Get season range as tuple (for backward compatibility)
    pub fn season_range_tuple(&self) -> Option<(i32, i32)> {
        self.season_range.as_ref().map(|r| r.as_tuple())
    }
}

/// A preset collection definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCollection {
    /// Collection name (fallback if TMDB not available)
    pub name: String,
    /// Description
    pub description: String,
    /// Optional TMDB collection ID to fetch name/images from
    pub tmdb_collection_id: Option<i64>,
    /// Items in timeline order
    pub items: Vec<PresetCollectionItem>,
}

impl PresetCollection {
    /// Get all TMDB movie IDs in this collection
    pub fn movie_tmdb_ids(&self) -> Vec<i64> {
        self.items
            .iter()
            .filter(|i| i.media_type == "movie")
            .map(|i| i.tmdb_id)
            .collect()
    }

    /// Get all TMDB TV show IDs in this collection
    pub fn tv_tmdb_ids(&self) -> Vec<i64> {
        self.items
            .iter()
            .filter(|i| i.media_type == "tv")
            .map(|i| i.tmdb_id)
            .collect()
    }

    /// Validate the preset collection
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.is_empty() {
            return Err(DomainError::ValidationError(
                "Preset collection name cannot be empty".to_string(),
            ));
        }

        if self.items.is_empty() {
            return Err(DomainError::ValidationError(
                format!("Preset collection '{}' has no items", self.name),
            ));
        }

        for (idx, item) in self.items.iter().enumerate() {
            // Validate media type
            if item.media_type != "movie" && item.media_type != "tv" {
                return Err(DomainError::ValidationError(
                    format!(
                        "Preset collection '{}', item {}: invalid media_type '{}', must be 'movie' or 'tv'",
                        self.name, idx, item.media_type
                    ),
                ));
            }

            // Validate TMDB ID
            if item.tmdb_id <= 0 {
                return Err(DomainError::ValidationError(
                    format!(
                        "Preset collection '{}', item {}: tmdb_id must be positive, got {}",
                        self.name, idx, item.tmdb_id
                    ),
                ));
            }

            // Validate timeline_order
            if item.timeline_order <= 0 {
                return Err(DomainError::ValidationError(
                    format!(
                        "Preset collection '{}', item {}: timeline_order must be positive, got {}",
                        self.name, idx, item.timeline_order
                    ),
                ));
            }

            // Validate season_range if present
            if let Some(ref range) = item.season_range {
                if range.start <= 0 || range.end <= 0 {
                    return Err(DomainError::ValidationError(
                        format!(
                            "Preset collection '{}', item {}: season_range start and end must be positive",
                            self.name, idx
                        ),
                    ));
                }
                if range.start > range.end {
                    return Err(DomainError::ValidationError(
                        format!(
                            "Preset collection '{}', item {}: season_range start ({}) must be <= end ({})",
                            self.name, idx, range.start, range.end
                        ),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Get all preset franchise collections (deprecated - use PresetLoader instead)
#[deprecated(note = "Use PresetLoader to load presets from YAML files")]
pub fn get_all_presets() -> Vec<PresetCollection> {
    // This function is kept for backward compatibility during migration
    // It will be removed once all code uses PresetLoader
    vec![]
}

// Note: The old static presets (STAR_TREK_TIMELINE, STARGATE_TIMELINE, MCU_TIMELINE) 
// have been removed. They are now loaded from YAML files in the presets directory.
// See server/presets/*.yaml for the preset definitions.
