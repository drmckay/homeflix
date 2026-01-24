export interface Media {
    id: number;
    file_path: string;
    title: string;
    overview: string | null;
    poster_url: string | null;
    backdrop_url: string | null;
    trailer_url: string | null;
    duration: number | null; // seconds
    release_date: string | null;
    resolution: string | null;
    genres: string | null;
    media_type: string; // "movie" | "episode" | "series" (synthetic for recently added)
    series_id: number | null;
    season_number: number | null;
    episode_number: number | null;
    created_at: string;
    tmdb_id: number | null;
    original_title: string | null;
    rating: number | null;
    content_rating: string | null; // e.g., "PG-13", "R", "TV-MA"
    content_warnings: string | null; // e.g., "violence, language"
    current_position: number;
    is_watched: boolean;
}

export interface GroupedLibrary {
    recent: Media[];
    continue_watching: Media[];
    categories: Record<string, Media[]>;
}

// Series types
export interface Series {
    id: number;
    title: string;
    overview: string | null;
    poster_url: string | null;
    backdrop_url: string | null;
    tmdb_id: number | null;
    total_seasons: number | null;
    total_episodes: number | null;
    rating: number | null;
    first_air_date: string | null;
}

export interface SeasonGroup {
    season_number: number;
    episodes: Media[];
}

export interface SeriesDetails {
    series: {
        id: number;
        title: string;
        overview: string | null;
        poster_url: string | null;
        tmdb_id: number | null;
    };
    seasons: SeasonGroup[];
}

// Collection types
export interface CollectionSummary {
    id: number;
    name: string;
    description: string | null;
    poster_url: string | null;
    backdrop_url: string | null;
    total_items: number;
    available_items: number;
    completion_percentage: number;
    sort_mode: string; // "timeline" | "release"
    collection_type: string;
}

export interface CollectionItem {
    id: number | null;
    collection_id: number;
    media_id: number | null;
    tmdb_id: number;
    media_type: string; // "movie", "tv", "episode"
    title: string;
    overview: string | null;
    poster_url: string | null;
    release_date: string | null;
    timeline_order: number;
    release_order: number;
    timeline_year: number | null;
    timeline_notes: string | null;
    season_number: number | null;  // For TV: start of season range
    episode_number: number | null; // For TV: end of season range (repurposed)
    is_available: boolean;
}

export interface CollectionDetails {
    id: number;
    name: string;
    description: string | null;
    poster_url: string | null;
    backdrop_url: string | null;
    total_items: number;
    available_items: number;
    sort_mode: string;
    collection_type: string;
    tmdb_collection_id: number | null;
    items: CollectionItem[];
    completion_percentage: number;
}

// Credits types
export interface CastMember {
    id: number;
    name: string;
    character: string;
    profile_url: string | null;
    order: number;
}

export interface CrewMember {
    id: number;
    name: string;
    job: string;
    department: string;
    profile_url: string | null;
}

export interface Credits {
    cast: CastMember[];
    crew: CrewMember[];
}
