<script lang="ts">
    import type { Series } from '$lib/types';
    import { getImageUrl } from '$lib/api';

    let { series, isFirst = false, isLast = false, onMoreInfo }: { 
        series: Series; 
        isFirst?: boolean; 
        isLast?: boolean;
        onMoreInfo: (series: Series) => void;
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
        onMoreInfo(series);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onMoreInfo(series);
        }
    }

    function handlePlayClick(e: MouseEvent) {
        e.preventDefault();
        e.stopPropagation();
        onMoreInfo(series);
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="series-card relative flex-shrink-0"
    onmouseenter={handleMouseEnter}
    onmouseleave={handleMouseLeave}
>
    <!-- Placeholder to maintain layout space (2:3 poster aspect ratio) -->
    <div class="aspect-[2/3] w-full"></div>

    <!-- Card content (positioned absolute for hover expansion) -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="series-card-content absolute top-0 left-0 w-full transition-all duration-300 ease-out
               {isHovered ? 'z-[100] scale-[1.3]' : 'z-0 scale-100'}"
        style="transform-origin: {getTransformOrigin()};"
        onclick={handleClick}
        onkeydown={handleKeydown}
        tabindex="0"
        role="button"
        aria-label="View details for {series.title}"
    >
        <!-- Image Container (2:3 poster aspect ratio) -->
        <div class="relative aspect-[2/3] w-full overflow-hidden rounded-md bg-gray-800
                    {isHovered ? 'shadow-2xl shadow-black/80 rounded-b-none' : ''}">
            {#if series.poster_url}
                <img
                    src={getImageUrl(series.poster_url)}
                    alt={series.title}
                    class="h-full w-full object-cover"
                    loading="lazy"
                />
            {:else if series.backdrop_url}
                <img
                    src={getImageUrl(series.backdrop_url)}
                    alt={series.title}
                    class="h-full w-full object-cover"
                    loading="lazy"
                />
            {:else}
                <div class="flex h-full w-full items-center justify-center text-center p-2 text-gray-500 text-xs bg-gray-900">
                    {series.title}
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
                        aria-label="Play {series.title}"
                        class="rounded-full bg-white p-1.5 text-black transition hover:bg-white/80"
                    >
                        <svg class="h-3 w-3" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M8 5v14l11-7z"/>
                        </svg>
                    </button>

                    <!-- More Info Button -->
                    <button
                        onclick={handleClick}
                        aria-label="More info about {series.title}"
                        class="rounded-full border border-gray-500 p-1.5 text-white transition hover:border-white"
                    >
                        <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
                        </svg>
                    </button>
                </div>

                <!-- Title -->
                <div class="text-xs text-white font-medium truncate">{series.title}</div>

                <!-- Metadata Row -->
                <div class="flex items-center gap-1.5 text-[10px] flex-wrap">
                    <span class="text-gray-300 font-medium whitespace-nowrap">
                        {series.total_seasons ?? 0} Season{series.total_seasons !== 1 ? 's' : ''}
                    </span>
                    <span class="text-gray-500">•</span>
                    <span class="text-gray-400 whitespace-nowrap">
                        {series.total_episodes ?? 0} Ep{series.total_episodes !== 1 ? 's' : ''}
                    </span>
                </div>
            </div>
        {/if}
    </div>
</div>

<style>
    .series-card {
        /* Fixed width for poster cards (same as MovieCard) */
        width: 130px;
    }

    @media (min-width: 640px) {
        .series-card {
            width: 145px;
        }
    }

    @media (min-width: 1024px) {
        .series-card {
            width: 160px;
        }
    }

    @media (min-width: 1280px) {
        .series-card {
            width: 175px;
        }
    }

    @media (min-width: 1536px) {
        .series-card {
            width: 190px;
        }
    }
</style>
