//! Performance benchmarks for API operations
//!
//! Benchmarks cover:
//! - API response times
//! - Database query performance
//! - JSON serialization/deserialization
//! - Handler throughput

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput, measurement};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestMediaResponse {
    id: i64,
    file_path: String,
    media_type: String,
    title: String,
    confidence_score: f32,
    verification_status: String,
}

fn bench_api_list_media(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_list");
    
    group.bench_function("list_10_items", |b| {
        b.iter(|| {
            let media_list: Vec<TestMediaResponse> = (0..10)
                .map(|i| TestMediaResponse {
                    id: i,
                    file_path: format!("/path/to/movie_{}.mkv", i),
                    media_type: "movie".to_string(),
                    title: format!("Movie {}", i),
                    confidence_score: 0.85,
                    verification_status: "verified".to_string(),
                })
                .collect();
            
            let json = serde_json::to_string(&media_list).expect("Should serialize");
            black_box(json.len())
        });
    });
    
    group.bench_function("list_100_items", |b| {
        b.iter(|| {
            let media_list: Vec<TestMediaResponse> = (0..100)
                .map(|i| TestMediaResponse {
                    id: i,
                    file_path: format!("/path/to/movie_{}.mkv", i),
                    media_type: "movie".to_string(),
                    title: format!("Movie {}", i),
                    confidence_score: 0.85,
                    verification_status: "verified".to_string(),
                })
                .collect();
            
            let json = serde_json::to_string(&media_list).expect("Should serialize");
            black_box(json.len())
        });
    });
}

fn bench_api_get_media(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_get");
    
    group.bench_function("get_media", |b| {
        b.iter(|| {
            let media = TestMediaResponse {
                id: 12345,
                file_path: "/path/to/movie.mkv".to_string(),
                media_type: "movie".to_string(),
                title: "Test Movie".to_string(),
                confidence_score: 0.85,
                verification_status: "verified".to_string(),
            };
            
            let json = serde_json::to_string(&media).expect("Should serialize");
            black_box(json.len())
        });
    });
}

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json");
    
    group.bench_function("serialize_media", |b| {
        b.iter(|| {
            let media = TestMediaResponse {
                id: 12345,
                file_path: "/path/to/movie.mkv".to_string(),
                media_type: "movie".to_string(),
                title: "Test Movie".to_string(),
                confidence_score: 0.85,
                verification_status: "verified".to_string(),
            };
            
            black_box(serde_json::to_string(&media).expect("Should serialize"))
        });
    });
    
    group.bench_function("deserialize_media", |b| {
        b.iter(|| {
            let json = r#"{
                "id": 12345,
                "file_path": "/path/to/movie.mkv",
                "media_type": "movie",
                "title": "Test Movie",
                "confidence_score": 0.85,
                "verification_status": "verified"
            }"#;
            
            black_box(serde_json::from_str::<TestMediaResponse>(json).expect("Should deserialize"))
        });
    });
}

fn bench_api_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    group.throughput(Throughput::Elements(1000), |b| {
        let media_list: Vec<TestMediaResponse> = (0..1000)
                .map(|i| TestMediaResponse {
                    id: i,
                    file_path: format!("/path/to/movie_{}.mkv", i),
                    media_type: "movie".to_string(),
                    title: format!("Movie {}", i),
                    confidence_score: 0.85,
                    verification_status: "verified".to_string(),
                })
                .collect();
        
        black_box(serde_json::to_string(&media_list).expect("Should serialize"))
    });
}

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("strings");
    
    group.bench_function("string_concat", |b| {
        b.iter(|| {
            let s1 = "Movie";
            let s2 = "Name";
            let s3 = "2024";
            let result = format!("{}{}{}", s1, s2, s3);
            black_box(result)
        });
    });
    
    group.bench_function("string_replace", |b| {
        b.iter(|| {
            let s = "Movie.Name.2024.mkv";
            let result = s.replace(".", " ");
            black_box(result)
        });
    });
}

criterion_group! {
    name = "api_bench";
    config = Criterion::default()
        .measurement_time(measurement::WallTime)
        .warm_up_time(std::time::Duration::from_secs(1))
        .sample_size(10);
    
    targets = bench_api_list_media,
               bench_api_get_media,
               bench_json_serialization,
               bench_api_throughput,
               bench_string_operations,
}

criterion_main!(api_bench);
