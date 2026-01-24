//! Collection Management Events
//!
//! Events emitted during collection creation and updates

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a collection is created
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionCreatedEvent {
    /// Collection ID
    pub collection_id: i64,
    /// Collection name
    pub name: String,
    /// TMDB collection ID (if applicable)
    pub tmdb_collection_id: Option<i64>,
    /// Collection type (auto, preset, manual)
    pub collection_type: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl CollectionCreatedEvent {
    /// Creates a new collection created event
    pub fn new(
        collection_id: i64,
        name: String,
        tmdb_collection_id: Option<i64>,
        collection_type: String,
    ) -> Self {
        Self {
            collection_id,
            name,
            tmdb_collection_id,
            collection_type,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for CollectionCreatedEvent {
    fn event_type(&self) -> &'static str {
        "collection_created"
    }
}

/// Event emitted when a collection is updated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionUpdatedEvent {
    /// Collection ID
    pub collection_id: i64,
    /// Collection name
    pub name: String,
    /// Total items in collection
    pub total_items: i32,
    /// Available items in collection
    pub available_items: i32,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl CollectionUpdatedEvent {
    /// Creates a new collection updated event
    pub fn new(
        collection_id: i64,
        name: String,
        total_items: i32,
        available_items: i32,
    ) -> Self {
        Self {
            collection_id,
            name,
            total_items,
            available_items,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for CollectionUpdatedEvent {
    fn event_type(&self) -> &'static str {
        "collection_updated"
    }
}

/// Event emitted when an item is added to a collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionItemAddedEvent {
    /// Collection ID
    pub collection_id: i64,
    /// Media ID (if applicable)
    pub media_id: Option<i64>,
    /// TMDB ID
    pub tmdb_id: i64,
    /// Media type (movie, episode)
    pub media_type: String,
    /// Title
    pub title: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl CollectionItemAddedEvent {
    /// Creates a new collection item added event
    pub fn new(
        collection_id: i64,
        media_id: Option<i64>,
        tmdb_id: i64,
        media_type: String,
        title: String,
    ) -> Self {
        Self {
            collection_id,
            media_id,
            tmdb_id,
            media_type,
            title,
            timestamp: Utc::now(),
        }
    }
}

impl crate::interfaces::messaging::DomainEvent for CollectionItemAddedEvent {
    fn event_type(&self) -> &'static str {
        "collection_item_added"
    }
}
