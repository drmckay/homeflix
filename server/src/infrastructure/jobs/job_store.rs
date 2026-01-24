//! Job Store - In-memory job status tracking
//!
//! Provides a thread-safe store for tracking long-running async jobs
//! like subtitle generation. Supports progress updates, completion,
//! and failure states.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Job state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Job is queued but not started
    Pending,
    /// Job is currently running
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
    /// Job was cancelled
    Cancelled,
}

/// Single job status with progress and result tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    /// Unique job identifier
    pub id: String,
    /// Current job state
    pub state: JobState,
    /// Progress percentage (0.0 - 100.0)
    pub progress: f32,
    /// Human-readable status message
    pub message: Option<String>,
    /// Job result data (serialized JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job was last updated
    pub updated_at: DateTime<Utc>,
    /// When the job completed (if finished)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// Batch job status for multi-item operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobStatus {
    /// Unique batch job identifier
    pub id: String,
    /// Current batch state
    pub state: JobState,
    /// Total number of items to process
    pub total: usize,
    /// Number of items completed successfully
    pub completed: usize,
    /// Number of items that failed
    pub failed: usize,
    /// Individual item errors (media_id -> error message)
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub errors: HashMap<i64, String>,
    /// When the batch job was created
    pub created_at: DateTime<Utc>,
    /// When the batch job was last updated
    pub updated_at: DateTime<Utc>,
    /// When the batch job completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// In-memory job store
///
/// Thread-safe storage for job status tracking.
/// Jobs are stored in memory and cleared after a configurable retention period.
#[derive(Debug)]
pub struct JobStore {
    /// Single jobs (subtitle generation for one media)
    jobs: Arc<RwLock<HashMap<String, JobStatus>>>,
    /// Batch jobs (series/season generation)
    batch_jobs: Arc<RwLock<HashMap<String, BatchJobStatus>>>,
}

impl JobStore {
    /// Creates a new JobStore
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            batch_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========== Single Job Operations ==========

    /// Creates a new job and returns its ID
    pub async fn create_job(&self) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let job = JobStatus {
            id: id.clone(),
            state: JobState::Pending,
            progress: 0.0,
            message: None,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.jobs.write().await.insert(id.clone(), job);
        id
    }

    /// Creates a job with a specific ID (for predictable testing)
    pub async fn create_job_with_id(&self, id: String) -> String {
        let now = Utc::now();

        let job = JobStatus {
            id: id.clone(),
            state: JobState::Pending,
            progress: 0.0,
            message: None,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.jobs.write().await.insert(id.clone(), job);
        id
    }

    /// Gets the status of a job
    pub async fn get_job(&self, job_id: &str) -> Option<JobStatus> {
        self.jobs.read().await.get(job_id).cloned()
    }

    /// Marks a job as processing
    pub async fn start_job(&self, job_id: &str) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.state = JobState::Processing;
            job.updated_at = Utc::now();
        }
    }

    /// Updates job progress (0.0 - 100.0)
    pub async fn update_progress(&self, job_id: &str, progress: f32, message: Option<&str>) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.progress = progress.clamp(0.0, 100.0);
            job.message = message.map(String::from);
            job.updated_at = Utc::now();
        }
    }

    /// Marks a job as completed with a result
    pub async fn complete_job<T: Serialize>(&self, job_id: &str, result: &T) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.state = JobState::Completed;
            job.progress = 100.0;
            job.result = Some(serde_json::to_value(result).unwrap_or(serde_json::Value::Null));
            job.completed_at = Some(Utc::now());
            job.updated_at = Utc::now();
        }
    }

    /// Marks a job as failed with an error message
    pub async fn fail_job(&self, job_id: &str, error: &str) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.state = JobState::Failed;
            job.error = Some(error.to_string());
            job.completed_at = Some(Utc::now());
            job.updated_at = Utc::now();
        }
    }

    /// Cancels a job
    pub async fn cancel_job(&self, job_id: &str) -> bool {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            if job.state == JobState::Pending || job.state == JobState::Processing {
                job.state = JobState::Cancelled;
                job.completed_at = Some(Utc::now());
                job.updated_at = Utc::now();
                return true;
            }
        }
        false
    }

    // ========== Batch Job Operations ==========

    /// Creates a new batch job and returns its ID
    pub async fn create_batch_job(&self, total_items: usize) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let batch = BatchJobStatus {
            id: id.clone(),
            state: JobState::Processing,
            total: total_items,
            completed: 0,
            failed: 0,
            errors: HashMap::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.batch_jobs.write().await.insert(id.clone(), batch);
        id
    }

    /// Gets the status of a batch job
    pub async fn get_batch_job(&self, job_id: &str) -> Option<BatchJobStatus> {
        self.batch_jobs.read().await.get(job_id).cloned()
    }

    /// Updates batch job progress (increments completed count)
    pub async fn update_batch_progress(&self, job_id: &str, completed: usize) {
        if let Some(batch) = self.batch_jobs.write().await.get_mut(job_id) {
            batch.completed = completed;
            batch.updated_at = Utc::now();
        }
    }

    /// Records an error for a specific item in a batch
    pub async fn add_batch_error(&self, job_id: &str, media_id: i64, error: String) {
        if let Some(batch) = self.batch_jobs.write().await.get_mut(job_id) {
            batch.failed += 1;
            batch.errors.insert(media_id, error);
            batch.updated_at = Utc::now();
        }
    }

    /// Marks a batch job as completed
    pub async fn complete_batch_job(&self, job_id: &str) {
        if let Some(batch) = self.batch_jobs.write().await.get_mut(job_id) {
            batch.state = if batch.failed == 0 {
                JobState::Completed
            } else if batch.completed == 0 {
                JobState::Failed
            } else {
                JobState::Completed // Partial success
            };
            batch.completed_at = Some(Utc::now());
            batch.updated_at = Utc::now();
        }
    }

    /// Cancels a batch job
    pub async fn cancel_batch_job(&self, job_id: &str) -> bool {
        if let Some(batch) = self.batch_jobs.write().await.get_mut(job_id) {
            if batch.state == JobState::Processing {
                batch.state = JobState::Cancelled;
                batch.completed_at = Some(Utc::now());
                batch.updated_at = Utc::now();
                return true;
            }
        }
        false
    }

    /// Checks if a batch job has been cancelled
    ///
    /// Used by the batch processing loop to check if it should stop.
    pub async fn is_batch_cancelled(&self, job_id: &str) -> bool {
        self.batch_jobs.read().await
            .get(job_id)
            .map(|b| b.state == JobState::Cancelled)
            .unwrap_or(false)
    }

    // ========== Cleanup Operations ==========

    /// Removes all completed/failed jobs older than the specified duration
    pub async fn cleanup_old_jobs(&self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;

        // Cleanup single jobs
        self.jobs.write().await.retain(|_, job| {
            match job.state {
                JobState::Completed | JobState::Failed | JobState::Cancelled => {
                    job.completed_at.map(|t| t > cutoff).unwrap_or(true)
                }
                _ => true,
            }
        });

        // Cleanup batch jobs
        self.batch_jobs.write().await.retain(|_, batch| {
            match batch.state {
                JobState::Completed | JobState::Failed | JobState::Cancelled => {
                    batch.completed_at.map(|t| t > cutoff).unwrap_or(true)
                }
                _ => true,
            }
        });
    }

    /// Returns count of active jobs (pending + processing)
    pub async fn active_job_count(&self) -> usize {
        self.jobs.read().await.values()
            .filter(|j| j.state == JobState::Pending || j.state == JobState::Processing)
            .count()
    }

    /// Returns count of active batch jobs
    pub async fn active_batch_job_count(&self) -> usize {
        self.batch_jobs.read().await.values()
            .filter(|b| b.state == JobState::Processing)
            .count()
    }
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for JobStore {
    fn clone(&self) -> Self {
        Self {
            jobs: self.jobs.clone(),
            batch_jobs: self.batch_jobs.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_lifecycle() {
        let store = JobStore::new();

        // Create job
        let job_id = store.create_job().await;
        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.state, JobState::Pending);
        assert_eq!(job.progress, 0.0);

        // Start job
        store.start_job(&job_id).await;
        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.state, JobState::Processing);

        // Update progress
        store.update_progress(&job_id, 50.0, Some("Halfway done")).await;
        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.progress, 50.0);
        assert_eq!(job.message.as_deref(), Some("Halfway done"));

        // Complete job
        #[derive(Serialize)]
        struct TestResult {
            path: String,
        }
        store.complete_job(&job_id, &TestResult { path: "/test.srt".into() }).await;
        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.state, JobState::Completed);
        assert_eq!(job.progress, 100.0);
        assert!(job.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_job_failure() {
        let store = JobStore::new();

        let job_id = store.create_job().await;
        store.start_job(&job_id).await;
        store.fail_job(&job_id, "Something went wrong").await;

        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.state, JobState::Failed);
        assert_eq!(job.error.as_deref(), Some("Something went wrong"));
    }

    #[tokio::test]
    async fn test_batch_job_lifecycle() {
        let store = JobStore::new();

        // Create batch job for 10 items
        let batch_id = store.create_batch_job(10).await;
        let batch = store.get_batch_job(&batch_id).await.unwrap();
        assert_eq!(batch.state, JobState::Processing);
        assert_eq!(batch.total, 10);
        assert_eq!(batch.completed, 0);

        // Update progress
        store.update_batch_progress(&batch_id, 5).await;
        let batch = store.get_batch_job(&batch_id).await.unwrap();
        assert_eq!(batch.completed, 5);

        // Add an error
        store.add_batch_error(&batch_id, 123, "Failed to process".into()).await;
        let batch = store.get_batch_job(&batch_id).await.unwrap();
        assert_eq!(batch.failed, 1);
        assert!(batch.errors.contains_key(&123));

        // Complete
        store.update_batch_progress(&batch_id, 9).await;
        store.complete_batch_job(&batch_id).await;
        let batch = store.get_batch_job(&batch_id).await.unwrap();
        assert_eq!(batch.state, JobState::Completed);
    }

    #[tokio::test]
    async fn test_job_cancellation() {
        let store = JobStore::new();

        let job_id = store.create_job().await;
        store.start_job(&job_id).await;

        assert!(store.cancel_job(&job_id).await);
        let job = store.get_job(&job_id).await.unwrap();
        assert_eq!(job.state, JobState::Cancelled);

        // Cannot cancel again
        assert!(!store.cancel_job(&job_id).await);
    }
}
