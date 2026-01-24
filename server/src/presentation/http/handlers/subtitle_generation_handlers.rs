//! Subtitle Generation Handlers
//!
//! HTTP handlers for automatic subtitle generation using Whisper + Ollama.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::application::use_cases::generate_subtitle::{
    GenerateSubtitleUseCase, GenerateSubtitleRequest, GenerateSubtitleResult, ServiceCapabilities,
};
use crate::application::use_cases::batch_generate_subtitles::{
    BatchGenerateSubtitlesUseCase, BatchGenerateRequest, BatchTargetType,
};
use crate::infrastructure::jobs::{JobStore, JobStatus, BatchJobStatus};

/// Request body for single subtitle generation
#[derive(Debug, Deserialize)]
pub struct GenerateSubtitleBody {
    /// Audio track index to transcribe (0-based)
    #[serde(default)]
    pub audio_track_index: usize,
    /// Source language code (null = auto-detect)
    #[serde(default)]
    pub source_language: Option<String>,
    /// Target language code for translation (null = no translation)
    #[serde(default)]
    pub target_language: Option<String>,
}

/// Response for subtitle generation request
#[derive(Debug, Serialize)]
pub struct GenerateResponse {
    /// Job ID for tracking progress
    pub job_id: String,
    /// Current status
    pub status: String,
}

/// Generate subtitle for a single media item
///
/// POST /v2/subtitles/:media_id/generate
///
/// Starts subtitle generation in the background and returns a job ID.
/// Use GET /v2/subtitles/jobs/:job_id to track progress.
pub async fn generate_subtitle(
    State(use_case): State<Arc<GenerateSubtitleUseCase>>,
    State(job_store): State<Arc<JobStore>>,
    Path(media_id): Path<i64>,
    Json(body): Json<GenerateSubtitleBody>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Create job for tracking
    let job_id = job_store.create_job().await;

    // Build request
    let request = GenerateSubtitleRequest {
        media_id,
        audio_track_index: body.audio_track_index,
        source_language: body.source_language,
        target_language: body.target_language,
    };

    // Spawn background task
    let use_case = use_case.clone();
    let job_store_clone = job_store.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        match use_case.execute(request, &job_id_clone).await {
            Ok(result) => {
                job_store_clone.complete_job(&job_id_clone, &result).await;
                tracing::info!("Subtitle generation completed: {}", result.subtitle_path);
            }
            Err(e) => {
                let error_msg = e.to_string();
                job_store_clone.fail_job(&job_id_clone, &error_msg).await;
                tracing::error!("Subtitle generation failed: {}", error_msg);
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(GenerateResponse {
            job_id,
            status: "processing".to_string(),
        }),
    ))
}

/// Get job status
///
/// GET /v2/subtitles/jobs/:job_id
///
/// Returns the current status of a subtitle generation job.
pub async fn get_job_status(
    State(job_store): State<Arc<JobStore>>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let job = job_store.get_job(&job_id).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job {} not found", job_id)))?;

    Ok(Json(job))
}

/// Cancel a running job
///
/// DELETE /v2/subtitles/jobs/:job_id
///
/// Attempts to cancel a running job. Note that cancellation may not be
/// immediate if the job is in the middle of transcription or translation.
pub async fn cancel_job(
    State(job_store): State<Arc<JobStore>>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cancelled = job_store.cancel_job(&job_id).await;

    if cancelled {
        Ok((StatusCode::OK, Json(serde_json::json!({
            "message": "Job cancelled",
            "job_id": job_id
        }))))
    } else {
        Err((StatusCode::CONFLICT, format!("Cannot cancel job {} (not running or already completed)", job_id)))
    }
}

/// Request body for batch subtitle generation
#[derive(Debug, Deserialize)]
pub struct BatchGenerateBody {
    /// Target type (series or season)
    pub target_type: BatchTargetType,
    /// Series ID
    pub target_id: i64,
    /// Season number (required for Season target type)
    #[serde(default)]
    pub season_number: Option<i32>,
    /// Preferred audio language code (e.g., "hun", "eng", "jpn")
    /// The system will automatically find the matching audio track for each episode.
    #[serde(default)]
    pub preferred_audio_language: Option<String>,
    /// Source language code (null = auto-detect)
    #[serde(default)]
    pub source_language: Option<String>,
    /// Target language code for translation (null = no translation)
    #[serde(default)]
    pub target_language: Option<String>,
}

/// Start batch subtitle generation
///
/// POST /v2/subtitles/batch/generate
///
/// Starts subtitle generation for multiple episodes in the background.
/// Returns a batch job ID for tracking progress.
pub async fn batch_generate_subtitles(
    State(use_case): State<Arc<BatchGenerateSubtitlesUseCase>>,
    Json(body): Json<BatchGenerateBody>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let request = BatchGenerateRequest {
        target_type: body.target_type,
        target_id: body.target_id,
        season_number: body.season_number,
        preferred_audio_language: body.preferred_audio_language,
        source_language: body.source_language,
        target_language: body.target_language,
    };

    let job_id = use_case.start(request).await
        .map_err(|e| {
            tracing::error!("Failed to start batch generation: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string())
        })?;

    Ok((
        StatusCode::ACCEPTED,
        Json(GenerateResponse {
            job_id,
            status: "processing".to_string(),
        }),
    ))
}

/// Get batch job status
///
/// GET /v2/subtitles/batch/jobs/:job_id
///
/// Returns the current status of a batch subtitle generation job.
pub async fn get_batch_job_status(
    State(job_store): State<Arc<JobStore>>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let batch = job_store.get_batch_job(&job_id).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Batch job {} not found", job_id)))?;

    Ok(Json(batch))
}

/// Cancel a running batch job
///
/// DELETE /v2/subtitles/batch/jobs/:job_id
///
/// Attempts to cancel a running batch job.
pub async fn cancel_batch_job(
    State(job_store): State<Arc<JobStore>>,
    Path(job_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let cancelled = job_store.cancel_batch_job(&job_id).await;

    if cancelled {
        Ok((StatusCode::OK, Json(serde_json::json!({
            "message": "Batch job cancelled",
            "job_id": job_id
        }))))
    } else {
        Err((StatusCode::CONFLICT, format!("Cannot cancel batch job {} (not running or already completed)", job_id)))
    }
}

/// Get service capabilities
///
/// GET /v2/subtitles/capabilities
///
/// Returns the availability status of subtitle generation services
/// (Whisper, Ollama, fpcalc).
pub async fn get_capabilities(
    State(use_case): State<Arc<GenerateSubtitleUseCase>>,
) -> impl IntoResponse {
    let capabilities = use_case.check_capabilities().await;
    Json(capabilities)
}

/// Response for active jobs count
#[derive(Debug, Serialize)]
pub struct ActiveJobsResponse {
    /// Number of active single jobs
    pub active_jobs: usize,
    /// Number of active batch jobs
    pub active_batch_jobs: usize,
}

/// Get count of active jobs
///
/// GET /v2/subtitles/active
///
/// Returns the number of currently running subtitle generation jobs.
pub async fn get_active_jobs(
    State(job_store): State<Arc<JobStore>>,
) -> impl IntoResponse {
    let active_jobs = job_store.active_job_count().await;
    let active_batch_jobs = job_store.active_batch_job_count().await;

    Json(ActiveJobsResponse {
        active_jobs,
        active_batch_jobs,
    })
}
