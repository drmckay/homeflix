<script lang="ts">
	import type { Media } from '$lib/types';
	import Top10Card from './Top10Card.svelte';
	import { onMount } from 'svelte';

	let {
		title,
		items,
		onMovieClick,
		onPlay
	}: {
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
		const targetScroll =
			direction === 'left'
				? sliderWrapper.scrollLeft - scrollAmount
				: sliderWrapper.scrollLeft + scrollAmount;

		sliderWrapper.scrollTo({
			left: targetScroll,
			behavior: 'smooth'
		});
	}

	onMount(() => {
		updateScrollState();
		window.addEventListener('resize', updateScrollState);
		return () => window.removeEventListener('resize', updateScrollState);
	});
</script>

<section
	class="top10-row pointer-events-none relative z-0 my-4 transition-all duration-300 hover:z-[200] md:my-0"
	aria-label="{title} section"
>
	<!-- Row Header -->
	<div class="group mb-2 flex items-center px-4 md:px-[60px]">
		<h2
			class="flex cursor-pointer items-center text-base font-medium text-white transition-colors hover:text-gray-300 md:text-[1.4vw]"
		>
			{title}
			<svg
				aria-hidden="true"
				xmlns="http://www.w3.org/2000/svg"
				class="ml-1 h-5 w-5 transform text-blue-400 opacity-0 transition-all group-hover:translate-x-1 group-hover:opacity-100"
				viewBox="0 0 20 20"
				fill="currentColor"
			>
				<path
					fill-rule="evenodd"
					d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
					clip-rule="evenodd"
				/>
			</svg>
			<span class="ml-1 text-sm text-blue-400 opacity-0 transition-opacity group-hover:opacity-100">
				Explore All
			</span>
		</h2>
	</div>

	<!-- Slider Container -->
	<div class="slider-container group relative md:-my-14">
		<!-- Left Gradient Fade -->
		{#if showLeftButton}
			<div
				class="pointer-events-none absolute top-0 left-0 z-30 hidden h-full w-12 bg-gradient-to-r from-[#141414] to-transparent md:block md:w-24"
			></div>
		{/if}

		<!-- Left Scroll Button -->
		<button
			class="pointer-events-auto absolute top-0 bottom-0 left-0 z-40 hidden h-auto w-12 cursor-pointer items-center justify-center bg-black/40 text-white
                   transition-all duration-200 hover:bg-black/70
                   md:top-16 md:bottom-16 md:flex md:w-14
                   {showLeftButton
				? 'opacity-0 group-hover:opacity-100'
				: 'pointer-events-none opacity-0'}"
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
			class="slider-wrapper scrollbar-hide pointer-events-none mx-4 snap-x snap-mandatory overflow-x-auto overflow-y-hidden scroll-smooth md:mx-[60px]"
		>
			<!-- Track -->
			<div class="slider-track flex w-max gap-1 py-4 md:py-16">
				{#each items.slice(0, 10) as media, index (media.id)}
					<div class="pointer-events-auto snap-start scroll-ml-4 md:scroll-ml-[60px]">
						<Top10Card
							{media}
							rank={index + 1}
							onClick={onMovieClick}
							{onPlay}
							isFirst={index === 0}
							isLast={index === 9}
						/>
					</div>
				{/each}
			</div>
		</div>

		<!-- Right Scroll Button -->
		<button
			class="pointer-events-auto absolute top-0 right-0 bottom-0 z-40 hidden h-auto w-12 cursor-pointer items-center justify-center bg-black/40 text-white
                   transition-all duration-200 hover:bg-black/70
                   md:top-16 md:bottom-16 md:flex md:w-14
                   {showRightButton
				? 'opacity-0 group-hover:opacity-100'
				: 'pointer-events-none opacity-0'}"
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
			<div
				class="pointer-events-none absolute top-0 right-0 z-30 hidden h-full w-12 bg-gradient-to-l from-[#141414] to-transparent md:block md:w-24"
			></div>
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
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}

	.top10-row {
		overflow: visible;
	}

	.slider-container {
		overflow: visible;
	}
</style>
