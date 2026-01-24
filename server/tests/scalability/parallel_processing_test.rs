//! Tests for Parallel File Processing (Phase 8, Task 8.1)
//!
//! Tests the enhanced parallel file processing in scan_library use case

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use homeflixd::application::use_cases::scan_library::{
    ScanLibraryUseCase, ScanResult, ScanProgress, ProcessResult,
};
use homeflixd::domain::entities::Media;
use homeflixd::domain::repositories::MediaRepository;
use homeflixd::domain::value_objects::{MediaType, ConfidenceScore};
use homeflixd::interfaces::filesystem::DirectoryWalker;
use homeflixd::interfaces::messaging::EventBus;
use homeflixd::shared::error::ApplicationError;

// Mock implementations for testing

struct MockMediaRepository {
    media: Mutex<Vec<Media>>,
}

struct MockDirectoryWalker {
    entries: Vec<homeflixd::interfaces::filesystem::WalkEntry>,
}

struct MockEventBus {
    events: Mutex<Vec<homeflixd::domain::events::MediaIdentifiedEvent>>,
}

#[async_trait::async_trait]
impl MediaRepository for MockMediaRepository {
    async fn find_by_id(&self, _id: i64) -> Result<Option<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.get(0).cloned())
    }

    async fn find_by_path(&self, _path: &str) -> Result<Option<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.get(0).cloned())
    }

    async fn find_all(&self) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }

    async fn find_by_type(&self, _media_type: MediaType) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }

    async fn find_by_series(&self, _series_id: i64) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }

    async fn find_by_season(&self, _series_id: i64, _season: i32) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }

    async fn find_unverified(&self) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(Vec::new())
    }

    async fn find_by_confidence(&self, _min_score: ConfidenceScore) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(Vec::new())
    }

    async fn save(&self, media: &Media) -> Result<i64, homeflixd::shared::error::RepositoryError> {
        let mut media_list = self.media.lock().await;
        media_list.push(media.clone());
        Ok(media_list.len() as i64)
    }

    async fn update(&self, media: &Media) -> Result<(), homeflixd::shared::error::RepositoryError> {
        let mut media_list = self.media.lock().await;
        if let Some(existing) = media_list.iter_mut().find(|m| m.id == media.id) {
            *existing = media.clone();
        } else {
            media_list.push(media.clone());
        }
        Ok(())
    }

    async fn delete(&self, _id: i64) -> Result<(), homeflixd::shared::error::RepositoryError> {
        Ok(())
    }

    async fn count(&self) -> Result<i64, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.len() as i64)
    }

    async fn count_by_type(&self, _media_type: MediaType) -> Result<i64, homeflixd::shared::error::RepositoryError> {
        Ok(0)
    }

    async fn exists_by_path(&self, _path: &str) -> Result<bool, homeflixd::shared::error::RepositoryError> {
        Ok(true)
    }

    async fn update_progress(&self, _media_id: i64, _position: i64, _watched: bool) -> Result<(), homeflixd::shared::error::RepositoryError> {
        Ok(())
    }

    async fn find_recent(&self, _limit: usize) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }

    async fn find_watched(&self) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(Vec::new())
    }

    async fn find_unwatched(&self) -> Result<Vec<Media>, homeflixd::shared::error::RepositoryError> {
        Ok(self.media.lock().await.clone())
    }
}

#[async_trait::async_trait]
impl DirectoryWalker for MockDirectoryWalker {
    async fn walk(&self, _root: &std::path::Path) -> Result<Vec<homeflixd::interfaces::filesystem::WalkEntry>, homeflixd::shared::error::FilesystemError> {
        Ok(self.entries.clone())
    }

    async fn walk_videos(&self, _root: &std::path::Path) -> Result<Vec<homeflixd::interfaces::filesystem::WalkEntry>, homeflixd::shared::error::FilesystemError> {
        Ok(self.entries.clone())
    }
}

#[async_trait::async_trait]
impl EventBus for MockEventBus {
    async fn publish<T>(&self, event: T) -> Result<(), homeflixd::shared::error::MessagingError> 
    where
        T: homeflixd::interfaces::messaging::DomainEvent + Send + Sync + 'static
    {
        let mut events = self.events.lock().await;
        events.push(event);
        Ok(())
    }
}

fn create_test_media(id: i64, path: &str) -> Media {
    Media {
        id: Some(id),
        file_path: path.to_string(),
        media_type: MediaType::Movie,
        title: "Test Movie".to_string(),
        year: Some(2020),
        tmdb_id: Some(12345),
        season: None,
        episode: None,
        duration_seconds: Some(7200),
        resolution: Some("1080p".to_string()),
        confidence_score: ConfidenceScore::new(0.9).unwrap(),
        verification_status: homeflixd::domain::value_objects::VerificationStatus::Verified,
        identification_strategy: "filename".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ..Default::default()
    }
}

fn create_test_entry(path: &str) -> homeflixd::interfaces::filesystem::WalkEntry {
    homeflixd::interfaces::filesystem::WalkEntry {
        path: std::path::PathBuf::from(path),
        is_file: true,
        is_dir: false,
        depth: 0,
        file_size: Some(1024 * 1024 * 1024), // 1GB
    }
}

#[tokio::test]
async fn test_scan_library_parallel_processing() {
    // Setup mocks
    let media_repo = Arc::new(MockMediaRepository {
        media: Mutex::new(Vec::new()),
    });

    let directory_walker = Arc::new(MockDirectoryWalker {
        entries: vec![
            create_test_entry("/path/to/movie1.mkv"),
            create_test_entry("/path/to/movie2.mkv"),
            create_test_entry("/path/to/movie3.mkv"),
        ],
    });

    let event_bus: Arc::new(MockEventBus {
        events: Mutex::new(Vec::new()),
    });

    // Create use case
    let use_case = ScanLibraryUseCase::new(
        Arc::clone(&media_repo),
        Arc::clone(&directory_walker),
        Arc::clone(&event_bus),
    )
    .with_max_concurrent(2)
    .with_force_rescan(true);

    // Execute scan
    let result = use_case.execute("/path/to").await.unwrap();

    // Verify results
    assert_eq!(result.processed_count, 3);
    assert_eq!(result.identified_count, 3);
    assert_eq!(result.failed_count, 0);
    assert_eq!(result.skipped_count, 0);
    assert!(result.scan_path, "/path/to");
    assert!(result.files_per_second > 0.0);
}

#[tokio::test]
async fn test_scan_library_with_progress_callback() {
    let media_repo = Arc::new(MockMediaRepository {
        media: Mutex::new(Vec::new()),
    });

    let directory_walker = Arc::new(MockDirectoryWalker {
        entries: vec![
            create_test_entry("/path/to/movie1.mkv"),
            create_test_entry("/path/to/movie2.mkv"),
            create_test_entry("/path/to/movie3.mkv"),
        ],
    });

    let event_bus = Arc::new(MockEventBus {
        events: Mutex::new(Vec::new()),
    });

    let progress_updates = Arc::new(Mutex::new(Vec::new()));

    // Create progress callback
    let callback = Arc::new({
        let updates = Arc::clone(&progress_updates);
        move |progress: ScanProgress| {
            updates.lock().unwrap().push(progress);
        }
    });

    // Create use case with progress callback
    let use_case = ScanLibraryUseCase::new(
        Arc::clone(&media_repo),
        Arc::clone(&directory_walker),
        Arc::clone(&event_bus),
    )
    .with_max_concurrent(2)
    .with_progress_callback(callback);

    // Execute scan
    let result = use_case.execute("/path/to").await.unwrap();

    // Verify progress was called
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty());
    assert!(updates.len() >= 3); // At least start, middle, and end

    // Verify progress values
    let final_progress = updates.last().unwrap();
    assert_eq!(final_progress.processed, 3);
    assert_eq!(final_progress.total, 3);
    assert_eq!(final_progress.percentage, 100.0);
}

#[tokio::test]
async fn test_scan_library_rescan_threshold() {
    let media_repo = Arc::new(MockMediaRepository {
        media: Mutex::new(vec![
            create_test_media(1, "/path/to/movie1.mkv"),
            create_test_media(2, "/path/to/movie2.mkv"),
        ]),
    });

    let directory_walker = Arc::new(MockDirectoryWalker {
        entries: vec![
            create_test_entry("/path/to/movie1.mkv"),
            create_test_entry("/path/to/movie2.mkv"),
        ],
    });

    let event_bus = Arc::new(MockEventBus {
        events: Mutex::new(Vec::new()),
    });

    // Create use case with rescan threshold
    let use_case = ScanLibraryUseCase::new(
        Arc::clone(&media_repo),
        Arc::clone(&directory_walker),
        Arc::clone(&event_bus),
    )
    .with_max_concurrent(2)
    .with_rescan_threshold(0.95);

    // Execute scan - should skip high confidence media
    let result = use_case.execute("/path/to").await.unwrap();

    // Verify results - both should be skipped
    assert_eq!(result.processed_count, 2);
    assert_eq!(result.identified_count, 0);
    assert_eq!(result.skipped_count, 2);
    assert_eq!(result.failed_count, 0);
}

#[tokio::test]
async fn test_scan_library_empty_directory() {
    let media_repo = Arc::new(MockMediaRepository {
        media: Mutex::new(Vec::new()),
    });

    let directory_walker = Arc::new(MockDirectoryWalker {
        entries: vec![],
    });

    let event_bus = Arc::new(MockEventBus {
        events: Mutex::new(Vec::new()),
    });

    let use_case = ScanLibraryUseCase::new(
        Arc::clone(&media_repo),
        Arc::clone(&directory_walker),
        Arc::clone(&event_bus),
    );

    // Execute scan on empty directory
    let result = use_case.execute("/path/to").await.unwrap();

    // Verify results
    assert_eq!(result.processed_count, 0);
    assert_eq!(result.identified_count, 0);
    assert_eq!(result.failed_count, 0);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.duration_secs, 0);
}

#[tokio::test]
fn test_scan_progress_new() {
    let progress = ScanProgress::new(100);
    
    assert_eq!(progress.total, 100);
    assert_eq!(progress.processed, 0);
    assert_eq!(progress.identified, 0);
    assert_eq!(progress.failed, 0);
    assert_eq!(progress.skipped, 0);
    assert_eq!(progress.percentage, 0.0);
    assert!(progress.estimated_seconds_remaining.is_none());
}

#[tokio::test]
fn test_scan_progress_update() {
    let mut progress = ScanProgress::new(100);
    
    progress.update(&ProcessResult::Identified(1));
    assert_eq!(progress.processed, 1);
    assert_eq!(progress.identified, 1);
    assert_eq!(progress.percentage, 1.0);
    
    progress.update(&ProcessResult::Skipped);
    assert_eq!(progress.processed, 2);
    assert_eq!(progress.skipped, 1);
    assert_eq!(progress.percentage, 2.0);
    
    progress.update(&ProcessResult::Failed("error".to_string()));
    assert_eq!(progress.processed, 3);
    assert_eq!(progress.failed, 1);
    assert_eq!(progress.percentage, 3.0);
}

#[tokio::test]
fn test_scan_progress_time_remaining() {
    let mut progress = ScanProgress::new(100);
    
    progress.processed = 50;
    progress.update_time_remaining(60); // 60 seconds for 50 files
    
    assert!(progress.estimated_seconds_remaining.is_some());
    let remaining = progress.estimated_seconds_remaining.unwrap();
    // Should be approximately 60 seconds for remaining 50 files
    assert!((remaining - 60.0).abs() < 5.0);
}
