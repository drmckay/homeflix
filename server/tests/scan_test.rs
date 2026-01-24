//! Integration test for scanning /storage/media directory
//!
//! Tests:
//! - Media identification with TMDB enrichment
//! - Episode/Season/Series hierarchy validation
//! - Missing episode detection
//! - Multi-episode file handling

use std::collections::{HashMap, HashSet, BTreeSet};
use std::sync::Arc;
use homeflixd::infrastructure::database::{ConnectionPool, ConnectionPoolConfig, initialize_schema};
use homeflixd::infrastructure::persistence::sqlite::{SqliteMediaRepository, SqliteCacheRepository};
use homeflixd::domain::repositories::MediaRepository;
use homeflixd::infrastructure::external::tmdb::TmdbClient;
use homeflixd::infrastructure::filesystem::WalkDirAdapter;
use homeflixd::infrastructure::messaging::in_memory_event_bus::InMemoryEventBus;
use homeflixd::domain::services::{DefaultIdentificationService, DefaultConfidenceService, TmdbCrossValidatorImpl};
use homeflixd::application::ScanLibraryUseCase;

/// Represents a TV series with its seasons and episodes
#[derive(Debug)]
struct SeriesInfo {
    name: String,
    tmdb_ids: HashSet<Option<i64>>,
    seasons: HashMap<i32, SeasonInfo>,
}

/// Represents a season with its episodes
#[derive(Debug)]
struct SeasonInfo {
    #[allow(dead_code)]
    season_number: i32,
    episodes: BTreeSet<i32>,
    episode_files: Vec<EpisodeFile>,
}

/// Represents an episode file (may contain multiple episodes)
#[derive(Debug)]
struct EpisodeFile {
    file_path: String,
    #[allow(dead_code)]
    season: i32,
    episode_start: i32,
    episode_end: Option<i32>,
    #[allow(dead_code)]
    tmdb_id: Option<i64>,
    #[allow(dead_code)]
    confidence: f32,
}

#[tokio::test]
async fn test_scan_storage_media() {
    // Skip if directory doesn't exist
    if !std::path::Path::new("/storage/media").exists() {
        println!("Skipping test: /storage/media does not exist");
        return;
    }

    // Use the provided TMDB API key
    let tmdb_api_key = std::env::var("TMDB_API_KEY").ok()
        .or_else(|| Some("a7fc3ea556c3395f3f5f3148b96696b0".to_string()));

    // Create in-memory database
    let pool_config = ConnectionPoolConfig::new("sqlite::memory:".to_string());
    let connection_pool = ConnectionPool::create(pool_config).await
        .expect("Failed to create connection pool");
    let pool = connection_pool.inner().clone();

    // Initialize schema
    initialize_schema(&pool).await.expect("Failed to initialize schema");

    // Create repositories
    let media_repo = Arc::new(SqliteMediaRepository::new(pool.clone()));
    let cache_repo = Arc::new(SqliteCacheRepository::new(pool.clone()));

    // Create services
    let directory_walker = Arc::new(WalkDirAdapter::new());
    let event_bus = Arc::new(InMemoryEventBus::new());
    let identification_service = Arc::new(DefaultIdentificationService::new());
    let confidence_service = Arc::new(DefaultConfidenceService::new());

    // Create scan use case
    let mut scan_use_case = ScanLibraryUseCase::new(
        media_repo.clone(),
        directory_walker,
        event_bus,
        identification_service,
        confidence_service,
    );

    // Add TMDB enrichment if API key is available
    if let Some(api_key) = tmdb_api_key {
        match TmdbClient::new(&api_key, cache_repo.clone()) {
            Ok(tmdb_client) => {
                println!("TMDB client created successfully");
                let tmdb_arc = Arc::new(tmdb_client);
                let cross_validator = Arc::new(TmdbCrossValidatorImpl::new(tmdb_arc.clone()));
                scan_use_case = scan_use_case
                    .with_tmdb_service(tmdb_arc)
                    .with_tmdb_cross_validator(cross_validator);
            }
            Err(e) => {
                println!("WARNING: Failed to create TMDB client: {:?}", e);
            }
        }
    } else {
        println!("Running without TMDB enrichment (no API key provided)");
    }

    // Run the scan
    println!("\n{}", "=".repeat(60));
    println!("=== Starting scan of /storage/media with TMDB enrichment ===");
    println!("{}\n", "=".repeat(60));

    let result = scan_use_case.execute("/storage/media").await
        .expect("Scan failed");

    println!("\n=== Scan Results ===");
    println!("Processed: {}", result.processed_count);
    println!("Identified: {}", result.identified_count);
    println!("Failed: {}", result.failed_count);
    println!("Skipped: {}", result.skipped_count);
    println!("Duration: {}s", result.duration_secs);

    // Fetch all media
    let all_media = media_repo.find_all().await.expect("Failed to fetch media");

    // Separate movies and episodes
    let movies: Vec<_> = all_media.iter().filter(|m| m.media_type.is_movie()).collect();
    let episodes: Vec<_> = all_media.iter().filter(|m| m.media_type.is_episode()).collect();

    println!("\n{}", "=".repeat(60));
    println!("=== Movies ({}) ===", movies.len());
    println!("{}", "=".repeat(60));

    for movie in &movies {
        let tmdb_status = match movie.tmdb_id {
            Some(id) => format!("TMDB: {}", id),
            None => "NO TMDB".to_string(),
        };
        println!("  {} ({}) - {} - conf: {:.2}",
            movie.title,
            movie.release_date.as_deref().unwrap_or("?"),
            tmdb_status,
            movie.confidence_score.value()
        );
    }

    // Build series hierarchy from episodes
    let mut series_map: HashMap<String, SeriesInfo> = HashMap::new();

    for ep in &episodes {
        // Extract series name (everything before "S" pattern or use full title)
        let series_name = extract_series_name(&ep.title);
        let season = ep.season.unwrap_or(0);
        let episode_start = ep.episode.unwrap_or(0);
        let episode_end = ep.episode_end;

        let series = series_map.entry(series_name.clone()).or_insert_with(|| SeriesInfo {
            name: series_name,
            tmdb_ids: HashSet::new(),
            seasons: HashMap::new(),
        });

        series.tmdb_ids.insert(ep.tmdb_id);

        let season_info = series.seasons.entry(season).or_insert_with(|| SeasonInfo {
            season_number: season,
            episodes: BTreeSet::new(),
            episode_files: Vec::new(),
        });

        // Add all episodes in the range (for multi-episode files like S01E01E02)
        let end = episode_end.unwrap_or(episode_start);
        for ep_num in episode_start..=end {
            season_info.episodes.insert(ep_num);
        }

        season_info.episode_files.push(EpisodeFile {
            file_path: ep.file_path.clone(),
            season,
            episode_start,
            episode_end,
            tmdb_id: ep.tmdb_id,
            confidence: ep.confidence_score.value(),
        });
    }

    println!("\n{}", "=".repeat(60));
    println!("=== TV Series Analysis ===");
    println!("{}", "=".repeat(60));

    let mut total_issues = 0;

    for (series_name, series) in series_map.iter() {
        println!("\n{}", "-".repeat(50));
        println!("TV: {}", series_name);
        println!("{}", "-".repeat(50));

        // Check TMDB ID consistency
        let unique_tmdb_ids: Vec<_> = series.tmdb_ids.iter().filter(|id| id.is_some()).collect();
        let none_count = series.tmdb_ids.iter().filter(|id| id.is_none()).count();

        if unique_tmdb_ids.len() > 1 {
            println!("  WARNING: Multiple TMDB IDs found: {:?}", unique_tmdb_ids);
            total_issues += 1;
        } else if unique_tmdb_ids.is_empty() {
            println!("  WARNING: No TMDB ID for any episode");
            total_issues += 1;
        } else {
            println!("  TMDB ID: {:?}", unique_tmdb_ids[0]);
        }

        if none_count > 0 && !unique_tmdb_ids.is_empty() {
            println!("  WARNING: {} episodes without TMDB ID", none_count);
        }

        // Sort seasons
        let mut season_nums: Vec<_> = series.seasons.keys().collect();
        season_nums.sort();

        let total_episodes: usize = series.seasons.values().map(|s| s.episodes.len()).sum();
        println!("  Seasons: {} | Total Episodes: {}", series.seasons.len(), total_episodes);

        for season_num in season_nums {
            let season = &series.seasons[season_num];
            let episodes_sorted: Vec<_> = season.episodes.iter().collect();

            // Detect gaps in episode numbers
            let min_ep = *episodes_sorted.first().unwrap_or(&&0);
            let max_ep = *episodes_sorted.last().unwrap_or(&&0);
            let expected_count = (max_ep - min_ep + 1) as usize;
            let actual_count = season.episodes.len();

            let gap_info = if expected_count != actual_count {
                let missing: Vec<_> = (*min_ep..=*max_ep)
                    .filter(|e| !season.episodes.contains(e))
                    .collect();
                format!(" [MISSING: {:?}]", missing)
            } else {
                " [OK]".to_string()
            };

            println!("    Season {:02}: E{:02}-E{:02} ({} episodes){}",
                season_num,
                min_ep,
                max_ep,
                actual_count,
                gap_info
            );

            // Show episode details if there are issues
            if expected_count != actual_count {
                total_issues += 1;
            }
        }

        // Check for multi-episode files using episode_end field
        let multi_ep_files: Vec<_> = series.seasons.values()
            .flat_map(|s| &s.episode_files)
            .filter(|f| f.episode_end.is_some() && f.episode_end != Some(f.episode_start))
            .collect();

        if !multi_ep_files.is_empty() {
            println!("  Multi-episode files detected: {}", multi_ep_files.len());
            for f in multi_ep_files.iter().take(5) {
                let filename = f.file_path.split('/').last().unwrap_or(&f.file_path);
                let ep_range = format!("E{:02}-E{:02}", f.episode_start, f.episode_end.unwrap_or(f.episode_start));
                println!("      - {} ({})", filename, ep_range);
            }
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("=== Summary ===");
    println!("{}", "=".repeat(60));
    println!("Total Movies: {}", movies.len());
    println!("Total TV Series: {}", series_map.len());
    println!("Total Episodes: {}", episodes.len());

    let with_tmdb: usize = all_media.iter().filter(|m| m.tmdb_id.is_some()).count();
    let without_tmdb: usize = all_media.iter().filter(|m| m.tmdb_id.is_none()).count();
    println!("With TMDB ID: {} ({:.1}%)", with_tmdb, (with_tmdb as f64 / all_media.len() as f64) * 100.0);
    println!("Without TMDB ID: {} ({:.1}%)", without_tmdb, (without_tmdb as f64 / all_media.len() as f64) * 100.0);

    let avg_confidence: f32 = all_media.iter().map(|m| m.confidence_score.value()).sum::<f32>() / all_media.len() as f32;
    println!("Average Confidence: {:.2}", avg_confidence);

    if total_issues > 0 {
        println!("\nTotal Issues Found: {}", total_issues);
    } else {
        println!("\nNo issues found!");
    }

    // Detailed issues report
    println!("\n{}", "=".repeat(60));
    println!("=== Detailed Issues Report ===");
    println!("{}", "=".repeat(60));

    // Items without TMDB ID
    let no_tmdb: Vec<_> = all_media.iter().filter(|m| m.tmdb_id.is_none()).collect();
    if !no_tmdb.is_empty() {
        println!("\nItems without TMDB ID ({}):", no_tmdb.len());
        for item in no_tmdb.iter().take(20) {
            let filename = item.file_path.split('/').last().unwrap_or(&item.file_path);
            println!("  - {} | S{:02}E{:02} | {}",
                item.title,
                item.season.unwrap_or(0),
                item.episode.unwrap_or(0),
                filename
            );
        }
        if no_tmdb.len() > 20 {
            println!("  ... and {} more", no_tmdb.len() - 20);
        }
    }

    // Low confidence items
    let low_conf: Vec<_> = all_media.iter().filter(|m| m.confidence_score.value() < 0.7).collect();
    if !low_conf.is_empty() {
        println!("\nLow confidence items (<0.7): {}", low_conf.len());
        for item in low_conf.iter().take(10) {
            println!("  - {} (conf: {:.2})", item.title, item.confidence_score.value());
        }
    }

    // Assertions
    assert!(result.processed_count > 0, "Should process some files");
    assert!(result.identified_count > 0, "Should identify some media");
}

/// Extract series name from episode title
fn extract_series_name(title: &str) -> String {
    // Common patterns: "Show Name S01E01" or "Show Name"
    // Try to extract everything before season/episode patterns
    let patterns = [" S0", " S1", " S2", " S3", " s0", " s1"];

    for pattern in patterns {
        if let Some(pos) = title.find(pattern) {
            return title[..pos].trim().to_string();
        }
    }

    // If no pattern found, use the full title
    title.to_string()
}
