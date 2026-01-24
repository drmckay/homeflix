//! Collection DTOs
//!
//! Data Transfer Objects for Collection-related operations

use serde::{Deserialize, Serialize};
use crate::domain::entities::Collection;

/// Collection response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionResponse {
    /// Collection ID
    pub id: i64,
    /// Name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Poster URL
    pub poster_url: Option<String>,
    /// Backdrop URL
    pub backdrop_url: Option<String>,
    /// TMDB Collection ID
    pub tmdb_collection_id: Option<i64>,
    /// Sort mode
    pub sort_mode: String,
    /// Collection type
    pub collection_type: String,
    /// Total items
    pub total_items: i32,
    /// Available items
    pub available_items: i32,
    /// Completion percentage
    pub completion_percentage: f32,
}

impl From<Collection> for CollectionResponse {
    fn from(collection: Collection) -> Self {
        let completion_percentage = collection.completion_percentage();
        Self {
            id: collection.id.unwrap_or(0),
            name: collection.name,
            description: collection.description,
            poster_url: collection.poster_url,
            backdrop_url: collection.backdrop_url,
            tmdb_collection_id: collection.tmdb_collection_id,
            sort_mode: collection.sort_mode,
            collection_type: collection.collection_type,
            total_items: collection.total_items,
            available_items: collection.available_items,
            completion_percentage,
        }
    }
}
