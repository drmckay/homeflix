<script lang="ts">
    import type { Media } from '$lib/types';
    import { getImageUrl, getThumbnailUrl } from '$lib/api';

    let { media, rank, onClick, onPlay, isFirst = false, isLast = false }: {
        media: Media;
        rank: number;
        onClick?: (media: Media) => void;
        onPlay?: (media: Media) => void;
        isFirst?: boolean;
        isLast?: boolean;
    } = $props();

    // Check if recently added (within last 7 days)
    let isRecentlyAdded = $derived(() => {
        if (!media.created_at) return false;
        const createdDate = new Date(media.created_at);
        const weekAgo = new Date();
        weekAgo.setDate(weekAgo.getDate() - 7);
        return createdDate > weekAgo;
    });

    function handleClick() {
        onClick?.(media);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onClick?.(media);
        }
    }

    let isHovered = $state(false);
    let hoverTimeout: ReturnType<typeof setTimeout> | null = null;

    function handleMouseEnter() {
        hoverTimeout = setTimeout(() => {
            isHovered = true;
        }, 200);
    }

    function handleMouseLeave() {
        if (hoverTimeout) {
            clearTimeout(hoverTimeout);
            hoverTimeout = null;
        }
        isHovered = false;
    }

    function getTransformOrigin(): string {
        if (isFirst) return 'left center';
        if (isLast) return 'right center';
        return 'center center';
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="top10-card relative flex-shrink-0 cursor-pointer"
    role="button"
    tabindex="0"
    aria-label="#{rank} {media.title}"
    onclick={handleClick}
    onkeydown={handleKeydown}
    onmouseenter={handleMouseEnter}
    onmouseleave={handleMouseLeave}
>
    <!-- Card layout: number on left, poster on right -->
    <div
        class="flex items-end transition-transform duration-300 ease-out {isHovered ? 'scale-105' : ''}"
        style="transform-origin: {getTransformOrigin()};"
    >
        <!-- Large Rank Number - Netflix style outlined -->
        <div class="rank-number flex-shrink-0 select-none pointer-events-none" aria-hidden="true">
            <span class="rank-text">{rank}</span>
        </div>

        <!-- Poster Container -->
        <div class="poster-wrapper relative -ml-4 md:-ml-6 {isHovered ? 'z-10' : 'z-0'}">
            <div class="poster-container relative rounded overflow-hidden bg-gray-800 {isHovered ? 'ring-1 ring-white/30' : ''}">
                {#if media.poster_url}
                    <img
                        src={getImageUrl(media.poster_url)}
                        alt={media.title}
                        class="h-full w-full object-cover"
                        loading="lazy"
                    />
                {:else}
                    <!-- Fallback: generated thumbnail from video -->
                    <img
                        src={getThumbnailUrl(media.id, 200)}
                        alt={media.title}
                        class="h-full w-full object-cover"
                        loading="lazy"
                        onerror={(e: Event) => {
                            const target = e.target as HTMLImageElement;
                            target.style.display = 'none';
                        }}
                    />
                    <div class="absolute inset-0 flex h-full w-full items-center justify-center text-center p-2 text-gray-500 text-xs bg-gray-900 -z-10">
                        {media.title}
                    </div>
                {/if}

                <!-- Recently Added Badge -->
                {#if isRecentlyAdded()}
                    <div class="absolute bottom-2 left-1/2 -translate-x-1/2">
                        <span class="recently-added-badge">Recently Added</span>
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
        </div>
    </div>
</div>

<style>
    .top10-card {
        /* Card dimensions */
        width: 180px;
    }

    @media (min-width: 640px) {
        .top10-card {
            width: 200px;
        }
    }

    @media (min-width: 1024px) {
        .top10-card {
            width: 220px;
        }
    }

    @media (min-width: 1280px) {
        .top10-card {
            width: 240px;
        }
    }

    /* Netflix-style rank number */
    .rank-text {
        font-size: 7rem;
        font-weight: 900;
        line-height: 0.8;
        color: #141414;
        -webkit-text-stroke: 3px #595959;
        paint-order: stroke fill;
    }

    @media (min-width: 640px) {
        .rank-text {
            font-size: 8rem;
            -webkit-text-stroke: 4px #595959;
        }
    }

    @media (min-width: 1024px) {
        .rank-text {
            font-size: 9rem;
        }
    }

    @media (min-width: 1280px) {
        .rank-text {
            font-size: 10rem;
        }
    }

    /* Poster sizing */
    .poster-container {
        width: 100px;
        aspect-ratio: 2/3;
    }

    @media (min-width: 640px) {
        .poster-container {
            width: 110px;
        }
    }

    @media (min-width: 1024px) {
        .poster-container {
            width: 120px;
        }
    }

    @media (min-width: 1280px) {
        .poster-container {
            width: 130px;
        }
    }

    /* Recently Added Badge - Netflix style */
    .recently-added-badge {
        background-color: #e50914;
        color: white;
        font-size: 10px;
        font-weight: 600;
        padding: 2px 6px;
        border-radius: 2px;
        white-space: nowrap;
        text-transform: capitalize;
    }

    @media (min-width: 640px) {
        .recently-added-badge {
            font-size: 11px;
            padding: 3px 8px;
        }
    }
</style>
