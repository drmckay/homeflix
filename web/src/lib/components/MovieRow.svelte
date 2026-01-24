<script lang="ts">
    import type { Media } from '$lib/types';
    import MovieCard from './MovieCard.svelte';
    import { onMount } from 'svelte';

    let { title, items, onMovieClick, onPlay }: { 
        title: string; 
        items: Media[]; 
        onMovieClick?: (media: Media) => void;
        onPlay?: (media: Media) => void;
    } = $props();

    let sliderTrack: HTMLElement;
    let sliderWrapper: HTMLElement;
    let scrollPosition = $state(0);
    let maxScroll = $state(0);
    let isRowHovered = $state(false);
    let showLeftButton = $derived(scrollPosition > 10);
    let showRightButton = $derived(scrollPosition < maxScroll - 10);

    function updateMaxScroll() {
        if (!sliderTrack || !sliderWrapper) return;
        const trackWidth = sliderTrack.scrollWidth;
        const wrapperWidth = sliderWrapper.clientWidth;
        maxScroll = Math.max(0, trackWidth - wrapperWidth);
    }

    function scroll(direction: 'left' | 'right') {
        if (!sliderWrapper) return;
        const scrollAmount = sliderWrapper.clientWidth - 100;
        const newPosition = direction === 'left'
            ? Math.max(0, scrollPosition - scrollAmount)
            : Math.min(maxScroll, scrollPosition + scrollAmount);
        scrollPosition = newPosition;
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'ArrowRight') {
            e.preventDefault();
            scroll('right');
        } else if (e.key === 'ArrowLeft') {
            e.preventDefault();
            scroll('left');
        }
    }

    function handleWheel(e: WheelEvent) {
        // Convert vertical scroll to horizontal, or use horizontal scroll directly
        const delta = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;

        if (delta === 0) return;

        // Only prevent default if we can actually scroll in that direction
        const canScrollRight = scrollPosition < maxScroll - 10;
        const canScrollLeft = scrollPosition > 10;

        if ((delta > 0 && canScrollRight) || (delta < 0 && canScrollLeft)) {
            e.preventDefault();
            const newPosition = Math.max(0, Math.min(maxScroll, scrollPosition + delta));
            scrollPosition = newPosition;
        }
    }

    onMount(() => {
        updateMaxScroll();
        window.addEventListener('resize', updateMaxScroll);
        return () => window.removeEventListener('resize', updateMaxScroll);
    });
</script>

<section
    class="movie-row relative py-0 {isRowHovered ? 'z-[200]' : 'z-0'}"
    aria-label="{title} section"
    onmouseenter={() => isRowHovered = true}
    onmouseleave={() => isRowHovered = false}
>
    <!-- Row Header -->
    <div class="px-4 md:px-[60px] mb-2 flex items-center group">
        <h2 class="text-white text-base md:text-[1.4vw] font-medium flex items-center cursor-pointer hover:text-gray-300 transition-colors">
            {title}
            <svg
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                class="h-5 w-5 ml-1 text-blue-400 opacity-0 group-hover:opacity-100 transition-all transform group-hover:translate-x-1"
                viewBox="0 0 20 20"
                fill="currentColor"
            >
                <path
                    fill-rule="evenodd"
                    d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                    clip-rule="evenodd"
                />
            </svg>
            <span class="text-sm text-blue-400 ml-1 opacity-0 group-hover:opacity-100 transition-opacity">
                Explore All
            </span>
        </h2>
    </div>

    <!-- Slider Container with overflow visible for hover cards -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="slider-container relative group" onwheel={handleWheel}>
        <!-- Left Gradient Fade -->
        {#if showLeftButton}
            <div class="absolute left-0 top-0 h-full w-12 md:w-24 bg-gradient-to-r from-[#141414] to-transparent z-30 pointer-events-none"></div>
        {/if}

        <!-- Left Scroll Button -->
        <button
            class="absolute left-0 top-0 h-full z-40 w-12 md:w-14 flex items-center justify-center
                   bg-black/40 hover:bg-black/70 text-white
                   transition-all duration-200 cursor-pointer
                   {showLeftButton ? 'opacity-0 group-hover:opacity-100' : 'opacity-0 pointer-events-none'}"
            onclick={() => scroll('left')}
            aria-label="Scroll {title} left"
            tabindex={showLeftButton ? 0 : -1}
        >
            <svg
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                class="h-8 w-8 md:h-10 md:w-10"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="2"
            >
                <path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" />
            </svg>
        </button>

        <!-- Slider Wrapper - clips horizontally but allows vertical overflow -->
        <div
            bind:this={sliderWrapper}
            class="slider-wrapper mx-4 md:mx-[60px]"
        >
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
            <div
                bind:this={sliderTrack}
                onkeydown={handleKeydown}
                tabindex="0"
                role="group"
                aria-label="{title} slider"
                class="slider-track flex gap-2 py-4 pb-8 transition-transform duration-500 ease-out focus:outline-none focus-visible:ring-2 focus-visible:ring-white"
                style="transform: translateX(-{scrollPosition}px);"
            >
                {#each items as media, index (media.id)}
                    <MovieCard {media} onClick={onMovieClick} {onPlay} isFirst={index === 0} isLast={index === items.length - 1} />
                {/each}
            </div>
        </div>

        <!-- Right Scroll Button -->
        <button
            class="absolute right-0 top-0 h-full z-40 w-12 md:w-14 flex items-center justify-center
                   bg-black/40 hover:bg-black/70 text-white
                   transition-all duration-200 cursor-pointer
                   {showRightButton ? 'opacity-0 group-hover:opacity-100' : 'opacity-0 pointer-events-none'}"
            onclick={() => scroll('right')}
            aria-label="Scroll {title} right"
            tabindex={showRightButton ? 0 : -1}
        >
            <svg
                aria-hidden="true"
                xmlns="http://www.w3.org/2000/svg"
                class="h-8 w-8 md:h-10 md:w-10"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="2"
            >
                <path stroke-linecap="round" stroke-linejoin="round" d="M9 5l7 7-7 7" />
            </svg>
        </button>

        <!-- Right Gradient Fade -->
        {#if showRightButton}
            <div class="absolute right-0 top-0 h-full w-12 md:w-24 bg-gradient-to-l from-[#141414] to-transparent z-30 pointer-events-none"></div>
        {/if}
    </div>
</section>

<style>
    /* Movie row allows overflow for expanded hover cards */
    .movie-row {
        overflow: visible;
    }

    /* Slider container with visible overflow for hover effects */
    .slider-container {
        overflow: visible;
    }

    /* Wrapper clips horizontal overflow but allows vertical */
    .slider-wrapper {
        overflow-x: clip;
        overflow-y: visible;
    }

    /* Track contains the cards and transforms for scrolling */
    .slider-track {
        width: max-content;
    }
</style>
