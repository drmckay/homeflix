//! Manage Series Use Case
//!
//! Handles series-related operations.

use std::sync::Arc;
use crate::domain::entities::Series;
use crate::domain::repositories::SeriesRepository;
use crate::shared::error::ApplicationError;

pub struct ManageSeriesUseCase {
    series_repository: Arc<dyn SeriesRepository>,
}

impl ManageSeriesUseCase {
    pub fn new(series_repository: Arc<dyn SeriesRepository>) -> Self {
        Self { series_repository }
    }

    pub async fn get_series(&self, id: i64) -> Result<Series, ApplicationError> {
        self.series_repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(format!("Series with ID {} not found", id))
            ))
    }

    pub async fn list_all(&self) -> Result<Vec<Series>, ApplicationError> {
        // Assuming find_all exists in SeriesRepository
        // I need to check if find_all is in SeriesRepository trait
        Ok(self.series_repository.find_all().await?)
    }
}
