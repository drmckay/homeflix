//! Unit tests for ScanLibraryUseCase
//!
//! Tests cover:
//! - Library scanning orchestration
//! - Parallel file processing
//! - Error handling
//! - Event publishing

use std::sync::Arc;
use mockall::mock;
use homeflixd::application::use_cases::ScanLibraryUseCase;
use homeflixd::domain::repositories::MediaRepository;
use homeflixd::domain::services::IdentificationService;
use homeflixd::domain::services::ConfidenceService;
use homeflixd::interfaces::filesystem::DirectoryWalker;
use homeflixd::interfaces::messaging::EventBus;
use homeflixd::shared::error::ApplicationError;

#[mock]
trait MockMediaRepository: MediaRepository {}
#[mock]
trait MockIdentificationService: IdentificationService {}
#[mock]
trait MockConfidenceService: ConfidenceService {}
#[mock]
trait MockDirectoryWalker: DirectoryWalker {}
#[mock]
trait MockEventBus: EventBus {}

#[tokio::test]
async fn test_scan_library_success() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_event_bus
        .expect_publish()
        .times(2) // Start and complete events
        .returning(Ok(()));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_scan_library_walk_error() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Err(homeflixd::shared::error::FilesystemError::IoError(
            std::io::Error::new(std::io::ErrorKind::NotFound, "not found").into(),
        )));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_scan_library_identification_error() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_identification
        .expect_identify_content()
        .returning(Err(homeflixd::shared::error::DomainError::InvalidInput(
            "Test error".into(),
        )));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_scan_library_save_error() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_repo
        .expect_save()
        .returning(Err(homeflixd::shared::error::RepositoryError::DatabaseError(
            "Test error".into(),
        )));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_scan_library_event_publish_error() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_event_bus
        .expect_publish()
        .returning(Err(homeflixd::shared::error::MessagingError::PublishError(
            "Test error".into(),
        )));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_scan_library_returns_result() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_event_bus
        .expect_publish()
        .returning(Ok(()));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await
        .expect("Should return result");

    assert_eq!(result.processed_count, 0);
    assert_eq!(result.identified_count, 0);
    assert_eq!(result.failed_count, 0);
    assert!(result.duration_secs >= 0);
}

#[tokio::test]
async fn test_scan_library_counts_processed() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_repo
        .expect_find_by_path()
        .times(3)
        .returning(Ok(None));

    mock_repo
        .expect_save()
        .times(3)
        .returning(Ok(1));

    mock_event_bus
        .expect_publish()
        .times(4) // 3 media identified + 1 scan complete
        .returning(Ok(()));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await
        .expect("Should return result");

    assert_eq!(result.processed_count, 3);
    assert_eq!(result.identified_count, 3);
}

#[tokio::test]
async fn test_scan_library_counts_failed() {
    let mut mock_repo = MockMediaRepository::new();
    let mut mock_identification = MockIdentificationService::new();
    let mut mock_confidence = MockConfidenceService::new();
    let mut mock_walker = MockDirectoryWalker::new();
    let mut mock_event_bus = MockEventBus::new();

    // Setup expectations - first 2 succeed, 3rd fails
    mock_walker
        .expect_walk()
        .returning(Ok(vec![]));

    mock_identification
        .expect_identify_content()
        .times(3)
        .returning(Ok(create_test_identification_result()));

    mock_repo
        .expect_find_by_path()
        .times(3)
        .returning(Ok(None));

    mock_repo
        .expect_save()
        .times(2)
        .returning(Ok(1));

    mock_event_bus
        .expect_publish()
        .times(4)
        .returning(Ok(()));

    let use_case = ScanLibraryUseCase::new(
        Arc::new(mock_repo),
        Arc::new(mock_identification),
        Arc::new(mock_confidence),
        Arc::new(mock_walker),
        Arc::new(mock_event_bus),
    );

    let result = use_case.execute("/test/path").await
        .expect("Should return result");

    assert_eq!(result.processed_count, 3);
    assert_eq!(result.identified_count, 2);
    assert_eq!(result.failed_count, 1);
}

fn create_test_identification_result() -> homeflixd::domain::value_objects::IdentificationResult {
    use homeflixd::domain::value_objects::{MediaType, ConfidenceScore, MatchStrategy};

    homeflixd::domain::value_objects::IdentificationResult {
        media_type: MediaType::Movie,
        title: "Test Movie".to_string(),
        season: None,
        episode: None,
        confidence: ConfidenceScore::new(0.75).expect("Should create score"),
        strategy: MatchStrategy::FilenameOnly,
    }
}
