//! Batch Generate Subtitles Use Case
//!
//! Orchestrates subtitle generation for multiple media items:
//! - Entire series (all seasons, all episodes)
//! - Single season (all episodes)
//!
//! Processes sequentially to avoid GPU conflicts.

use std::sync::Arc;
use tracing::{info, debug, error, warn};

use crate::domain::repositories::MediaRepository;
use crate::infrastructure::jobs::JobStore;
use crate::interfaces::external_services::VideoAnalyzer;
use crate::shared::error::ApplicationError;

use super::generate_subtitle::{GenerateSubtitleUseCase, GenerateSubtitleRequest, GenerateSubtitleResult};

/// Target type for batch generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchTargetType {
    /// Generate for all episodes in a series
    Series,
    /// Generate for all episodes in a single season
    Season,
}

/// Request for batch subtitle generation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BatchGenerateRequest {
    /// Target type (series or season)
    pub target_type: BatchTargetType,
    /// Series ID
    pub target_id: i64,
    /// Season number (required for Season target type)
    #[serde(default)]
    pub season_number: Option<i32>,
    /// Preferred audio language code (e.g., "hun", "eng", "jpn")
    /// The system will automatically find the matching audio track for each episode.
    /// If not specified or no match found, uses the first audio track.
    #[serde(default)]
    pub preferred_audio_language: Option<String>,
    /// Source language code (None = auto-detect)
    #[serde(default)]
    pub source_language: Option<String>,
    /// Target language code for translation (None = no translation)
    #[serde(default)]
    pub target_language: Option<String>,
}

/// Individual episode result in batch
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchItemResult {
    /// Media ID
    pub media_id: i64,
    /// Whether successful
    pub success: bool,
    /// Result if successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<GenerateSubtitleResult>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of batch subtitle generation
#[derive(Debug, Clone, serde::Serialize)]
pub struct BatchGenerateResult {
    /// Total episodes processed
    pub total: usize,
    /// Successful generations
    pub successful: usize,
    /// Failed generations
    pub failed: usize,
    /// Individual results
    pub items: Vec<BatchItemResult>,
}

/// Batch Generate Subtitles Use Case
///
/// Processes multiple media items sequentially, generating subtitles for each.
/// The sequential processing is intentional - GPU resources are shared between
/// Whisper and Ollama, so parallel processing would cause conflicts.
///
/// # Progress Tracking
/// Progress is tracked via the batch job store, which tracks:
/// - Total items to process
/// - Completed items
/// - Failed items with error messages
pub struct BatchGenerateSubtitlesUseCase {
    /// Media repository for episode lookup
    media_repository: Arc<dyn MediaRepository>,
    /// Single subtitle generation use case
    generate_subtitle_use_case: Arc<GenerateSubtitleUseCase>,
    /// Job store for progress tracking
    job_store: Arc<JobStore>,
    /// Video analyzer for audio track detection
    video_analyzer: Arc<dyn VideoAnalyzer>,
}

impl BatchGenerateSubtitlesUseCase {
    /// Creates a new BatchGenerateSubtitlesUseCase
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        generate_subtitle_use_case: Arc<GenerateSubtitleUseCase>,
        job_store: Arc<JobStore>,
        video_analyzer: Arc<dyn VideoAnalyzer>,
    ) -> Self {
        Self {
            media_repository,
            generate_subtitle_use_case,
            job_store,
            video_analyzer,
        }
    }

    /// Starts batch subtitle generation (returns job ID)
    ///
    /// This spawns a background task and returns immediately with a job ID.
    /// Use the job store to track progress.
    ///
    /// # Arguments
    /// * `request` - Batch generation request
    ///
    /// # Returns
    /// * Job ID for tracking progress
    pub async fn start(&self, request: BatchGenerateRequest) -> Result<String, ApplicationError> {
        // Validate request
        if request.target_type == BatchTargetType::Season && request.season_number.is_none() {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::InvalidInput(
                    "season_number is required for Season target type".to_string()
                )
            ));
        }

        // Get episodes to process
        let episodes = self.get_episodes(&request).await?;

        if episodes.is_empty() {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(
                    "No episodes found for the specified target".to_string()
                )
            ));
        }

        info!(
            "Starting batch subtitle generation: {} episodes for {:?} {}",
            episodes.len(),
            request.target_type,
            request.target_id
        );

        // Create batch job
        let batch_job_id = self.job_store.create_batch_job(episodes.len()).await;

        // Spawn background processing
        let use_case = self.generate_subtitle_use_case.clone();
        let job_store = self.job_store.clone();
        let media_repository = self.media_repository.clone();
        let video_analyzer = self.video_analyzer.clone();
        let job_id = batch_job_id.clone();

        tokio::spawn(async move {
            Self::process_batch(
                use_case,
                job_store,
                media_repository,
                video_analyzer,
                &job_id,
                episodes,
                request,
            ).await;
        });

        Ok(batch_job_id)
    }

    /// Finds the best matching audio track index for the preferred language
    ///
    /// # Returns
    /// - The index of the track matching the preferred language
    /// - If no match, returns the default track index
    /// - If no default, returns 0
    async fn find_audio_track_for_language(
        video_analyzer: &Arc<dyn VideoAnalyzer>,
        file_path: &str,
        preferred_language: &Option<String>,
    ) -> usize {
        // If no preferred language, use first track
        let Some(preferred) = preferred_language else {
            return 0;
        };

        // Normalize language code for comparison
        let preferred_lower = preferred.to_lowercase();
        let preferred_variants: Vec<String> = match preferred_lower.as_str() {
            "hun" | "hu" | "hungarian" => vec!["hun".into(), "hu".into(), "hungarian".into()],
            "eng" | "en" | "english" => vec!["eng".into(), "en".into(), "english".into()],
            "jpn" | "ja" | "japanese" => vec!["jpn".into(), "ja".into(), "japanese".into()],
            "ger" | "de" | "deu" | "german" => vec!["ger".into(), "de".into(), "deu".into(), "german".into()],
            other => vec![other.into()],
        };

        // Get audio tracks
        match video_analyzer.get_audio_tracks(file_path).await {
            Ok(tracks) => {
                // Find track matching preferred language
                for track in &tracks {
                    if let Some(lang) = &track.language {
                        let lang_lower = lang.to_lowercase();
                        if preferred_variants.iter().any(|v| v == &lang_lower) {
                            debug!(
                                "Found audio track {} for language '{}' in {}",
                                track.index, lang, file_path
                            );
                            return track.index;
                        }
                    }
                }

                // No match - try to find default track
                if let Some(default_track) = tracks.iter().find(|t| t.is_default) {
                    warn!(
                        "No audio track found for language '{}' in {}, using default track {}",
                        preferred, file_path, default_track.index
                    );
                    return default_track.index;
                }

                // No match and no default - use first track
                warn!(
                    "No audio track found for language '{}' in {}, using first track",
                    preferred, file_path
                );
                0
            }
            Err(e) => {
                warn!(
                    "Failed to get audio tracks for {}: {}, using first track",
                    file_path, e
                );
                0
            }
        }
    }

    /// Processes batch sequentially (runs in background)
    async fn process_batch(
        use_case: Arc<GenerateSubtitleUseCase>,
        job_store: Arc<JobStore>,
        media_repository: Arc<dyn MediaRepository>,
        video_analyzer: Arc<dyn VideoAnalyzer>,
        batch_job_id: &str,
        episodes: Vec<i64>,
        request: BatchGenerateRequest,
    ) {
        let total = episodes.len();
        let mut completed = 0;

        for (index, media_id) in episodes.iter().enumerate() {
            // Check if batch was cancelled before processing next episode
            if job_store.is_batch_cancelled(batch_job_id).await {
                info!(
                    "Batch job {} cancelled after {}/{} episodes",
                    batch_job_id,
                    completed,
                    total
                );
                return; // Exit without marking complete (already marked as Cancelled)
            }

            debug!(
                "Processing episode {}/{}: media_id={}",
                index + 1,
                total,
                media_id
            );

            // Get media to find file_path for audio track detection
            let media = match media_repository.find_by_id(*media_id).await {
                Ok(Some(m)) => m,
                Ok(None) => {
                    let error_msg = format!("Media {} not found", media_id);
                    job_store.add_batch_error(batch_job_id, *media_id, error_msg.clone()).await;
                    error!("Episode {}/{} failed: {}", index + 1, total, error_msg);
                    continue;
                }
                Err(e) => {
                    let error_msg = format!("Failed to fetch media {}: {}", media_id, e);
                    job_store.add_batch_error(batch_job_id, *media_id, error_msg.clone()).await;
                    error!("Episode {}/{} failed: {}", index + 1, total, error_msg);
                    continue;
                }
            };

            // Find the best audio track for the preferred language
            let audio_track_index = Self::find_audio_track_for_language(
                &video_analyzer,
                &media.file_path,
                &request.preferred_audio_language,
            ).await;

            debug!(
                "Using audio track {} for episode {} (preferred: {:?})",
                audio_track_index, media_id, request.preferred_audio_language
            );

            // Create individual job for this episode
            let item_job_id = job_store.create_job().await;

            let req = GenerateSubtitleRequest {
                media_id: *media_id,
                audio_track_index,
                source_language: request.source_language.clone(),
                target_language: request.target_language.clone(),
            };

            match use_case.execute(req, &item_job_id).await {
                Ok(result) => {
                    completed += 1;
                    job_store.update_batch_progress(batch_job_id, completed).await;
                    job_store.complete_job(&item_job_id, &result).await;

                    info!(
                        "Episode {}/{} completed: {} -> {}",
                        index + 1,
                        total,
                        media_id,
                        result.subtitle_path
                    );
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    job_store.add_batch_error(batch_job_id, *media_id, error_msg.clone()).await;
                    job_store.fail_job(&item_job_id, &error_msg).await;

                    error!(
                        "Episode {}/{} failed: {} - {}",
                        index + 1,
                        total,
                        media_id,
                        error_msg
                    );
                }
            }
        }

        // Mark batch as complete (only if not cancelled)
        if !job_store.is_batch_cancelled(batch_job_id).await {
            job_store.complete_batch_job(batch_job_id).await;

            info!(
                "Batch subtitle generation complete: {}/{} successful",
                completed,
                total
            );
        }
    }

    /// Gets episode IDs based on target type
    async fn get_episodes(&self, request: &BatchGenerateRequest) -> Result<Vec<i64>, ApplicationError> {
        let mut episodes = match request.target_type {
            BatchTargetType::Series => {
                // Get all episodes for the series
                self.media_repository
                    .find_by_series(request.target_id)
                    .await?
            }
            BatchTargetType::Season => {
                // Get episodes for specific season
                let season = request.season_number.ok_or_else(|| {
                    ApplicationError::Domain(
                        crate::shared::error::DomainError::InvalidInput(
                            "season_number is required for Season target type".to_string()
                        )
                    )
                })?;

                self.media_repository
                    .find_by_season(request.target_id, season)
                    .await?
            }
        };

        // Sort by season number first, then by episode number (not by media ID!)
        episodes.sort_by(|a, b| {
            let season_cmp = a.season.unwrap_or(0).cmp(&b.season.unwrap_or(0));
            if season_cmp != std::cmp::Ordering::Equal {
                return season_cmp;
            }
            a.episode.unwrap_or(0).cmp(&b.episode.unwrap_or(0))
        });

        // Extract IDs after proper sorting
        let ids: Vec<i64> = episodes
            .iter()
            .filter_map(|m| m.id)
            .collect();

        Ok(ids)
    }

    /// Executes batch generation synchronously (for testing or direct calls)
    ///
    /// Unlike `start()`, this blocks until all episodes are processed.
    pub async fn execute(&self, request: BatchGenerateRequest) -> Result<BatchGenerateResult, ApplicationError> {
        // Validate request
        if request.target_type == BatchTargetType::Season && request.season_number.is_none() {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::InvalidInput(
                    "season_number is required for Season target type".to_string()
                )
            ));
        }

        // Get episodes
        let episodes = self.get_episodes(&request).await?;

        if episodes.is_empty() {
            return Err(ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(
                    "No episodes found for the specified target".to_string()
                )
            ));
        }

        let total = episodes.len();
        let mut successful = 0;
        let mut failed = 0;
        let mut items = Vec::with_capacity(total);

        info!(
            "Executing batch subtitle generation: {} episodes",
            total
        );

        for media_id in episodes {
            // Get media to find file_path for audio track detection
            let media = match self.media_repository.find_by_id(media_id).await {
                Ok(Some(m)) => m,
                Ok(None) => {
                    failed += 1;
                    items.push(BatchItemResult {
                        media_id,
                        success: false,
                        result: None,
                        error: Some(format!("Media {} not found", media_id)),
                    });
                    continue;
                }
                Err(e) => {
                    failed += 1;
                    items.push(BatchItemResult {
                        media_id,
                        success: false,
                        result: None,
                        error: Some(format!("Failed to fetch media: {}", e)),
                    });
                    continue;
                }
            };

            // Find the best audio track for the preferred language
            let audio_track_index = Self::find_audio_track_for_language(
                &self.video_analyzer,
                &media.file_path,
                &request.preferred_audio_language,
            ).await;

            // Create job for tracking
            let job_id = self.job_store.create_job().await;

            let req = GenerateSubtitleRequest {
                media_id,
                audio_track_index,
                source_language: request.source_language.clone(),
                target_language: request.target_language.clone(),
            };

            match self.generate_subtitle_use_case.execute(req, &job_id).await {
                Ok(result) => {
                    successful += 1;
                    items.push(BatchItemResult {
                        media_id,
                        success: true,
                        result: Some(result),
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    items.push(BatchItemResult {
                        media_id,
                        success: false,
                        result: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(BatchGenerateResult {
            total,
            successful,
            failed,
            items,
        })
    }
}
