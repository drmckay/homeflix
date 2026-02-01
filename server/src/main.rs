mod application;
mod domain;
mod interfaces;
mod presentation;
mod shared;
mod infrastructure;

use axum::http::{header, Method};
use axum::{
    extract::FromRef,
    routing::{get, post, delete},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use crate::infrastructure::database::{ConnectionPool, ConnectionPoolConfig, initialize_schema};
use crate::shared::di::{ServiceRegistry, ServiceLifetime};

// Type alias for backward compatibility during migration
pub type DbPool = sqlx::Pool<sqlx::Sqlite>;

// Imports for DI
use crate::infrastructure::persistence::sqlite::{
    SqliteMediaRepository, SqliteSeriesRepository, SqliteCollectionRepository, SqliteCacheRepository,
    SqliteCreditsRepository,
};
use crate::infrastructure::external::tmdb::TmdbClient;
use crate::infrastructure::external::ffmpeg::FFprobeAdapter;
use crate::infrastructure::external::{WhisperAdapter, OllamaClient, FpcalcAdapter};
use crate::infrastructure::gpu::GpuCoordinator;
use crate::infrastructure::jobs::JobStore;
use crate::infrastructure::filesystem::WalkDirAdapter;
use crate::infrastructure::messaging::{InMemoryEventBus, PersistentEventBus};
use crate::infrastructure::event_sourcing::{event_store::EventStore, sqlite_event_persistence::SqliteEventPersistence};
use crate::infrastructure::cache::ImageCache;
use crate::domain::services::{DefaultIdentificationService, DefaultConfidenceService, TmdbCrossValidatorImpl};
use crate::application::{
    ScanLibraryUseCase, IdentifyMediaUseCase, StreamMediaUseCase, ManageSeriesUseCase
};
use crate::application::use_cases::get_recently_added::GetRecentlyAddedUseCase;
use crate::application::use_cases::generate_subtitle::GenerateSubtitleUseCase;
use crate::application::use_cases::batch_generate_subtitles::BatchGenerateSubtitlesUseCase;
use crate::application::handlers::{
    MediaIdentifiedHandler, ScanCompletedHandler, CollectionDetectedHandler,
    CacheInvalidationHandler, NotificationHandler, MetricsHandler,
    SubtitleGenerationHandler, ProgressTrackingHandler, StreamingHandler,
    CollectionManagementHandler, ThumbnailGenerationHandler, BackgroundTaskHandler,
};
use crate::interfaces::messaging::EventBus;
use crate::presentation::http::handlers::{
    media_handlers, series_handlers, streaming_handlers,
    collection_handlers, progress_handlers, search_handlers, proxy_handlers,
    subtitle_generation_handlers, health_handlers,
};
use crate::presentation::http::middleware::{auth, cors, logging};

// Import repository traits for handlers
use crate::domain::repositories::{MediaRepository, SeriesRepository, CollectionRepository, CreditsRepository};
use crate::interfaces::external_services::{VideoAnalyzer, TmdbService, TmdbCreditsFetcher};

/// Application state containing DI registry and core services
#[derive(Clone)]
struct AppState {
    registry: Arc<ServiceRegistry>,
    pool: DbPool,
    // Repositories
    media_repo: Arc<dyn MediaRepository>,
    series_repo: Arc<dyn SeriesRepository>,
    collection_repo: Arc<dyn CollectionRepository>,
    credits_repo: Arc<dyn CreditsRepository>,
    // External Services
    video_analyzer: Arc<dyn VideoAnalyzer>,
    tmdb_service: Arc<dyn TmdbService>,
    tmdb_credits: Arc<dyn TmdbCreditsFetcher + Send + Sync>,
    // Cache
    image_cache: Arc<ImageCache>,
    // Use Cases
    scan_use_case: Arc<ScanLibraryUseCase<InMemoryEventBus>>,
    identify_use_case: Arc<IdentifyMediaUseCase<InMemoryEventBus>>,
    stream_use_case: Arc<StreamMediaUseCase>,
    manage_series_use_case: Arc<ManageSeriesUseCase>,
    recently_added_use_case: Arc<GetRecentlyAddedUseCase>,
    generate_subtitle_use_case: Arc<GenerateSubtitleUseCase<InMemoryEventBus>>,
    batch_generate_subtitles_use_case: Arc<BatchGenerateSubtitlesUseCase>,
    // Job Management
    job_store: Arc<JobStore>,
    // Event Bus (for handlers that need it)
    event_bus: Arc<InMemoryEventBus>,
}

impl AppState {
    /// Create new application state with DI registry
    async fn new(pool: DbPool, config: &Config) -> anyhow::Result<Self> {
        let mut registry = ServiceRegistry::new();

        // Register database pool
        registry.register(pool.clone(), ServiceLifetime::Singleton);

        // Repositories
        let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
        let series_repo = Arc::new(SqliteSeriesRepository::new(pool.clone()));
        let collection_repo = Arc::new(SqliteCollectionRepository::new(pool.clone()));
        let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));
        let credits_repo = Arc::new(SqliteCreditsRepository::new(pool.clone()));

        // External Services
        let tmdb_client = Arc::new(TmdbClient::new(&config.tmdb_api_key, cache_repo.clone())?);
        let video_analyzer = Arc::new(FFprobeAdapter::new(std::time::Duration::from_secs(10)));
        let directory_walker = Arc::new(WalkDirAdapter::new());
        
        // Event Bus Setup (with event sourcing)
        // Initialize event store for persistence
        let event_persistence = Arc::new(SqliteEventPersistence::new(Arc::new(pool.clone())));
        let event_store = Arc::new(EventStore::new(event_persistence));
        
        // Create persistent event bus wrapper
        let inner_event_bus = Arc::new(InMemoryEventBus::new());
        let persistent_event_bus = Arc::new(PersistentEventBus::new(inner_event_bus.clone(), event_store));
        
        // For backward compatibility, use cases still use InMemoryEventBus type
        // But we intercept publishes through a custom wrapper or modify InMemoryEventBus
        // For now, we'll use the inner_event_bus directly and add persistence later via interceptor
        // TODO: Integrate PersistentEventBus properly with use cases
        let event_bus = inner_event_bus.clone();
        
        info!("Event bus initialized (event sourcing infrastructure ready)");

        // Domain Services
        let identification_service = Arc::new(DefaultIdentificationService::new());
        let confidence_service = Arc::new(DefaultConfidenceService::new());
        let tmdb_cross_validator = Arc::new(TmdbCrossValidatorImpl::new(tmdb_client.clone()));

        // Use Cases
        let scan_use_case = Arc::new(
            ScanLibraryUseCase::new(
                media_repo.clone(),
                series_repo.clone(),
                collection_repo.clone(),
                directory_walker.clone(),
                event_bus.clone(),
                identification_service.clone(),
                confidence_service.clone(),
            )
            .with_tmdb_service(tmdb_client.clone())
            .with_tmdb_cross_validator(tmdb_cross_validator)
            .with_video_analyzer(video_analyzer.clone())
        );

        let identify_use_case = Arc::new(IdentifyMediaUseCase::new(
            media_repo.clone(),
            tmdb_client.clone(),
            event_bus.clone(),
        ));

        let stream_use_case = Arc::new(StreamMediaUseCase::new(
            media_repo.clone(),
            video_analyzer.clone(),
        ));

        let manage_series_use_case = Arc::new(ManageSeriesUseCase::new(
            series_repo.clone(),
        ));

        let recently_added_use_case = Arc::new(GetRecentlyAddedUseCase::new(
            media_repo.clone(),
            series_repo.clone(),
        ));

        // Subtitle Generation Services
        let gpu_coordinator = Arc::new(GpuCoordinator::new());
        let job_store = Arc::new(JobStore::new());
        let fpcalc_adapter = Arc::new(FpcalcAdapter::new(
            std::time::Duration::from_secs(120),
        ));

        // Whisper adapter (optional - depends on environment)
        let whisper_model_path = std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| "/app/models/ggml-small.bin".to_string());
        let whisper_cli_path = std::env::var("WHISPER_CLI_PATH")
            .unwrap_or_else(|_| "whisper-cli".to_string());
        let whisper_adapter = Arc::new(WhisperAdapter::with_cli_path(
            std::path::PathBuf::from(&whisper_model_path),
            whisper_cli_path,
            std::time::Duration::from_secs(3600), // 1 hour timeout for long videos
        ));

        // Ollama client (optional - for translation)
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let ollama_model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "gemma3:4b".to_string());
        let ollama_client = Some(Arc::new(OllamaClient::new(&ollama_url, &ollama_model)));

        // Generate Subtitle Use Case
        let generate_subtitle_use_case = Arc::new(GenerateSubtitleUseCase::new(
            media_repo.clone(),
            whisper_adapter.clone(),
            ollama_client.clone(),
            fpcalc_adapter.clone(),
            gpu_coordinator.clone(),
            job_store.clone(),
            event_bus.clone(),
        ));

        // Batch Generate Subtitles Use Case
        let batch_generate_subtitles_use_case = Arc::new(BatchGenerateSubtitlesUseCase::new(
            media_repo.clone(),
            generate_subtitle_use_case.clone(),
            job_store.clone(),
            video_analyzer.clone(),
        ));

        info!(
            "Subtitle generation initialized: whisper_model={}, ollama_url={}",
            whisper_model_path, ollama_url
        );

        // Event Handlers - Create and subscribe to event bus
        {
            // MediaIdentifiedEvent handlers
            let media_identified_handler = Arc::new(MediaIdentifiedHandler::new(
                media_repo.clone(),
                cache_repo.clone(),
            ));
            event_bus.subscribe(media_identified_handler).await?;

            let metrics_handler: Arc<dyn crate::interfaces::messaging::EventHandler<crate::domain::events::MediaIdentifiedEvent>> = Arc::new(MetricsHandler::new());
            event_bus.subscribe(metrics_handler).await?;

            // ScanCompletedEvent handlers
            let scan_completed_handler = Arc::new(ScanCompletedHandler::new(
                media_repo.clone(),
            ));
            event_bus.subscribe(scan_completed_handler).await?;

            let notification_handler = Arc::new(NotificationHandler::new());
            event_bus.subscribe(notification_handler).await?;

            let metrics_handler_scan: Arc<dyn crate::interfaces::messaging::EventHandler<crate::domain::events::ScanCompletedEvent>> = Arc::new(MetricsHandler::new());
            event_bus.subscribe(metrics_handler_scan).await?;

            // CollectionDetectedEvent handler
            let collection_detected_handler = Arc::new(CollectionDetectedHandler::new(
                media_repo.clone(),
                collection_repo.clone(),
            ));
            event_bus.subscribe(collection_detected_handler).await?;

            // MediaVerifiedEvent handler
            let cache_invalidation_handler = Arc::new(CacheInvalidationHandler::new(
                cache_repo.clone(),
            ));
            event_bus.subscribe(cache_invalidation_handler).await?;

            // SubtitleGenerationEvent handlers
            let subtitle_generation_handler = Arc::new(SubtitleGenerationHandler::new());
            event_bus.subscribe::<crate::domain::events::SubtitleGenerationCompletedEvent>(
                subtitle_generation_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::SubtitleGenerationFailedEvent>(
                subtitle_generation_handler
            ).await?;

            // ProgressTrackingEvent handlers
            let progress_tracking_handler = Arc::new(ProgressTrackingHandler::new());
            event_bus.subscribe::<crate::domain::events::ProgressUpdatedEvent>(
                progress_tracking_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::MediaWatchedEvent>(
                progress_tracking_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::MediaUnwatchedEvent>(
                progress_tracking_handler
            ).await?;

            // StreamingEvent handlers
            let streaming_handler = Arc::new(StreamingHandler::new());
            event_bus.subscribe::<crate::domain::events::StreamStartedEvent>(
                streaming_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::StreamEndedEvent>(
                streaming_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::StreamErrorEvent>(
                streaming_handler
            ).await?;

            // CollectionManagementEvent handlers
            let collection_management_handler = Arc::new(CollectionManagementHandler::new());
            event_bus.subscribe::<crate::domain::events::CollectionCreatedEvent>(
                collection_management_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::CollectionUpdatedEvent>(
                collection_management_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::CollectionItemAddedEvent>(
                collection_management_handler
            ).await?;

            // ThumbnailGenerationEvent handler
            let thumbnail_generation_handler = Arc::new(ThumbnailGenerationHandler::new());
            event_bus.subscribe::<crate::domain::events::ThumbnailGeneratedEvent>(
                thumbnail_generation_handler
            ).await?;

            // BackgroundTaskEvent handlers
            let background_task_handler = Arc::new(BackgroundTaskHandler::new());
            event_bus.subscribe::<crate::domain::events::BackgroundScanScheduledEvent>(
                background_task_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::BackgroundScanStartedEvent>(
                background_task_handler.clone()
            ).await?;
            event_bus.subscribe::<crate::domain::events::BackgroundTaskCompletedEvent>(
                background_task_handler
            ).await?;

            info!("Event handlers registered successfully");
        }

        // Initialize Image Cache
        let image_cache = Arc::new(
            ImageCache::new(&config.data_dir)
                .map_err(|e| anyhow::anyhow!("Failed to initialize image cache: {}", e))?
        );
        info!("Image cache initialized at: {:?}", image_cache.cache_dir());

        // Note: In a full DI implementation, we would register these in registry and resolve them.
        // For simplicity and compiler safety here, we construct them manually and store in AppState.

        Ok(Self {
            registry: Arc::new(registry),
            pool,
            media_repo,
            series_repo,
            collection_repo,
            credits_repo,
            video_analyzer,
            tmdb_service: tmdb_client.clone(),
            tmdb_credits: tmdb_client,
            image_cache,
            scan_use_case,
            identify_use_case,
            stream_use_case,
            manage_series_use_case,
            recently_added_use_case,
            generate_subtitle_use_case,
            batch_generate_subtitles_use_case,
            job_store,
            event_bus: event_bus.clone(),
        })
    }
}

// Implement FromRef for sub-states
impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<dyn MediaRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.media_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SeriesRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.series_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn CollectionRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.collection_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn CreditsRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.credits_repo.clone()
    }
}

impl FromRef<AppState> for Arc<ScanLibraryUseCase<InMemoryEventBus>> {
    fn from_ref(state: &AppState) -> Self {
        state.scan_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<IdentifyMediaUseCase<InMemoryEventBus>> {
    fn from_ref(state: &AppState) -> Self {
        state.identify_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<StreamMediaUseCase> {
    fn from_ref(state: &AppState) -> Self {
        state.stream_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<ManageSeriesUseCase> {
    fn from_ref(state: &AppState) -> Self {
        state.manage_series_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<dyn VideoAnalyzer> {
    fn from_ref(state: &AppState) -> Self {
        state.video_analyzer.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TmdbService> {
    fn from_ref(state: &AppState) -> Self {
        state.tmdb_service.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TmdbCreditsFetcher + Send + Sync> {
    fn from_ref(state: &AppState) -> Self {
        state.tmdb_credits.clone()
    }
}

impl FromRef<AppState> for Arc<GetRecentlyAddedUseCase> {
    fn from_ref(state: &AppState) -> Self {
        state.recently_added_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<GenerateSubtitleUseCase<InMemoryEventBus>> {
    fn from_ref(state: &AppState) -> Self {
        state.generate_subtitle_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<BatchGenerateSubtitlesUseCase> {
    fn from_ref(state: &AppState) -> Self {
        state.batch_generate_subtitles_use_case.clone()
    }
}

impl FromRef<AppState> for Arc<JobStore> {
    fn from_ref(state: &AppState) -> Self {
        state.job_store.clone()
    }
}

impl FromRef<AppState> for Arc<ImageCache> {
    fn from_ref(state: &AppState) -> Self {
        state.image_cache.clone()
    }
}

impl FromRef<AppState> for Option<Arc<InMemoryEventBus>> {
    fn from_ref(state: &AppState) -> Self {
        Some(state.event_bus.clone())
    }
}

struct Config {
    database_url: String,
    media_dir: String,
    data_dir: String,
    port: u16,
    tmdb_api_key: String,
    /// Interval between library scans in seconds (0 to disable)
    scan_interval_secs: u64,
}

impl Config {
    /// Extracts the data directory from DATABASE_URL
    ///
    /// Examples:
    /// - `sqlite:data.db?mode=rwc` -> `./data` (or current dir)
    /// - `sqlite:/data/data.db?mode=rwc` -> `/data`
    /// - `sqlite:./data/data.db?mode=rwc` -> `./data`
    fn extract_data_dir(database_url: &str) -> String {
        // Remove sqlite: prefix
        let path_part = database_url
            .strip_prefix("sqlite:")
            .unwrap_or(database_url)
            .split('?')
            .next()
            .unwrap_or("");

        if path_part.is_empty() {
            return "./data".to_string();
        }

        let db_path = std::path::Path::new(path_part);
        
        // If absolute path (starts with /), use parent directory
        if db_path.is_absolute() {
            if let Some(parent) = db_path.parent() {
                return parent.to_string_lossy().to_string();
            }
            return "/data".to_string();
        }

        // For relative paths, use parent directory or default to ./data
        if let Some(parent) = db_path.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if parent_str.is_empty() || parent_str == "." {
                return "./data".to_string();
            }
            return parent_str;
        }

        "./data".to_string()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Config
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string());
    let data_dir = Config::extract_data_dir(&database_url);
    let config = Config {
        database_url: database_url.clone(),
        media_dir: std::env::var("MEDIA_DIR").expect("MEDIA_DIR must be set"),
        data_dir: data_dir.clone(),
        port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse()?,
        tmdb_api_key: std::env::var("TMDB_API_KEY").unwrap_or_default(), // Should be required in prod
        scan_interval_secs: std::env::var("SCAN_INTERVAL_SECS")
            .unwrap_or_else(|_| "3600".to_string()) // Default: 1 hour
            .parse()
            .unwrap_or(3600),
    };
    
    info!("Data directory: {}", config.data_dir);

    // Initialize presets directory
    let presets_dir = std::path::Path::new(&config.data_dir).join("presets");
    
    // Try to find builtin presets - check multiple possible locations
    let builtin_presets_path = if std::path::Path::new("server/presets").exists() {
        std::path::Path::new("server/presets").to_path_buf()
    } else if std::path::Path::new("presets").exists() {
        std::path::Path::new("presets").to_path_buf()
    } else {
        // If running from different directory, try to find presets relative to executable
        // This is a best-effort approach
        std::path::Path::new("server/presets").to_path_buf()
    };

    match crate::infrastructure::presets::PresetLoader::initialize_presets_directory(
        &presets_dir,
        &builtin_presets_path,
    ) {
        Ok(copied) => {
            if copied > 0 {
                info!("Initialized presets directory: {} presets copied", copied);
            } else {
                info!("Presets directory ready: {:?}", presets_dir);
            }
        }
        Err(e) => {
            warn!("Failed to initialize presets directory: {}", e);
        }
    }

    // Load presets
    let presets = match crate::infrastructure::presets::PresetLoader::load_from_directory(&presets_dir) {
        Ok(p) => {
            info!("Loaded {} presets from {:?}", p.len(), presets_dir);
            p
        }
        Err(e) => {
            warn!("Failed to load presets: {}", e);
            Vec::new()
        }
    };

    // Initialize Database with new infrastructure
    let pool_config = ConnectionPoolConfig::new(config.database_url.clone());
    let connection_pool = ConnectionPool::create(pool_config).await
        .map_err(|e| anyhow::anyhow!("Failed to create connection pool: {}", e))?;
    let pool = connection_pool.inner().clone();

    // Initialize database schema
    initialize_schema(&pool).await?;
    info!("Database initialized with new infrastructure");

    // Initialize Application State
    let state = AppState::new(pool.clone(), &config).await?;

    // Start background scanner if interval > 0
    if config.scan_interval_secs > 0 {
        let scan_use_case = state.scan_use_case.clone();
        let event_bus_for_collection = state.event_bus.clone();
        let collection_manager = Arc::new(crate::application::services::CollectionManager::new(
            state.media_repo.clone(),
            state.series_repo.clone(),
            state.collection_repo.clone(),
            state.tmdb_service.clone(),
            event_bus_for_collection,
        ));
        let media_dir = config.media_dir.clone();
        let scan_interval = std::time::Duration::from_secs(config.scan_interval_secs);
        let presets_dir_clone = presets_dir.clone();

        info!(
            "Background scanner enabled: scanning {} every {} seconds",
            media_dir, config.scan_interval_secs
        );

        let event_bus_for_background = state.event_bus.clone();
        tokio::spawn(async move {
            // Initial scan on startup (with small delay to let server start)
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info!("Running initial library scan at: {}", media_dir);

            // Publish background scan scheduled event
            let scheduled_event = crate::domain::events::BackgroundScanScheduledEvent::new(
                media_dir.clone(),
                chrono::Utc::now(),
                config.scan_interval_secs,
            );
            if let Err(e) = event_bus_for_background.publish(scheduled_event).await {
                tracing::warn!("Failed to publish background scan scheduled event: {}", e);
            }

            loop {
                // Publish background scan started event
                let started_event = crate::domain::events::BackgroundScanStartedEvent::new(
                    media_dir.clone(),
                );
                if let Err(e) = event_bus_for_background.publish(started_event).await {
                    tracing::warn!("Failed to publish background scan started event: {}", e);
                }

                match scan_use_case.execute(&media_dir).await {
                    Ok(result) => {
                        info!(
                            "Library scan completed: {} files processed, {} identified, {} failed",
                            result.processed_count, result.identified_count, result.failed_count
                        );

                        // Publish background task completed event
                        let completed_event = crate::domain::events::BackgroundTaskCompletedEvent::new(
                            "library_scan".to_string(),
                            None,
                            true,
                            Some(format!("{} files processed, {} identified", result.processed_count, result.identified_count)),
                        );
                        if let Err(e) = event_bus_for_background.publish(completed_event).await {
                            tracing::warn!("Failed to publish background task completed event: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Library scan failed: {}", e);

                        // Publish background task completed event (failed)
                        let completed_event = crate::domain::events::BackgroundTaskCompletedEvent::new(
                            "library_scan".to_string(),
                            None,
                            false,
                            Some(e.to_string()),
                        );
                        if let Err(e) = event_bus_for_background.publish(completed_event).await {
                            tracing::warn!("Failed to publish background task completed event: {}", e);
                        }
                    }
                }

                // Post-scan: create/update preset franchise collections (Star Trek, Stargate, MCU)
                info!("Creating/updating preset franchise collections...");
                // Reload presets in case they were updated
                let presets = match crate::infrastructure::presets::PresetLoader::load_from_directory(&presets_dir_clone) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Failed to reload presets: {}", e);
                        Vec::new()
                    }
                };
                match collection_manager.create_preset_collections(presets).await {
                    Ok(stats) => {
                        info!(
                            "Preset collections complete: {} created, {}/{} items available",
                            stats.collections_created, stats.available_items, stats.total_items
                        );
                    }
                    Err(e) => {
                        tracing::error!("Preset collections failed: {}", e);
                    }
                }

                // Post-scan: detect TMDB-based movie collections
                info!("Running TMDB collection detection for movies...");
                match collection_manager.detect_and_create_collections().await {
                    Ok(stats) => {
                        if stats.total_collections > 0 || stats.total_media_linked > 0 {
                            info!(
                                "TMDB collection detection complete: {} collections, {} media linked",
                                stats.total_collections, stats.total_media_linked
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("TMDB collection detection failed: {}", e);
                    }
                }

                // Wait for next scan interval
                tokio::time::sleep(scan_interval).await;
            }
        });
    } else {
        info!("Background scanner disabled (SCAN_INTERVAL_SECS=0)");
    }

    // Routes
    let app = Router::new()
        // Health Check (must be before middleware to avoid auth requirement)
        .route("/health", get(health_handlers::health_check))
        
        // V2 Routes - Media
        .route("/v2/media", get(media_handlers::list_grouped_library))
        .route("/v2/media/recent", get(media_handlers::list_recently_added))
        .route("/v2/media/all", get(media_handlers::list_media))
        .route("/v2/media/:id", get(media_handlers::get_media))
        .route("/v2/media/:id/tracks", get(media_handlers::get_media_tracks))
        .route("/v2/media/:id/credits", get(media_handlers::get_media_credits))
        .route("/v2/media/:id/similar", get(media_handlers::get_media_similar))
        .route("/v2/media/:id/identify", post(media_handlers::manual_identify))
        .route("/v2/scan", post(media_handlers::scan_library))

        // V2 Routes - Series
        .route("/v2/series", get(series_handlers::list_series))
        .route("/v2/series/:id", get(series_handlers::get_series))

        // V2 Routes - Collections
        .route("/v2/collections", get(collection_handlers::list_collections))
        .route("/v2/collections/:id", get(collection_handlers::get_collection))

        // V2 Routes - Watch Progress
        .route("/v2/progress/:id", get(progress_handlers::get_progress).post(progress_handlers::update_progress))
        .route("/v2/progress/:id/watched", post(progress_handlers::mark_watched).delete(progress_handlers::mark_unwatched))

        // V2 Routes - Search
        .route("/v2/search", get(search_handlers::search_media))
        .route("/v2/search/series", get(search_handlers::search_series))

        // V2 Routes - Streaming
        .route("/v2/stream/:id", get(streaming_handlers::stream_media))
        .route("/v2/stream/web/:id", get(streaming_handlers::stream_web))
        .route("/v2/stream/diagnostic/:id", get(streaming_handlers::stream_diagnostic))
        .route("/v2/thumbnail/:id", get(streaming_handlers::generate_thumbnail))
        .route("/v2/subtitles/:media_id/:index", get(streaming_handlers::get_subtitle))

        // V2 Routes - Subtitle Generation (Whisper + Ollama)
        .route("/v2/subtitles/capabilities", get(subtitle_generation_handlers::get_capabilities))
        .route("/v2/subtitles/active", get(subtitle_generation_handlers::get_active_jobs))
        .route("/v2/subtitles/:media_id/generate", post(subtitle_generation_handlers::generate_subtitle))
        .route("/v2/subtitles/jobs/:job_id", get(subtitle_generation_handlers::get_job_status).delete(subtitle_generation_handlers::cancel_job))
        .route("/v2/subtitles/batch/generate", post(subtitle_generation_handlers::batch_generate_subtitles))
        .route("/v2/subtitles/batch/jobs/:job_id", get(subtitle_generation_handlers::get_batch_job_status).delete(subtitle_generation_handlers::cancel_batch_job))

        // V2 Routes - Proxy (for TMDB images - CORS bypass)
        .route("/v2/images/proxy", get(proxy_handlers::proxy_image))

        // Apply Middleware
        .layer(axum::middleware::from_fn(auth::auth_middleware))
        .layer(axum::middleware::from_fn(logging::logging_middleware))
        .layer(cors::cors_layer())

        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
