//! Shared error types for the application
//!
//! This module defines all error types used across the application,
//! following domain-driven design principles.

use thiserror::Error;

/// Domain errors - errors that occur in the domain layer
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Repository errors - errors that occur during data access
#[derive(Debug, Clone, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() {
                    RepositoryError::Duplicate(db_err.message().to_string())
                } else if db_err.is_check_violation() {
                    RepositoryError::ConstraintViolation(db_err.message().to_string())
                } else {
                    RepositoryError::Database(db_err.message().to_string())
                }
            }
            sqlx::Error::PoolTimedOut => {
                RepositoryError::Connection("Connection pool timeout".to_string())
            }
            sqlx::Error::PoolClosed => {
                RepositoryError::Connection("Connection pool closed".to_string())
            }
            _ => RepositoryError::Database(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for RepositoryError {
    fn from(err: serde_json::Error) -> Self {
        RepositoryError::Serialization(err.to_string())
    }
}

/// TMDB service errors
#[derive(Debug, Clone, Error)]
pub enum TmdbError {
    #[error("API error: {0}")]
    ApiError(u16),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<reqwest::Error> for TmdbError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TmdbError::Network("Request timeout".to_string())
        } else if err.is_connect() {
            TmdbError::Network("Connection failed".to_string())
        } else {
            TmdbError::Network(err.to_string())
        }
    }
}

impl From<serde_json::Error> for TmdbError {
    fn from(err: serde_json::Error) -> Self {
        TmdbError::Deserialization(err.to_string())
    }
}

/// Video analyzer errors
#[derive(Debug, Error)]
pub enum VideoAnalyzerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFprobe execution failed: {0}")]
    ExecutionFailed(String),

    #[error("FFprobe not found")]
    FfprobeNotFound,

    #[error("Invalid output: {0}")]
    InvalidOutput(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Not a video file: {0}")]
    NotAVideo(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Thumbnail generator errors
#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFmpeg execution failed: {0}")]
    ExecutionFailed(String),

    #[error("FFmpeg not found")]
    FfmpegNotFound,

    #[error("Invalid output: {0}")]
    InvalidOutput(String),

    #[error("Timestamp out of range: {0}")]
    TimestampOutOfRange(String),

    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),

    #[error("Not a video file: {0}")]
    NotAVideo(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Image encoding error: {0}")]
    ImageEncoding(String),
}

/// Filesystem errors
#[derive(Debug, Error)]
pub enum FilesystemError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Not a file: {0}")]
    NotAFile(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("Walk error: {0}")]
    WalkError(String),

    #[error("UTF-8 encoding error: {0}")]
    Utf8Error(String),
}

/// Messaging/Event bus errors
#[derive(Debug, Clone, Error)]
pub enum MessagingError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Handler error: {0}")]
    HandlerError(String),

    #[error("Handler timeout: {0}")]
    HandlerTimeout(String),

    #[error("No subscribers for event: {0}")]
    NoSubscribers(String),

    #[error("Event bus error: {0}")]
    EventBusError(String),
}

impl From<serde_json::Error> for MessagingError {
    fn from(err: serde_json::Error) -> Self {
        MessagingError::Serialization(err.to_string())
    }
}

/// Event sourcing errors
#[derive(Debug, Error)]
pub enum EventSourcingError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),
}

impl From<serde_json::Error> for EventSourcingError {
    fn from(err: serde_json::Error) -> Self {
        EventSourcingError::Serialization(err.to_string())
    }
}

impl From<sqlx::Error> for EventSourcingError {
    fn from(err: sqlx::Error) -> Self {
        EventSourcingError::Persistence(err.to_string())
    }
}

/// Subtitle processing errors
#[derive(Debug, Error)]
pub enum SubtitleError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Subtitle file not found: {0}")]
    FileNotFound(String),

    #[error("Invalid subtitle format: {0}")]
    InvalidFormat(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}

/// Speech-to-text (Whisper) errors
#[derive(Debug, Error)]
pub enum SpeechToTextError {
    #[error("Whisper not found - please install whisper.cpp")]
    WhisperNotFound,

    #[error("Audio extraction failed: {0}")]
    AudioExtractionFailed(String),

    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Translation (Ollama) errors
#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Translation failed: {0}")]
    TranslationFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Audio fingerprinting (fpcalc/Chromaprint) errors
#[derive(Debug, Error)]
pub enum FingerprintError {
    #[error("fpcalc not found - please install libchromaprint-tools")]
    FpcalcNotFound,

    #[error("Audio extraction failed: {0}")]
    AudioExtractionFailed(String),

    #[error("Fingerprint generation failed: {0}")]
    FingerprintFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Batch job errors
#[derive(Debug, Clone, Error)]
pub enum JobError {
    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Job failed: {0}")]
    Failed(String),

    #[error("Job cancelled: {0}")]
    Cancelled(String),
}

/// Preset loading errors
#[derive(Debug, Error)]
pub enum PresetLoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error in {0}: {1}")]
    YamlParse(String, String),

    #[error("Validation error in {0}: {1}")]
    Validation(String, String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
}

/// Application errors - errors that occur in the application layer
#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("TMDB error: {0}")]
    Tmdb(#[from] TmdbError),

    #[error("Filesystem error: {0}")]
    Filesystem(#[from] FilesystemError),

    #[error("Messaging error: {0}")]
    Messaging(#[from] MessagingError),

    #[error("Event sourcing error: {0}")]
    EventSourcing(#[from] EventSourcingError),

    #[error("Video analyzer error: {0}")]
    VideoAnalyzer(#[from] VideoAnalyzerError),

    #[error("Thumbnail generator error: {0}")]
    Thumbnail(#[from] ThumbnailError),

    #[error("Subtitle error: {0}")]
    Subtitle(#[from] SubtitleError),

    #[error("Speech-to-text error: {0}")]
    SpeechToText(#[from] SpeechToTextError),

    #[error("Translation error: {0}")]
    Translation(#[from] TranslationError),

    #[error("Fingerprint error: {0}")]
    Fingerprint(#[from] FingerprintError),

    #[error("Job error: {0}")]
    Job(#[from] JobError),

    #[error("Preset load error: {0}")]
    PresetLoad(#[from] PresetLoadError),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
