//! Database Schema Management
//!
//! Provides schema initialization and migrations for HomeFlixD.
//! Migrated from legacy db.rs to align with Clean Architecture.

use sqlx::{Pool, Sqlite};
use tracing::info;

/// Initialize all database tables
///
/// Creates tables if they don't exist and applies column migrations.
/// This is idempotent - safe to call multiple times.
pub async fn initialize_schema(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("Initializing database schema");

    // 1. Create Media Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS media (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            media_type TEXT DEFAULT 'movie',
            title TEXT NOT NULL,
            overview TEXT,
            poster_url TEXT,
            backdrop_url TEXT,
            trailer_url TEXT,
            duration_seconds INTEGER,
            release_date TEXT,
            resolution TEXT,
            genres TEXT,
            series_id INTEGER REFERENCES series(id),
            season INTEGER,
            episode INTEGER,
            episode_end INTEGER,
            tmdb_id INTEGER,
            original_title TEXT,
            rating REAL,
            confidence_score REAL DEFAULT 0.0,
            verification_status TEXT DEFAULT 'unverified',
            identification_strategy TEXT,
            error_notes TEXT,
            alternative_matches TEXT,
            content_rating TEXT,
            content_warnings TEXT,
            current_position INTEGER DEFAULT 0,
            is_watched INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Create Watch Progress Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS watch_progress (
            media_id INTEGER PRIMARY KEY,
            current_position_seconds INTEGER NOT NULL DEFAULT 0,
            is_watched BOOLEAN NOT NULL DEFAULT 0,
            last_updated DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(media_id) REFERENCES media(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 3. Create Series Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS series (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tmdb_id INTEGER,
            title TEXT NOT NULL,
            overview TEXT,
            poster_url TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 4. Create Collections Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS collections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            poster_url TEXT,
            backdrop_url TEXT,
            tmdb_collection_id INTEGER,
            sort_mode TEXT DEFAULT 'timeline',
            collection_type TEXT DEFAULT 'auto',
            total_items INTEGER DEFAULT 0,
            available_items INTEGER DEFAULT 0
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 5. Create Collection Items Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS collection_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_id INTEGER NOT NULL,
            media_id INTEGER,
            tmdb_id INTEGER NOT NULL,
            media_type TEXT DEFAULT 'movie',
            title TEXT NOT NULL,
            overview TEXT,
            poster_url TEXT,
            release_date TEXT,
            timeline_order INTEGER NOT NULL,
            release_order INTEGER NOT NULL,
            timeline_year INTEGER,
            timeline_notes TEXT,
            season_number INTEGER,
            episode_number INTEGER,
            is_available INTEGER DEFAULT 0,
            FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            FOREIGN KEY(media_id) REFERENCES media(id) ON DELETE SET NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 6. Create Verification History Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS verification_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_id TEXT NOT NULL,
            content_type TEXT NOT NULL,
            original_match_id INTEGER NOT NULL,
            corrected_match_id INTEGER,
            confidence_before REAL NOT NULL,
            confidence_after REAL NOT NULL,
            verified_by TEXT NOT NULL,
            verification_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            notes TEXT
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 7. Create TMDB Cache Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tmdb_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_id TEXT NOT NULL,
            external_type TEXT NOT NULL,
            resolved_tmdb_id INTEGER NOT NULL,
            resolved_type TEXT NOT NULL,
            cached_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            ttl DATETIME NOT NULL,
            UNIQUE(external_id, external_type)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // 8. Create General Cache Table (for CacheRepository)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS cache (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            expires_at INTEGER NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for cache expiration cleanup
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_cache_expires ON cache(expires_at)")
        .execute(pool)
        .await?;

    // 8.5. Create Events Table (for event sourcing)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            aggregate_id TEXT,
            aggregate_type TEXT,
            payload TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            correlation_id TEXT,
            causation_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for event queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_type, aggregate_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at)")
        .execute(pool)
        .await?;

    // 9. Create Media Credits Table (cast/crew cache from TMDB)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS media_credits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            media_id INTEGER NOT NULL,
            person_id INTEGER NOT NULL,
            person_name TEXT NOT NULL,
            role TEXT NOT NULL,
            character_name TEXT,
            department TEXT,
            profile_url TEXT,
            credit_order INTEGER DEFAULT 0,
            credit_type TEXT NOT NULL,
            FOREIGN KEY(media_id) REFERENCES media(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for credits lookup by media_id
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_media_credits_media_id ON media_credits(media_id)")
        .execute(pool)
        .await?;

    // 10. Create Generated Subtitles Table (for tracking auto-generated subtitles)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS generated_subtitles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            media_id INTEGER NOT NULL,
            audio_track_index INTEGER NOT NULL,
            audio_fingerprint TEXT NOT NULL,
            source_language TEXT,
            target_language TEXT,
            srt_filename TEXT NOT NULL,
            duration_seconds REAL,
            was_translated INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(media_id) REFERENCES media(id) ON DELETE CASCADE,
            UNIQUE(media_id, audio_track_index, target_language)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for generated subtitles lookup
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_generated_subtitles_media_id ON generated_subtitles(media_id)")
        .execute(pool)
        .await?;

    // Create index for fingerprint lookup (for finding existing subtitles by audio track)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_generated_subtitles_fingerprint ON generated_subtitles(audio_fingerprint)")
        .execute(pool)
        .await?;

    // 11. Create Seasons Table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS seasons (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            series_id INTEGER NOT NULL,
            season_number INTEGER NOT NULL,
            tmdb_id INTEGER,
            name TEXT,
            overview TEXT,
            poster_url TEXT,
            air_date TEXT,
            episode_count INTEGER,
            rating REAL,
            FOREIGN KEY(series_id) REFERENCES series(id) ON DELETE CASCADE,
            UNIQUE(series_id, season_number)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Apply column migrations
    apply_column_migrations(pool).await?;

    info!("Database schema initialized successfully");
    Ok(())
}

/// Apply column migrations for existing tables
///
/// Uses ALTER TABLE to add columns that may not exist in older schemas.
/// All operations are idempotent (silently ignore if column exists).
async fn apply_column_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    // Media table migrations
    let media_columns = [
        "ALTER TABLE media ADD COLUMN resolution TEXT",
        "ALTER TABLE media ADD COLUMN genres TEXT",
        "ALTER TABLE media ADD COLUMN backdrop_url TEXT",
        "ALTER TABLE media ADD COLUMN media_type TEXT DEFAULT 'movie'",
        "ALTER TABLE media ADD COLUMN series_id INTEGER REFERENCES series(id)",
        "ALTER TABLE media ADD COLUMN season INTEGER",
        "ALTER TABLE media ADD COLUMN episode INTEGER",
        "ALTER TABLE media ADD COLUMN episode_end INTEGER",
        "ALTER TABLE media ADD COLUMN tmdb_id INTEGER",
        "ALTER TABLE media ADD COLUMN original_title TEXT",
        "ALTER TABLE media ADD COLUMN rating REAL",
        "ALTER TABLE media ADD COLUMN confidence_score REAL DEFAULT 0.0",
        "ALTER TABLE media ADD COLUMN verification_status TEXT DEFAULT 'unverified'",
        "ALTER TABLE media ADD COLUMN identification_strategy TEXT",
        "ALTER TABLE media ADD COLUMN error_notes TEXT",
        "ALTER TABLE media ADD COLUMN alternative_matches TEXT",
        "ALTER TABLE media ADD COLUMN content_rating TEXT",
        "ALTER TABLE media ADD COLUMN content_warnings TEXT",
        "ALTER TABLE media ADD COLUMN current_position INTEGER DEFAULT 0",
        "ALTER TABLE media ADD COLUMN is_watched INTEGER DEFAULT 0",
        "ALTER TABLE media ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP",
        "ALTER TABLE media ADD COLUMN duration_seconds INTEGER",
    ];

    for sql in &media_columns {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    // Series table migrations
    let series_columns = [
        "ALTER TABLE series ADD COLUMN confidence_score REAL DEFAULT 0.0",
        "ALTER TABLE series ADD COLUMN verification_status TEXT DEFAULT 'unverified'",
        "ALTER TABLE series ADD COLUMN first_air_date TEXT",
        "ALTER TABLE series ADD COLUMN last_air_date TEXT",
        "ALTER TABLE series ADD COLUMN status TEXT",
        "ALTER TABLE series ADD COLUMN total_seasons INTEGER",
        "ALTER TABLE series ADD COLUMN total_episodes INTEGER",
        "ALTER TABLE series ADD COLUMN original_title TEXT",
        "ALTER TABLE series ADD COLUMN genres TEXT",
        "ALTER TABLE series ADD COLUMN rating REAL",
        "ALTER TABLE series ADD COLUMN backdrop_url TEXT",
        "ALTER TABLE series ADD COLUMN alternative_matches TEXT",
        "ALTER TABLE series ADD COLUMN error_notes TEXT",
        "ALTER TABLE series ADD COLUMN last_verified DATETIME",
        "ALTER TABLE series ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP",
        "ALTER TABLE series ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP",
    ];

    for sql in &series_columns {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    // Collections table migrations
    let collection_columns = [
        "ALTER TABLE collections ADD COLUMN backdrop_url TEXT",
        "ALTER TABLE collections ADD COLUMN tmdb_collection_id INTEGER",
        "ALTER TABLE collections ADD COLUMN sort_mode TEXT DEFAULT 'timeline'",
        "ALTER TABLE collections ADD COLUMN collection_type TEXT DEFAULT 'auto'",
        "ALTER TABLE collections ADD COLUMN total_items INTEGER DEFAULT 0",
        "ALTER TABLE collections ADD COLUMN available_items INTEGER DEFAULT 0",
        "ALTER TABLE collections ADD COLUMN confidence REAL DEFAULT 1.0",
        "ALTER TABLE collections ADD COLUMN created_at DATETIME DEFAULT CURRENT_TIMESTAMP",
        "ALTER TABLE collections ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP",
    ];

    for sql in &collection_columns {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    // Collection items table migrations
    let collection_item_columns = [
        "ALTER TABLE collection_items ADD COLUMN id INTEGER",
        "ALTER TABLE collection_items ADD COLUMN tmdb_id INTEGER",
        "ALTER TABLE collection_items ADD COLUMN media_type TEXT DEFAULT 'movie'",
        "ALTER TABLE collection_items ADD COLUMN title TEXT",
        "ALTER TABLE collection_items ADD COLUMN overview TEXT",
        "ALTER TABLE collection_items ADD COLUMN poster_url TEXT",
        "ALTER TABLE collection_items ADD COLUMN backdrop_url TEXT",
        "ALTER TABLE collection_items ADD COLUMN release_date TEXT",
        "ALTER TABLE collection_items ADD COLUMN timeline_order INTEGER",
        "ALTER TABLE collection_items ADD COLUMN release_order INTEGER",
        "ALTER TABLE collection_items ADD COLUMN timeline_year INTEGER",
        "ALTER TABLE collection_items ADD COLUMN timeline_notes TEXT",
        "ALTER TABLE collection_items ADD COLUMN season_number INTEGER",
        "ALTER TABLE collection_items ADD COLUMN episode_number INTEGER",
        "ALTER TABLE collection_items ADD COLUMN is_available INTEGER DEFAULT 0",
        "ALTER TABLE collection_items ADD COLUMN rating REAL",
    ];

    for sql in &collection_item_columns {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_initialize_schema() {
        // Create in-memory database
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        // Initialize schema
        initialize_schema(&pool)
            .await
            .expect("Failed to initialize schema");

        // Verify tables exist
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='media'")
            .fetch_one(&pool)
            .await
            .expect("Failed to check media table");
        assert_eq!(result.0, 1);

        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='series'")
            .fetch_one(&pool)
            .await
            .expect("Failed to check series table");
        assert_eq!(result.0, 1);

        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='collections'")
            .fetch_one(&pool)
            .await
            .expect("Failed to check collections table");
        assert_eq!(result.0, 1);
    }

    #[tokio::test]
    async fn test_idempotent_schema_initialization() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        // Initialize schema twice - should not fail
        initialize_schema(&pool)
            .await
            .expect("First initialization failed");
        initialize_schema(&pool)
            .await
            .expect("Second initialization should be idempotent");
    }
}
