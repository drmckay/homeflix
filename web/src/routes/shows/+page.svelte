<script lang="ts">
	import type { PageData } from './$types';
	import type { Series, Media } from '$lib/types';
	import SeriesCard from '$lib/components/SeriesCard.svelte';
	import SeriesRow from '$lib/components/SeriesRow.svelte';
	import SeriesModal from '$lib/components/SeriesModal.svelte';
	import VideoPlayer from '$lib/components/VideoPlayer.svelte';
	import { getImageUrl } from '$lib/api';

	let { data }: { data: PageData } = $props();

	// View mode: 'list' or 'grid'
	let viewMode = $state<'list' | 'grid'>('list');

	// Sort option (for grid view)
	type SortOption = 'suggestions' | 'az' | 'za' | 'episodes';
	let sortBy = $state<SortOption>('suggestions');

	// Dropdown states
	let sortDropdownOpen = $state(false);

	// Series modal state
	let selectedSeries: Series | null = $state(null);

	// Video player state
	let playingMedia: Media | null = $state(null);

	function openSeriesModal(series: Series) {
		selectedSeries = series;
	}

	function closeSeriesModal() {
		selectedSeries = null;
	}

	function playEpisode(episode: Media) {
		closeSeriesModal();
		playingMedia = episode;
	}

	function closePlayer() {
		playingMedia = null;
	}

	// Sort series
	let sortedSeries = $derived(() => {
		const series = [...data.series];
		switch (sortBy) {
			case 'az':
				return series.sort((a, b) => a.title.localeCompare(b.title));
			case 'za':
				return series.sort((a, b) => b.title.localeCompare(a.title));
			case 'episodes':
				return series.sort((a, b) => (b.total_episodes ?? 0) - (a.total_episodes ?? 0));
			case 'suggestions':
			default:
				// Sort by episode count (suggestions/popularity proxy)
				return series.sort((a, b) => (b.total_episodes ?? 0) - (a.total_episodes ?? 0));
		}
	});

	// Group series by criteria for list view (by season count ranges)
	let seriesByCategory = $derived(() => {
		const categories: { name: string; items: Series[] }[] = [];
		const series = data.series;

		// Recently added (first 10 by natural order)
		const recent = series.slice(0, 10);
		if (recent.length > 0) {
			categories.push({ name: 'Recently Added', items: recent });
		}

		// Long-running shows (5+ seasons)
		const longRunning = series.filter((s) => (s.total_seasons ?? 0) >= 5);
		if (longRunning.length > 0) {
			categories.push({ name: 'Long-Running Series', items: longRunning });
		}

		// Mini-series (1 season)
		const miniSeries = series.filter((s) => (s.total_seasons ?? 0) === 1);
		if (miniSeries.length > 0) {
			categories.push({ name: 'Limited Series', items: miniSeries });
		}

		// Multi-season (2-4 seasons)
		const multiSeason = series.filter((s) => {
			const seasons = s.total_seasons ?? 0;
			return seasons >= 2 && seasons <= 4;
		});
		if (multiSeason.length > 0) {
			categories.push({ name: 'Multi-Season Series', items: multiSeason });
		}

		return categories;
	});

	function handleSortSelect(option: SortOption) {
		sortBy = option;
		sortDropdownOpen = false;
	}

	const sortLabels: Record<SortOption, string> = {
		suggestions: 'Suggestions For You',
		az: 'A-Z',
		za: 'Z-A',
		episodes: 'Most Episodes'
	};
</script>

<svelte:head>
	<title>Shows - Homeflix</title>
</svelte:head>

<main class="min-h-screen bg-[#141414] text-white pt-20">
	<!-- Filter Bar -->
	<div class="relative z-10 bg-[#141414] pb-4">
		<div class="px-4 md:px-[60px] flex items-center gap-4 flex-wrap">
			<!-- Title -->
			<h1 class="text-2xl md:text-4xl font-bold text-white">Shows</h1>

			<div class="flex-1"></div>

			<!-- View Toggle -->
			<div class="flex items-center gap-1 bg-black/60 rounded p-1 border border-gray-700">
				<button
					class="p-2 rounded transition {viewMode === 'list' ? 'bg-white/20 text-white' : 'text-gray-400 hover:text-white'}"
					onclick={() => (viewMode = 'list')}
					aria-label="List view"
				>
					<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16" />
					</svg>
				</button>
				<button
					class="p-2 rounded transition {viewMode === 'grid' ? 'bg-white/20 text-white' : 'text-gray-400 hover:text-white'}"
					onclick={() => (viewMode = 'grid')}
					aria-label="Grid view"
				>
					<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
						<path stroke-linecap="round" stroke-linejoin="round" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
					</svg>
				</button>
			</div>

			<!-- Sort Dropdown (Grid view only) -->
			{#if viewMode === 'grid'}
				<div class="relative">
					<button
						class="flex items-center gap-2 px-4 py-2 bg-black/60 border border-gray-600 rounded text-white text-sm hover:bg-black/80 transition"
						onclick={() => (sortDropdownOpen = !sortDropdownOpen)}
					>
						<span>{sortLabels[sortBy]}</span>
						<svg
							class="w-4 h-4 transition-transform {sortDropdownOpen ? 'rotate-180' : ''}"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
						>
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
						</svg>
					</button>

					{#if sortDropdownOpen}
						<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
						<div
							class="fixed inset-0 z-40"
							onclick={() => (sortDropdownOpen = false)}
						></div>
						<div class="absolute top-full right-0 mt-1 z-50 bg-black/95 border border-gray-700 rounded-md shadow-2xl min-w-[180px]">
							<div class="py-1">
								{#each Object.entries(sortLabels) as [key, label]}
									<button
										class="w-full text-left px-4 py-2 text-sm hover:bg-white/10 transition {sortBy === key ? 'text-white font-semibold' : 'text-gray-300'}"
										onclick={() => handleSortSelect(key as SortOption)}
									>
										{label}
									</button>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- Content -->
	<div class="pb-20">
		{#if viewMode === 'list'}
			<!-- List View: Horizontal rows by category -->
			<div class="space-y-4">
				{#each seriesByCategory() as { name, items }}
					{#if items.length > 0}
						<SeriesRow title={name} {items} onSeriesClick={openSeriesModal} />
					{/if}
				{/each}
			</div>

			{#if seriesByCategory().length === 0}
				<div class="text-center py-20 text-gray-400 px-4 md:px-[60px]">
					<p class="text-xl">No shows found</p>
				</div>
			{/if}
		{:else}
			<!-- Grid View -->
			<div class="px-4 md:px-[60px]">
				<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
					{#each sortedSeries() as series (series.id)}
						<button
							class="series-grid-item group relative text-left w-full"
							onclick={() => openSeriesModal(series)}
							aria-label="View details for {series.title}"
						>
							<div class="relative aspect-video rounded-md overflow-hidden bg-gray-800 transition-all duration-200 group-hover:ring-2 group-hover:ring-white/40">
								{#if series.poster_url}
									<img
										src={getImageUrl(series.poster_url)}
										alt={series.title}
										class="h-full w-full object-cover"
										loading="lazy"
									/>
								{:else}
									<div class="flex h-full w-full items-center justify-center text-gray-500 text-xs p-2 text-center bg-gray-900">
										No Image
									</div>
								{/if}

								<!-- Hover Overlay -->
								<div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity">
									<div class="absolute bottom-0 left-0 right-0 p-2">
										<h3 class="text-white text-xs font-semibold line-clamp-2">{series.title}</h3>
										<p class="text-gray-400 text-xs">
											{series.total_seasons ?? 0} Season{series.total_seasons !== 1 ? 's' : ''}
										</p>
									</div>
								</div>
							</div>
						</button>
					{/each}
				</div>

				{#if sortedSeries().length === 0}
					<div class="text-center py-20 text-gray-400">
						<p class="text-xl">No shows found</p>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</main>

<!-- Video Player (fullscreen overlay when playing) -->
{#if playingMedia}
	<div class="fixed inset-0 z-[1000] bg-black">
		<VideoPlayer
			mediaId={playingMedia?.id ?? 0}
			title={playingMedia?.title ?? ''}
			posterUrl={playingMedia?.poster_url ? getImageUrl(playingMedia.poster_url) : ''}
			initialPosition={playingMedia?.current_position ?? 0}
			seriesId={playingMedia?.series_id ?? undefined}
			seasonNumber={playingMedia?.season_number ?? undefined}
			episodeNumber={playingMedia?.episode_number ?? undefined}
			onClose={closePlayer}
			onEpisodeChange={(episode) => {
				// Switch to the selected episode
				playingMedia = episode;
			}}
		/>
	</div>
{/if}

<!-- Series Modal -->
{#if selectedSeries}
	<SeriesModal series={selectedSeries} onClose={closeSeriesModal} onPlay={playEpisode} />
{/if}
