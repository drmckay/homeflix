//! CollectionRepository trait
//!
//! Repository interface for collection data access

use async_trait::async_trait;
use crate::domain::entities::{Collection, CollectionItem};

/// Repository for collection data access
#[async_trait]
pub trait CollectionRepository: Send + Sync {
    /// Finds collection by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Collection>, crate::shared::error::RepositoryError>;

    /// Finds collection by name
    async fn find_by_name(&self, name: &str) -> Result<Option<Collection>, crate::shared::error::RepositoryError>;

    /// Finds collection by TMDB collection ID
    async fn find_by_tmdb_id(&self, tmdb_id: i64) -> Result<Option<Collection>, crate::shared::error::RepositoryError>;

    /// Finds all collections
    async fn find_all(&self) -> Result<Vec<Collection>, crate::shared::error::RepositoryError>;

    /// Finds collections by type
    async fn find_by_type(&self, collection_type: &str) -> Result<Vec<Collection>, crate::shared::error::RepositoryError>;

    /// Saves collection (returns new ID)
    async fn save(&self, collection: &Collection) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Updates collection
    async fn update(&self, collection: &Collection) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes collection by ID
    async fn delete(&self, id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Counts total collections
    async fn count(&self) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Checks if collection exists by name
    async fn exists_by_name(&self, name: &str) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Checks if collection exists by TMDB ID
    async fn exists_by_tmdb_id(&self, tmdb_id: i64) -> Result<bool, crate::shared::error::RepositoryError>;

    /// Finds auto-generated collections
    async fn find_auto(&self) -> Result<Vec<Collection>, crate::shared::error::RepositoryError>;

    /// Finds preset collections
    async fn find_preset(&self) -> Result<Vec<Collection>, crate::shared::error::RepositoryError>;

    /// Finds custom collections
    async fn find_custom(&self) -> Result<Vec<Collection>, crate::shared::error::RepositoryError>;

    /// Updates collection item counts
    async fn update_counts(&self, id: i64, total: i32, available: i32) -> Result<(), crate::shared::error::RepositoryError>;

    /// Finds all items in a collection
    async fn find_items(&self, collection_id: i64) -> Result<Vec<CollectionItem>, crate::shared::error::RepositoryError>;

    /// Saves a collection item
    async fn save_item(&self, item: &CollectionItem) -> Result<i64, crate::shared::error::RepositoryError>;

    /// Saves multiple collection items
    async fn save_items(&self, items: &[CollectionItem]) -> Result<usize, crate::shared::error::RepositoryError>;

    /// Updates a collection item (e.g., to link media_id)
    async fn update_item(&self, item: &CollectionItem) -> Result<(), crate::shared::error::RepositoryError>;

    /// Deletes all items in a collection
    async fn delete_items(&self, collection_id: i64) -> Result<(), crate::shared::error::RepositoryError>;

    /// Finds collection item by collection_id and tmdb_id
    async fn find_item_by_tmdb(&self, collection_id: i64, tmdb_id: i64, media_type: &str) -> Result<Option<CollectionItem>, crate::shared::error::RepositoryError>;
}
