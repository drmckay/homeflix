//! TMDB API Data Transfer Objects
//!
//! DTOs for mapping TMDB API responses to domain types

use serde::{Deserialize, Serialize};

/// TMDB search response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSearchResponse {
    pub results: Vec<TmdbSearchResult>,
}

/// TMDB search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSearchResult {
    pub id: i64,
    pub title: Option<String>,
    pub name: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub genre_ids: Option<Vec<i32>>,
}

/// TMDB find response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbFindResponse {
    pub movie_results: Vec<TmdbMovieResult>,
    pub tv_results: Option<Vec<TmdbTvResult>>,
}

/// TMDB movie result from find endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMovieResult {
    pub id: i64,
    pub title: String,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub genre_ids: Option<Vec<i32>>,
    pub imdb_id: Option<String>,
    pub belongs_to_collection: Option<i64>,
}

/// TMDB TV result from find endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbTvResult {
    pub id: i64,
    pub name: String,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub genre_ids: Option<Vec<i32>>,
    pub imdb_id: Option<String>,
}

/// TMDB movie details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMovieDetailResponse {
    pub id: i64,
    pub title: String,
    pub overview: String,
    pub release_date: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub genres: Vec<TmdbGenre>,
    pub runtime: Option<i32>,
    pub vote_average: f32,
    pub vote_count: i32,
    pub imdb_id: Option<String>,
    pub belongs_to_collection: Option<TmdbCollectionInfo>,
}

/// TMDB TV details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbTvDetailResponse {
    pub id: i64,
    pub name: String,
    pub overview: String,
    pub first_air_date: String,
    pub last_air_date: Option<String>,
    pub status: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub genres: Vec<TmdbGenre>,
    pub number_of_seasons: i32,
    pub number_of_episodes: i32,
    pub vote_average: f32,
    pub vote_count: i32,
    pub imdb_id: Option<String>,
}

/// TMDB season details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSeasonDetailResponse {
    pub id: i64,
    pub season_number: i32,
    pub episode_count: i32,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: String,
    pub episodes: Vec<TmdbEpisodeDetail>,
}

/// TMDB episode details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbEpisodeDetail {
    pub id: i64,
    pub episode_number: i32,
    pub season_number: i32,
    pub name: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub runtime: Option<i32>,
}

/// TMDB genre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbGenre {
    pub id: i32,
    pub name: String,
}

/// TMDB collection info (basic, from movie details)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionInfo {
    pub id: i64,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

/// TMDB collection details response (full, from /collection/{id} endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionDetailsResponse {
    pub id: i64,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<TmdbCollectionPart>,
}

/// A movie in a TMDB collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionPart {
    pub id: i64,
    pub title: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f32>,
}
