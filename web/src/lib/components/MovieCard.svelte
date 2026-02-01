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
                    {isHovered ? 'shadow-2xl shadow-black/80' : ''}">
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

            <!-- Expanded Info Panel (overlay on hover) -->
            {#if isHovered}
                <div class="absolute inset-0 bg-gradient-to-t from-black via-black/60 to-transparent transition-opacity duration-300"></div>
                <div class="absolute bottom-0 left-0 w-full p-3 space-y-2 z-10">
                    <!-- Action Buttons Row -->
                    <div class="flex items-center gap-2">
                        <!-- Play Button -->
                        <button
                            onclick={handlePlayClick}
                            aria-label="Play {media.title}"
                            class="rounded-full bg-white p-2 text-black transition hover:bg-white/80"
                        >
                            <svg class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M8 5v14l11-7z"/>
                            </svg>
                        </button>

                        <!-- More Info Button -->
                        <button
                            onclick={handleClick}
                            aria-label="More info about {media.title}"
                            class="rounded-full border border-gray-400 p-2 text-white transition hover:border-white hover:bg-white/10"
                        >
                            <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
                            </svg>
                        </button>
                    </div>

                    <!-- Title -->
                    <div class="text-sm text-white font-bold truncate drop-shadow-md">{media.title}</div>

                    <!-- Metadata Row - compact -->
                    <div class="flex items-center gap-2 text-[10px] flex-wrap font-medium">
                        {#if media.rating}
                            <span class="text-green-400 whitespace-nowrap flex items-center gap-0.5">
                                {Math.round(media.rating * 10)}% Match
                            </span>
                        {/if}
                        {#if media.release_date}
                            <span class="text-gray-300">{getYear(media.release_date)}</span>
                        {/if}
                        {#if media.resolution}
                            <span class="border border-gray-500 px-1 text-[9px] text-gray-300 rounded whitespace-nowrap bg-black/40">
                                {media.resolution}
                            </span>
                        {/if}
                    </div>
                    
                    <!-- Genre Row -->
                    {#if genres.length > 0}
                        <div class="text-[9px] text-gray-300 flex items-center gap-1 flex-wrap">
                            {#each genres as genre, i}
                                <span>{genre}</span>
                                {#if i < genres.length - 1}
                                    <span class="text-gray-500">•</span>
                                {/if}
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}

            <!-- Progress Bar -->
            {#if media.current_position > 0 && media.duration}
                {@const progressPercent = Math.min((media.current_position / media.duration) * 100, 100)}
                <div class="absolute bottom-0 left-0 right-0 h-1 bg-gray-700/80 z-20">
                    <div class="h-full bg-red-600" style="width: {progressPercent}%"></div>
                </div>
            {/if}
        </div>
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
