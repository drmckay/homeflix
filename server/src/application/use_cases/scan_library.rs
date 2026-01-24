//! Scan Library Use Case
//!
//! Orchestrates library scanning process including:
//! - Directory traversal
//! - Media identification
//! - Confidence scoring
//! - Database persistence
//! - Event publishing
//!
//! # Scalability Features
//! - Bounded parallelism for file processing
//! - Progress tracking with callbacks
//! - Batch database operations
//! - Adaptive concurrency based on system resources

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::stream::{self, StreamExt};
use tokio::sync::Semaphore;
use tracing::{info, warn, error, debug, instrument};

use crate::domain::entities::{Media, Series, Collection};
use crate::domain::events::{MediaIdentifiedEvent, ScanCompletedEvent};
use crate::domain::value_objects::ConfidenceScore;
use crate::domain::repositories::{MediaRepository, SeriesRepository, CollectionRepository};
use crate::domain::services::{IdentificationService, ConfidenceService, TmdbCrossValidator};
use crate::interfaces::filesystem::DirectoryWalker;
use crate::interfaces::messaging::EventBus;
use crate::interfaces::external_services::{TmdbService, VideoAnalyzer};
use crate::shared::error::ApplicationError;
use crate::shared::text::{TitleNormalizer, FuzzyMatcher};

/// Result of a library scan operation
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Total number of files processed
    pub processed_count: usize,
    /// Number of items successfully identified
    pub identified_count: usize,
    /// Number of items that failed to identify
    pub failed_count: usize,
    /// Number of items skipped (already verified)
    pub skipped_count: usize,
    /// Scan duration in seconds
    pub duration_secs: u64,
    /// Path that was scanned
    pub scan_path: String,
    /// Files processed per second
    pub files_per_second: f64,
}

/// TMDB metadata enrichment result
#[derive(Debug, Clone)]
pub struct TmdbEnrichment {
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub rating: Option<f32>,
    pub genres: Option<Vec<String>>,
    pub release_date: Option<String>,
    pub duration_seconds: Option<i32>,
    /// For TV shows, include series-specific data
    pub is_tv_show: bool,
    pub status: Option<String>,
    pub total_seasons: Option<i32>,
    pub total_episodes: Option<i32>,
    /// Episode-specific metadata (for episodes)
    pub episode_title: Option<String>,
    pub episode_overview: Option<String>,
    pub episode_still_url: Option<String>,
    pub episode_air_date: Option<String>,
    /// Collection info (for movies that belong to a collection)
    pub collection_id: Option<i64>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    pub collection_backdrop_url: Option<String>,
}

/// Progress callback for scan operations
///
/// Called periodically during scan to report progress
pub type ProgressCallback = Arc<dyn Fn(ScanProgress) + Send + Sync>;

/// Scan progress information
#[derive(Debug, Clone)]
pub struct ScanProgress {
    /// Number of files processed so far
    pub processed: usize,
    /// Total number of files to process
    pub total: usize,
    /// Percentage complete (0.0 to 100.0)
    pub percentage: f64,
    /// Number of files identified
    pub identified: usize,
    /// Number of files failed
    pub failed: usize,
    /// Number of files skipped
    pub skipped: usize,
    /// Estimated time remaining in seconds
    pub estimated_seconds_remaining: Option<f64>,
}

impl ScanProgress {
    /// Creates new scan progress
    pub fn new(total: usize) -> Self {
        Self {
            processed: 0,
            total,
            percentage: 0.0,
            identified: 0,
            failed: 0,
            skipped: 0,
            estimated_seconds_remaining: None,
        }
    }

    /// Updates progress with a processed result
    pub fn update(&mut self, result: &ProcessResult) {
        self.processed += 1;
        match result {
            ProcessResult::Identified(_) => self.identified += 1,
            ProcessResult::Skipped => self.skipped += 1,
            ProcessResult::Failed(_) => self.failed += 1,
        }
        self.percentage = (self.processed as f64 / self.total as f64) * 100.0;
    }

    /// Updates estimated time remaining based on elapsed time
    pub fn update_time_remaining(&mut self, elapsed_secs: u64) {
        if self.processed > 0 {
            let avg_time_per_file = elapsed_secs as f64 / self.processed as f64;
            let remaining = self.total.saturating_sub(self.processed);
            self.estimated_seconds_remaining = Some(avg_time_per_file * remaining as f64);
        }
    }
}

/// Scan Library Use Case
///
/// Orchestrates complete library scanning workflow:
/// 1. Walks directory tree
/// 2. Processes files in parallel
/// 3. Identifies media content
/// 4. Calculates confidence scores
/// 5. Saves to database
/// 6. Publishes events
///
/// # Architecture Notes
/// - Uses dependency injection for all dependencies
/// - Publishes domain events for decoupled side effects
/// - Supports parallel processing for performance
/// - Handles errors gracefully without failing entire scan
/// - Provides progress tracking for large scans
///
/// # Scalability Features
/// - Bounded parallelism using semaphore
/// - Adaptive concurrency based on CPU cores
/// - Progress callbacks for real-time updates
/// - Batch operations where possible
pub struct ScanLibraryUseCase<E: EventBus + ?Sized> {
    /// Media repository for persistence
    media_repository: Arc<dyn MediaRepository>,
    /// Series repository for TV show persistence
    series_repository: Arc<dyn SeriesRepository>,
    /// Collection repository for movie collection persistence
    collection_repository: Arc<dyn CollectionRepository>,
    /// Directory walker for file traversal
    directory_walker: Arc<dyn DirectoryWalker>,
    /// Event bus for publishing events
    event_bus: Arc<E>,
    /// Identification service for content type detection
    identification_service: Arc<dyn IdentificationService>,
    /// Confidence scoring service for multi-signal scoring
    confidence_service: Arc<dyn ConfidenceService>,
    /// TMDB service for metadata lookup (optional for offline mode)
    tmdb_service: Option<Arc<dyn TmdbService>>,
    /// TMDB cross-validator for verifying episodes exist (optional)
    tmdb_cross_validator: Option<Arc<dyn TmdbCrossValidator>>,
    /// Video analyzer for extracting duration from video files (optional)
    video_analyzer: Option<Arc<dyn VideoAnalyzer>>,
    /// Semaphore for bounded parallelism
    concurrency_limiter: Arc<Semaphore>,
    /// Minimum confidence threshold for re-scanning
    rescan_threshold: f32,
    /// Whether to force re-scan all files
    force_rescan: bool,
    /// Progress callback for scan updates
    progress_callback: Option<ProgressCallback>,
    /// Progress update interval in milliseconds
    progress_interval_ms: u64,
}

impl<E: EventBus + ?Sized> ScanLibraryUseCase<E> {
    /// Creates a new scan library use case
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media persistence
    /// * `series_repository` - Repository for series persistence
    /// * `directory_walker` - Walker for directory traversal
    /// * `event_bus` - Event bus for publishing events
    /// * `identification_service` - Service for content type detection
    /// * `confidence_service` - Service for confidence scoring
    ///
    /// # Defaults
    /// - Max concurrent: 4 (or CPU cores if higher)
    /// - Rescan threshold: 0.85
    /// - Force rescan: false
    /// - Progress interval: 1000ms
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        series_repository: Arc<dyn SeriesRepository>,
        collection_repository: Arc<dyn CollectionRepository>,
        directory_walker: Arc<dyn DirectoryWalker>,
        event_bus: Arc<E>,
        identification_service: Arc<dyn IdentificationService>,
        confidence_service: Arc<dyn ConfidenceService>,
    ) -> Self {
        // Use number of CPU cores as default concurrency, capped at 8
        let max_concurrent = num_cpus::get().min(8);

        Self {
            media_repository,
            series_repository,
            collection_repository,
            directory_walker,
            event_bus,
            identification_service,
            confidence_service,
            tmdb_service: None,
            tmdb_cross_validator: None,
            video_analyzer: None,
            concurrency_limiter: Arc::new(Semaphore::new(max_concurrent)),
            rescan_threshold: 0.85,
            force_rescan: false,
            progress_callback: None,
            progress_interval_ms: 1000,
        }
    }

    /// Sets the TMDB service for metadata lookup
    ///
    /// When TMDB service is provided, the scanner will:
    /// - Search TMDB for matching movies/TV shows
    /// - Fetch detailed metadata (posters, ratings, etc.)
    /// - Use higher confidence scoring
    ///
    /// Without TMDB, only filename-based identification is used.
    pub fn with_tmdb_service(mut self, tmdb_service: Arc<dyn TmdbService>) -> Self {
        self.tmdb_service = Some(tmdb_service);
        self
    }

    /// Sets the TMDB cross-validator for episode validation
    ///
    /// When cross-validator is provided, the scanner will:
    /// - Verify that detected season/episode numbers exist in TMDB
    /// - Boost confidence for validated episodes
    /// - Flag episodes that don't exist in TMDB
    ///
    /// This is useful for catching misidentified episodes.
    pub fn with_tmdb_cross_validator(mut self, validator: Arc<dyn TmdbCrossValidator>) -> Self {
        self.tmdb_cross_validator = Some(validator);
        self
    }

    /// Sets the video analyzer for extracting duration from files
    ///
    /// When video analyzer is provided, the scanner will:
    /// - Use FFprobe to extract video duration for all media files
    /// - Use this as a fallback when TMDB doesn't provide duration
    ///
    /// This is especially important for TV episodes where TMDB doesn't provide runtime.
    pub fn with_video_analyzer(mut self, analyzer: Arc<dyn VideoAnalyzer>) -> Self {
        self.video_analyzer = Some(analyzer);
        self
    }

    /// Sets maximum concurrent file processing
    ///
    /// # Arguments
    /// * `max` - Maximum number of concurrent file operations
    ///
    /// # Note
    /// If set to 0, uses number of CPU cores (capped at 8)
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        let max_concurrent = if max == 0 {
            num_cpus::get().min(8)
        } else {
            max.max(1)
        };
        self.concurrency_limiter = Arc::new(Semaphore::new(max_concurrent));
        self
    }

    /// Sets minimum confidence threshold for re-scanning
    ///
    /// # Arguments
    /// * `threshold` - Confidence threshold (0.0 to 1.0)
    pub fn with_rescan_threshold(mut self, threshold: f32) -> Self {
        self.rescan_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Forces re-scan of all files regardless of confidence
    pub fn with_force_rescan(mut self, force: bool) -> Self {
        self.force_rescan = force;
        self
    }

    /// Sets progress callback for scan updates
    ///
    /// # Arguments
    /// * `callback` - Callback function for progress updates
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Sets progress update interval
    ///
    /// # Arguments
    /// * `interval_ms` - Interval in milliseconds between progress updates
    pub fn with_progress_interval(mut self, interval_ms: u64) -> Self {
        self.progress_interval_ms = interval_ms.max(100);
        self
    }

    /// Executes library scan with enhanced parallel processing
    ///
    /// # Arguments
    /// * `root_path` - Root directory path to scan
    ///
    /// # Returns
    /// * `Result<ScanResult, ApplicationError>` - Scan results or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Directory walk fails
    /// - File system errors occur
    /// - Database errors occur
    ///
    /// # Performance Characteristics
    /// - Processes files in parallel with bounded concurrency
    /// - Uses semaphore to limit concurrent operations
    /// - Provides progress updates at configured intervals
    /// - Calculates throughput metrics
    #[instrument(skip(self, root_path))]
    pub async fn execute(&self, root_path: &str) -> Result<ScanResult, ApplicationError> {
        let start_time = Instant::now();
        let path = std::path::Path::new(root_path);
        let last_progress_update = Arc::new(std::sync::Mutex::new(Instant::now()));

        info!("Starting library scan at: {}", root_path);
        debug!("Using {} concurrent workers", self.concurrency_limiter.available_permits());

        // Walk directory to find video files
        let entries = self.directory_walker.walk_videos(path).await?;

        let total_files = entries.len();
        if total_files == 0 {
            info!("No video files found in {}", root_path);
            return Ok(ScanResult {
                processed_count: 0,
                identified_count: 0,
                failed_count: 0,
                skipped_count: 0,
                duration_secs: 0,
                scan_path: root_path.to_string(),
                files_per_second: 0.0,
            });
        }

        info!("Found {} video files to process", total_files);

        // Atomic counters for thread-safe progress tracking
        let processed_count = Arc::new(AtomicUsize::new(0));
        let identified_count = Arc::new(AtomicUsize::new(0));
        let failed_count = Arc::new(AtomicUsize::new(0));
        let skipped_count = Arc::new(AtomicUsize::new(0));

        // Clone Arcs for the async closure
        let processed_count_clone = Arc::clone(&processed_count);
        let identified_count_clone = Arc::clone(&identified_count);
        let failed_count_clone = Arc::clone(&failed_count);
        let skipped_count_clone = Arc::clone(&skipped_count);
        let last_progress_update_clone = Arc::clone(&last_progress_update);
        let progress_callback = self.progress_callback.clone();
        let progress_interval = Duration::from_millis(self.progress_interval_ms);

        // Process files in parallel with bounded concurrency
        let results = stream::iter(entries)
            .map(move |entry| {
                let limiter = Arc::clone(&self.concurrency_limiter);
                let repo = Arc::clone(&self.media_repository);
                let event_bus = Arc::clone(&self.event_bus);
                let force_rescan = self.force_rescan;
                let rescan_threshold = self.rescan_threshold;
                
                async move {
                    // Acquire permit for bounded parallelism
                    let _permit = limiter.acquire().await;
                    
                    self.process_entry_internal(
                        entry,
                        repo,
                        event_bus,
                        force_rescan,
                        rescan_threshold,
                    ).await
                }
            })
            .buffer_unordered(self.concurrency_limiter.available_permits())
            .collect::<Vec<_>>()
            .await;

        // Aggregate results
        for result in &results {
            match result {
                Ok(ProcessResult::Identified(_)) => {
                    identified_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                Ok(ProcessResult::Skipped) => {
                    skipped_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                Ok(ProcessResult::Failed(_)) => {
                    failed_count_clone.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => {
                    failed_count_clone.fetch_add(1, Ordering::SeqCst);
                }
            }
            
            // Update processed count and check for progress update
            let processed = processed_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            
            if let Some(ref callback) = progress_callback {
                let mut last_update = last_progress_update_clone.lock().unwrap();
                let now = Instant::now();
                
                if now.duration_since(*last_update) >= progress_interval {
                    let elapsed = start_time.elapsed();
                    let mut progress = ScanProgress::new(total_files);
                    progress.processed = processed;
                    progress.identified = identified_count_clone.load(Ordering::SeqCst);
                    progress.failed = failed_count_clone.load(Ordering::SeqCst);
                    progress.skipped = skipped_count_clone.load(Ordering::SeqCst);
                    progress.update_time_remaining(elapsed.as_secs());
                    
                    callback(progress);
                    *last_update = now;
                }
            }
        }

        // Final progress update
        if let Some(ref callback) = progress_callback {
            let elapsed = start_time.elapsed();
            let mut progress = ScanProgress::new(total_files);
            progress.processed = total_files;
            progress.identified = identified_count.load(Ordering::SeqCst);
            progress.failed = failed_count.load(Ordering::SeqCst);
            progress.skipped = skipped_count.load(Ordering::SeqCst);
            progress.update_time_remaining(elapsed.as_secs());
            callback(progress);
        }

        let duration = start_time.elapsed();
        let processed = processed_count.load(Ordering::SeqCst);
        let identified = identified_count.load(Ordering::SeqCst);
        let failed = failed_count.load(Ordering::SeqCst);
        let skipped = skipped_count.load(Ordering::SeqCst);
        let files_per_second = if duration.as_secs() > 0 {
            processed as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        // Publish scan completed event
        let event = ScanCompletedEvent::new(
            processed,
            identified,
            failed,
            duration.as_secs(),
            root_path.to_string(),
        );

        if let Err(e) = self.event_bus.publish(event).await {
            error!("Failed to publish scan completed event: {}", e);
        }

        info!(
            "Scan completed: {} processed, {} identified, {} failed, {} skipped in {:.2}s ({:.2} files/sec)",
            processed,
            identified,
            failed,
            skipped,
            duration.as_secs_f64(),
            files_per_second
        );

        Ok(ScanResult {
            processed_count: processed,
            identified_count: identified,
            failed_count: failed,
            skipped_count: skipped,
            duration_secs: duration.as_secs(),
            scan_path: root_path.to_string(),
            files_per_second,
        })
    }

    /// Internal method to process a single directory entry
    ///
    /// Separated to allow use in async closure
    async fn process_entry_internal(
        &self,
        entry: crate::interfaces::filesystem::WalkEntry,
        media_repository: Arc<dyn MediaRepository>,
        event_bus: Arc<E>,
        force_rescan: bool,
        rescan_threshold: f32,
    ) -> Result<ProcessResult, ApplicationError> {
        let file_path = entry.path.to_string_lossy().to_string();

        // Check if media already exists in database
        if let Some(existing) = media_repository.find_by_path(&file_path).await? {
            // Skip if already verified and not forcing rescan
            if !force_rescan && existing.confidence_score.value() >= rescan_threshold {
                debug!("Skipping verified media: {}", file_path);
                return Ok(ProcessResult::Skipped);
            }

            // Re-identify if confidence is low or forcing rescan
            debug!("Re-identifying media with low confidence: {}", file_path);
        }

        // Perform identification using the domain IdentificationService
        let mut identification_result = self.identify_media(&file_path, &entry).await?;

        // Enrich with TMDB metadata if service is available
        // TMDB failures are non-fatal - we continue without enrichment
        let tmdb_enrichment = match self.enrich_with_tmdb(&mut identification_result, &file_path).await {
            Ok(enrichment) => enrichment,
            Err(e) => {
                debug!("TMDB enrichment failed for {}: {}", file_path, e);
                None
            }
        };

        // Cross-validate episodes with TMDB if validator is available
        let validation_adjustment = self.cross_validate_episode(&identification_result).await;

        // Calculate confidence score using the ConfidenceService
        let mut confidence = self.calculate_confidence(&identification_result).await;

        // Apply validation adjustment to confidence
        if validation_adjustment != 0.0 {
            let adjusted = (confidence.value() + validation_adjustment).clamp(0.0, 1.0);
            confidence = ConfidenceScore::new(adjusted).unwrap_or(confidence);
        }

        // Create media entity
        let mut media = Media::new(
            file_path.clone(),
            identification_result.media_type.clone(),
            identification_result.title.clone(),
        )?;

        // Set additional properties from identification
        if let Some(year) = identification_result.year {
            media = media.with_release_date(Some(format!("{}-01-01", year)));
        }

        if let Some(season) = identification_result.season {
            media = media.with_season(Some(season));
        }

        if let Some(episode) = identification_result.episode {
            media = media.with_episode(Some(episode));
        }

        // Set episode_end for multi-episode files
        if let Some(ref multi_eps) = identification_result.multi_episode {
            if multi_eps.len() > 1 {
                if let Some(&end_ep) = multi_eps.last() {
                    media = media.with_episode_end(Some(end_ep));
                }
            }
        }

        if let Some(tmdb_id) = identification_result.tmdb_id {
            media = media.with_tmdb_id(Some(tmdb_id));
        }

        // Apply TMDB enrichment if available (use TMDB title for consistency)
        // Also capture TV show data for series creation
        let tv_show_data = if let Some(ref enrichment) = tmdb_enrichment {
            if enrichment.is_tv_show {
                Some((
                    enrichment.title.clone(),
                    enrichment.overview.clone(),
                    enrichment.poster_url.clone(),
                    enrichment.backdrop_url.clone(),
                    enrichment.rating,
                    enrichment.genres.clone(),
                    enrichment.release_date.clone(),
                    enrichment.status.clone(),
                    enrichment.total_seasons,
                    enrichment.total_episodes,
                ))
            } else {
                None
            }
        } else {
            None
        };

        // Capture collection data for movies
        let collection_data = if let Some(ref enrichment) = tmdb_enrichment {
            if !enrichment.is_tv_show {
                if let (Some(id), Some(name)) = (enrichment.collection_id, enrichment.collection_name.clone()) {
                    Some((
                        id,
                        name,
                        enrichment.collection_poster_url.clone(),
                        enrichment.collection_backdrop_url.clone(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(enrichment) = tmdb_enrichment {
            media.title = enrichment.title;
            media.overview = enrichment.overview;
            media.poster_url = enrichment.poster_url;
            media.backdrop_url = enrichment.backdrop_url;
            media.rating = enrichment.rating;
            if let Some(genres) = enrichment.genres {
                media.genres = Some(genres.join(", "));
            }
            if let Some(release_date) = enrichment.release_date {
                media.release_date = Some(release_date);
            }
            if let Some(duration) = enrichment.duration_seconds {
                media.duration_seconds = Some(duration);
            }

            // Apply episode-specific metadata for episodes
            if identification_result.media_type.is_episode() {
                // Use episode title as the media title (more specific than series title)
                if let Some(episode_title) = enrichment.episode_title {
                    media.title = episode_title;
                }
                // Use episode overview if available (more specific than series overview)
                if let Some(episode_overview) = enrichment.episode_overview {
                    media.overview = Some(episode_overview);
                }
                // Use episode still image as poster (shows the specific episode)
                if let Some(episode_still_url) = enrichment.episode_still_url {
                    media.poster_url = Some(episode_still_url);
                }
                // Use episode air date as release date
                if let Some(episode_air_date) = enrichment.episode_air_date {
                    media.release_date = Some(episode_air_date);
                }
            }
        }

        // If duration is still not set, try to get it from FFprobe
        // This is especially important for TV episodes where TMDB doesn't provide runtime
        if media.duration_seconds.is_none() {
            if let Some(ref analyzer) = self.video_analyzer {
                match analyzer.get_duration(&file_path).await {
                    Ok(duration) => {
                        let duration_secs = duration.round() as i32;
                        media.duration_seconds = Some(duration_secs);
                        debug!("Got duration {} seconds from FFprobe for '{}'", duration_secs, media.title);
                    }
                    Err(e) => {
                        warn!("Failed to get duration from FFprobe for '{}': {}", file_path, e);
                    }
                }
            }
        }

        // Create or find Series for TV episodes
        if identification_result.media_type.is_episode() {
            if let Some(tmdb_id) = identification_result.tmdb_id {
                // Check if series already exists
                let series_id = match self.series_repository.find_by_tmdb_id(tmdb_id).await {
                    Ok(Some(existing_series)) => {
                        debug!("Found existing series '{}' (ID: {:?})", existing_series.title, existing_series.id);
                        existing_series.id
                    }
                    Ok(None) => {
                        // Create new series from TV show data
                        if let Some((title, overview, poster_url, backdrop_url, rating, genres, first_air_date, status, total_seasons, total_episodes)) = tv_show_data {
                            let mut series = Series::new(title.clone())
                                .map_err(|e| ApplicationError::Domain(e))?;
                            series = series
                                .with_tmdb_id(Some(tmdb_id))
                                .with_overview(overview)
                                .with_poster_url(poster_url)
                                .with_backdrop_url(backdrop_url)
                                .with_rating(rating)
                                .with_first_air_date(first_air_date)
                                .with_status(status)
                                .with_total_seasons(total_seasons)
                                .with_total_episodes(total_episodes);
                            if let Some(genres) = genres {
                                series = series.with_genres(Some(genres.join(", ")));
                            }
                            series.update_confidence(confidence.clone());

                            match self.series_repository.save(&series).await {
                                Ok(id) => {
                                    info!("Created new series '{}' with ID {}", title, id);
                                    Some(id)
                                }
                                Err(e) => {
                                    warn!("Failed to create series '{}': {}", title, e);
                                    None
                                }
                            }
                        } else {
                            // No TV show data, try to create series from media data
                            let mut series = Series::new(media.title.clone())
                                .map_err(|e| ApplicationError::Domain(e))?;
                            series = series
                                .with_tmdb_id(Some(tmdb_id))
                                .with_overview(media.overview.clone())
                                .with_poster_url(media.poster_url.clone())
                                .with_backdrop_url(media.backdrop_url.clone())
                                .with_rating(media.rating)
                                .with_genres(media.genres.clone());
                            series.update_confidence(confidence.clone());

                            match self.series_repository.save(&series).await {
                                Ok(id) => {
                                    info!("Created new series '{}' with ID {} (from media data)", media.title, id);
                                    Some(id)
                                }
                                Err(e) => {
                                    warn!("Failed to create series '{}': {}", media.title, e);
                                    None
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error checking for existing series: {}", e);
                        None
                    }
                };

                // Link episode to series
                if let Some(sid) = series_id {
                    media.series_id = Some(sid);
                }
            }
        }

        // Update confidence
        media.update_confidence(confidence);

        // Save to database
        let media_id = if let Some(existing) = media_repository.find_by_path(&file_path).await? {
            // Update existing media
            media.id = existing.id;
            media_repository.update(&media).await?;
            existing.id.unwrap_or(0)
        } else {
            // Insert new media
            media_repository.save(&media).await?
        };

        // Publish media identified event
        let event = MediaIdentifiedEvent::new(
            media_id,
            file_path,
            identification_result.media_type.as_str().to_string(),
            identification_result.tmdb_id,
            confidence.value(),
            identification_result.strategy.as_str().to_string(),
        );

        if let Err(e) = event_bus.publish(event).await {
            error!("Failed to publish media identified event: {}", e);
        }

        // Handle collection for movies
        if let Some((tmdb_collection_id, collection_name, poster_url, backdrop_url)) = collection_data {
            // Check if collection already exists
            match self.collection_repository.find_by_tmdb_id(tmdb_collection_id).await {
                Ok(Some(mut existing)) => {
                    // Update available count if media was added
                    debug!("Found existing collection '{}' (ID: {:?})", existing.name, existing.id);
                    if let Some(collection_id) = existing.id {
                        // Update available items count
                        existing.available_items += 1;
                        if let Err(e) = self.collection_repository.update(&existing).await {
                            warn!("Failed to update collection '{}': {}", collection_name, e);
                        }
                    }
                }
                Ok(None) => {
                    // Create new collection with available_items = 1 (current movie is available)
                    let mut collection = Collection::new(collection_name.clone())
                        .map_err(|e| ApplicationError::Domain(e))?
                        .with_tmdb_collection_id(Some(tmdb_collection_id))
                        .with_poster_url(poster_url)
                        .with_backdrop_url(backdrop_url)
                        .with_collection_type("auto".to_string());
                    collection.available_items = 1;

                    match self.collection_repository.save(&collection).await {
                        Ok(id) => {
                            info!("Created new collection '{}' (TMDB: {}) with ID {}", collection_name, tmdb_collection_id, id);
                        }
                        Err(e) => {
                            warn!("Failed to create collection '{}': {}", collection_name, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Error checking for existing collection: {}", e);
                }
            }
        }

        Ok(ProcessResult::Identified(media_id))
    }

    /// Identifies media from file path using the domain IdentificationService
    ///
    /// Uses the proper IdentificationService with:
    /// - Multi-pass regex patterns for season/episode detection
    /// - Folder structure analysis
    /// - Anime detection
    /// - Title cleaning with comprehensive tag removal
    async fn identify_media(
        &self,
        file_path: &str,
        _entry: &crate::interfaces::filesystem::WalkEntry,
    ) -> Result<crate::domain::value_objects::IdentificationResult, ApplicationError> {
        // Use the proper identification service
        let result = self.identification_service
            .identify_content(file_path, None)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(result)
    }

    /// Enriches identification result with TMDB metadata
    ///
    /// If TMDB service is available, searches for matching content
    /// and enriches the result with TMDB ID, poster, rating, etc.
    ///
    /// For TV shows with multiple candidates, uses structure-based
    /// disambiguation to find the best match based on local episode structure.
    async fn enrich_with_tmdb(
        &self,
        result: &mut crate::domain::value_objects::IdentificationResult,
        file_path: &str,
    ) -> Result<Option<TmdbEnrichment>, ApplicationError> {
        let tmdb_service = match &self.tmdb_service {
            Some(s) => s,
            None => return Ok(None),
        };

        // Search TMDB based on media type
        // First try the original title, then try variants if no results found
        info!("TMDB lookup for '{}' (year: {:?}), media_type: {:?}",
            result.title, result.year, result.media_type);

        let mut matches = if result.media_type.is_movie() {
            let mut results = tmdb_service.search_movie(&result.title, result.year).await?;
            info!("Initial TMDB search for '{}' returned {} results", result.title, results.len());
            if !results.is_empty() {
                for (i, r) in results.iter().enumerate().take(3) {
                    info!("  Result {}: '{}' (TMDB ID: {})", i, r.title, r.tmdb_id);
                }
            }

            // If no results found, try title variants (e.g., "Part Two" -> "Part II")
            if results.is_empty() {
                let variants = TitleNormalizer::get_search_variants(&result.title);
                debug!("No TMDB results for '{}', trying {} variants: {:?}",
                    result.title, variants.len(), variants);

                for variant in &variants {
                    // Skip if same as original
                    if variant.to_lowercase() == result.title.to_lowercase() {
                        continue;
                    }

                    debug!("Trying variant: '{}'", variant);
                    results = tmdb_service.search_movie(variant, result.year).await?;

                    if !results.is_empty() {
                        info!("Found TMDB match using variant '{}' for original title '{}'",
                            variant, result.title);
                        break;
                    }
                }
            }
            results
        } else {
            // For TV shows, always search WITHOUT year to get all possible candidates
            // Structure matching will pick the right one based on season/episode counts
            let mut results = tmdb_service.search_tv(&result.title, None).await?;

            // If no results, try variants
            if results.is_empty() {
                let variants = TitleNormalizer::get_search_variants(&result.title);
                debug!("No TMDB results for TV '{}', trying {} variants",
                    result.title, variants.len());

                for variant in &variants {
                    if variant.to_lowercase() == result.title.to_lowercase() {
                        continue;
                    }

                    results = tmdb_service.search_tv(variant, None).await?;

                    if !results.is_empty() {
                        info!("Found TV match using variant '{}' for original title '{}'",
                            variant, result.title);
                        break;
                    }
                }
            }
            results
        };

        // For TV shows with multiple candidates, filter by episode existence
        // Check if the season/episode from filename exists in each TMDB candidate
        if !result.media_type.is_movie() && matches.len() > 1 {
            if let (Some(validator), Some(season), Some(episode)) = (&self.tmdb_cross_validator, result.season, result.episode) {
                let candidate_ids: Vec<i64> = matches.iter().map(|m| m.tmdb_id).collect();

                info!(
                    "Filtering {} TV candidates for '{}' S{:02}E{:02}",
                    matches.len(), result.title, season, episode
                );

                let valid_ids = validator.filter_candidates_by_episode(&candidate_ids, season, episode).await;

                if !valid_ids.is_empty() {
                    let original_count = matches.len();

                    // Reorder matches according to valid_ids order (sorted by preference)
                    // valid_ids is already sorted with most seasons first
                    let mut reordered: Vec<_> = valid_ids.iter()
                        .filter_map(|id| matches.iter().find(|m| m.tmdb_id == *id).cloned())
                        .collect();

                    if !reordered.is_empty() {
                        let eliminated = original_count - reordered.len();
                        matches = reordered;

                        if let Some(selected) = matches.first() {
                            info!(
                                "Episode filtering selected '{}' (TMDB {}) - {} candidates eliminated, {} remaining",
                                selected.title, selected.tmdb_id, eliminated, matches.len()
                            );
                        }
                    }
                } else {
                    warn!(
                        "No TMDB candidate has S{:02}E{:02} for '{}' - keeping all candidates",
                        season, episode, result.title
                    );
                }
            }
        }

        // Select the best match using fuzzy title matching
        // This prevents selecting "Behind-the-Scenes" specials over actual movies
        let best_match = if matches.len() > 1 {
            // Score each match by title similarity and filter out making-of/special content
            let mut scored: Vec<_> = matches.iter()
                .filter(|m| {
                    // Filter out making-of, behind-the-scenes, etc.
                    let title_lower = m.title.to_lowercase();
                    !title_lower.contains("making of") &&
                    !title_lower.contains("behind-the-scenes") &&
                    !title_lower.contains("behind the scenes") &&
                    !title_lower.contains("special presentation") &&
                    !title_lower.contains("featurette")
                })
                .map(|m| {
                    let fuzzy_score = FuzzyMatcher::compare_titles(&result.title, &m.title).score;
                    (m, fuzzy_score)
                })
                .collect();

            // Sort by fuzzy score descending
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((best, score)) = scored.first() {
                info!("Selected best match '{}' (TMDB {}) with fuzzy score {:.2} from {} candidates",
                    best.title, best.tmdb_id, score, matches.len());
                Some((*best).clone())
            } else {
                // All matches were filtered out, fall back to first
                matches.first().cloned()
            }
        } else {
            matches.first().cloned()
        };

        if let Some(ref best_match) = best_match {
            result.tmdb_id = Some(best_match.tmdb_id);
            result.strategy = best_match.strategy.clone();

            // Fetch detailed metadata including proper TMDB title
            let enrichment = if result.media_type.is_movie() {
                if let Some(details) = tmdb_service.fetch_movie_details(best_match.tmdb_id).await? {
                    // Extract collection info if present
                    let (collection_id, collection_name, collection_poster_url, collection_backdrop_url) =
                        if let Some(ref collection) = details.belongs_to_collection {
                            debug!("Movie '{}' belongs to collection: '{}' (ID: {})",
                                details.title, collection.name, collection.id);
                            (
                                Some(collection.id),
                                Some(collection.name.clone()),
                                collection.poster_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                collection.backdrop_path.as_ref().map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                            )
                        } else {
                            (None, None, None, None)
                        };

                    Some(TmdbEnrichment {
                        title: details.title,
                        overview: Some(details.overview),
                        poster_url: details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                        backdrop_url: details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                        rating: Some(details.vote_average),
                        genres: Some(details.genres.iter().map(|g| g.name.clone()).collect()),
                        release_date: Some(details.release_date),
                        duration_seconds: details.runtime.map(|r| r * 60),
                        // Not a TV show
                        is_tv_show: false,
                        status: None,
                        total_seasons: None,
                        total_episodes: None,
                        // No episode metadata for movies
                        episode_title: None,
                        episode_overview: None,
                        episode_still_url: None,
                        episode_air_date: None,
                        // Collection info
                        collection_id,
                        collection_name,
                        collection_poster_url,
                        collection_backdrop_url,
                    })
                } else {
                    None
                }
            } else {
                if let Some(details) = tmdb_service.fetch_tv_details(best_match.tmdb_id).await? {
                    // Fetch episode-specific metadata if we have season/episode numbers
                    let (episode_title, episode_overview, episode_still_url, episode_air_date) =
                        if let (Some(season), Some(episode)) = (result.season, result.episode) {
                            match tmdb_service.fetch_episode(best_match.tmdb_id, season, episode).await {
                                Ok(Some(ep_details)) => {
                                    debug!(
                                        "Fetched episode metadata for S{:02}E{:02}: '{}'",
                                        season, episode, ep_details.name
                                    );
                                    (
                                        Some(ep_details.name),
                                        if ep_details.overview.is_empty() { None } else { Some(ep_details.overview) },
                                        ep_details.still_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                                        ep_details.air_date,
                                    )
                                }
                                Ok(None) => {
                                    debug!("Episode S{:02}E{:02} not found in TMDB", season, episode);
                                    (None, None, None, None)
                                }
                                Err(e) => {
                                    debug!("Failed to fetch episode S{:02}E{:02}: {}", season, episode, e);
                                    (None, None, None, None)
                                }
                            }
                        } else {
                            (None, None, None, None)
                        };

                    Some(TmdbEnrichment {
                        title: details.name,
                        overview: Some(details.overview),
                        poster_url: details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)),
                        backdrop_url: details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)),
                        rating: Some(details.vote_average),
                        genres: Some(details.genres.iter().map(|g| g.name.clone()).collect()),
                        release_date: Some(details.first_air_date),
                        duration_seconds: None,
                        // TV show specific fields
                        is_tv_show: true,
                        status: Some(details.status),
                        total_seasons: Some(details.number_of_seasons),
                        total_episodes: Some(details.number_of_episodes),
                        // Episode-specific metadata
                        episode_title,
                        episode_overview,
                        episode_still_url,
                        episode_air_date,
                        // No collection for TV shows
                        collection_id: None,
                        collection_name: None,
                        collection_poster_url: None,
                        collection_backdrop_url: None,
                    })
                } else {
                    None
                }
            };

            return Ok(enrichment);
        }

        Ok(None)
    }

    /// Calculates confidence score based on identification result using the ConfidenceService
    ///
    /// Uses the multi-signal confidence scoring system from ConfidenceService
    /// which considers:
    /// - Match strategy weights
    /// - Year presence
    /// - Season/episode presence for TV
    /// - Title quality
    async fn calculate_confidence(
        &self,
        result: &crate::domain::value_objects::IdentificationResult,
    ) -> ConfidenceScore {
        self.confidence_service.calculate_confidence(result).await
    }

    /// Cross-validates episode identification with TMDB
    ///
    /// Verifies that detected season/episode numbers actually exist in TMDB.
    /// Returns a confidence adjustment:
    /// - Positive (+0.15) if episode is confirmed to exist
    /// - Negative (-0.20) if episode doesn't exist (likely misidentified)
    /// - Zero if not applicable (not an episode, no TMDB ID, or no validator)
    async fn cross_validate_episode(
        &self,
        result: &crate::domain::value_objects::IdentificationResult,
    ) -> f32 {
        // Only validate episodes with TMDB ID, season, and episode numbers
        let validator = match &self.tmdb_cross_validator {
            Some(v) => v,
            None => return 0.0,
        };

        let tmdb_id = match result.tmdb_id {
            Some(id) => id,
            None => return 0.0,
        };

        // Only validate TV episodes
        if !result.media_type.is_episode() {
            return 0.0;
        }

        let (season, episode) = match (result.season, result.episode) {
            (Some(s), Some(e)) => (s, e),
            _ => return 0.0,
        };

        // Validate the episode exists in TMDB
        let validation = validator.validate_episode(tmdb_id, season, episode).await;

        if validation.exists {
            debug!(
                "Episode S{:02}E{:02} confirmed in TMDB for series {}",
                season, episode, tmdb_id
            );
            0.15 // Boost confidence for confirmed episodes
        } else {
            warn!(
                "Episode S{:02}E{:02} NOT found in TMDB for series {}: {:?}",
                season, episode, tmdb_id, validation.notes
            );
            -0.20 // Reduce confidence for unconfirmed episodes
        }
    }
}

/// Result of processing a single entry
#[derive(Debug)]
enum ProcessResult {
    /// Successfully identified media with ID
    Identified(i64),
    /// Skipped (already verified)
    Skipped,
    /// Failed to identify
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_progress_new() {
        let progress = ScanProgress::new(100);
        assert_eq!(progress.total, 100);
        assert_eq!(progress.processed, 0);
        assert_eq!(progress.percentage, 0.0);
    }

    #[test]
    fn test_scan_progress_update() {
        let mut progress = ScanProgress::new(100);
        progress.update(&ProcessResult::Identified(1));
        assert_eq!(progress.processed, 1);
        assert_eq!(progress.identified, 1);
        assert_eq!(progress.percentage, 1.0);
        
        progress.update(&ProcessResult::Skipped);
        assert_eq!(progress.processed, 2);
        assert_eq!(progress.skipped, 1);
        
        progress.update(&ProcessResult::Failed("error".to_string()));
        assert_eq!(progress.processed, 3);
        assert_eq!(progress.failed, 1);
    }

    #[test]
    fn test_scan_progress_time_remaining() {
        let mut progress = ScanProgress::new(100);
        progress.processed = 50;
        progress.update_time_remaining(60); // 60 seconds elapsed for 50 files
        
        assert!(progress.estimated_seconds_remaining.is_some());
        let remaining = progress.estimated_seconds_remaining.unwrap();
        // Should be approximately 60 seconds for remaining 50 files
        assert!((remaining - 60.0).abs() < 1.0);
    }
}
