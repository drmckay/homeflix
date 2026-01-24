// Caching Layer
//
// This module provides multi-level caching implementations including:
// - In-memory cache (L1)
// - Database cache (L2)
// - Multi-level cache with eviction policies
// - TMDB-specific cache for external ID lookups

pub mod in_memory_cache;
pub mod database_cache;
pub mod multi_level_cache;
pub mod tmdb_cache;
pub mod image_cache;

pub use in_memory_cache::InMemoryCache;
pub use database_cache::DatabaseCache;
pub use multi_level_cache::MultiLevelCache;
pub use tmdb_cache::{TmdbCache, TmdbCacheEntry};
pub use image_cache::ImageCache;
