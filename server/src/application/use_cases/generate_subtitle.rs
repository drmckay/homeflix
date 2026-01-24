//! Generate Subtitle Use Case
//!
//! Orchestrates automatic subtitle generation using:
//! - Whisper.cpp for speech-to-text transcription
//! - Ollama for LLM-based translation
//! - Audio fingerprinting for tracking and deduplication

use std::path::Path;
use std::sync::Arc;
use tracing::{info, debug};

use crate::domain::repositories::MediaRepository;
use crate::domain::events::{
    SubtitleGenerationStartedEvent,
    SubtitleGenerationCompletedEvent,
    SubtitleGenerationFailedEvent,
};
use crate::infrastructure::external::{
    WhisperAdapter, TranscriptionSegment, segments_to_srt,
    OllamaClient, language_code_to_name,
    FpcalcAdapter, AudioFingerprint,
};
use crate::infrastructure::gpu::GpuCoordinator;
use crate::infrastructure::jobs::JobStore;
use crate::interfaces::messaging::EventBus;
use crate::infrastructure::messaging::InMemoryEventBus;
use crate::shared::error::ApplicationError;

/// Request for subtitle generation
#[derive(Debug, Clone)]
pub struct GenerateSubtitleRequest {
    /// Media ID to generate subtitles for
    pub media_id: i64,
    /// Audio track index to transcribe (0-based)
    pub audio_track_index: usize,
    /// Source language code (None = auto-detect)
    pub source_language: Option<String>,
    /// Target language code for translation (None = no translation)
    pub target_language: Option<String>,
}

/// Result of subtitle generation
#[derive(Debug, Clone, serde::Serialize)]
pub struct GenerateSubtitleResult {
    /// Path to the generated SRT file
    pub subtitle_path: String,
    /// Language code of the subtitle
    pub language: String,
    /// Whether translation was applied
    pub was_translated: bool,
    /// Audio fingerprint for this track (hex string)
    pub audio_fingerprint: String,
    /// Duration of the audio in seconds
    pub duration_seconds: f64,
}

/// Generate Subtitle Use Case
///
/// Orchestrates the complete subtitle generation workflow:
/// 1. Validates media exists and file is accessible
/// 2. Acquires GPU lock (prevents Whisper/Ollama conflict)
/// 3. Optionally generates audio fingerprint for tracking
/// 4. Extracts audio and runs Whisper transcription
/// 5. Optionally translates with Ollama
/// 6. Writes SRT file next to video
///
/// # GPU Coordination
/// Both Whisper and Ollama use the GPU. This use case holds the GPU lock
/// for the entire duration to prevent conflicts. Batch operations will
/// process one at a time.
pub struct GenerateSubtitleUseCase<E: EventBus + ?Sized = InMemoryEventBus> {
    /// Media repository for file path lookup
    media_repository: Arc<dyn MediaRepository>,
    /// Whisper adapter for transcription
    whisper_adapter: Arc<WhisperAdapter>,
    /// Ollama client for translation (optional)
    ollama_client: Option<Arc<OllamaClient>>,
    /// Fpcalc adapter for audio fingerprinting
    fpcalc_adapter: Arc<FpcalcAdapter>,
    /// GPU coordinator for exclusive access
    gpu_coordinator: Arc<GpuCoordinator>,
    /// Job store for progress tracking
    job_store: Arc<JobStore>,
    /// Event bus for publishing events
    event_bus: Arc<E>,
}

// Type alias for backward compatibility
pub type GenerateSubtitleUseCaseDefault = GenerateSubtitleUseCase<InMemoryEventBus>;

impl<E: EventBus + ?Sized> GenerateSubtitleUseCase<E> {
    /// Creates a new GenerateSubtitleUseCase
    ///
    /// # Arguments
    /// * `media_repository` - Repository for media lookup
    /// * `whisper_adapter` - Whisper CLI adapter
    /// * `ollama_client` - Ollama HTTP client (None if translation disabled)
    /// * `fpcalc_adapter` - Chromaprint fpcalc adapter
    /// * `gpu_coordinator` - GPU semaphore for exclusive access
    /// * `job_store` - Job status store
    /// * `event_bus` - Event bus for publishing domain events
    pub fn new(
        media_repository: Arc<dyn MediaRepository>,
        whisper_adapter: Arc<WhisperAdapter>,
        ollama_client: Option<Arc<OllamaClient>>,
        fpcalc_adapter: Arc<FpcalcAdapter>,
        gpu_coordinator: Arc<GpuCoordinator>,
        job_store: Arc<JobStore>,
        event_bus: Arc<E>,
    ) -> Self {
        Self {
            media_repository,
            whisper_adapter,
            ollama_client,
            fpcalc_adapter,
            gpu_coordinator,
            job_store,
            event_bus,
        }
    }

    /// Executes subtitle generation
    ///
    /// This is a long-running operation. Progress is tracked via the job store.
    ///
    /// # Arguments
    /// * `request` - Generation request parameters
    /// * `job_id` - Job ID for progress tracking
    ///
    /// # Returns
    /// * `Result<GenerateSubtitleResult, ApplicationError>` - Generation result
    pub async fn execute(
        &self,
        request: GenerateSubtitleRequest,
        job_id: &str,
    ) -> Result<GenerateSubtitleResult, ApplicationError> {
        info!(
            "Starting subtitle generation for media {} (audio track {}, target: {:?})",
            request.media_id,
            request.audio_track_index,
            request.target_language
        );

        // Publish subtitle generation started event
        let event = SubtitleGenerationStartedEvent::new(
            request.media_id,
            job_id.to_string(),
            request.audio_track_index,
            request.source_language.clone(),
            request.target_language.clone(),
        );
        if let Err(e) = self.event_bus.publish(event).await {
            tracing::warn!("Failed to publish subtitle generation started event: {}", e);
        }

        // Update job status
        self.job_store.start_job(job_id).await;
        self.job_store.update_progress(job_id, 5.0, Some("Looking up media...")).await;

        // 1. Fetch media to get file path
        let media = self.media_repository
            .find_by_id(request.media_id)
            .await?
            .ok_or_else(|| ApplicationError::Domain(
                crate::shared::error::DomainError::NotFound(
                    format!("Media with ID {} not found", request.media_id)
                )
            ))?;

        let video_path = &media.file_path;
        let path = Path::new(video_path);

        if !path.exists() {
            let error = ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::PathNotFound(video_path.clone())
            );
            self.publish_failed_event(request.media_id, job_id, &error.to_string()).await;
            return Err(error);
        }

        self.job_store.update_progress(job_id, 10.0, Some("Acquiring GPU lock...")).await;

        // 2. Acquire GPU lock (holds for entire operation)
        let _gpu_permit = self.gpu_coordinator.acquire().await;
        debug!("GPU lock acquired for subtitle generation");

        self.job_store.update_progress(job_id, 15.0, Some("Generating audio fingerprint...")).await;

        // 3. Generate audio fingerprint (for tracking)
        let fingerprint = match self.generate_fingerprint(video_path, request.audio_track_index).await {
            Ok(f) => f,
            Err(e) => {
                self.publish_failed_event(request.media_id, job_id, &e.to_string()).await;
                return Err(e);
            }
        };
        let fingerprint_hex = FpcalcAdapter::fingerprint_to_hex(&fingerprint);

        debug!(
            "Audio fingerprint generated: {} chars, duration: {}s",
            fingerprint_hex.len(),
            fingerprint.duration
        );

        // 4. Unload Ollama model before Whisper to free VRAM (important for 8GB systems)
        if let Some(ollama) = &self.ollama_client {
            self.job_store.update_progress(job_id, 20.0, Some("Unloading Ollama model from VRAM...")).await;
            if let Err(e) = ollama.unload_model().await {
                debug!("Failed to unload Ollama model (may not have been loaded): {}", e);
            }
        }

        self.job_store.update_progress(job_id, 25.0, Some("Transcribing audio with Whisper...")).await;

        // 5. Transcribe audio with Whisper
        let transcription = match self.whisper_adapter
            .transcribe(
                video_path,
                request.audio_track_index,
                request.source_language.as_deref(),
            )
            .await
        {
            Ok(t) => t,
            Err(e) => {
                let error = ApplicationError::SpeechToText(e);
                self.publish_failed_event(request.media_id, job_id, &error.to_string()).await;
                return Err(error);
            }
        };

        let detected_language = transcription.detected_language.clone()
            .unwrap_or_else(|| request.source_language.clone().unwrap_or("en".to_string()));

        info!(
            "Transcription complete: {} segments, detected language: {}",
            transcription.segments.len(),
            detected_language
        );

        self.job_store.update_progress(job_id, 60.0, Some("Transcription complete")).await;

        // DEBUG: Save raw transcription for comparison (before translation)
        // This helps diagnose whether issues come from Whisper or Ollama
        if let Err(e) = self.write_debug_transcription(video_path, &detected_language, &transcription.segments) {
            debug!("Failed to write debug transcription: {}", e);
        }

        // 6. Optionally translate
        let (final_segments, output_language, was_translated) = if let Some(target_lang) = &request.target_language {
            if target_lang != &detected_language {
                self.job_store.update_progress(job_id, 65.0, Some("Translating with Ollama...")).await;

                let translated = match self.translate_segments(
                    transcription.segments,
                    &detected_language,
                    target_lang,
                ).await {
                    Ok(t) => t,
                    Err(e) => {
                        self.publish_failed_event(request.media_id, job_id, &e.to_string()).await;
                        return Err(e);
                    }
                };

                info!(
                    "Translation complete: {} -> {}, {} segments",
                    detected_language,
                    target_lang,
                    translated.len()
                );

                (translated, target_lang.clone(), true)
            } else {
                // Source and target are same, no translation needed
                (transcription.segments, detected_language, false)
            }
        } else {
            // No translation requested
            (transcription.segments, detected_language, false)
        };

        self.job_store.update_progress(job_id, 90.0, Some("Writing SRT file...")).await;

        // 6. Write SRT file
        let srt_path = self.write_srt_file(video_path, &output_language, &final_segments)?;

        info!("Subtitle written to: {}", srt_path);

        self.job_store.update_progress(job_id, 100.0, Some("Complete")).await;

        let result = GenerateSubtitleResult {
            subtitle_path: srt_path.clone(),
            language: output_language.clone(),
            was_translated,
            audio_fingerprint: fingerprint_hex.clone(),
            duration_seconds: fingerprint.duration,
        };

        // Publish subtitle generation completed event
        let event = SubtitleGenerationCompletedEvent::new(
            request.media_id,
            job_id.to_string(),
            srt_path,
            output_language,
            was_translated,
            fingerprint_hex,
            fingerprint.duration,
        );
        if let Err(e) = self.event_bus.publish(event).await {
            tracing::warn!("Failed to publish subtitle generation completed event: {}", e);
        }

        Ok(result)
    }

    /// Publishes subtitle generation failed event (helper method)
    async fn publish_failed_event(&self, media_id: i64, job_id: &str, error: &str) {
        let event = SubtitleGenerationFailedEvent::new(
            media_id,
            job_id.to_string(),
            error.to_string(),
        );
        if let Err(e) = self.event_bus.publish(event).await {
            tracing::warn!("Failed to publish subtitle generation failed event: {}", e);
        }
    }

    /// Generates audio fingerprint for tracking
    async fn generate_fingerprint(
        &self,
        video_path: &str,
        audio_track_index: usize,
    ) -> Result<AudioFingerprint, ApplicationError> {
        self.fpcalc_adapter
            .fingerprint(video_path, audio_track_index)
            .await
            .map_err(|e| ApplicationError::Fingerprint(e))
    }

    /// Translates transcription segments using Ollama
    async fn translate_segments(
        &self,
        segments: Vec<TranscriptionSegment>,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<TranscriptionSegment>, ApplicationError> {
        let ollama = self.ollama_client.as_ref()
            .ok_or_else(|| ApplicationError::Translation(
                crate::shared::error::TranslationError::ServiceUnavailable(
                    "Ollama client not configured".to_string()
                )
            ))?;

        // Convert language codes to names for better LLM results
        let source_name = language_code_to_name(source_lang);
        let target_name = language_code_to_name(target_lang);

        ollama
            .translate_segments(segments, source_name, target_name)
            .await
            .map_err(|e| ApplicationError::Translation(e))
    }

    /// Writes SRT file next to the video
    fn write_srt_file(
        &self,
        video_path: &str,
        language: &str,
        segments: &[TranscriptionSegment],
    ) -> Result<String, ApplicationError> {
        let video_path = Path::new(video_path);

        // Build SRT filename: video.LANG.srt
        let stem = video_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ApplicationError::Internal("Invalid video filename".to_string()))?;

        let parent = video_path.parent()
            .ok_or_else(|| ApplicationError::Internal("Invalid video path".to_string()))?;

        let srt_filename = format!("{}.{}.srt", stem, language);
        let srt_path = parent.join(&srt_filename);

        // Generate SRT content
        let srt_content = segments_to_srt(segments);

        // Write to file
        std::fs::write(&srt_path, srt_content).map_err(|e| {
            ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::Io(e)
            )
        })?;

        Ok(srt_path.to_string_lossy().to_string())
    }

    /// Writes debug transcription file (raw Whisper output before translation)
    ///
    /// Creates a .transcribe.srt file for debugging purposes, allowing
    /// comparison between raw transcription and translated output.
    fn write_debug_transcription(
        &self,
        video_path: &str,
        language: &str,
        segments: &[TranscriptionSegment],
    ) -> Result<String, ApplicationError> {
        let video_path = Path::new(video_path);

        let stem = video_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ApplicationError::Internal("Invalid video filename".to_string()))?;

        let parent = video_path.parent()
            .ok_or_else(|| ApplicationError::Internal("Invalid video path".to_string()))?;

        // Use .transcribe.srt suffix to distinguish from translated output
        let srt_filename = format!("{}.{}.transcribe.srt", stem, language);
        let srt_path = parent.join(&srt_filename);

        let srt_content = segments_to_srt(segments);

        std::fs::write(&srt_path, srt_content).map_err(|e| {
            ApplicationError::Filesystem(
                crate::shared::error::FilesystemError::Io(e)
            )
        })?;

        info!("Debug transcription written to: {}", srt_path.display());
        Ok(srt_path.to_string_lossy().to_string())
    }

    /// Checks if services are available
    pub async fn check_capabilities(&self) -> ServiceCapabilities {
        ServiceCapabilities {
            whisper_available: self.whisper_adapter.is_available().await,
            whisper_model_exists: self.whisper_adapter.model_exists(),
            ollama_available: match &self.ollama_client {
                Some(client) => client.is_available().await,
                None => false,
            },
            fpcalc_available: self.fpcalc_adapter.is_available().await,
        }
    }
}

/// Service availability status
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceCapabilities {
    /// Whether whisper-cli is available
    pub whisper_available: bool,
    /// Whether the Whisper model file exists
    pub whisper_model_exists: bool,
    /// Whether Ollama API is available
    pub ollama_available: bool,
    /// Whether fpcalc is available
    pub fpcalc_available: bool,
}

impl ServiceCapabilities {
    /// Returns true if basic transcription is possible
    pub fn can_transcribe(&self) -> bool {
        self.whisper_available && self.whisper_model_exists
    }

    /// Returns true if translation is possible
    pub fn can_translate(&self) -> bool {
        self.ollama_available
    }
}
