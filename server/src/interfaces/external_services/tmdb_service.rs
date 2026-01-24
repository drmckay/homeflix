// TMDB Service Interfaces
//
// This module defines interfaces for The Movie Database (TMDB) API.
// Following Interface Segregation Principle (ISP), the service is split into
// focused traits: TmdbSearcher, TmdbFetcher, and TmdbResolver.
//
// This design allows:
// - Testing with mock implementations
// - Swapping implementations (e.g., caching layer)
// - Implementing only needed methods (ISP compliance)

use async_trait::async_trait;
use crate::domain::value_objects::{MatchStrategy, ConfidenceScore};
use crate::shared::error::TmdbError;

/// Search interface for TMDB API
/// 
/// Provides methods for searching movies and TV shows by query.
#[async_trait]
pub trait TmdbSearcher: Send + Sync {
    /// Search for movies by query and optional year
    /// 
    /// # Arguments
    /// * `query` - Search query string
    /// * `year` - Optional year filter
    /// 
    /// # Returns
    /// * `Result<Vec<TmdbMatch>, TmdbError>` - List of matching results
    async fn search_movie(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError>;
    
    /// Search for TV shows by query and optional year
    /// 
    /// # Arguments
    /// * `query` - Search query string
    /// * `year` - Optional year filter
    /// 
    /// # Returns
    /// * `Result<Vec<TmdbMatch>, TmdbError>` - List of matching results
    async fn search_tv(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMatch>, TmdbError>;
}

/// Fetch interface for TMDB API
/// 
/// Provides methods for fetching detailed information about movies, TV shows,
/// seasons, and episodes.
#[async_trait]
pub trait TmdbFetcher: Send + Sync {
    /// Fetch detailed movie information
    /// 
    /// # Arguments
    /// * `id` - TMDB movie ID
    /// 
    /// # Returns
    /// * `Result<Option<MovieDetail>, TmdbError>` - Movie details or None if not found
    async fn fetch_movie_details(&self, id: i64) -> Result<Option<MovieDetail>, TmdbError>;
    
    /// Fetch detailed TV show information
    /// 
    /// # Arguments
    /// * `id` - TMDB TV show ID
    /// 
    /// # Returns
    /// * `Result<Option<TvDetail>, TmdbError>` - TV show details or None if not found
    async fn fetch_tv_details(&self, id: i64) -> Result<Option<TvDetail>, TmdbError>;
    
    /// Fetch season details for a TV show
    /// 
    /// # Arguments
    /// * `tv_id` - TMDB TV show ID
    /// * `season_number` - Season number
    /// 
    /// # Returns
    /// * `Result<Option<SeasonDetail>, TmdbError>` - Season details or None if not found
    async fn fetch_season(&self, tv_id: i64, season_number: i32) -> Result<Option<SeasonDetail>, TmdbError>;
    
    /// Fetch episode details for a TV show
    ///
    /// # Arguments
    /// * `tv_id` - TMDB TV show ID
    /// * `season` - Season number
    /// * `episode` - Episode number
    ///
    /// # Returns
    /// * `Result<Option<EpisodeDetail>, TmdbError>` - Episode details or None if not found
    async fn fetch_episode(
        &self,
        tv_id: i64,
        season: i32,
        episode: i32,
    ) -> Result<Option<EpisodeDetail>, TmdbError>;

    /// Fetch collection details
    ///
    /// Returns the collection info including total number of movies in the collection.
    ///
    /// # Arguments
    /// * `collection_id` - TMDB collection ID
    ///
    /// # Returns
    /// * `Result<Option<CollectionDetail>, TmdbError>` - Collection details or None if not found
    async fn fetch_collection_details(&self, collection_id: i64) -> Result<Option<CollectionDetail>, TmdbError>;
}

/// Resolver interface for TMDB API
///
/// Provides methods for finding content by external IDs (e.g., IMDB ID).
#[async_trait]
pub trait TmdbResolver: Send + Sync {
    /// Find content by external ID
    ///
    /// # Arguments
    /// * `id` - External ID (e.g., IMDB ID like "tt1234567")
    /// * `source` - Source type (e.g., "imdb_id", "tvdb_id")
    ///
    /// # Returns
    /// * `Result<Option<TmdbMatch>, TmdbError>` - Match result or None if not found
    async fn find_by_external_id(&self, id: &str, source: &str) -> Result<Option<TmdbMatch>, TmdbError>;
}

/// Content rating fetcher interface
///
/// Provides methods for fetching content ratings (certifications) from TMDB.
#[async_trait]
pub trait TmdbContentRatingFetcher: Send + Sync {
    /// Fetch content rating for a movie
    ///
    /// # Arguments
    /// * `tmdb_id` - TMDB movie ID
    ///
    /// # Returns
    /// * `Result<ContentRatingInfo, TmdbError>` - Content rating information
    async fn fetch_movie_content_rating(&self, tmdb_id: i64) -> Result<ContentRatingInfo, TmdbError>;

    /// Fetch content rating for a TV series
    ///
    /// # Arguments
    /// * `tmdb_id` - TMDB TV show ID
    ///
    /// # Returns
    /// * `Result<ContentRatingInfo, TmdbError>` - Content rating information
    async fn fetch_tv_content_rating(&self, tmdb_id: i64) -> Result<ContentRatingInfo, TmdbError>;
}

/// Similar content fetcher interface
///
/// Provides methods for fetching similar movies/TV shows.
#[async_trait]
pub trait TmdbSimilarFetcher: Send + Sync {
    /// Fetch similar movies/TV shows
    ///
    /// # Arguments
    /// * `tmdb_id` - TMDB ID
    /// * `media_type` - "movie" or "tv"
    ///
    /// # Returns
    /// * `Result<Vec<SimilarResult>, TmdbError>` - List of similar content
    async fn fetch_similar(&self, tmdb_id: i64, media_type: &str) -> Result<Vec<SimilarResult>, TmdbError>;
}

/// Credits fetcher interface
///
/// Provides methods for fetching cast and crew information.
#[async_trait]
pub trait TmdbCreditsFetcher: Send + Sync {
    /// Fetch credits (cast and crew) for a movie
    ///
    /// # Arguments
    /// * `tmdb_id` - TMDB movie ID
    ///
    /// # Returns
    /// * `Result<Credits, TmdbError>` - Cast and crew information
    async fn fetch_movie_credits(&self, tmdb_id: i64) -> Result<Credits, TmdbError>;

    /// Fetch credits (cast and crew) for a TV show
    ///
    /// # Arguments
    /// * `tmdb_id` - TMDB TV show ID
    ///
    /// # Returns
    /// * `Result<Credits, TmdbError>` - Cast and crew information
    async fn fetch_tv_credits(&self, tmdb_id: i64) -> Result<Credits, TmdbError>;
}

/// Reconciler interface for multi-strategy TMDB matching
///
/// Provides advanced reconciliation with fuzzy matching and scoring.
#[async_trait]
pub trait TmdbReconciler: Send + Sync {
    /// Reconcile media with TMDB using multiple strategies
    ///
    /// This method implements the multi-strategy matching from Spec V2:
    /// - Strategy 2: Filename + Year
    /// - Strategy 3: Folder + Year
    /// - Strategy 4: Filename Only (Year Agnostic)
    /// - Strategy 5: Cross-Validation
    /// - Strategy 6: Alternative Titles
    ///
    /// # Arguments
    /// * `target_title` - Cleaned title to match
    /// * `target_year` - Year extracted from filename/folder (if any)
    /// * `folder_name` - Parent folder name (if available)
    /// * `detected_type` - "movie" or "tv"
    /// * `nfo_is_xml` - Whether NFO file was valid XML (higher confidence)
    /// * `detected_season` - Season number for episode cross-validation
    /// * `detected_episode` - Episode number for cross-validation
    ///
    /// # Returns
    /// * `Result<(Option<TmdbMatch>, Vec<TmdbMatch>), TmdbError>` - (best match, alternatives)
    async fn reconcile(
        &self,
        target_title: &str,
        target_year: Option<i32>,
        folder_name: Option<&str>,
        detected_type: &str,
        nfo_is_xml: bool,
        detected_season: Option<i32>,
        detected_episode: Option<i32>,
    ) -> Result<(Option<TmdbMatch>, Vec<TmdbMatch>), TmdbError>;
}

/// Combined TMDB service interface
/// 
/// Convenience trait that combines all TMDB interfaces for implementations
/// that provide full TMDB functionality.
#[async_trait]
pub trait TmdbService: TmdbSearcher + TmdbFetcher + TmdbResolver {}

// Blanket implementation for any type that implements all three traits
#[async_trait]
impl<T> TmdbService for T where T: TmdbSearcher + TmdbFetcher + TmdbResolver {}

// ============================================================================
// Types used by TMDB interfaces
// ============================================================================

/// Represents a match result from TMDB search
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TmdbMatch {
    /// TMDB ID
    pub tmdb_id: i64,
    /// Title or name
    pub title: String,
    /// Release year
    pub year: Option<i32>,
    /// Media type ("movie" or "tv")
    pub media_type: String,
    /// Confidence score for this match
    pub confidence: ConfidenceScore,
    /// Match strategy used
    pub strategy: MatchStrategy,
}

/// Detailed movie information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MovieDetail {
    /// TMDB ID
    pub id: i64,
    /// Movie title
    pub title: String,
    /// Plot overview
    pub overview: String,
    /// Release date
    pub release_date: String,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Backdrop path (relative to TMDB base URL)
    pub backdrop_path: Option<String>,
    /// List of genres
    pub genres: Vec<Genre>,
    /// Runtime in minutes
    pub runtime: Option<i32>,
    /// Vote average (0-10)
    pub vote_average: f32,
    /// Vote count
    pub vote_count: i32,
    /// IMDB ID
    pub imdb_id: Option<String>,
    /// Collection ID (if part of a collection)
    pub belongs_to_collection: Option<CollectionInfo>,
}

/// Detailed TV show information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TvDetail {
    /// TMDB ID
    pub id: i64,
    /// TV show name
    pub name: String,
    /// Plot overview
    pub overview: String,
    /// First air date
    pub first_air_date: String,
    /// Last air date
    pub last_air_date: Option<String>,
    /// Status (Returning Series, Ended, etc.)
    pub status: String,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Backdrop path (relative to TMDB base URL)
    pub backdrop_path: Option<String>,
    /// List of genres
    pub genres: Vec<Genre>,
    /// Number of seasons
    pub number_of_seasons: i32,
    /// Number of episodes
    pub number_of_episodes: i32,
    /// Vote average (0-10)
    pub vote_average: f32,
    /// Vote count
    pub vote_count: i32,
    /// IMDB ID
    pub imdb_id: Option<String>,
}

/// Season details for a TV show
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeasonDetail {
    /// Season ID
    pub id: i64,
    /// Season number
    pub season_number: i32,
    /// Number of episodes in season
    #[serde(default)]
    pub episode_count: i32,
    /// Air date
    pub air_date: Option<String>,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Season overview
    #[serde(default)]
    pub overview: String,
    /// List of episodes
    #[serde(default)]
    pub episodes: Vec<EpisodeDetail>,
}

/// Episode details for a TV show
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EpisodeDetail {
    /// Episode ID
    pub id: i64,
    /// Episode number
    pub episode_number: i32,
    /// Season number
    pub season_number: i32,
    /// Episode name
    #[serde(default)]
    pub name: String,
    /// Episode overview
    #[serde(default)]
    pub overview: String,
    /// Air date
    pub air_date: Option<String>,
    /// Still image path (relative to TMDB base URL)
    pub still_path: Option<String>,
    /// Vote average (0-10)
    #[serde(default)]
    pub vote_average: f32,
    /// Vote count
    #[serde(default)]
    pub vote_count: i32,
    /// Runtime in minutes
    pub runtime: Option<i32>,
}

/// Genre information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Genre {
    /// Genre ID
    pub id: i32,
    /// Genre name
    pub name: String,
}

/// Collection information (for movie collections) - basic info from movie details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectionInfo {
    /// Collection ID
    pub id: i64,
    /// Collection name
    pub name: String,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Backdrop path (relative to TMDB base URL)
    pub backdrop_path: Option<String>,
}

/// Collection detail (full info including all movies in the collection)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectionDetail {
    /// Collection ID
    pub id: i64,
    /// Collection name
    pub name: String,
    /// Overview/description
    pub overview: Option<String>,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Backdrop path (relative to TMDB base URL)
    pub backdrop_path: Option<String>,
    /// Total number of movies in the collection
    pub total_parts: i32,
    /// List of movies in the collection
    pub parts: Vec<CollectionPartInfo>,
}

/// A movie in a collection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectionPartInfo {
    /// TMDB movie ID
    pub tmdb_id: i64,
    /// Movie title
    pub title: String,
    /// Overview/description
    pub overview: Option<String>,
    /// Poster path
    pub poster_path: Option<String>,
    /// Release date
    pub release_date: Option<String>,
}

/// Content rating information
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ContentRatingInfo {
    /// Rating string (e.g., "PG-13", "R", "TV-MA")
    pub rating: Option<String>,
    /// Content descriptors (e.g., ["violence", "language"])
    pub descriptors: Vec<String>,
}

impl ContentRatingInfo {
    /// Creates new empty content rating info
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates content rating info with a rating
    pub fn with_rating(rating: &str) -> Self {
        Self {
            rating: Some(rating.to_string()),
            descriptors: Vec::new(),
        }
    }

    /// Checks if this has a valid rating
    pub fn has_rating(&self) -> bool {
        self.rating.as_ref().is_some_and(|r| !r.is_empty())
    }
}

/// Similar content result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarResult {
    /// TMDB ID
    pub id: i64,
    /// Title (for movies) or name (for TV shows)
    pub title: String,
    /// Poster path (relative to TMDB base URL)
    pub poster_path: Option<String>,
    /// Backdrop path (relative to TMDB base URL)
    pub backdrop_path: Option<String>,
    /// Release date (for movies) or first air date (for TV)
    pub release_date: Option<String>,
    /// Vote average (0-10)
    pub vote_average: f32,
    /// Media type ("movie" or "tv")
    pub media_type: String,
}

/// Credits information (cast and crew)
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Credits {
    /// Cast members (actors)
    pub cast: Vec<CastMember>,
    /// Crew members (director, writer, etc.) - limited to key roles
    pub crew: Vec<CrewMember>,
}

/// Cast member (actor) information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CastMember {
    /// TMDB person ID
    pub id: i64,
    /// Actor's name
    pub name: String,
    /// Character name they play
    pub character: String,
    /// Profile image path (relative to TMDB base URL)
    pub profile_path: Option<String>,
    /// Order in credits (lower = more prominent)
    pub order: i32,
}

/// Crew member information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrewMember {
    /// TMDB person ID
    pub id: i64,
    /// Person's name
    pub name: String,
    /// Job title (e.g., "Director", "Writer", "Producer")
    pub job: String,
    /// Department (e.g., "Directing", "Writing", "Production")
    pub department: String,
    /// Profile image path (relative to TMDB base URL)
    pub profile_path: Option<String>,
}
