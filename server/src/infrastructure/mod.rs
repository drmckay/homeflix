// Infrastructure Layer
//
// This module contains all infrastructure implementations including:
// - Persistence (SQLite repositories)
// - External service adapters (TMDB, FFmpeg)
// - Filesystem adapters
// - Messaging (Event bus)
// - Caching layer
// - Database connection pooling

pub mod persistence;
pub mod external;
pub mod filesystem;
pub mod messaging;
pub mod cache;
pub mod event_sourcing;
pub mod database;
pub mod subtitle;
pub mod gpu;
pub mod jobs;
pub mod presets;

pub use persistence::sqlite::*;
pub use external::tmdb::*;
pub use external::ffmpeg::*;
pub use filesystem::*;
pub use messaging::*;
pub use cache::in_memory_cache::*;
pub use cache::database_cache::*;
pub use cache::multi_level_cache::*;
pub use database::*;
pub use subtitle::*;
pub use gpu::*;
pub use jobs::*;
