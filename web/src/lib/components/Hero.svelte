<script lang="ts">
    import type { Media } from '$lib/types';
    import { getImageUrl } from '$lib/api';
    import { onMount, onDestroy } from 'svelte';

    let { items = [], onMoreInfo, onPlay, onPlayFromStart }: { 
        items: Media[]; 
        onMoreInfo?: (media: Media) => void;
        onPlay?: (media: Media) => void;
        onPlayFromStart?: (media: Media) => void;
    } = $props();

    // Check if media has watch progress (at least 10 seconds)
    function hasWatchProgress(media: Media): boolean {
        return (media.current_position ?? 0) > 10;
    }

    // Format remaining time
    function formatRemainingTime(media: Media): string {
        if (!media.duration || !media.current_position) return '';
        const remaining = media.duration - media.current_position;
        const hours = Math.floor(remaining / 3600);
        const mins = Math.floor((remaining % 3600) / 60);
        if (hours > 0) {
            return `${hours}h ${mins}m remaining`;
        }
        return `${mins}m remaining`;
    }

    let currentIndex = $state(0);
    let isPaused = $state(false);
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const AUTO_SCROLL_INTERVAL = 8000; // 8 seconds per slide

    function nextSlide() {
        if (items.length === 0) return;
        currentIndex = (currentIndex + 1) % items.length;
    }

    function prevSlide() {
        if (items.length === 0) return;
        currentIndex = (currentIndex - 1 + items.length) % items.length;
    }

    function goToSlide(index: number) {
        currentIndex = index;
        resetInterval();
    }

    function resetInterval() {
        if (intervalId) clearInterval(intervalId);
        if (!isPaused && items.length > 1) {
            intervalId = setInterval(nextSlide, AUTO_SCROLL_INTERVAL);
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'ArrowLeft') {
            e.preventDefault();
            prevSlide();
            resetInterval();
        } else if (e.key === 'ArrowRight') {
            e.preventDefault();
            nextSlide();
            resetInterval();
        }
    }

    onMount(() => {
        if (items.length > 1) {
            intervalId = setInterval(nextSlide, AUTO_SCROLL_INTERVAL);
        }
    });

    onDestroy(() => {
        if (intervalId) clearInterval(intervalId);
    });

    let currentMedia = $derived(items[currentIndex]);
</script>

{#if items.length > 0 && currentMedia}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- Carousel container - mouse/keyboard interactions are intentional for pause-on-hover and navigation -->
    <div
        class="relative h-[60vh] md:h-[80vh] w-full overflow-hidden"
        role="region"
        aria-label="Featured content carousel"
        aria-roledescription="carousel"
        onmouseenter={() => { isPaused = true; if (intervalId) clearInterval(intervalId); }}
        onmouseleave={() => { isPaused = false; resetInterval(); }}
        onkeydown={handleKeydown}
    >
        <!-- Background Images with Crossfade -->
        {#each items as media, index (media.id)}
            <div
                class="absolute inset-0 transition-opacity duration-1000"
                class:opacity-100={index === currentIndex}
                class:opacity-0={index !== currentIndex}
                aria-hidden={index !== currentIndex}
            >
                <img
                    src={getImageUrl(media.backdrop_url || media.poster_url)}
                    alt=""
                    class="h-full w-full object-cover object-top"
                    loading={index === 0 ? 'eager' : 'lazy'}
                />
                <div class="absolute inset-0 bg-gradient-to-t from-[#141414] via-[#141414]/60 to-transparent"></div>
                <div class="absolute inset-0 bg-gradient-to-r from-[#141414] via-[#141414]/60 to-transparent"></div>
            </div>
        {/each}

        <!-- Content -->
        <div class="absolute bottom-20 md:bottom-32 left-0 p-6 md:p-12 lg:p-16 w-full md:w-2/3 lg:w-1/2 flex flex-col items-start space-y-4 z-10">
            {#if currentMedia.series_id}
                <span class="uppercase tracking-widest text-gray-300 font-bold text-sm flex items-center">
                    <span class="text-red-600 mr-2">N</span> SERIES
                </span>
            {/if}

            <!-- Title with crossfade -->
            {#key currentMedia.id}
                <h2
                    class="text-3xl sm:text-4xl md:text-6xl lg:text-7xl font-black text-white drop-shadow-xl leading-tight line-clamp-3 animate-fade-in"
                    style="text-shadow: 2px 4px 12px rgba(0,0,0,0.8)"
                >
                    {currentMedia.title}
                </h2>
            {/key}

            <div class="flex items-center space-x-3 text-sm text-gray-300 font-medium">
                {#if currentMedia.rating}
                    <span class="text-yellow-400 font-bold flex items-center gap-1">
                        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                            <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z"/>
                        </svg>
                        {currentMedia.rating.toFixed(1)}
                    </span>
                {/if}
                {#if currentMedia.release_date}
                    <span>{currentMedia.release_date.split('-')[0]}</span>
                {/if}
                <span class="border border-gray-500 px-1.5 py-0.5 text-xs rounded-[2px] uppercase">
                    {currentMedia.resolution || 'HD'}
                </span>
                {#if currentMedia.duration && currentMedia.media_type === 'movie'}
                    <span>{Math.floor(currentMedia.duration / 3600)}h {Math.floor((currentMedia.duration % 3600) / 60)}m</span>
                {/if}
            </div>

            {#key currentMedia.id}
                <p class="text-white text-base md:text-lg line-clamp-3 drop-shadow-md font-light animate-fade-in">
                    {currentMedia.overview}
                </p>
            {/key}

            <div class="flex flex-wrap gap-3 pt-4">
                {#if hasWatchProgress(currentMedia)}
                    <!-- Resume button (continues from current position) -->
                    <button
                        aria-label="Resume {currentMedia.title}"
                        class="flex items-center space-x-2 bg-white text-black px-6 py-2.5 rounded font-bold hover:bg-gray-200 active:scale-95 transition"
                        onclick={() => onPlay?.(currentMedia)}
                    >
                        <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-7 w-7" viewBox="0 0 20 20" fill="currentColor">
                            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                        </svg>
                        <span class="text-lg">Resume</span>
                    </button>
                    <!-- Start from Beginning button -->
                    <button
                        aria-label="Start {currentMedia.title} from beginning"
                        class="flex items-center space-x-2 bg-gray-500/40 text-white px-6 py-2.5 rounded font-bold hover:bg-gray-500/60 active:scale-95 transition backdrop-blur-md"
                        onclick={() => onPlayFromStart?.(currentMedia)}
                    >
                        <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        <span class="text-lg">Start from Beginning</span>
                    </button>
                {:else}
                    <!-- Normal Play button -->
                <button
                    aria-label="Play {currentMedia.title}"
                    class="flex items-center space-x-2 bg-white text-black px-6 py-2.5 rounded font-bold hover:bg-gray-200 active:scale-95 transition"
                        onclick={() => onPlay?.(currentMedia)}
                >
                    <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-7 w-7" viewBox="0 0 20 20" fill="currentColor">
                        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                    </svg>
                    <span class="text-lg">Play</span>
                </button>
                {/if}
                <button
                    aria-label="More information about {currentMedia.title}"
                    class="flex items-center space-x-2 bg-gray-500/40 text-white px-6 py-2.5 rounded font-bold hover:bg-gray-500/60 active:scale-95 transition backdrop-blur-md"
                    onclick={() => onMoreInfo?.(currentMedia)}
                >
                    <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-7 w-7" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <span class="text-lg">More Info</span>
                </button>
            </div>
        </div>

        <!-- Navigation Arrows -->
        {#if items.length > 1}
            <button
                onclick={() => { prevSlide(); resetInterval(); }}
                aria-label="Previous slide"
                class="absolute left-4 top-1/2 -translate-y-1/2 z-20 p-2 rounded-full bg-black/30 hover:bg-black/60 text-white opacity-0 hover:opacity-100 focus:opacity-100 transition-opacity"
            >
                <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                </svg>
            </button>
            <button
                onclick={() => { nextSlide(); resetInterval(); }}
                aria-label="Next slide"
                class="absolute right-4 top-1/2 -translate-y-1/2 z-20 p-2 rounded-full bg-black/30 hover:bg-black/60 text-white opacity-0 hover:opacity-100 focus:opacity-100 transition-opacity"
            >
                <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
            </button>
        {/if}

        <!-- Slide Indicators - positioned above the content rows overlap zone -->
        {#if items.length > 1}
            <div class="absolute bottom-36 md:bottom-40 left-1/2 -translate-x-1/2 z-30 flex space-x-2" role="tablist" aria-label="Slides">
                {#each items as media, index (media.id)}
                    <button
                        onclick={() => goToSlide(index)}
                        role="tab"
                        aria-selected={index === currentIndex}
                        aria-label="Go to slide {index + 1}: {media.title}"
                        class="group relative h-1 rounded-full transition-all duration-300 {index === currentIndex ? 'w-8 bg-white' : 'w-4 bg-white/40 hover:bg-white/60'}"
                    >
                        <!-- Progress bar for current slide -->
                        {#if index === currentIndex && !isPaused}
                            <div class="absolute inset-0 bg-white/60 rounded-full origin-left animate-progress"></div>
                        {/if}
                    </button>
                {/each}
            </div>
        {/if}
    </div>
{:else}
    <!-- Fallback when no items -->
    <div class="h-[60vh] md:h-[80vh] bg-gradient-to-br from-gray-800 to-gray-900 flex items-center justify-center">
        <p class="text-gray-400 text-xl">No featured content available</p>
    </div>
{/if}

<style>
    @keyframes fade-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }

    .animate-fade-in {
        animation: fade-in 0.5s ease-out;
    }

    @keyframes progress {
        from { transform: scaleX(0); }
        to { transform: scaleX(1); }
    }

    .animate-progress {
        animation: progress 8s linear;
    }
</style>
