//! SQLite Collection Repository Implementation
//!
//! Provides SQLite-based implementation of CollectionRepository trait

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use crate::domain::entities::{Collection, CollectionItem};
use crate::domain::repositories::CollectionRepository;
use crate::shared::error::RepositoryError;

/// SQLite implementation of CollectionRepository
pub struct SqliteCollectionRepository {
    pool: Pool<Sqlite>,
}

impl SqliteCollectionRepository {
    /// Creates a new SQLite collection repository
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Maps a database row to Collection entity
    fn map_row_to_collection(row: sqlx::sqlite::SqliteRow) -> Result<Collection, RepositoryError> {
        Ok(Collection {
            id: Some(row.try_get("id")?),
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            tmdb_collection_id: row.try_get("tmdb_collection_id")?,
            sort_mode: row.try_get("sort_mode")?,
            collection_type: row.try_get("collection_type")?,
            total_items: row.try_get("total_items")?,
            available_items: row.try_get("available_items")?,
        })
    }
}

#[async_trait]
impl CollectionRepository for SqliteCollectionRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Collection>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM collections WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_collection(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Collection>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM collections WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_collection(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_tmdb_id(&self, tmdb_id: i64) -> Result<Option<Collection>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM collections WHERE tmdb_collection_id = ?"
        )
        .bind(tmdb_id)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_collection(row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Collection>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM collections ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }

    async fn find_by_type(&self, collection_type: &str) -> Result<Vec<Collection>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM collections WHERE collection_type = ? ORDER BY name ASC"
        )
        .bind(collection_type)
        .fetch_all(&self.pool)
        .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }

    async fn save(&self, collection: &Collection) -> Result<i64, RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO collections (
                name, description, poster_url, backdrop_url, tmdb_collection_id,
                sort_mode, collection_type, total_items, available_items
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&collection.name)
        .bind(&collection.description)
        .bind(&collection.poster_url)
        .bind(&collection.backdrop_url)
        .bind(collection.tmdb_collection_id)
        .bind(&collection.sort_mode)
        .bind(&collection.collection_type)
        .bind(collection.total_items)
        .bind(collection.available_items)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, collection: &Collection) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE collections SET
                name = ?, description = ?, poster_url = ?, backdrop_url = ?,
                tmdb_collection_id = ?, sort_mode = ?, collection_type = ?,
                total_items = ?, available_items = ?
            WHERE id = ?"
        )
        .bind(&collection.name)
        .bind(&collection.description)
        .bind(&collection.poster_url)
        .bind(&collection.backdrop_url)
        .bind(collection.tmdb_collection_id)
        .bind(&collection.sort_mode)
        .bind(&collection.collection_type)
        .bind(collection.total_items)
        .bind(collection.available_items)
        .bind(collection.id.ok_or(RepositoryError::InvalidInput("Collection ID is required".into()))?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn count(&self) -> Result<i64, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM collections")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get("count")?)
    }

    async fn exists_by_name(&self, name: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM collections WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn exists_by_tmdb_id(&self, tmdb_id: i64) -> Result<bool, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM collections WHERE tmdb_collection_id = ?")
            .bind(tmdb_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn find_auto(&self) -> Result<Vec<Collection>, RepositoryError> {
        let collection_type = "auto";
        let rows = sqlx::query(
            "SELECT * FROM collections WHERE collection_type = ? ORDER BY name ASC"
        )
        .bind(collection_type)
        .fetch_all(&self.pool)
        .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }

    async fn find_preset(&self) -> Result<Vec<Collection>, RepositoryError> {
        let collection_type = "preset";
        let rows = sqlx::query(
            "SELECT * FROM collections WHERE collection_type = ? ORDER BY name ASC"
        )
        .bind(collection_type)
        .fetch_all(&self.pool)
        .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }

    async fn find_custom(&self) -> Result<Vec<Collection>, RepositoryError> {
        let collection_type = "custom";
        let rows = sqlx::query(
            "SELECT * FROM collections WHERE collection_type = ? ORDER BY name ASC"
        )
        .bind(collection_type)
        .fetch_all(&self.pool)
        .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }

    async fn update_counts(&self, id: i64, total: i32, available: i32) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE collections SET total_items = ?, available_items = ? WHERE id = ?"
        )
        .bind(total)
        .bind(available)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_items(&self, collection_id: i64) -> Result<Vec<CollectionItem>, RepositoryError> {
        let rows: Vec<(i64, i64, i64, String, String, Option<String>, Option<String>, Option<String>, i32, i32, Option<i32>, Option<String>, i32, Option<i64>)> =
            sqlx::query_as(
                r#"SELECT id, collection_id, tmdb_id, COALESCE(media_type, 'movie'), title, overview,
                          poster_url, release_date, timeline_order, COALESCE(release_order, timeline_order),
                          timeline_year, timeline_notes, COALESCE(is_available, 0), media_id
                   FROM collection_items
                   WHERE collection_id = ?
                   ORDER BY timeline_order"#
            )
            .bind(collection_id)
            .fetch_all(&self.pool)
            .await?;

        let items = rows
            .into_iter()
            .map(|(id, collection_id, tmdb_id, media_type, title, overview, poster_url, release_date, timeline_order, release_order, timeline_year, timeline_notes, is_available, media_id)| {
                CollectionItem {
                    id,
                    collection_id,
                    tmdb_id,
                    media_type,
                    title,
                    overview,
                    poster_url,
                    release_date,
                    timeline_order,
                    release_order,
                    timeline_year,
                    timeline_notes,
                    is_available: is_available != 0,
                    media_id,
                }
            })
            .collect();

        Ok(items)
    }

    async fn save_item(&self, item: &CollectionItem) -> Result<i64, RepositoryError> {
        let result = sqlx::query(
            r#"INSERT INTO collection_items (
                collection_id, tmdb_id, media_type, title, overview, poster_url,
                release_date, timeline_order, release_order, timeline_year, timeline_notes,
                is_available, media_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(item.collection_id)
        .bind(item.tmdb_id)
        .bind(&item.media_type)
        .bind(&item.title)
        .bind(&item.overview)
        .bind(&item.poster_url)
        .bind(&item.release_date)
        .bind(item.timeline_order)
        .bind(item.release_order)
        .bind(item.timeline_year)
        .bind(&item.timeline_notes)
        .bind(item.is_available)
        .bind(item.media_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn save_items(&self, items: &[CollectionItem]) -> Result<usize, RepositoryError> {
        let mut saved = 0;
        for item in items {
            self.save_item(item).await?;
            saved += 1;
        }
        Ok(saved)
    }

    async fn update_item(&self, item: &CollectionItem) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"UPDATE collection_items SET
                tmdb_id = ?, media_type = ?, title = ?, overview = ?, poster_url = ?,
                release_date = ?, timeline_order = ?, release_order = ?, timeline_year = ?,
                timeline_notes = ?, is_available = ?, media_id = ?
            WHERE id = ?"#
        )
        .bind(item.tmdb_id)
        .bind(&item.media_type)
        .bind(&item.title)
        .bind(&item.overview)
        .bind(&item.poster_url)
        .bind(&item.release_date)
        .bind(item.timeline_order)
        .bind(item.release_order)
        .bind(item.timeline_year)
        .bind(&item.timeline_notes)
        .bind(item.is_available)
        .bind(item.media_id)
        .bind(item.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_items(&self, collection_id: i64) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM collection_items WHERE collection_id = ?")
            .bind(collection_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_item_by_tmdb(&self, collection_id: i64, tmdb_id: i64, media_type: &str) -> Result<Option<CollectionItem>, RepositoryError> {
        let result: Option<(i64, i64, i64, String, String, Option<String>, Option<String>, Option<String>, i32, i32, Option<i32>, Option<String>, i32, Option<i64>)> =
            sqlx::query_as(
                r#"SELECT id, collection_id, tmdb_id, COALESCE(media_type, 'movie'), title, overview,
                          poster_url, release_date, timeline_order, COALESCE(release_order, timeline_order),
                          timeline_year, timeline_notes, COALESCE(is_available, 0), media_id
                   FROM collection_items
                   WHERE collection_id = ? AND tmdb_id = ? AND media_type = ?"#
            )
            .bind(collection_id)
            .bind(tmdb_id)
            .bind(media_type)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.map(|(id, collection_id, tmdb_id, media_type, title, overview, poster_url, release_date, timeline_order, release_order, timeline_year, timeline_notes, is_available, media_id)| {
            CollectionItem {
                id,
                collection_id,
                tmdb_id,
                media_type,
                title,
                overview,
                poster_url,
                release_date,
                timeline_order,
                release_order,
                timeline_year,
                timeline_notes,
                is_available: is_available != 0,
                media_id,
            }
        }))
    }

    async fn find_collections_by_item_tmdb_id(&self, tmdb_id: i64) -> Result<Vec<Collection>, RepositoryError> {
        let rows = sqlx::query(
            r#"SELECT c.* 
               FROM collections c
               JOIN collection_items ci ON c.id = ci.collection_id
               WHERE ci.tmdb_id = ?"#
        )
        .bind(tmdb_id)
        .fetch_all(&self.pool)
        .await?;

        let mut collection_list = Vec::with_capacity(rows.len());
        for row in rows {
            collection_list.push(Self::map_row_to_collection(row)?);
        }

        Ok(collection_list)
    }
}
