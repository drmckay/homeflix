//! Get Recently Added Use Case
//!
//! Retrieves recently added content combining movies and series
//! (ranked by most recent episode date).

use std::sync::Arc;
use serde::Serialize;
use crate::domain::entities::{Media, Series};
use crate::domain::repositories::{MediaRepository, SeriesRepository};
use crate::shared::error::ApplicationError;

/// Represents a recently added item (either movie or series)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum RecentlyAddedItem {
    #[serde(rename = "movie")]
    Movie {
        media: Media,
        added_at: String,
    },
    #[serde(rename = "series")]
    Series {
        series: Series,
        added_at: String,
    },
}

impl RecentlyAddedItem {
    pub fn added_at(&self) -> &str {
        match self {
            RecentlyAddedItem::Movie { added_at, .. } => added_at,
            RecentlyAddedItem::Series { added_at, .. } => added_at,
        }
    }
}

pub struct GetRecentlyAddedUseCase {
    media_repository: Arc<dyn MediaRepository>,
    series_repository: Arc<dyn SeriesRepository>,
}

impl GetRecentlyAddedUseCase {
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        series_repository: Arc<dyn SeriesRepository>,
    ) -> Self {
        Self {
            media_repository,
            series_repository,
        }
    }

    /// Gets the most recently added content
    ///
    /// Combines:
    /// - Recently added movies (by created_at)
    /// - Series with recently added episodes (by episode's created_at)
    ///
    /// Returns combined list sorted by date, limited to specified count
    pub async fn execute(&self, limit: usize) -> Result<Vec<RecentlyAddedItem>, ApplicationError> {
        // Fetch more than needed to ensure we have enough after combining
        let fetch_limit = limit * 2;

        // Get recent movies and series with recent episodes concurrently
        let (movies_result, series_result) = tokio::join!(
            self.media_repository.find_recent_movies(fetch_limit),
            self.series_repository.find_recent_by_episode(fetch_limit)
        );

        let movies = movies_result?;
        let series_with_dates = series_result?;

        // Convert to unified items
        let mut items: Vec<RecentlyAddedItem> = Vec::with_capacity(movies.len() + series_with_dates.len());

        for movie in movies {
            let added_at = movie.created_at.to_rfc3339();
            items.push(RecentlyAddedItem::Movie {
                media: movie,
                added_at,
            });
        }

        for (series, latest_episode_date) in series_with_dates {
            items.push(RecentlyAddedItem::Series {
                series,
                added_at: latest_episode_date,
            });
        }

        // Sort by added_at descending (most recent first)
        items.sort_by(|a, b| b.added_at().cmp(a.added_at()));

        // Return top N items
        items.truncate(limit);

        Ok(items)
    }
}
