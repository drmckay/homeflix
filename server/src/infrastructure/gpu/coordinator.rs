//! GPU Coordinator - Resource coordination for GPU-intensive tasks
//!
//! Uses a Tokio Semaphore to ensure that only one GPU-intensive operation
//! runs at a time. This prevents conflicts between Whisper and Ollama
//! which both use the same GPU.

use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit, OwnedSemaphorePermit};

/// GPU Coordinator for managing exclusive GPU access
///
/// Ensures that only one GPU-intensive task runs at a time.
/// Both Whisper (speech-to-text) and Ollama (LLM translation) use the GPU,
/// so they must not run concurrently to avoid memory conflicts and poor performance.
///
/// # Usage
/// ```ignore
/// let coordinator = GpuCoordinator::new();
///
/// // Before running Whisper
/// let permit = coordinator.acquire().await;
/// whisper.transcribe(...).await?;
/// drop(permit); // or let it go out of scope
///
/// // Ollama will wait for the permit
/// let permit = coordinator.acquire().await;
/// ollama.translate(...).await?;
/// ```
#[derive(Debug, Clone)]
pub struct GpuCoordinator {
    semaphore: Arc<Semaphore>,
}

impl GpuCoordinator {
    /// Creates a new GPU coordinator with single-permit access
    ///
    /// Only one GPU task can hold the permit at a time.
    pub fn new() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(1)),
        }
    }

    /// Creates a GPU coordinator with custom concurrent task limit
    ///
    /// # Arguments
    /// * `permits` - Number of concurrent GPU tasks allowed (typically 1)
    pub fn with_permits(permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
        }
    }

    /// Acquires exclusive GPU access
    ///
    /// This will wait until the GPU is available.
    /// The returned permit automatically releases when dropped.
    pub async fn acquire(&self) -> GpuPermit<'_> {
        GpuPermit {
            _permit: self.semaphore.acquire().await.expect("Semaphore closed"),
        }
    }

    /// Acquires exclusive GPU access with ownership
    ///
    /// Returns an owned permit that can be moved across tasks.
    /// Useful when the permit needs to live longer than the borrow scope.
    pub async fn acquire_owned(self: &Arc<Self>) -> OwnedGpuPermit {
        OwnedGpuPermit {
            _permit: self.semaphore.clone().acquire_owned().await.expect("Semaphore closed"),
        }
    }

    /// Tries to acquire GPU access without waiting
    ///
    /// Returns `Some(GpuPermit)` if available, `None` if GPU is busy.
    pub fn try_acquire(&self) -> Option<GpuPermit<'_>> {
        self.semaphore.try_acquire().ok().map(|permit| GpuPermit {
            _permit: permit,
        })
    }

    /// Returns the number of currently available permits
    ///
    /// Useful for monitoring GPU availability.
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Checks if the GPU is currently in use
    pub fn is_busy(&self) -> bool {
        self.semaphore.available_permits() == 0
    }
}

impl Default for GpuCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU permit that releases automatically when dropped
///
/// Holding this permit grants exclusive access to GPU resources.
/// The permit is released when this struct goes out of scope or is dropped.
pub struct GpuPermit<'a> {
    _permit: SemaphorePermit<'a>,
}

impl<'a> GpuPermit<'a> {
    /// Explicitly releases the GPU permit
    ///
    /// This is the same as dropping the permit, but makes the intent clearer.
    pub fn release(self) {
        // Drop happens automatically
    }
}

/// Owned GPU permit that can be moved across async tasks
///
/// Unlike `GpuPermit`, this can be stored in structs and moved between tasks.
pub struct OwnedGpuPermit {
    _permit: OwnedSemaphorePermit,
}

impl OwnedGpuPermit {
    /// Explicitly releases the GPU permit
    pub fn release(self) {
        // Drop happens automatically
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_exclusive_access() {
        let coordinator = GpuCoordinator::new();

        // Acquire first permit
        let permit1 = coordinator.acquire().await;
        assert!(coordinator.is_busy());
        assert_eq!(coordinator.available_permits(), 0);

        // Try to acquire without waiting - should fail
        assert!(coordinator.try_acquire().is_none());

        // Release first permit
        drop(permit1);
        assert!(!coordinator.is_busy());
        assert_eq!(coordinator.available_permits(), 1);

        // Now we can acquire again
        let _permit2 = coordinator.acquire().await;
        assert!(coordinator.is_busy());
    }

    #[tokio::test]
    async fn test_sequential_access() {
        let coordinator = Arc::new(GpuCoordinator::new());
        let coordinator_clone = coordinator.clone();

        // First task acquires GPU
        let permit = coordinator.acquire().await;

        // Second task should wait
        let handle = tokio::spawn(async move {
            // This should wait until permit is released
            let _permit = coordinator_clone.acquire().await;
            "acquired"
        });

        // Give the spawned task time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Release the permit
        drop(permit);

        // Second task should complete now
        let result = timeout(Duration::from_secs(1), handle)
            .await
            .expect("Should complete in time")
            .expect("Task should succeed");

        assert_eq!(result, "acquired");
    }

    #[tokio::test]
    async fn test_owned_permit() {
        let coordinator = Arc::new(GpuCoordinator::new());

        let permit = coordinator.acquire_owned().await;
        assert!(coordinator.is_busy());

        // Owned permit can be moved
        let moved_permit = permit;
        assert!(coordinator.is_busy());

        drop(moved_permit);
        assert!(!coordinator.is_busy());
    }
}
