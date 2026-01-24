//! Scanner Orchestrator
//!
//! High-level service that orchestrates the complete scanning workflow.
//! Coordinates between scan library, identify media, and metadata enrichment.

use std::sync::Arc;
use tracing::{info, debug, warn};

use crate::application::use_cases::scan_library::{ScanLibraryUseCase, ScanResult};
use crate::application::use_cases::identify_media::{IdentifyMediaUseCase, IdentificationResult};
use crate::shared::error::ApplicationError;

use crate::interfaces::messaging::EventBus;

/// Scan Orchestrator
///
/// Orchestrates the complete library scanning workflow:
/// 1. Scans library for new files
/// 2. Identifies media with TMDB
/// 3. Enriches metadata
/// 4. Tracks progress and statistics
///
/// # Architecture Notes
/// - Coordinates multiple use cases
/// - Provides batch processing for efficiency
/// - Handles errors gracefully
pub struct ScannerOrchestrator<E: EventBus + ?Sized> {
    /// Scan library use case
    scan_library_use_case: Arc<ScanLibraryUseCase<E>>,
    /// Identify media use case
    identify_media_use_case: Arc<IdentifyMediaUseCase<E>>,
}

impl<E: EventBus + ?Sized> ScannerOrchestrator<E> {
    /// Creates a new scanner orchestrator
    ///
    /// # Arguments
    /// * `scan_library_use_case` - Use case for library scanning
    /// * `identify_media_use_case` - Use case for media identification
    pub fn new(
        scan_library_use_case: Arc<ScanLibraryUseCase<E>>,
        identify_media_use_case: Arc<IdentifyMediaUseCase<E>>,
    ) -> Self {
        Self {
            scan_library_use_case,
            identify_media_use_case,
        }
    }

    /// Executes complete scan workflow
    ///
    /// # Arguments
    /// * `root_path` - Root directory path to scan
    ///
    /// # Returns
    /// * `Result<OrchestrationResult, ApplicationError>` - Orchestration results
    ///
    /// # Errors
    /// Returns error if:
    /// - Scan fails
    /// - Identification fails
    pub async fn execute_full_scan(
        &self,
        root_path: &str,
    ) -> Result<OrchestrationResult, ApplicationError> {
        info!("Starting full scan workflow for: {}", root_path);

        let start_time = std::time::Instant::now();

        // Step 1: Scan library
        let scan_result = self.scan_library_use_case
            .clone()
            .execute(root_path)
            .await?;

        info!(
            "Scan complete: {} processed, {} identified, {} failed",
            scan_result.processed_count,
            scan_result.identified_count,
            scan_result.failed_count
        );

        // Step 2: Identify all media with TMDB
        // Get all unverified media
        let media_ids = self.get_media_to_identify(&scan_result).await?;

        if !media_ids.is_empty() {
            info!("Identifying {} media items with TMDB", media_ids.len());
            let identification_results = self.identify_media_use_case
                .execute_batch(media_ids)
                .await?;

            info!(
                "Identification complete: {} successful, {} failed",
                identification_results.len(),
                identification_results.len() - identification_results.iter().filter(|r| r.1.confidence > 0.7).count()
            );
        }

        let duration = start_time.elapsed();

        Ok(OrchestrationResult {
            scan_result: scan_result.clone(),
            identification_count: scan_result.identified_count,
            duration_secs: duration.as_secs(),
        })
    }

    /// Executes incremental scan (only new files)
    ///
    /// # Arguments
    /// * `root_path` - Root directory path to scan
    ///
    /// # Returns
    /// * `Result<OrchestrationResult, ApplicationError>` - Orchestration results
    pub async fn execute_incremental_scan(
        &self,
        root_path: &str,
    ) -> Result<OrchestrationResult, ApplicationError> {
        info!("Starting incremental scan workflow for: {}", root_path);

        let start_time = std::time::Instant::now();

        // Step 1: Scan library (only new files)
        let scan_result = self.scan_library_use_case
            .clone()
            .execute(root_path)
            .await?;

        info!(
            "Scan complete: {} processed, {} identified, {} skipped",
            scan_result.processed_count,
            scan_result.identified_count,
            scan_result.skipped_count
        );

        let duration = start_time.elapsed();

        info!(
            "Incremental scan workflow completed in {:.2}s",
            duration.as_secs_f64()
        );

        Ok(OrchestrationResult {
            scan_result: scan_result.clone(),
            identification_count: scan_result.identified_count,
            duration_secs: duration.as_secs(),
        })
    }

    /// Gets media IDs that need identification
    async fn get_media_to_identify(
        &self,
        scan_result: &ScanResult,
    ) -> Result<Vec<i64>, ApplicationError> {
        // Get all media from database
        // In a real implementation, this would query the repository
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }
}

/// Result of orchestration workflow
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    /// Scan results
    pub scan_result: ScanResult,
    /// Number of items identified with TMDB
    pub identification_count: usize,
    /// Total duration in seconds
    pub duration_secs: u64,
}
