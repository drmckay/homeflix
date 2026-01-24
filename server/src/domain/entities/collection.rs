//! Collection entity
//!
//! Represents a collection of media items and their items

use serde::{Deserialize, Serialize};

/// Collection item entity - represents an item within a collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionItem {
    /// Unique identifier
    pub id: i64,
    /// Collection ID this item belongs to
    pub collection_id: i64,
    /// TMDB ID of the media
    pub tmdb_id: i64,
    /// Media type (movie or tv)
    pub media_type: String,
    /// Title of the media
    pub title: String,
    /// Overview/description
    pub overview: Option<String>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Release date
    pub release_date: Option<String>,
    /// Order in the timeline (in-universe chronology)
    pub timeline_order: i32,
    /// Order by release date
    pub release_order: i32,
    /// Timeline year (when story takes place)
    pub timeline_year: Option<i32>,
    /// Notes about timeline placement
    pub timeline_notes: Option<String>,
    /// Whether the media is available in the library
    pub is_available: bool,
    /// ID of the media item in library (if available)
    pub media_id: Option<i64>,
}

/// Collection entity - represents a collection of media items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Collection {
    /// Unique identifier (None for new entities)
    pub id: Option<i64>,
    /// Name of the collection
    pub name: String,
    /// Description of the collection
    pub description: Option<String>,
    /// Poster image URL
    pub poster_url: Option<String>,
    /// Backdrop image URL
    pub backdrop_url: Option<String>,
    /// TMDB collection ID
    pub tmdb_collection_id: Option<i64>,
    /// Sort mode ("timeline" or "release")
    pub sort_mode: String,
    /// Collection type ("auto", "preset", "custom")
    pub collection_type: String,
    /// Total number of items in the collection
    pub total_items: i32,
    /// Number of items available in the library
    pub available_items: i32,
}

impl Collection {
    /// Creates a new collection entity
    ///
    /// # Errors
    /// Returns error if name is empty
    pub fn new(name: String) -> Result<Self, crate::shared::error::DomainError> {
        if name.is_empty() {
            return Err(crate::shared::error::DomainError::InvalidInput(
                "Collection name cannot be empty".into(),
            ));
        }

        Ok(Self {
            id: None,
            name,
            description: None,
            poster_url: None,
            backdrop_url: None,
            tmdb_collection_id: None,
            sort_mode: "timeline".to_string(),
            collection_type: "auto".to_string(),
            total_items: 0,
            available_items: 0,
        })
    }

    /// Sets the description
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
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

    /// Sets the TMDB collection ID
    pub fn with_tmdb_collection_id(mut self, tmdb_collection_id: Option<i64>) -> Self {
        self.tmdb_collection_id = tmdb_collection_id;
        self
    }

    /// Sets the sort mode
    pub fn with_sort_mode(mut self, sort_mode: String) -> Self {
        self.sort_mode = sort_mode;
        self
    }

    /// Sets the collection type
    pub fn with_collection_type(mut self, collection_type: String) -> Self {
        self.collection_type = collection_type;
        self
    }

    /// Updates the item counts
    pub fn update_counts(&mut self, total: i32, available: i32) {
        self.total_items = total;
        self.available_items = available;
    }

    /// Checks if this is an auto-generated collection
    pub fn is_auto(&self) -> bool {
        self.collection_type == "auto"
    }

    /// Checks if this is a preset collection
    pub fn is_preset(&self) -> bool {
        self.collection_type == "preset"
    }

    /// Checks if this is a custom collection
    pub fn is_custom(&self) -> bool {
        self.collection_type == "custom"
    }

    /// Checks if this is sorted by timeline
    pub fn is_timeline_sorted(&self) -> bool {
        self.sort_mode == "timeline"
    }

    /// Checks if this is sorted by release date
    pub fn is_release_sorted(&self) -> bool {
        self.sort_mode == "release"
    }

    /// Calculates the completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.total_items == 0 {
            return 0.0;
        }
        (self.available_items as f32 / self.total_items as f32) * 100.0
    }
}
