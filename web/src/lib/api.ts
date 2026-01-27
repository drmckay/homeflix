import type { GroupedLibrary, Series, SeriesDetails, CollectionSummary, CollectionDetails, CollectionItem, Media, Credits } from './types';
import { env } from '$env/dynamic/public';
import { browser } from '$app/environment';

/**
 * Get API base URL from runtime configuration
 * 
 * - Server-side: Uses $env/dynamic/public which reads from runtime environment variables
 * - Client-side: Uses relative URLs (same domain) for simplicity
 * 
 * This allows the API URL to be configured at runtime via PUBLIC_API_URL environment variable
 * for server-side requests. Client-side requests use relative URLs since they're on the same domain.
 */
export function getApiBase(): string {
	if (!browser) {
		// Server-side: use runtime environment variable (full URL needed for server-to-server calls)
		return env.PUBLIC_API_URL || import.meta.env.VITE_API_URL || 'http://localhost:3000';
	}
	
	// Client-side: use relative URLs (empty string) since frontend and backend are on same domain
	// This works because the browser will use the current origin
	return '';
}

type FetchFn = typeof fetch;

export async function fetchGroupedLibrary(customFetch: FetchFn = fetch): Promise<GroupedLibrary> {
    const res = await customFetch(`${getApiBase()}/v2/media`);
    if (!res.ok) {
        throw new Error('Failed to fetch library');
    }
    return res.json();
}

export async function fetchAllSeries(customFetch: FetchFn = fetch): Promise<Series[]> {
    const res = await customFetch(`${getApiBase()}/v2/series`);
    if (!res.ok) {
        throw new Error('Failed to fetch series');
    }
    return res.json();
}

export async function fetchSeriesDetails(id: number, customFetch: FetchFn = fetch): Promise<SeriesDetails> {
    const res = await customFetch(`${getApiBase()}/v2/series/${id}`);
    if (!res.ok) {
        throw new Error('Failed to fetch series details');
    }
    return res.json();
}

// Raw types from API (before enrichment)
interface RawCollectionSummary extends Omit<CollectionSummary, 'completion_percentage'> {
    // API response doesn't have completion_percentage
}

interface RawCollectionItem extends Omit<CollectionItem, 'release_order' | 'season_number' | 'episode_number'> {
    // API response doesn't have these fields
}

interface RawCollectionDetails extends Omit<CollectionDetails, 'items' | 'completion_percentage'> {
    items: RawCollectionItem[];
}

export async function fetchCollections(customFetch: FetchFn = fetch): Promise<CollectionSummary[]> {
    const res = await customFetch(`${getApiBase()}/v2/collections`);
    if (!res.ok) {
        throw new Error('Failed to fetch collections');
    }
    const collections: RawCollectionSummary[] = await res.json();
    return collections.map((collection) => ({
        ...collection,
        completion_percentage:
            collection.total_items > 0
                ? (collection.available_items / collection.total_items) * 100
                : 0
    }));
}

export async function fetchCollectionDetails(id: number, customFetch: FetchFn = fetch): Promise<CollectionDetails> {
    const res = await customFetch(`${getApiBase()}/v2/collections/${id}`);
    if (!res.ok) {
        throw new Error('Failed to fetch collection details');
    }
    const details: RawCollectionDetails = await res.json();
    const completion_percentage =
        details.total_items > 0 ? (details.available_items / details.total_items) * 100 : 0;

    return {
        ...details,
        completion_percentage,
        items: details.items.map((item) => ({
            ...item,
            release_order: item.timeline_order, // Default to timeline order if release order is missing (it is missing in API)
            season_number: null,
            episode_number: null
        }))
    };
}

export function getImageUrl(path: string | null): string {
	if (!path) return '';
	const apiBase = getApiBase();
	if (path.startsWith('https://image.tmdb.org/')) {
		return `${apiBase}/v2/images/proxy?url=${encodeURIComponent(path)}`;
	}
	if (path.startsWith('http')) return path;
	return `${apiBase}${path}`; // Handle relative paths served by backend
}

/**
 * Get thumbnail URL for a media item.
 * Used as fallback when media doesn't have a poster image.
 */
export function getThumbnailUrl(mediaId: number, width: number = 320): string {
	return `${getApiBase()}/v2/thumbnail/${mediaId}?width=${width}`;
}

// Media details with audio tracks
export interface MediaDetails {
	id: number;
	file_path: string;
	title: string;
	overview: string | null;
	poster_url: string | null;
	duration: number | null;
	release_date: string | null;
	resolution: string | null;
	genres: string | null;
	media_type: string;
	series_id: number | null;
	season_number: number | null;
	episode_number: number | null;
	current_position: number;
	is_watched: boolean;
	subtitles: SubtitleTrack[];
	audio_tracks: AudioTrack[];
}

/// Subtitle track from backend /v2/media/{id}/tracks endpoint
export interface SubtitleTrack {
	/// Track index for URL construction
	index: number;
	/// ISO 639-1 language code (e.g., "hu", "en")
	language: string | null;
	/// Human-readable language name (e.g., "Magyar", "English")
	language_name: string | null;
	/// Source of subtitle: "external" (.srt file) or "embedded" (in video container)
	source: 'external' | 'embedded';
	/// Whether this is the default track
	is_default: boolean;
}

export interface AudioTrack {
	index: number;
	codec: string | null;
	language: string | null;
	channels: number | null;
	title: string | null;
	is_default: boolean;
}

/// Response from /v2/media/{id}/tracks endpoint
export interface MediaTracksResponse {
	duration: number;
	current_position: number;
	is_watched: boolean;
	audio_tracks: AudioTrack[];
	subtitle_tracks: SubtitleTrack[];
}

export async function fetchMediaDetails(
	id: number,
	customFetch: FetchFn = fetch
): Promise<MediaDetails> {
	const res = await customFetch(`${getApiBase()}/v2/media/${id}`);
	if (!res.ok) {
		throw new Error('Failed to fetch media details');
	}
	return res.json();
}

// Streaming API
export function getStreamUrl(mediaId: number): string {
	return `${getApiBase()}/v2/stream/${mediaId}`;
}

/**
 * Get subtitle URL for a media item.
 * Returns WebVTT format subtitle content.
 */
export function getSubtitleUrl(mediaId: number, index: number): string {
	return `${getApiBase()}/v2/subtitles/${mediaId}/${index}`;
}

/**
 * Fetch media tracks (audio and subtitles) for a media item.
 */
export async function fetchMediaTracks(
	mediaId: number,
	customFetch: FetchFn = fetch
): Promise<MediaTracksResponse> {
	const res = await customFetch(`${getApiBase()}/v2/media/${mediaId}/tracks`);
	if (!res.ok) {
		throw new Error('Failed to fetch media tracks');
	}
	return res.json();
}

// Progress saving
export async function saveProgress(
	mediaId: number,
	position: number,
	isWatched: boolean
): Promise<void> {
	await fetch(`${getApiBase()}/v2/progress/${mediaId}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({
			current_position_seconds: Math.floor(position),
			is_watched: isWatched
		})
	});
}

// Fetch similar media based on shared genres
export async function fetchSimilarMedia(
	mediaId: number,
	customFetch: FetchFn = fetch
): Promise<Media[]> {
	const res = await customFetch(`${getApiBase()}/v2/media/${mediaId}/similar`);
	if (!res.ok) {
		return []; // Return empty array on error
	}
	return res.json();
}

// Fetch cast and crew credits for a media item
export async function fetchMediaCredits(
	mediaId: number,
	customFetch: FetchFn = fetch
): Promise<Credits> {
	const res = await customFetch(`${getApiBase()}/v2/media/${mediaId}/credits`);
	if (!res.ok) {
		return { cast: [], crew: [] }; // Return empty on error
	}
	return res.json();
}

// ============== Subtitle Generation API ==============

/// Service capabilities response
export interface ServiceCapabilities {
	whisper_available: boolean;
	whisper_model_exists: boolean;
	ollama_available: boolean;
	fpcalc_available: boolean;
}

/// Job status for tracking subtitle generation
export interface JobStatus {
	id: string;
	state: 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';
	progress: number;
	message: string | null;
	result: GenerateSubtitleResult | null;
	error: string | null;
	created_at: string;
	updated_at: string;
	completed_at: string | null;
}

/// Batch job status
export interface BatchJobStatus {
	id: string;
	state: 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';
	total: number;
	completed: number;
	failed: number;
	errors: Record<number, string>;
	created_at: string;
	updated_at: string;
	completed_at: string | null;
}

/// Result of subtitle generation
export interface GenerateSubtitleResult {
	subtitle_path: string;
	language: string;
	was_translated: boolean;
	audio_fingerprint: string;
	duration_seconds: number;
}

/// Request body for subtitle generation
export interface GenerateSubtitleRequest {
	audio_track_index: number;
	source_language: string | null;
	target_language: string | null;
}

/// Request body for batch subtitle generation
export interface BatchGenerateRequest {
	target_type: 'series' | 'season';
	target_id: number;
	season_number?: number;
	/// Preferred audio language code (e.g., "hun", "eng", "jpn")
	/// The system will automatically find the matching audio track for each episode.
	preferred_audio_language: string | null;
	source_language: string | null;
	target_language: string | null;
}

/// Fetch subtitle generation service capabilities
export async function fetchSubtitleCapabilities(
	customFetch: FetchFn = fetch
): Promise<ServiceCapabilities> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/capabilities`);
	if (!res.ok) {
		throw new Error('Failed to fetch subtitle capabilities');
	}
	return res.json();
}

/// Start subtitle generation for a single media item
export async function generateSubtitle(
	mediaId: number,
	request: GenerateSubtitleRequest,
	customFetch: FetchFn = fetch
): Promise<{ job_id: string; status: string }> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/${mediaId}/generate`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) {
		const error = await res.text();
		throw new Error(error || 'Failed to start subtitle generation');
	}
	return res.json();
}

/// Get job status
export async function fetchJobStatus(
	jobId: string,
	customFetch: FetchFn = fetch
): Promise<JobStatus> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/jobs/${jobId}`);
	if (!res.ok) {
		throw new Error('Failed to fetch job status');
	}
	return res.json();
}

/// Cancel a running job
export async function cancelJob(
	jobId: string,
	customFetch: FetchFn = fetch
): Promise<void> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/jobs/${jobId}`, {
		method: 'DELETE'
	});
	if (!res.ok) {
		const error = await res.text();
		throw new Error(error || 'Failed to cancel job');
	}
}

/// Start batch subtitle generation
export async function batchGenerateSubtitles(
	request: BatchGenerateRequest,
	customFetch: FetchFn = fetch
): Promise<{ job_id: string; status: string }> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/batch/generate`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(request)
	});
	if (!res.ok) {
		const error = await res.text();
		throw new Error(error || 'Failed to start batch subtitle generation');
	}
	return res.json();
}

/// Get batch job status
export async function fetchBatchJobStatus(
	jobId: string,
	customFetch: FetchFn = fetch
): Promise<BatchJobStatus> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/batch/jobs/${jobId}`);
	if (!res.ok) {
		throw new Error('Failed to fetch batch job status');
	}
	return res.json();
}

/// Cancel a running batch job
export async function cancelBatchJob(
	jobId: string,
	customFetch: FetchFn = fetch
): Promise<void> {
	const res = await customFetch(`${getApiBase()}/v2/subtitles/batch/jobs/${jobId}`, {
		method: 'DELETE'
	});
	if (!res.ok) {
		const error = await res.text();
		throw new Error(error || 'Failed to cancel batch job');
	}
}
