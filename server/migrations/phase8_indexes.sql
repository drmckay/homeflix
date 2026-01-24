-- Phase 8: Scalability Enhancements - Database Indexes
--
-- This migration adds indexes to optimize query performance
-- for the HomeFlixD application.
--
-- Performance improvements:
-- - Faster lookups by file_path (unique constraint)
-- - Faster filtering by media_type
-- - Faster queries by series_id and season
-- - Faster confidence-based queries
-- - Faster watch status queries
-- - Faster date-based queries

-- Index on file_path for fast lookups
-- This is the most common query pattern during scanning
CREATE INDEX IF NOT EXISTS idx_media_file_path 
    ON media(file_path);

-- Index on media_type for filtering
-- Used when listing movies vs TV shows
CREATE INDEX IF NOT EXISTS idx_media_type 
    ON media(media_type);

-- Composite index on series_id and season
-- Used when fetching episodes for a specific season
CREATE INDEX IF NOT EXISTS idx_media_series_season 
    ON media(series_id, season);

-- Composite index on series_id, season, and episode
-- Used when fetching specific episodes
CREATE INDEX IF NOT EXISTS idx_media_series_season_episode 
    ON media(series_id, season, episode);

-- Index on confidence_score for filtering
-- Used when finding unverified or low-confidence media
CREATE INDEX IF NOT EXISTS idx_media_confidence 
    ON media(confidence_score);

-- Index on verification_status for filtering
-- Used when finding unverified media
CREATE INDEX IF NOT EXISTS idx_media_verification_status 
    ON media(verification_status);

-- Index on is_watched for filtering
-- Used when finding watched/unwatched media
CREATE INDEX IF NOT EXISTS idx_media_is_watched 
    ON media(is_watched);

-- Index on created_at for sorting
-- Used when finding recent media
CREATE INDEX IF NOT EXISTS idx_media_created_at 
    ON media(created_at DESC);

-- Index on updated_at for sorting
-- Used for tracking recently updated items
CREATE INDEX IF NOT EXISTS idx_media_updated_at 
    ON media(updated_at DESC);

-- Composite index on is_watched and updated_at
-- Used for finding recently watched media
CREATE INDEX IF NOT EXISTS idx_media_watched_updated 
    ON media(is_watched, updated_at DESC);

-- Index on tmdb_id for lookups
-- Used when fetching metadata by TMDB ID
CREATE INDEX IF NOT EXISTS idx_media_tmdb_id 
    ON media(tmdb_id);

-- Series table indexes

-- Index on tmdb_id for series lookups
CREATE INDEX IF NOT EXISTS idx_series_tmdb_id 
    ON series(tmdb_id);

-- Index on name for series search
CREATE INDEX IF NOT EXISTS idx_series_name 
    ON series(name COLLATE NOCASE);

-- Index on created_at for sorting
CREATE INDEX IF NOT EXISTS idx_series_created_at 
    ON series(created_at DESC);

-- Collections table indexes

-- Index on tmdb_id for collection lookups
CREATE INDEX IF NOT EXISTS idx_collections_tmdb_id 
    ON collections(tmdb_id);

-- Index on name for collection search
CREATE INDEX IF NOT EXISTS idx_collections_name 
    ON collections(name COLLATE NOCASE);

-- Index on created_at for sorting
CREATE INDEX IF NOT EXISTS idx_collections_created_at 
    ON collections(created_at DESC);

-- Verification history table indexes

-- Index on media_id for history lookups
CREATE INDEX IF NOT EXISTS idx_verification_history_media_id 
    ON verification_history(media_id);

-- Index on timestamp for sorting
CREATE INDEX IF NOT EXISTS idx_verification_history_timestamp 
    ON verification_history(timestamp DESC);

-- Composite index on media_id and timestamp
-- Used for finding latest verification for a media item
CREATE INDEX IF NOT EXISTS idx_verification_history_media_timestamp 
    ON verification_history(media_id, timestamp DESC);

-- Analyze tables after index creation
-- This updates query planner statistics
ANALYZE media;
ANALYZE series;
ANALYZE collections;
ANALYZE verification_history;
