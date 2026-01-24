// External Service Interfaces
//
// This module defines interfaces for external services that HomeFlixD depends on.
// Following Interface Segregation Principle (ISP), services are split into focused traits.
//
// Interfaces:
// - tmdb_service: TMDB API interfaces (TmdbSearcher, TmdbFetcher, TmdbResolver)
// - video_analyzer: FFprobe/FFmpeg video analysis interface
// - thumbnail_generator: Thumbnail generation interface

pub mod tmdb_service;
pub mod video_analyzer;
pub mod thumbnail_generator;

// Re-export all external service traits and types
pub use tmdb_service::{
    TmdbSearcher, TmdbFetcher, TmdbResolver, TmdbService,
    TmdbContentRatingFetcher, TmdbSimilarFetcher, TmdbCreditsFetcher, TmdbReconciler,
    TmdbMatch, MovieDetail, TvDetail, SeasonDetail, EpisodeDetail,
    Genre, CollectionInfo, CollectionDetail, CollectionPartInfo, ContentRatingInfo, SimilarResult,
    Credits, CastMember, CrewMember,
};
pub use video_analyzer::{VideoAnalyzer, VideoAnalysis, AudioTrack, SubtitleTrack};
pub use thumbnail_generator::{ThumbnailGenerator, ThumbnailOptions, ThumbnailResult};
