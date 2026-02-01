<script lang="ts">
	import type { PageData } from './$types';
	import type { CollectionItem, Media, Series } from '$lib/types';
	import { type MediaDetails, getImageUrl, fetchMediaDetails, fetchSeriesDetails } from '$lib/api';
	import VideoPlayer from '$lib/components/VideoPlayer.svelte';
	import SeriesModal from '$lib/components/SeriesModal.svelte';

	let { data }: { data: PageData } = $props();

	// Sort mode: timeline or release
	// Initialize with data from loader, but allow user overrides
	let sortMode = $state<'timeline' | 'release'>('timeline');
	
	// Update sort mode when collection data changes
	$effect(() => {
		if (data.collection) {
			sortMode = data.collection.sort_mode === 'release' ? 'release' : 'timeline';
		}
	});

	// Filter: all, available, missing
	let filterMode = $state<'all' | 'available' | 'missing'>('all');

	// Video player state - can be Media (from series episodes) or MediaDetails (from fetchMediaDetails)
	let playingMedia: (Media | MediaDetails) | null = $state(null);

	// Series modal state
	let selectedSeries: Series | null = $state(null);

	// Loading state for fetching
	let isLoading = $state(false);

	let sortedItems = $derived(() => {
		let items = [...data.collection.items];

		// Sort by selected order
		if (sortMode === 'release') {
			items = items.sort((a, b) => a.release_order - b.release_order);
		} else {
			items = items.sort((a, b) => a.timeline_order - b.timeline_order);
		}

		// Filter
		if (filterMode === 'available') {
			items = items.filter((i) => i.is_available);
		} else if (filterMode === 'missing') {
			items = items.filter((i) => !i.is_available);
		}

		return items;
	});

	function getProgressBarColor(percentage: number): string {
		if (percentage >= 100) return 'bg-green-500';
		if (percentage >= 75) return 'bg-yellow-500';
		if (percentage >= 50) return 'bg-blue-500';
		return 'bg-red-500';
	}

	function formatYear(dateStr: string | null): string {
		if (!dateStr) return 'TBA';
		return dateStr.split('-')[0];
	}

	// Selected item for info modal (missing items only)
	let selectedItem: CollectionItem | null = $state(null);

	async function openItemDetails(item: CollectionItem) {
		if (item.is_available && item.media_id) {
			if (item.media_type === 'movie') {
				// Fetch movie details and play
				isLoading = true;
				try {
					const mediaDetails = await fetchMediaDetails(item.media_id);
					playingMedia = mediaDetails;
				} catch (e) {
					console.error('Failed to fetch movie details:', e);
				} finally {
					isLoading = false;
				}
			} else if (item.media_type === 'tv') {
				// Fetch series details and show modal
				isLoading = true;
				try {
					const seriesDetails = await fetchSeriesDetails(item.media_id);
					// Build a Series object from the details
					selectedSeries = {
						id: seriesDetails.series.id,
						title: seriesDetails.series.title,
						overview: seriesDetails.series.overview,
						poster_url: seriesDetails.series.poster_url,
						backdrop_url: item.poster_url, // Use collection item's poster as fallback
						tmdb_id: seriesDetails.series.tmdb_id,
						total_seasons: seriesDetails.seasons.length,
						total_episodes: seriesDetails.seasons.reduce((sum, s) => sum + s.episodes.length, 0),
						rating: null,
						first_air_date: item.release_date
					};
				} catch (e) {
					console.error('Failed to fetch series details:', e);
				} finally {
					isLoading = false;
				}
			}
		} else {
			// Show info modal for missing items
			selectedItem = item;
		}
	}

	function closeModal() {
		selectedItem = null;
	}

	function closeSeriesModal() {
		selectedSeries = null;
	}

	function closePlayer() {
		playingMedia = null;
	}

	function playEpisode(episode: Media) {
		closeSeriesModal();
		playingMedia = episode;
	}
</script>

<svelte:head>
	<title>{data.collection.name} - Collections - Homeflix</title>
</svelte:head>

<main class="min-h-screen bg-[#141414] text-white">
	<!-- Hero Section with Backdrop -->
	<div class="relative h-[40vh] md:h-[50vh]">
		{#if data.collection.backdrop_url}
			<img
				src={getImageUrl(data.collection.backdrop_url)}
				alt={data.collection.name}
				class="w-full h-full object-cover"
			/>
		{:else}
			<div class="w-full h-full bg-gradient-to-br from-gray-800 to-gray-900"></div>
		{/if}

		<!-- Gradient Overlay -->
		<div class="absolute inset-0 bg-gradient-to-t from-[#141414] via-[#141414]/60 to-transparent"></div>

		<!-- Back Button -->
		<a
			href="/collections"
			class="absolute top-20 left-4 md:left-[60px] flex items-center gap-2 text-white/80 hover:text-white transition"
		>
			<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
				<path stroke-linecap="round" stroke-linejoin="round" d="M15 19l-7-7 7-7" />
			</svg>
			<span>Back to Collections</span>
		</a>

		<!-- Collection Info -->
		<div class="absolute bottom-8 left-4 md:left-[60px] right-4 md:right-[60px]">
			<h1 class="text-3xl md:text-5xl font-black text-white drop-shadow-lg mb-4">{data.collection.name}</h1>

			{#if data.collection.description}
				<p class="text-gray-300 text-sm md:text-base max-w-3xl line-clamp-2 mb-4">{data.collection.description}</p>
			{/if}

			<!-- Stats -->
			<div class="flex flex-wrap items-center gap-4 text-sm">
				<span class="flex items-center gap-1.5">
					<span class="text-green-400 font-bold">{data.collection.available_items}</span>
					<span class="text-gray-400">available</span>
				</span>
				<span class="text-gray-600">|</span>
				<span class="flex items-center gap-1.5">
					<span class="text-red-400 font-bold">{data.collection.total_items - data.collection.available_items}</span>
					<span class="text-gray-400">missing</span>
				</span>
				<span class="text-gray-600">|</span>
				<span class="flex items-center gap-1.5">
					<span class="font-bold">{Math.round(data.collection.completion_percentage)}%</span>
					<span class="text-gray-400">complete</span>
				</span>

				<!-- Progress Bar -->
				<div class="w-32 h-2 bg-gray-700 rounded-full overflow-hidden">
					<div
						class="h-full {getProgressBarColor(data.collection.completion_percentage)}"
						style="width: {data.collection.completion_percentage}%"
					></div>
				</div>
			</div>
		</div>
	</div>

	<!-- Controls Bar -->
	<div class="sticky top-[56px] z-30 bg-[#141414]/95 backdrop-blur-sm border-b border-gray-800/50 py-4">
		<div class="px-4 md:px-[60px] flex flex-wrap items-center gap-4">
			<!-- Sort Mode Toggle -->
			<div class="flex items-center gap-2">
				<span class="text-gray-400 text-sm">Order:</span>
				<div class="flex bg-black/60 rounded border border-gray-700 overflow-hidden">
					<button
						class="px-4 py-2 text-sm transition {sortMode === 'timeline' ? 'bg-white/20 text-white' : 'text-gray-400 hover:text-white'}"
						onclick={() => (sortMode = 'timeline')}
					>
						Timeline
					</button>
					<button
						class="px-4 py-2 text-sm transition {sortMode === 'release' ? 'bg-white/20 text-white' : 'text-gray-400 hover:text-white'}"
						onclick={() => (sortMode = 'release')}
					>
						Release
					</button>
				</div>
			</div>

			<div class="flex-1"></div>

			<!-- Filter Tabs -->
			<div class="flex items-center gap-2">
				<span class="text-gray-400 text-sm">Show:</span>
				<div class="flex gap-1">
					<button
						class="px-3 py-1.5 text-sm rounded transition {filterMode === 'all' ? 'bg-white/20 text-white' : 'text-gray-400 hover:text-white'}"
						onclick={() => (filterMode = 'all')}
					>
						All ({data.collection.total_items})
					</button>
					<button
						class="px-3 py-1.5 text-sm rounded transition {filterMode === 'available' ? 'bg-green-600/30 text-green-400' : 'text-gray-400 hover:text-white'}"
						onclick={() => (filterMode = 'available')}
					>
						Available ({data.collection.available_items})
					</button>
					<button
						class="px-3 py-1.5 text-sm rounded transition {filterMode === 'missing' ? 'bg-red-600/30 text-red-400' : 'text-gray-400 hover:text-white'}"
						onclick={() => (filterMode = 'missing')}
					>
						Missing ({data.collection.total_items - data.collection.available_items})
					</button>
				</div>
			</div>
		</div>
	</div>

	<!-- Items Timeline -->
	<div class="px-4 md:px-[60px] py-8 pb-20">
		{#if sortedItems().length === 0}
			<div class="text-center py-20 text-gray-400">
				<p class="text-xl">No items to display</p>
			</div>
		{:else}
			<div class="space-y-4">
				{#each sortedItems() as item, index (`${item.tmdb_id}-${item.timeline_order}`)}
					{@const orderNum = sortMode === 'timeline' ? item.timeline_order : item.release_order}
					<button
						class="w-full flex items-start gap-4 p-4 rounded-lg transition-all duration-200
						       {item.is_available
							? 'bg-gray-800/50 hover:bg-gray-800 cursor-pointer'
							: 'bg-gray-900/30 opacity-60 hover:opacity-80 cursor-pointer'}"
						onclick={() => openItemDetails(item)}
						aria-label="{item.is_available ? 'Play' : 'View details for'} {item.title}"
					>
						<!-- Order Number -->
						<div class="flex-shrink-0 w-10 h-10 flex items-center justify-center rounded-full
						            {item.is_available ? 'bg-green-600/20 text-green-400' : 'bg-gray-700/50 text-gray-500'}
						            font-bold text-lg">
							{orderNum}
						</div>

						<!-- Poster -->
						<div class="flex-shrink-0 w-16 md:w-20 aspect-[2/3] rounded overflow-hidden bg-gray-800">
							{#if item.poster_url}
								<img
									src={getImageUrl(item.poster_url)}
									alt={item.title}
									class="w-full h-full object-cover {item.is_available ? '' : 'grayscale'}"
									loading="lazy"
								/>
							{:else}
								<div class="w-full h-full flex items-center justify-center text-gray-600 text-xs">
									No Image
								</div>
							{/if}
						</div>

						<!-- Info -->
						<div class="flex-1 min-w-0 text-left">
							<div class="flex items-start justify-between gap-2 mb-1">
								<div class="flex items-center gap-2">
									<h3 class="text-lg font-semibold {item.is_available ? 'text-white' : 'text-gray-400'} line-clamp-1">
										{item.title}
									</h3>
									<!-- Media Type Badge -->
									{#if item.media_type === 'tv'}
										<span class="flex-shrink-0 px-1.5 py-0.5 bg-purple-600/30 text-purple-400 text-[10px] rounded font-medium">
											TV
										</span>
									{:else}
										<span class="flex-shrink-0 px-1.5 py-0.5 bg-blue-600/30 text-blue-400 text-[10px] rounded font-medium">
											MOVIE
										</span>
									{/if}
								</div>

								<!-- Status Badge -->
								{#if item.is_available}
									<span class="flex-shrink-0 px-2 py-0.5 bg-green-600/20 text-green-400 text-xs rounded">
										Available
									</span>
								{:else}
									<span class="flex-shrink-0 px-2 py-0.5 bg-red-600/20 text-red-400 text-xs rounded">
										Missing
									</span>
								{/if}
							</div>

							<div class="flex items-center gap-3 text-sm text-gray-400 mb-2">
								<span>{formatYear(item.release_date)}</span>
								<!-- Season Range for TV -->
								{#if item.media_type === 'tv' && item.season_number}
									<span>•</span>
									{#if item.episode_number && item.season_number !== item.episode_number}
										<span class="text-purple-400">Seasons {item.season_number}-{item.episode_number}</span>
									{:else}
										<span class="text-purple-400">Season {item.season_number}</span>
									{/if}
								{/if}
								{#if item.timeline_year && sortMode === 'timeline'}
									<span>•</span>
									<span class="text-gray-500">In-Universe: {item.timeline_year}</span>
								{/if}
							</div>

							{#if item.overview}
								<p class="text-sm text-gray-500 line-clamp-2">{item.overview}</p>
							{/if}

							{#if item.timeline_notes}
								<p class="mt-1 text-xs text-blue-400 italic">{item.timeline_notes}</p>
							{/if}
						</div>

						<!-- Action Icon -->
						<div class="flex-shrink-0 self-center">
							{#if item.is_available}
								{#if item.media_type === 'tv'}
									<!-- TV Series - show episodes icon -->
									<div class="w-10 h-10 flex items-center justify-center rounded-full bg-purple-600 text-white">
										<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
											<path stroke-linecap="round" stroke-linejoin="round" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
										</svg>
									</div>
								{:else}
									<!-- Movie - show play icon -->
									<div class="w-10 h-10 flex items-center justify-center rounded-full bg-white text-black">
										<svg class="w-5 h-5 ml-0.5" viewBox="0 0 24 24" fill="currentColor">
											<path d="M8 5v14l11-7z" />
										</svg>
									</div>
								{/if}
							{:else}
								<div class="w-10 h-10 flex items-center justify-center rounded-full border-2 border-gray-600 text-gray-400">
									<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
										<path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
									</svg>
								</div>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</main>

<!-- Item Info Modal (for missing items or available TV series) -->
{#if selectedItem}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[500] bg-black/80 flex items-center justify-center p-4 backdrop-blur-sm"
		onclick={closeModal}
	>
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div
			class="bg-[#181818] rounded-lg max-w-lg w-full overflow-hidden shadow-2xl"
			onclick={(e) => e.stopPropagation()}
		>
			<!-- Poster Header -->
			{#if selectedItem.poster_url}
				<div class="relative h-48">
					<img
						src={getImageUrl(selectedItem.poster_url)}
						alt={selectedItem.title}
						class="w-full h-full object-cover {selectedItem.is_available ? '' : 'grayscale'}"
					/>
					<div class="absolute inset-0 bg-gradient-to-t from-[#181818] via-[#181818]/60 to-transparent"></div>

					<button
						onclick={closeModal}
						class="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-full bg-black/60 text-white hover:bg-black/80"
						aria-label="Close"
					>
						<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>
			{/if}

			<div class="p-6">
				<div class="flex items-center gap-2 mb-2">
					{#if selectedItem.is_available}
						<span class="px-2 py-0.5 bg-green-600/20 text-green-400 text-xs rounded font-semibold">
							Available
						</span>
					{:else}
						<span class="px-2 py-0.5 bg-red-600/20 text-red-400 text-xs rounded font-semibold">
							Missing from Library
						</span>
					{/if}
					{#if selectedItem.media_type === 'tv'}
						<span class="px-2 py-0.5 bg-purple-600/20 text-purple-400 text-xs rounded font-semibold">
							TV Series
						</span>
					{/if}
				</div>

				<h2 class="text-2xl font-bold text-white mb-2">{selectedItem.title}</h2>

				<div class="flex items-center gap-3 text-sm text-gray-400 mb-4">
					<span>{formatYear(selectedItem.release_date)}</span>
					{#if selectedItem.media_type === 'tv' && selectedItem.season_number}
						<span>•</span>
						{#if selectedItem.episode_number && selectedItem.season_number !== selectedItem.episode_number}
							<span class="text-purple-400">Seasons {selectedItem.season_number}-{selectedItem.episode_number}</span>
						{:else}
							<span class="text-purple-400">Season {selectedItem.season_number}</span>
						{/if}
					{/if}
					{#if selectedItem.timeline_year}
						<span>•</span>
						<span>In-Universe: {selectedItem.timeline_year}</span>
					{/if}
				</div>

				{#if selectedItem.timeline_notes}
					<p class="text-blue-400 text-sm mb-3 italic">{selectedItem.timeline_notes}</p>
				{/if}

				{#if selectedItem.overview}
					<p class="text-gray-300 text-sm mb-6">{selectedItem.overview}</p>
				{/if}

				<!-- Actions -->
				<div class="flex flex-col gap-3">
					{#if selectedItem.is_available && selectedItem.media_type === 'tv'}
						<a
							href="/shows"
							class="flex items-center justify-center gap-2 w-full py-3 bg-purple-600 hover:bg-purple-700 rounded text-white font-semibold transition"
						>
							<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
								<path d="M8 5v14l11-7z"/>
							</svg>
							Browse Shows
						</a>
					{/if}
					
					<a
						href="https://www.themoviedb.org/{selectedItem.media_type === 'tv' ? 'tv' : 'movie'}/{selectedItem.tmdb_id}"
						target="_blank"
						rel="noopener noreferrer"
						class="flex items-center justify-center gap-2 w-full py-3 bg-blue-600 hover:bg-blue-700 rounded text-white font-semibold transition"
					>
						<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
							<path stroke-linecap="round" stroke-linejoin="round" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
						</svg>
						View on TMDB
					</a>

					<button
						onclick={closeModal}
						class="w-full py-3 bg-gray-700 hover:bg-gray-600 rounded text-white font-semibold transition"
					>
						Close
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

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

<!-- Loading Overlay -->
{#if isLoading}
	<div class="fixed inset-0 z-[600] bg-black/60 flex items-center justify-center backdrop-blur-sm">
		<div class="animate-spin rounded-full h-12 w-12 border-b-2 border-white"></div>
	</div>
{/if}
