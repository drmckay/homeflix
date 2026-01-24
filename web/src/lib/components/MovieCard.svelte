<script lang="ts">
    import type { Media } from '$lib/types';
    import { getImageUrl, getThumbnailUrl } from '$lib/api';

    let { media, onClick, onPlay, isFirst = false, isLast = false }: { 
        media: Media; 
        onClick?: (media: Media) => void;
        onPlay?: (media: Media) => void;
        isFirst?: boolean;
        isLast?: boolean;
    } = $props();

    function getTransformOrigin(): string {
        if (isFirst) return 'left center';
        if (isLast) return 'right center';
        return 'center center';
    }

    let isHovered = $state(false);
    let hoverTimeout: ReturnType<typeof setTimeout> | null = null;

    function handleMouseEnter() {
        hoverTimeout = setTimeout(() => {
            isHovered = true;
        }, 300); // Netflix-style delayed expansion
    }

    function handleMouseLeave() {
        if (hoverTimeout) {
            clearTimeout(hoverTimeout);
            hoverTimeout = null;
        }
        isHovered = false;
    }

    function handleClick(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        onClick?.(media);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onClick?.(media);
        }
    }

    function handlePlayClick(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        onPlay?.(media);
    }

    function handleAddToListClick(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        // TODO: Add to list
        console.log('Add to list:', media.title);
    }

    function handleLikeClick(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        // TODO: Like
        console.log('Like:', media.title);
    }

    function formatDuration(seconds: number | null): string {
        if (!seconds) return '';
        const hours = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        if (hours > 0) {
            return `${hours}h ${mins}m`;
        }
        return `${mins}m`;
    }

    function getYear(date: string | null): string {
        if (!date) return '';
        return new Date(date).getFullYear().toString();
    }

    function getGenres(genres: string | null): string[] {
        if (!genres) return [];
        return genres.split(',').map(g => g.trim()).slice(0, 3);
    }

    let genres = $derived(getGenres(media.genres));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="movie-card relative flex-shrink-0"
    onmouseenter={handleMouseEnter}
    onmouseleave={handleMouseLeave}
>
    <!-- Placeholder to maintain layout space (2:3 poster aspect ratio) -->
    <div class="aspect-[2/3] w-full"></div>

    <!-- Card content (positioned absolute for hover expansion) -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="movie-card-content absolute top-0 left-0 w-full transition-all duration-300 ease-out
               {isHovered ? 'z-[100] scale-[1.3]' : 'z-0 scale-100'}"
        style="transform-origin: {getTransformOrigin()};"
        onclick={handleClick}
        onkeydown={handleKeydown}
        tabindex="0"
        role="button"
        aria-label="View details for {media.title}"
    >
        <!-- Image Container (2:3 poster aspect ratio) -->
        <div class="relative aspect-[2/3] w-full overflow-hidden rounded-md bg-gray-800
                    {isHovered ? 'shadow-2xl shadow-black/80 rounded-b-none' : ''}">
            {#if media.poster_url}
                <img
                    src={getImageUrl(media.poster_url)}
                    alt={media.title}
                    class="h-full w-full object-cover"
                    loading="lazy"
                />
            {:else if media.backdrop_url}
                <img
                    src={getImageUrl(media.backdrop_url)}
                    alt={media.title}
                    class="h-full w-full object-cover"
                    loading="lazy"
                />
            {:else}
                <!-- Fallback: generated thumbnail from video -->
                <img
                    src={getThumbnailUrl(media.id, 320)}
                    alt={media.title}
                    class="h-full w-full object-cover"
                    loading="lazy"
                    onerror={(e: Event) => {
                        const target = e.target as HTMLImageElement;
                        target.style.display = 'none';
                    }}
                />
                <!-- Show title as backup if thumbnail fails -->
                <div class="absolute inset-0 flex h-full w-full items-center justify-center text-center p-2 text-gray-500 text-xs bg-gray-900 -z-10">
                    {media.title}
                </div>
            {/if}

            <!-- Progress Bar -->
            {#if media.current_position > 0 && media.duration}
                {@const progressPercent = Math.min((media.current_position / media.duration) * 100, 100)}
                <div class="absolute bottom-0 left-0 right-0 h-1 bg-gray-700/80">
                    <div class="h-full bg-red-600" style="width: {progressPercent}%"></div>
                </div>
            {/if}
        </div>

        <!-- Expanded Info Panel (only visible on hover) -->
        {#if isHovered}
            <div class="bg-[#181818] p-2 rounded-b-md space-y-1.5 shadow-2xl shadow-black/80">
                <!-- Action Buttons Row -->
                <div class="flex items-center gap-1.5">
                    <!-- Play Button -->
                    <button
                        onclick={handlePlayClick}
                        aria-label="Play {media.title}"
                        class="rounded-full bg-white p-1.5 text-black transition hover:bg-white/80"
                    >
                        <svg class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M8 5v14l11-7z"/>
                        </svg>
                    </button>

                    <!-- More Info Button -->
                    <button
                        onclick={handleClick}
                        aria-label="More info about {media.title}"
                        class="rounded-full border border-gray-500 p-1.5 text-white transition hover:border-white"
                    >
                        <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
                        </svg>
                    </button>
                </div>

                <!-- Title (visible on hover since poster doesn't have title) -->
                <div class="text-xs text-white font-medium truncate">{media.title}</div>

                <!-- Metadata Row - compact -->
                <div class="flex items-center gap-1.5 text-[10px] flex-wrap">
                    {#if media.rating}
                        <span class="text-yellow-400 font-semibold whitespace-nowrap flex items-center gap-0.5">
                            <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 20 20">
                                <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"/>
                            </svg>
                            {media.rating.toFixed(1)}
                        </span>
                    {/if}
                    {#if media.release_date}
                        <span class="text-gray-400">{getYear(media.release_date)}</span>
                    {/if}
                    {#if media.duration && media.media_type === 'movie'}
                        <span class="text-gray-400 whitespace-nowrap">{formatDuration(media.duration)}</span>
                    {/if}
                    {#if media.resolution}
                        <span class="border border-gray-600 px-1 text-[8px] text-gray-400 rounded whitespace-nowrap">
                            {media.resolution}
                        </span>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    .movie-card {
        /* Fixed width for poster cards (narrower than backdrop) */
        width: 130px;
    }

    @media (min-width: 640px) {
        .movie-card {
            width: 145px;
        }
    }

    @media (min-width: 1024px) {
        .movie-card {
            width: 160px;
        }
    }

    @media (min-width: 1280px) {
        .movie-card {
            width: 175px;
        }
    }

    @media (min-width: 1536px) {
        .movie-card {
            width: 190px;
        }
    }
</style>
