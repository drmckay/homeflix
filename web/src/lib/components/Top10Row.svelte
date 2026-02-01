<script lang="ts">
    import type { Media } from '$lib/types';
    import Top10Card from './Top10Card.svelte';
    import { onMount } from 'svelte';

    let { title, items, onMovieClick, onPlay }: { 
        title: string; 
        items: Media[]; 
        onMovieClick?: (media: Media) => void;
        onPlay?: (media: Media) => void;
    } = $props();

    let sliderWrapper: HTMLElement;
    let showLeftButton = $state(false);
    let showRightButton = $state(false);

    function updateScrollState() {
        if (!sliderWrapper) return;
        const { scrollLeft, scrollWidth, clientWidth } = sliderWrapper;
        showLeftButton = scrollLeft > 10;
        showRightButton = scrollLeft < scrollWidth - clientWidth - 10;
    }

    function scroll(direction: 'left' | 'right') {
        if (!sliderWrapper) return;
        const scrollAmount = sliderWrapper.clientWidth * 0.8;
        const targetScroll = direction === 'left'
            ? sliderWrapper.scrollLeft - scrollAmount
            : sliderWrapper.scrollLeft + scrollAmount;
            
        sliderWrapper.scrollTo({
            left: targetScroll,
            behavior: 'smooth'
        });
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

    onMount(() => {
        updateScrollState();
        window.addEventListener('resize', updateScrollState);
        return () => window.removeEventListener('resize', updateScrollState);
    });
</script>

<section
    class="top10-row relative my-4 md:my-0 hover:z-[200] z-0 transition-all duration-300 pointer-events-none"
    aria-label="{title} section"
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

    <!-- Slider Container -->
    <div class="slider-container relative group md:-my-14">
        <!-- Left Gradient Fade -->
        {#if showLeftButton}
            <div class="hidden md:block absolute left-0 top-0 h-full w-12 md:w-24 bg-gradient-to-r from-[#141414] to-transparent z-30 pointer-events-none"></div>
        {/if}

        <!-- Left Scroll Button -->
        <button
            class="hidden md:flex absolute left-0 top-0 md:top-16 bottom-0 md:bottom-16 h-auto z-40 w-12 md:w-14 items-center justify-center
                   bg-black/40 hover:bg-black/70 text-white
                   transition-all duration-200 cursor-pointer pointer-events-auto
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

        <!-- Slider Wrapper - native scrolling -->
        <div 
            bind:this={sliderWrapper} 
            onscroll={updateScrollState}
            class="slider-wrapper mx-4 md:mx-[60px] overflow-x-auto overflow-y-hidden scrollbar-hide scroll-smooth snap-x snap-mandatory pointer-events-none"
        >
            <!-- Track -->
            <div
                onkeydown={handleKeydown}
                tabindex="0"
                role="group"
                aria-label="{title} slider"
                class="slider-track flex gap-1 py-4 md:py-16 w-max"
            >
                {#each items.slice(0, 10) as media, index (media.id)}
                    <div class="snap-start scroll-ml-4 md:scroll-ml-[60px] pointer-events-auto">
                        <Top10Card {media} rank={index + 1} onClick={onMovieClick} {onPlay} isFirst={index === 0} isLast={index === 9} />
                    </div>
                {/each}
            </div>
        </div>

        <!-- Right Scroll Button -->
        <button
            class="hidden md:flex absolute right-0 top-0 md:top-16 bottom-0 md:bottom-16 h-auto z-40 w-12 md:w-14 items-center justify-center
                   bg-black/40 hover:bg-black/70 text-white
                   transition-all duration-200 cursor-pointer pointer-events-auto
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
            <div class="hidden md:block absolute right-0 top-0 h-full w-12 md:w-24 bg-gradient-to-l from-[#141414] to-transparent z-30 pointer-events-none"></div>
        {/if}
    </div>
</section>

<style>
    /* Hide scrollbar for Chrome, Safari and Opera */
    .scrollbar-hide::-webkit-scrollbar {
        display: none;
    }

    /* Hide scrollbar for IE, Edge and Firefox */
    .scrollbar-hide {
        -ms-overflow-style: none;  /* IE and Edge */
        scrollbar-width: none;  /* Firefox */
    }

    .top10-row {
        overflow: visible;
    }

    .slider-container {
        overflow: visible;
    }
</style>
