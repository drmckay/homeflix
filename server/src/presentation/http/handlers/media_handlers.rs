//! Media Handlers
//!
//! HTTP handlers for media operations.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::application::{IdentifyMediaUseCase, ScanLibraryUseCase};
use crate::application::use_cases::get_recently_added::{GetRecentlyAddedUseCase, RecentlyAddedItem};
use crate::domain::entities::Series;
use crate::domain::repositories::{MediaRepository, SeriesRepository, CollectionRepository, CreditsRepository, CreditEntry, CreditType};
use crate::domain::value_objects::MediaType;
use crate::presentation::http::dto::media_dto::{
    GroupedLibraryResponse, LibraryMediaResponse, MediaResponse, ScanRequest, ScanResponse,
    ManualIdentifyRequest, ManualIdentifyResponse,
};
use crate::interfaces::external_services::{TmdbService, TmdbCreditsFetcher, Credits, CastMember, CrewMember};
use crate::shared::error::ApplicationError;
use crate::infrastructure::messaging::in_memory_event_bus::InMemoryEventBus;
use crate::interfaces::external_services::VideoAnalyzer;
use crate::infrastructure::subtitle::SubtitleDetector;

fn series_to_library_media(series: &Series, created_at: &chrono::DateTime<chrono::Utc>) -> LibraryMediaResponse {
    let series_id = series.id.unwrap_or(0);
    LibraryMediaResponse {
        id: -series_id,
        file_path: String::new(),
        title: series.title.clone(),
        overview: series.overview.clone(),
        poster_url: series.poster_url.clone(),
        backdrop_url: series.backdrop_url.clone(),
        trailer_url: None,
        duration: None,
        release_date: series.first_air_date.clone(),
        resolution: None,
        genres: series.genres.clone(),
        media_type: "series".to_string(),
        series_id: Some(series_id),
        season_number: None,
        episode_number: None,
        created_at: created_at.to_rfc3339(),
        tmdb_id: series.tmdb_id,
        original_title: series.original_title.clone(),
        rating: series.rating,
        content_rating: None,
        content_warnings: None,
        current_position: 0,
        is_watched: false,
    }
}

/// Grouped library for homeflix-web (movies + series, no episodes)
pub async fn list_grouped_library(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(series_repo): State<Arc<dyn SeriesRepository>>,
    State(recently_added_use_case): State<Arc<GetRecentlyAddedUseCase>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    const RECENT_LIMIT: usize = 10;
    const CONTINUE_WATCHING_LIMIT: usize = 20;
    // Fetch more items than needed since episodes collapse into series
    const FETCH_LIMIT: usize = 50;

    // Use the GetRecentlyAddedUseCase for properly combined and sorted recent items
    let recent_items = recently_added_use_case
        .execute(RECENT_LIMIT)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert RecentlyAddedItem to LibraryMediaResponse
    let recent: Vec<LibraryMediaResponse> = recent_items
        .into_iter()
        .map(|item| match item {
            RecentlyAddedItem::Movie { media, added_at: _ } => {
                LibraryMediaResponse::from_media(media)
            }
            RecentlyAddedItem::Series { series, added_at } => {
                // Parse the added_at string back to DateTime for the series_to_library_media function
                let created_at = chrono::DateTime::parse_from_rfc3339(&added_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                series_to_library_media(&series, &created_at)
            }
        })
        .collect();

    // Fetch in-progress items (started but not finished)
    // Fetch more items since episodes will collapse into series
    let in_progress_items = media_repo
        .find_in_progress(FETCH_LIMIT)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Collapse episodes into series for continue watching
    let mut continue_watching = Vec::new();
    let mut seen_series_continue = HashSet::new();

    for media in in_progress_items {
        if continue_watching.len() >= CONTINUE_WATCHING_LIMIT {
            break;
        }

        if media.media_type.is_movie() {
            continue_watching.push(LibraryMediaResponse::from_media(media));
            continue;
        }

        if media.media_type.is_episode() {
            if let Some(series_id) = media.series_id {
                if seen_series_continue.insert(series_id) {
                    if let Some(series) = series_repo
                        .find_by_id(series_id)
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    {
                        if series.id.is_some() {
                            // Create series entry with the episode's progress info
                            let mut series_response = series_to_library_media(&series, &media.updated_at);
                            // Keep the episode's progress and watched status for display
                            series_response.current_position = media.current_position;
                            series_response.duration = media.duration_seconds;
                            continue_watching.push(series_response);
                        }
                    }
                }
            }
        }
    }

    let movies = media_repo
        .find_by_type(MediaType::Movie)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut categories: HashMap<String, Vec<LibraryMediaResponse>> = HashMap::new();

    for movie in movies {
        let entry = LibraryMediaResponse::from_media(movie);
        let genre_list: Vec<&str> = entry
            .genres
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|g| g.trim())
            .filter(|g| !g.is_empty())
            .collect();

        if genre_list.is_empty() {
            categories
                .entry("Uncategorized".to_string())
                .or_default()
                .push(entry);
        } else {
            for genre in genre_list {
                categories
                    .entry(genre.to_string())
                    .or_default()
                    .push(entry.clone());
            }
        }
    }

    Ok(Json(GroupedLibraryResponse { recent, continue_watching, categories }))
}

/// Get media by ID
pub async fn get_media(
    State(use_case): State<Arc<IdentifyMediaUseCase<InMemoryEventBus>>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match use_case.execute(id).await {
        Ok(result) => {
            let response = MediaResponse::from(result.media);
            Ok(Json(response))
        }
        Err(ApplicationError::Domain(crate::shared::error::DomainError::NotFound(msg))) => {
            Err((StatusCode::NOT_FOUND, msg))
        }
        Err(e) => {
            tracing::error!("Error getting media: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))
        }
    }
}

/// List all media
pub async fn list_media(
    State(use_case): State<Arc<IdentifyMediaUseCase<InMemoryEventBus>>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match use_case.list_all().await {
        Ok(media_list) => {
            let response: Vec<MediaResponse> = media_list
                .into_iter()
                .map(MediaResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Error listing media: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))
        }
    }
}

/// Scan library
pub async fn scan_library(
    State(use_case): State<Arc<ScanLibraryUseCase<InMemoryEventBus>>>,
    Json(request): Json<ScanRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match use_case.execute(&request.path).await {
        Ok(result) => {
            let response = ScanResponse {
                processed_count: result.processed_count,
                identified_count: result.identified_count,
                failed_count: result.failed_count,
                duration_secs: result.duration_secs,
            };
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Error scanning library: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))
        }
    }
}

/// Audio track response DTO
#[derive(Debug, serde::Serialize)]
pub struct AudioTrackResponse {
    pub index: usize,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub channels: Option<u32>,
    pub title: Option<String>,
    pub is_default: bool,
}

/// Subtitle track response DTO
#[derive(Debug, serde::Serialize)]
pub struct SubtitleTrackResponse {
    /// Track index (used for selection)
    pub index: usize,
    /// ISO 639-1 language code (e.g., "hu", "en")
    pub language: Option<String>,
    /// Human-readable language name (e.g., "Magyar", "English")
    pub language_name: Option<String>,
    /// Source of the subtitle: "external" (.srt file) or "embedded" (in video)
    pub source: String,
    /// Whether this is the default subtitle track
    pub is_default: bool,
}

/// Media tracks response DTO
#[derive(Debug, serde::Serialize)]
pub struct MediaTracksResponse {
    pub duration: f64,
    pub current_position: i64,
    pub is_watched: bool,
    pub audio_tracks: Vec<AudioTrackResponse>,
    pub subtitle_tracks: Vec<SubtitleTrackResponse>,
}

/// Get media tracks (audio/subtitle info) by ID
pub async fn get_media_tracks(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(video_analyzer): State<Arc<dyn VideoAnalyzer>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get media to find file path
    let media = media_repo
        .find_by_id(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Media {} not found", id)))?;

    // Analyze video file for tracks
    let analysis = video_analyzer
        .analyze(&media.file_path)
        .await
        .map_err(|e| {
            tracing::error!("Failed to analyze video {}: {}", media.file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to analyze video".to_string())
        })?;

    let audio_tracks: Vec<AudioTrackResponse> = analysis.audio_tracks
        .into_iter()
        .enumerate()
        .map(|(audio_index, track)| AudioTrackResponse {
            index: audio_index, // Use audio-only index (0, 1, 2...) for FFmpeg -map 0:a:N
            language: track.language,
            codec: track.codec,
            channels: track.channels,
            title: track.title,
            is_default: track.is_default,
        })
        .collect();

    // Discover external subtitle files (.srt)
    let subtitle_detector = SubtitleDetector::new();
    let video_path = std::path::Path::new(&media.file_path);
    let external_subtitles = subtitle_detector.discover(video_path);

    // Build subtitle tracks list: external subtitles first, then embedded
    let mut subtitle_tracks: Vec<SubtitleTrackResponse> = Vec::new();
    let mut index = 0;

    // Add external subtitles
    for ext_sub in external_subtitles {
        subtitle_tracks.push(SubtitleTrackResponse {
            index,
            language: ext_sub.language,
            language_name: ext_sub.language_name,
            source: "external".to_string(),
            is_default: index == 0, // First subtitle is default
        });
        index += 1;
    }

    // Add embedded subtitles from video analysis
    for embedded in analysis.subtitle_tracks {
        subtitle_tracks.push(SubtitleTrackResponse {
            index,
            language: embedded.language,
            language_name: None, // Embedded subtitles don't have display names
            source: "embedded".to_string(),
            is_default: subtitle_tracks.is_empty() && embedded.is_default,
        });
        index += 1;
    }

    Ok(Json(MediaTracksResponse {
        duration: analysis.duration_seconds,
        current_position: media.current_position,
        is_watched: media.is_watched,
        audio_tracks,
        subtitle_tracks,
    }))
}

/// Credits response DTO
#[derive(Debug, serde::Serialize)]
pub struct CreditsResponse {
    pub cast: Vec<CastMemberResponse>,
    pub crew: Vec<CrewMemberResponse>,
}

#[derive(Debug, serde::Serialize)]
pub struct CastMemberResponse {
    pub id: i64,
    pub name: String,
    pub character: String,
    pub profile_url: Option<String>,
    pub order: i32,
}

#[derive(Debug, serde::Serialize)]
pub struct CrewMemberResponse {
    pub id: i64,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_url: Option<String>,
}

/// Get credits (cast and crew) for a media item by ID
/// First checks the database cache, then fetches from TMDB if not cached.
pub async fn get_media_credits(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(series_repo): State<Arc<dyn SeriesRepository>>,
    State(credits_repo): State<Arc<dyn CreditsRepository>>,
    State(tmdb_service): State<Arc<dyn TmdbCreditsFetcher + Send + Sync>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get media to find TMDB ID and media type
    let media = media_repo
        .find_by_id(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Media {} not found", id)))?;

    // Check if credits are already cached in DB
    if let Ok(true) = credits_repo.has_credits(id).await {
        let cached_credits = credits_repo.get_credits(id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let response = CreditsResponse {
            cast: cached_credits.iter()
                .filter(|c| c.credit_type == CreditType::Cast)
                .map(|c| CastMemberResponse {
                    id: c.person_id,
                    name: c.person_name.clone(),
                    character: c.character_name.clone().unwrap_or_default(),
                    profile_url: c.profile_url.clone(),
                    order: c.credit_order,
                }).collect(),
            crew: cached_credits.iter()
                .filter(|c| c.credit_type == CreditType::Crew)
                .map(|c| CrewMemberResponse {
                    id: c.person_id,
                    name: c.person_name.clone(),
                    job: c.role.clone(),
                    department: c.department.clone().unwrap_or_default(),
                    profile_url: c.profile_url.clone(),
                }).collect(),
        };
        return Ok(Json(response));
    }

    // Not cached - fetch from TMDB
    let tmdb_id = media.tmdb_id
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Media has no TMDB ID".to_string()))?;

    // Fetch credits based on media type
    let credits = if media.media_type.is_movie() {
        tmdb_service.fetch_movie_credits(tmdb_id).await
    } else {
        // For TV episodes, get series TMDB ID and fetch TV credits
        if let Some(series_id) = media.series_id {
            if let Ok(Some(series)) = series_repo.find_by_id(series_id).await {
                if let Some(series_tmdb_id) = series.tmdb_id {
                    tmdb_service.fetch_tv_credits(series_tmdb_id).await
                } else {
                    Ok(Credits::default())
                }
            } else {
                Ok(Credits::default())
            }
        } else {
            Ok(Credits::default())
        }
    }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch credits: {}", e)))?;

    // Cache credits in database
    let credit_entries: Vec<CreditEntry> = credits.cast.iter().map(|c| CreditEntry {
        person_id: c.id,
        person_name: c.name.clone(),
        role: "Actor".to_string(),
        character_name: Some(c.character.clone()),
        department: Some("Acting".to_string()),
        profile_url: c.profile_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w185{}", p)),
        credit_order: c.order,
        credit_type: CreditType::Cast,
    }).chain(credits.crew.iter().map(|c| CreditEntry {
        person_id: c.id,
        person_name: c.name.clone(),
        role: c.job.clone(),
        character_name: None,
        department: Some(c.department.clone()),
        profile_url: c.profile_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w185{}", p)),
        credit_order: 0,
        credit_type: CreditType::Crew,
    })).collect();

    // Save to DB (ignore errors - caching is best effort)
    let _ = credits_repo.save_credits(id, &credit_entries).await;

    let response = CreditsResponse {
        cast: credits.cast.into_iter().map(|c| CastMemberResponse {
            id: c.id,
            name: c.name,
            character: c.character,
            profile_url: c.profile_path.map(|p| format!("https://image.tmdb.org/t/p/w185{}", p)),
            order: c.order,
        }).collect(),
        crew: credits.crew.into_iter().map(|c| CrewMemberResponse {
            id: c.id,
            name: c.name,
            job: c.job,
            department: c.department,
            profile_url: c.profile_path.map(|p| format!("https://image.tmdb.org/t/p/w185{}", p)),
        }).collect(),
    };

    Ok(Json(response))
}

/// Manually identify a media item with a specific TMDB ID
pub async fn manual_identify(
    State(media_repo): State<Arc<dyn MediaRepository>>,
    State(collection_repo): State<Arc<dyn CollectionRepository>>,
    State(tmdb_service): State<Arc<dyn TmdbService>>,
    Path(id): Path<i64>,
    Json(request): Json<ManualIdentifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get existing media
    let mut media = media_repo
        .find_by_id(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Media {} not found", id)))?;

    let tmdb_id = request.tmdb_id;
    tracing::info!("Manual identify: media_id={}, tmdb_id={}", id, tmdb_id);

    // Fetch TMDB metadata based on media type
    if media.media_type.is_movie() {
        if let Some(details) = tmdb_service
            .fetch_movie_details(tmdb_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("TMDB error: {}", e)))?
        {
            media.tmdb_id = Some(tmdb_id);
            media.title = details.title.clone();
            media.overview = Some(details.overview);
            media.poster_url = details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
            media.backdrop_url = details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b));
            media.rating = Some(details.vote_average);
            media.release_date = Some(details.release_date);
            media.genres = Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", "));
            media.updated_at = chrono::Utc::now();

            tracing::info!("Fetched TMDB metadata for movie: {}", details.title);

            // Handle collection if movie belongs to one
            if let Some(ref collection_info) = details.belongs_to_collection {
                let collection_id = collection_info.id;
                let collection_name = &collection_info.name;

                match collection_repo.find_by_tmdb_id(collection_id).await {
                    Ok(Some(mut existing)) => {
                        // Update available count
                        existing.available_items += 1;
                        if let Err(e) = collection_repo.update(&existing).await {
                            tracing::warn!("Failed to update collection '{}': {}", collection_name, e);
                        } else {
                            tracing::info!("Updated collection '{}' available count", collection_name);
                        }
                    }
                    Ok(None) => {
                        // Create new collection
                        use crate::domain::entities::Collection;
                        let mut collection = Collection::new(collection_name.clone())
                            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create collection: {}", e)))?
                            .with_tmdb_collection_id(Some(collection_id))
                            .with_poster_url(collection_info.poster_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w500{}", p)))
                            .with_backdrop_url(collection_info.backdrop_path.as_ref().map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b)))
                            .with_collection_type("auto".to_string());
                        collection.available_items = 1;

                        match collection_repo.save(&collection).await {
                            Ok(cid) => {
                                tracing::info!("Created new collection '{}' (TMDB: {}) with ID {}", collection_name, collection_id, cid);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to create collection '{}': {}", collection_name, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Error checking for existing collection: {}", e);
                    }
                }
            }
        } else {
            return Err((StatusCode::NOT_FOUND, format!("TMDB movie {} not found", tmdb_id)));
        }
    } else if media.media_type.is_episode() {
        // For episodes, we need series_id, season, episode from TMDB
        return Err((StatusCode::BAD_REQUEST, "Episode identification requires series_id, season, and episode numbers".to_string()));
    } else {
        // TV show
        if let Some(details) = tmdb_service
            .fetch_tv_details(tmdb_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("TMDB error: {}", e)))?
        {
            media.tmdb_id = Some(tmdb_id);
            media.title = details.name.clone();
            media.overview = Some(details.overview);
            media.poster_url = details.poster_path.map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));
            media.backdrop_url = details.backdrop_path.map(|b| format!("https://image.tmdb.org/t/p/w1280{}", b));
            media.rating = Some(details.vote_average);
            media.release_date = Some(details.first_air_date);
            media.genres = Some(details.genres.iter().map(|g| g.name.as_str()).collect::<Vec<_>>().join(", "));
            media.updated_at = chrono::Utc::now();

            tracing::info!("Fetched TMDB metadata for TV show: {}", details.name);
        } else {
            return Err((StatusCode::NOT_FOUND, format!("TMDB TV show {} not found", tmdb_id)));
        }
    }

    // Save updated media
    media_repo
        .update(&media)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save: {}", e)))?;

    Ok(Json(ManualIdentifyResponse {
        message: format!("Successfully identified as TMDB ID {}", tmdb_id),
        media: MediaResponse::from(media),
    }))
}

/// Get recently added content (movies + series ranked by latest episode)
///
/// Returns combined list of recently added movies and series,
/// where series are ranked by their most recently added episode.
pub async fn list_recently_added(
    State(use_case): State<Arc<GetRecentlyAddedUseCase>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    const LIMIT: usize = 10;

    match use_case.execute(LIMIT).await {
        Ok(items) => Ok(Json(items)),
        Err(e) => {
            tracing::error!("Error getting recently added: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to get recently added items".to_string()))
        }
    }
}
