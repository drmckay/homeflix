//! SQLite Series Repository Implementation
//!
//! Provides SQLite-based implementation of the SeriesRepository trait

use async_trait::async_trait;
use sqlx::{Pool, Sqlite, Row};
use std::str::FromStr;
use crate::domain::entities::Series;
use crate::domain::repositories::SeriesRepository;
use crate::domain::value_objects::{ConfidenceScore, VerificationStatus};
use crate::shared::error::RepositoryError;

/// SQLite implementation of SeriesRepository
pub struct SqliteSeriesRepository {
    pool: Pool<Sqlite>,
}

impl SqliteSeriesRepository {
    /// Creates a new SQLite series repository
    ///
    /// # Arguments
    /// * `pool` - SQLite connection pool
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Maps a database row to Series entity
    fn map_row_to_series(row: sqlx::sqlite::SqliteRow) -> Result<Series, RepositoryError> {
        Ok(Series {
            id: Some(row.try_get("id")?),
            tmdb_id: row.try_get("tmdb_id")?,
            title: row.try_get("title")?,
            overview: row.try_get("overview")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            confidence_score: ConfidenceScore::new(row.try_get("confidence_score")?)?,
            verification_status: VerificationStatus::from_str(row.try_get("verification_status")?)?,
            first_air_date: row.try_get("first_air_date")?,
            last_air_date: row.try_get("last_air_date")?,
            status: row.try_get("status")?,
            total_seasons: row.try_get("total_seasons")?,
            total_episodes: row.try_get("total_episodes")?,
            original_title: row.try_get("original_title")?,
            genres: row.try_get("genres")?,
            rating: row.try_get("rating")?,
            alternative_matches: row.try_get("alternative_matches")?,
            error_notes: row.try_get("error_notes")?,
            last_verified: row.try_get("last_verified")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[async_trait]
impl SeriesRepository for SqliteSeriesRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<Series>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM series WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_series(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_tmdb_id(&self, tmdb_id: i64) -> Result<Option<Series>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM series WHERE tmdb_id = ?"
        )
        .bind(tmdb_id)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_series(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_title(&self, title: &str) -> Result<Option<Series>, RepositoryError> {
        let result = sqlx::query(
            "SELECT * FROM series WHERE title = ?"
        )
        .bind(title)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => Ok(Some(Self::map_row_to_series(row)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Series>, RepositoryError> {
        // Deduplicate by tmdb_id to avoid showing same series twice if there are
        // duplicate entries. For each tmdb_id, pick the series with most episodes.
        let rows = sqlx::query(
            r#"
            SELECT s.*
            FROM series s
            LEFT JOIN media m ON m.series_id = s.id AND m.media_type = 'episode'
            GROUP BY COALESCE(s.tmdb_id, s.id)
            HAVING s.id = (
                SELECT s2.id FROM series s2
                LEFT JOIN media m2 ON m2.series_id = s2.id AND m2.media_type = 'episode'
                WHERE COALESCE(s2.tmdb_id, s2.id) = COALESCE(s.tmdb_id, s.id)
                GROUP BY s2.id
                ORDER BY COUNT(m2.id) DESC, s2.created_at DESC
                LIMIT 1
            )
            ORDER BY s.created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn find_by_verification_status(
        &self,
        status: VerificationStatus,
    ) -> Result<Vec<Series>, RepositoryError> {
        let status_str = status.as_str();
        let rows = sqlx::query(
            "SELECT * FROM series WHERE verification_status = ? ORDER BY created_at DESC"
        )
        .bind(status_str)
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn find_by_confidence(
        &self,
        min_score: ConfidenceScore,
    ) -> Result<Vec<Series>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM series WHERE confidence_score >= ? ORDER BY confidence_score DESC"
        )
        .bind(min_score.value())
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn save(&self, series: &Series) -> Result<i64, RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO series (
                tmdb_id, title, overview, poster_url, backdrop_url, confidence_score,
                verification_status, first_air_date, last_air_date, status, total_seasons,
                total_episodes, original_title, genres, rating, alternative_matches,
                error_notes, last_verified, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(series.tmdb_id)
        .bind(&series.title)
        .bind(&series.overview)
        .bind(&series.poster_url)
        .bind(&series.backdrop_url)
        .bind(series.confidence_score.value())
        .bind(series.verification_status.as_str())
        .bind(&series.first_air_date)
        .bind(&series.last_air_date)
        .bind(&series.status)
        .bind(series.total_seasons)
        .bind(series.total_episodes)
        .bind(&series.original_title)
        .bind(&series.genres)
        .bind(series.rating)
        .bind(&series.alternative_matches)
        .bind(&series.error_notes)
        .bind(series.last_verified)
        .bind(series.created_at)
        .bind(series.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    async fn update(&self, series: &Series) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE series SET
                tmdb_id = ?, title = ?, overview = ?, poster_url = ?, backdrop_url = ?,
                confidence_score = ?, verification_status = ?, first_air_date = ?,
                last_air_date = ?, status = ?, total_seasons = ?, total_episodes = ?,
                original_title = ?, genres = ?, rating = ?, alternative_matches = ?,
                error_notes = ?, last_verified = ?, updated_at = ?
            WHERE id = ?"
        )
        .bind(series.tmdb_id)
        .bind(&series.title)
        .bind(&series.overview)
        .bind(&series.poster_url)
        .bind(&series.backdrop_url)
        .bind(series.confidence_score.value())
        .bind(series.verification_status.as_str())
        .bind(&series.first_air_date)
        .bind(&series.last_air_date)
        .bind(&series.status)
        .bind(series.total_seasons)
        .bind(series.total_episodes)
        .bind(&series.original_title)
        .bind(&series.genres)
        .bind(series.rating)
        .bind(&series.alternative_matches)
        .bind(&series.error_notes)
        .bind(series.last_verified)
        .bind(series.updated_at)
        .bind(series.id.ok_or(RepositoryError::InvalidInput("Series ID is required".into()))?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM series WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn count(&self) -> Result<i64, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM series")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get("count")?)
    }

    async fn exists_by_title(&self, title: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM series WHERE title = ?")
            .bind(title)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn exists_by_tmdb_id(&self, tmdb_id: i64) -> Result<bool, RepositoryError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM series WHERE tmdb_id = ?")
            .bind(tmdb_id)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.try_get("count")?;
        Ok(count > 0)
    }

    async fn find_recent(&self, limit: usize) -> Result<Vec<Series>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT * FROM series ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn find_low_confidence(&self) -> Result<Vec<Series>, RepositoryError> {
        let threshold = ConfidenceScore::LOW_THRESHOLD;
        let rows = sqlx::query(
            "SELECT * FROM series WHERE confidence_score < ? ORDER BY confidence_score ASC"
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn find_requires_review(&self) -> Result<Vec<Series>, RepositoryError> {
        let status = VerificationStatus::ManualReview.as_str();
        let rows = sqlx::query(
            "SELECT * FROM series WHERE verification_status = ? ORDER BY created_at DESC"
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Series>, RepositoryError> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            "SELECT * FROM series WHERE title LIKE ? ORDER BY title LIMIT ?"
        )
        .bind(&search_pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut series_list = Vec::with_capacity(rows.len());
        for row in rows {
            series_list.push(Self::map_row_to_series(row)?);
        }

        Ok(series_list)
    }

    async fn find_recent_by_episode(
        &self,
        limit: usize,
    ) -> Result<Vec<(Series, String)>, RepositoryError> {
        // Query series with their most recent episode date
        // Deduplicates by tmdb_id to avoid showing same series twice if there are
        // duplicate entries. Picks the series entry with the most linked episodes.
        let rows = sqlx::query(
            r#"
            SELECT s.*,
                   MAX(m.created_at) as latest_episode_date,
                   COUNT(m.id) as episode_count
            FROM series s
            INNER JOIN media m ON m.series_id = s.id
            WHERE m.media_type = 'episode'
            GROUP BY COALESCE(s.tmdb_id, s.id)
            HAVING s.id = (
                SELECT s2.id FROM series s2
                INNER JOIN media m2 ON m2.series_id = s2.id
                WHERE COALESCE(s2.tmdb_id, s2.id) = COALESCE(s.tmdb_id, s.id)
                AND m2.media_type = 'episode'
                GROUP BY s2.id
                ORDER BY COUNT(m2.id) DESC
                LIMIT 1
            )
            ORDER BY latest_episode_date DESC
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            // Extract the latest_episode_date first before consuming the row
            let latest_date: String = row.try_get("latest_episode_date")?;
            let series = Self::map_row_to_series(row)?;
            result.push((series, latest_date));
        }

        Ok(result)
    }
}
