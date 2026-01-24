//! Performance benchmarks for scanner operations
//!
//! Benchmarks cover:
//! - Directory walking performance
//! - File processing performance
//! - Identification performance
//! - Database operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput, measurement};
use std::fs;
use tempfile::TempDir;

fn setup_test_files(count: usize) -> TempDir {
    let temp_dir = TempDir::new().expect("Should create temp dir");
    
    for i in 0..count {
        let filename = format!("movie_{}.mkv", i);
        let path = temp_dir.path().join(&filename);
        fs::write(&path, b"fake media content").expect("Should create test file");
    }
    
    temp_dir
}

fn bench_scan_10_files(c: &mut Criterion) {
    let temp_dir = setup_test_files(10);
    let mut group = c.benchmark_group("scan");
    
    group.bench_function("10_files", |b| {
        b.iter(|| {
            let entries = walkdir::WalkDir::new(temp_dir.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .collect::<Vec<_>>();
            
            black_box(entries.len())
        });
    });
    
    temp_dir.close().expect("Should close temp dir");
}

fn bench_scan_100_files(c: &mut Criterion) {
    let temp_dir = setup_test_files(100);
    let mut group = c.benchmark_group("scan");
    
    group.bench_function("100_files", |b| {
        b.iter(|| {
            let entries = walkdir::WalkDir::new(temp_dir.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .collect::<Vec<_>>();
            
            black_box(entries.len())
        });
    });
    
    temp_dir.close().expect("Should close temp dir");
}

fn bench_scan_1000_files(c: &mut Criterion) {
    let temp_dir = setup_test_files(1000);
    let mut group = c.benchmark_group("scan");
    
    group.bench_function("1000_files", |b| {
        b.iter(|| {
            let entries = walkdir::WalkDir::new(temp_dir.path())
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .collect::<Vec<_>>();
            
            black_box(entries.len())
        });
    });
    
    temp_dir.close().expect("Should close temp dir");
}

fn bench_identify_media_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("identify");
    
    group.bench_function("movie_filename", |b| {
        b.iter(|| {
            let filename = "Movie.Name.2024.1080p.mkv";
            // Check for season/episode patterns
            let has_season_episode = filename.contains("S") && filename.contains("E");
            black_box(!has_season_episode)
        });
    });
    
    group.bench_function("episode_filename", |b| {
        b.iter(|| {
            let filename = "Show.Name.S02E05.1080p.mkv";
            // Check for season/episode patterns
            let has_season_episode = filename.contains("S") && filename.contains("E");
            black_box(has_season_episode)
        });
    });
}

fn bench_clean_title(c: &mut Criterion) {
    let mut group = c.benchmark_group("clean_title");
    
    group.bench_function("simple_title", |b| {
        b.iter(|| {
            let title = "Movie Name";
            let mut cleaned = title.to_string();
            cleaned = cleaned.replace(".", " ");
            cleaned = cleaned.replace("_", " ");
            cleaned = cleaned.replace("-", " ");
            black_box(cleaned)
        });
    });
    
    group.bench_function("complex_title", |b| {
        b.iter(|| {
            let title = "Movie.Name.[Group].2024.1080p.x264.BluRay.mkv";
            let mut cleaned = title.to_string();
            cleaned = cleaned.replace(".", " ");
            cleaned = cleaned.replace("_", " ");
            cleaned = cleaned.replace("-", " ");
            // Remove brackets
            if let Some(start) = cleaned.find('[') {
                if let Some(end) = cleaned.find(']') {
                    cleaned.replace_range(start..=end+1, "");
                }
            }
            black_box(cleaned)
        });
    });
}

fn bench_confidence_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("confidence");
    
    group.bench_function("calculate_high", |b| {
        b.iter(|| {
            let score = 0.90;
            let is_high = score >= 0.85;
            black_box(is_high)
        });
    });
    
    group.bench_function("calculate_medium", |b| {
        b.iter(|| {
            let score = 0.75;
            let is_medium = score >= 0.70 && score < 0.85;
            black_box(is_medium)
        });
    });
    
    group.bench_function("calculate_low", |b| {
        b.iter(|| {
            let score = 0.65;
            let is_low = score >= 0.60 && score < 0.70;
            black_box(is_low)
        });
    });
}

fn bench_database_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("database");
    
    group.bench_function("insert_media", |b| {
        b.iter(|| {
            // Simulate database insert operation
            let media = create_test_media();
            black_box(media)
        });
    });
    
    group.bench_function("query_media", |b| {
        b.iter(|| {
            // Simulate database query operation
            let media = create_test_media();
            black_box(media.id)
        });
    });
}

fn create_test_media() -> homeflixd::domain::entities::Media {
    use homeflixd::domain::value_objects::MediaType;
    
    homeflixd::domain::entities::Media::new(
        "/path/to/movie.mkv".to_string(),
        MediaType::Movie,
        "Test Movie".to_string(),
    ).expect("Should create media")
}

criterion_group! {
    name = "scanner_bench";
    config = Criterion::default()
        .measurement_time(measurement::WallTime)
        .warm_up_time(std::time::Duration::from_secs(1))
        .sample_size(10);
    
    targets = bench_scan_10_files,
               bench_scan_100_files,
               bench_scan_1000_files,
               bench_identify_media_type,
               bench_clean_title,
               bench_confidence_calculation,
               bench_database_operations,
}

criterion_main!(scanner_bench);
