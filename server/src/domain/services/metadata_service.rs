//! MetadataService trait
//!
//! Service for working with metadata

use async_trait::async_trait;
use crate::domain::entities::{Media, Series, Season};

/// Service for metadata operations
#[async_trait]
pub trait MetadataService: Send + Sync {
    /// Enriches media with metadata
    async fn enrich_media(&self, media: &mut Media) -> Result<(), crate::shared::error::DomainError>;

    /// Enriches series with metadata
    async fn enrich_series(&self, series: &mut Series) -> Result<(), crate::shared::error::DomainError>;

    /// Enriches season with metadata
    async fn enrich_season(&self, season: &mut Season) -> Result<(), crate::shared::error::DomainError>;

    /// Extracts year from filename
    async fn extract_year(&self, filename: &str) -> Result<Option<i32>, crate::shared::error::DomainError>;

    /// Extracts resolution from filename
    async fn extract_resolution(&self, filename: &str) -> Result<Option<String>, crate::shared::error::DomainError>;
}

/// Default implementation of metadata service
pub struct DefaultMetadataService;

#[async_trait]
impl MetadataService for DefaultMetadataService {
    async fn enrich_media(&self, _media: &mut Media) -> Result<(), crate::shared::error::DomainError> {
        // Placeholder for metadata enrichment
        // In a real implementation, this would fetch from TMDB, NFO files, etc.
        Ok(())
    }

    async fn enrich_series(&self, _series: &mut Series) -> Result<(), crate::shared::error::DomainError> {
        // Placeholder for metadata enrichment
        // In a real implementation, this would fetch from TMDB, NFO files, etc.
        Ok(())
    }

    async fn enrich_season(&self, _season: &mut Season) -> Result<(), crate::shared::error::DomainError> {
        // Placeholder for metadata enrichment
        // In a real implementation, this would fetch from TMDB, NFO files, etc.
        Ok(())
    }

    async fn extract_year(&self, filename: &str) -> Result<Option<i32>, crate::shared::error::DomainError> {
        let year_regex = regex::Regex::new(r"\b(19|20)\d{2}\b")?;
        if let Some(captures) = year_regex.captures(filename) {
            let year_str = captures.get(0).map(|m| m.as_str()).unwrap_or("");
            if let Ok(year) = year_str.parse::<i32>() {
                // Validate reasonable year range
                if year >= 1900 && year <= 2100 {
                    return Ok(Some(year));
                }
            }
        }
        Ok(None)
    }

    async fn extract_resolution(&self, filename: &str) -> Result<Option<String>, crate::shared::error::DomainError> {
        let resolutions = [
            ("4K", r"(?i)\b4k\b"),
            ("2160p", r"(?i)\b2160p\b"),
            ("1080p", r"(?i)\b1080p\b"),
            ("720p", r"(?i)\b720p\b"),
            ("480p", r"(?i)\b480p\b"),
            ("360p", r"(?i)\b360p\b"),
        ];

        let lower_filename = filename.to_lowercase();
        for (resolution, pattern) in resolutions {
            let regex = regex::Regex::new(pattern)?;
            if regex.is_match(&lower_filename) {
                return Ok(Some(resolution.to_string()));
            }
        }

        Ok(None)
    }
}
