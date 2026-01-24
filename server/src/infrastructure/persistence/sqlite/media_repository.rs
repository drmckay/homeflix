//! SQLite Media Repository Implementation
//!
//! Provides SQLite-based implementation of the MediaRepository trait

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use std::str::FromStr;
use crate::domain::entities::Media;
use crate::domain::repositories::MediaRepository;
use crate::domain::value_objects::{MediaType, ConfidenceScore, VerificationStatus};
use crate::shared::error::RepositoryError;

/// SQLite implementation of MediaRepository
pub struct SqliteMediaRepository {
    pool: Pool<Sqlite>,
}

impl SqliteMediaRepository {
    /// Creates a new SQLite media repository
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Maps a database row to Media entity
    fn map_row_to_media(row: sqlx::sqlite::SqliteRow) -> Result<Media, RepositoryError> {
        Ok(Media {
            id: Some(row.try_get("id")?),
            file_path: row.try_get("file_path")?,
            media_type: MediaType::from_str(row.try_get("media_type")?)?,
            title: row.try_get("title")?,
            overview: row.try_get("overview")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            trailer_url: row.try_get("trailer_url")?,
            duration_seconds: row.try_get("duration_seconds")?,
            release_date: row.try_get("release_date")?,
            resolution: row.try_get("resolution")?,
            genres: row.try_get("genres")?,
            series_id: row.try_get("series_id")?,
            season: row.try_get("season")?,
            episode: row.try_get("episode")?,
            episode_end: row.try_get("episode_end")?,
            tmdb_id: row.try_get("tmdb_id")?,
            original_title: row.try_get("original_title")?,
            rating: row.try_get("rating")?,
            confidence_score: ConfidenceScore::new(row.try_get("confidence_score")?)?,
            verification_status: VerificationStatus::from_str(row.try_get("verification_status")?)?,
            identification_strategy: row.try_get("identification_strategy")?,
            error_notes: row.try_get("error_notes")?,
            alternative_matches: row.try_get("alternative_matches")?,
            content_rating: row.try_get("content_rating")?,
            content_warnings: row.try_get("content_warnings")?,
            current_position: row.try_get("current_position")?,
            is_watched: row.try_get("is_watched")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[async_trait]
impl MediaRepository for SqliteMediaRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Media>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM media WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_media(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_path(&self, path: &str) -> Result<Option<Media>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM media WHERE file_path = ?"
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_media(row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM media ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_by_type(&self, media_type: MediaType) -> Result<Vec<Media>, RepositoryError> {
        let type_str = media_type.as_str();
        let rows = sqlx::query(
            "SELECT * FROM media WHERE media_type = ? ORDER BY created_at DESC"
        )
        .bind(type_str)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_by_series(&self, series_id: i64) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE series_id = ? ORDER BY season, episode"
        )
        .bind(series_id)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_by_season(&self, series_id: i64, season: i32) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE series_id = ? AND season = ? ORDER BY episode"
        )
        .bind(series_id)
        .bind(season)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_unverified(&self) -> Result<Vec<Media>, RepositoryError> {
        let status = VerificationStatus::Unverified.as_str();
        let rows = sqlx::query(
            "SELECT * FROM media WHERE verification_status = ? ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_by_confidence(&self, min_score: ConfidenceScore) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE confidence_score >= ? ORDER BY confidence_score DESC"
        )
        .bind(min_score.value())
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn save(&self, media: &Media) -> Result<i64, RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO media (
                file_path, media_type, title, overview, poster_url, backdrop_url, trailer_url,
                duration_seconds, release_date, resolution, genres, series_id, season, episode,
                episode_end, tmdb_id, original_title, rating, confidence_score, verification_status,
                identification_strategy, error_notes, alternative_matches, content_rating,
                content_warnings, current_position, is_watched, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&media.file_path)
        .bind(media.media_type.as_str())
        .bind(&media.title)
        .bind(&media.overview)
        .bind(&media.poster_url)
        .bind(&media.backdrop_url)
        .bind(&media.trailer_url)
        .bind(media.duration_seconds)
        .bind(&media.release_date)
        .bind(&media.resolution)
        .bind(&media.genres)
        .bind(media.series_id)
        .bind(media.season)
        .bind(media.episode)
        .bind(media.episode_end)
        .bind(media.tmdb_id)
        .bind(&media.original_title)
        .bind(media.rating)
        .bind(media.confidence_score.value())
        .bind(media.verification_status.as_str())
        .bind(&media.identification_strategy)
        .bind(&media.error_notes)
        .bind(&media.alternative_matches)
        .bind(&media.content_rating)
        .bind(&media.content_warnings)
        .bind(media.current_position)
        .bind(media.is_watched)
        .bind(media.created_at)
        .bind(media.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, media: &Media) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE media SET
                file_path = ?, media_type = ?, title = ?, overview = ?, poster_url = ?,
                backdrop_url = ?, trailer_url = ?, duration_seconds = ?, release_date = ?,
                resolution = ?, genres = ?, series_id = ?, season = ?, episode = ?,
                episode_end = ?, tmdb_id = ?, original_title = ?, rating = ?, confidence_score = ?,
                verification_status = ?, identification_strategy = ?, error_notes = ?,
                alternative_matches = ?, content_rating = ?, content_warnings = ?,
                current_position = ?, is_watched = ?, updated_at = ?
            WHERE id = ?"
        )
        .bind(&media.file_path)
        .bind(media.media_type.as_str())
        .bind(&media.title)
        .bind(&media.overview)
        .bind(&media.poster_url)
        .bind(&media.backdrop_url)
        .bind(&media.trailer_url)
        .bind(media.duration_seconds)
        .bind(&media.release_date)
        .bind(&media.resolution)
        .bind(&media.genres)
        .bind(media.series_id)
        .bind(media.season)
        .bind(media.episode)
        .bind(media.episode_end)
        .bind(media.tmdb_id)
        .bind(&media.original_title)
        .bind(media.rating)
        .bind(media.confidence_score.value())
        .bind(media.verification_status.as_str())
        .bind(&media.identification_strategy)
        .bind(&media.error_notes)
        .bind(&media.alternative_matches)
        .bind(&media.content_rating)
        .bind(&media.content_warnings)
        .bind(media.current_position)
        .bind(media.is_watched)
        .bind(media.updated_at)
        .bind(media.id.ok_or(RepositoryError::InvalidInput("Media ID is required".into()))?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM media WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn count(&self) -> Result<i64, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM media")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get("count")?)
    }

    async fn count_by_type(&self, media_type: MediaType) -> Result<i64, RepositoryError> {
        let type_str = media_type.as_str();
        let result = sqlx::query("SELECT COUNT(*) as count FROM media WHERE media_type = ?")
            .bind(type_str)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get("count")?)
    }

    async fn exists_by_path(&self, path: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM media WHERE file_path = ?")
            .bind(path)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn update_progress(&self, media_id: i64, position: i64, watched: bool) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE media SET current_position = ?, is_watched = ?, updated_at = ? WHERE id = ?"
        )
        .bind(position)
        .bind(watched)
        .bind(chrono::Utc::now())
        .bind(media_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_recent(&self, limit: usize) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_watched(&self) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE is_watched = 1 ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_unwatched(&self) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE is_watched = 0 ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn search(
        &self,
        query: &str,
        media_type: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Media>, RepositoryError> {
        let search_pattern = format!("%{}%", query);

        let rows = match media_type {
            Some(mt) => {
                sqlx::query(
                    "SELECT * FROM media WHERE title LIKE ? AND media_type = ? ORDER BY title LIMIT ?"
                )
                .bind(&search_pattern)
                .bind(mt)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    "SELECT * FROM media WHERE title LIKE ? ORDER BY title LIMIT ?"
                )
                .bind(&search_pattern)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await?
            }
        };

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn get_progress(&self, media_id: i64) -> Result<Option<(i64, bool, String)>, RepositoryError> {
        let result: Option<(i64, bool, String)> = sqlx::query_as(
            "SELECT current_position_seconds, is_watched, last_updated FROM watch_progress WHERE media_id = ?"
        )
        .bind(media_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn mark_watched(&self, media_id: i64) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"INSERT INTO watch_progress (media_id, current_position_seconds, is_watched, last_updated)
               VALUES (?, 0, 1, CURRENT_TIMESTAMP)
               ON CONFLICT(media_id) DO UPDATE SET
               is_watched = 1,
               last_updated = CURRENT_TIMESTAMP"#
        )
        .bind(media_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn mark_unwatched(&self, media_id: i64) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE watch_progress SET is_watched = 0, last_updated = CURRENT_TIMESTAMP WHERE media_id = ?"
        )
        .bind(media_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_in_progress(&self, limit: usize) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE current_position > 0 AND is_watched = 0 ORDER BY updated_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }

    async fn find_recent_movies(&self, limit: usize) -> Result<Vec<Media>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM media WHERE media_type = 'movie' ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut media_list = Vec::with_capacity(rows.len());
        for row in rows {
            media_list.push(Self::map_row_to_media(row)?);
        }

        Ok(media_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_map_row_to_media() {
        // This would require a real database connection
        // In a real scenario, use testcontainers or sqlite in-memory
    }
}
