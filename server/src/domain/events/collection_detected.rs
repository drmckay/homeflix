//! CollectionDetected event
//!
//! Emitted when a new collection is detected

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event emitted when a new collection is detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionDetectedEvent {
    /// Collection ID
    pub collection_id: i64,
    /// Collection name
    pub collection_name: String,
    /// TMDB collection ID
    pub tmdb_id: i64,
    /// Number of items in collection
    pub item_count: usize,
    /// Collection type ("auto", "preset", "custom")
    pub collection_type: String,
    /// Timestamp of event
    pub timestamp: DateTime<Utc>,
}

impl CollectionDetectedEvent {
    /// Creates a new collection detected event
    pub fn new(
        collection_id: i64,
        collection_name: String,
        tmdb_id: i64,
        item_count: usize,
        collection_type: String,
    ) -> Self {
        Self {
            collection_id,
            collection_name,
            tmdb_id,
            item_count,
            collection_type,
            timestamp: Utc::now(),
        }
    }

    /// Gets the event type name
    pub fn event_type() -> &'static str {
        "collection_detected"
    }
}

impl crate::interfaces::messaging::DomainEvent for CollectionDetectedEvent {
    fn event_type(&self) -> &'static str {
        Self::event_type()
    }
}
