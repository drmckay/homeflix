//! TMDB Mapper
//!
//! Provides mapping functions between TMDB DTOs and domain types

use crate::interfaces::external_services::{
    TmdbMatch, MovieDetail, TvDetail, SeasonDetail, EpisodeDetail, Genre, CollectionInfo,
};
use crate::infrastructure::external::tmdb::dto::{
    TmdbMovieDetailResponse, TmdbTvDetailResponse, TmdbSeasonDetailResponse,
    TmdbEpisodeDetail, TmdbGenre, TmdbCollectionInfo,
};

/// Maps TMDB genre DTO to domain Genre
pub fn map_genre(dto: TmdbGenre) -> Genre {
    Genre {
        id: dto.id,
        name: dto.name,
    }
}

/// Maps TMDB movie detail DTO to domain MovieDetail
pub fn map_movie_detail(dto: TmdbMovieDetailResponse) -> MovieDetail {
    MovieDetail {
        id: dto.id,
        title: dto.title,
        overview: dto.overview,
        release_date: dto.release_date,
        poster_path: dto.poster_path,
        backdrop_path: dto.backdrop_path,
        genres: dto.genres.into_iter().map(map_genre).collect(),
        runtime: dto.runtime,
        vote_average: dto.vote_average,
        vote_count: dto.vote_count,
        imdb_id: dto.imdb_id,
        belongs_to_collection: dto.belongs_to_collection.map(|c| CollectionInfo {
            id: c.id,
            name: c.name,
            poster_path: c.poster_path,
            backdrop_path: c.backdrop_path,
        }),
    }
}

/// Maps TMDB TV detail DTO to domain TvDetail
pub fn map_tv_detail(dto: TmdbTvDetailResponse) -> TvDetail {
    TvDetail {
        id: dto.id,
        name: dto.name,
        overview: dto.overview,
                    first_air_date: dto.first_air_date,
                    last_air_date: dto.last_air_date,
                    status: dto.status,
                    poster_path: dto.poster_path,        backdrop_path: dto.backdrop_path,
        genres: dto.genres.into_iter().map(map_genre).collect(),
        number_of_seasons: dto.number_of_seasons,
        number_of_episodes: dto.number_of_episodes,
        vote_average: dto.vote_average,
        vote_count: dto.vote_count,
        imdb_id: dto.imdb_id,
    }
}

/// Maps TMDB season detail DTO to domain SeasonDetail
pub fn map_season_detail(dto: TmdbSeasonDetailResponse) -> SeasonDetail {
    SeasonDetail {
        id: dto.id,
        season_number: dto.season_number,
        episode_count: dto.episode_count,
        air_date: dto.air_date,
        poster_path: dto.poster_path,
        overview: dto.overview,
        episodes: dto.episodes.into_iter().map(map_episode_detail).collect(),
    }
}

/// Maps TMDB episode detail DTO to domain EpisodeDetail
pub fn map_episode_detail(dto: TmdbEpisodeDetail) -> EpisodeDetail {
    EpisodeDetail {
        id: dto.id,
        episode_number: dto.episode_number,
        season_number: dto.season_number,
        name: dto.name,
        overview: dto.overview,
        air_date: dto.air_date,
        still_path: dto.still_path,
        vote_average: dto.vote_average.unwrap_or(0.0),
        vote_count: dto.vote_count.unwrap_or(0),
        runtime: dto.runtime,
    }
}
