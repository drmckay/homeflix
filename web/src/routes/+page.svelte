<script lang="ts">
	import Hero from '$lib/components/Hero.svelte';
	import MovieRow from '$lib/components/MovieRow.svelte';
	import Top10Row from '$lib/components/Top10Row.svelte';
	import SeriesRow from '$lib/components/SeriesRow.svelte';
	import RecentlyAddedRow from '$lib/components/RecentlyAddedRow.svelte';
	import SeriesModal from '$lib/components/SeriesModal.svelte';
	import MovieModal from '$lib/components/MovieModal.svelte';
	import VideoPlayer from '$lib/components/VideoPlayer.svelte';
	import type { PageData } from './$types';
	import type { Media, Series } from '$lib/types';
	import { getImageUrl, fetchSimilarMedia } from '$lib/api';

	let { data }: { data: PageData } = $props();

	// Series modal state
	let selectedSeries: Series | null = $state(null);
	let initialEpisodeId: number | undefined = $state(undefined);

	// Movie modal state
	let selectedMovie: Media | null = $state(null);
	let similarMedia: Media[] = $state([]);

	// Video player state
	let playingMedia: Media | null = $state(null);
	let playFromStart: boolean = $state(false);

	function openSeriesModal(series: Series, episodeId?: number) {
		selectedSeries = series;
		initialEpisodeId = episodeId;
	}

	function closeSeriesModal() {
		selectedSeries = null;
		initialEpisodeId = undefined;
	}

	async function openMovieModal(movie: Media) {
		selectedMovie = movie;
		// Fetch similar media in the background
		similarMedia = await fetchSimilarMedia(movie.id);
	}

	function closeMovieModal() {
		selectedMovie = null;
		similarMedia = [];
	}

	function playMedia(media: Media, fromStart: boolean = false) {
		// Close any open modals
		closeSeriesModal();
		closeMovieModal();
		// Set whether to start from beginning
		playFromStart = fromStart;
		// Open the player
		playingMedia = media;
	}

	function closePlayer() {
		playingMedia = null;
		playFromStart = false;
	}

	function playEpisode(episode: Media) {
		playMedia(episode);
	}

	function playMovie(movie: Media) {
		playMedia(movie);
	}

	function playMovieFromStart(movie: Media) {
		playMedia(movie, true);
	}

	// Handle Continue Watching item click - opens series modal for episodes, movie modal for movies
	async function handleContinueWatchingClick(item: Media) {
		if (item.media_type === 'episode' || item.series_id) {
			// It's an episode - find the corresponding series and open series modal
			const series = data.series.find((s) => s.id === item.series_id);
			if (series) {
				// Pass the episode ID so the modal can auto-select season and scroll to it
				openSeriesModal(series, item.id);
			} else {
				// Fallback: play the episode directly if series not found
				playMedia(item);
			}
		} else {
			// It's a movie - open movie modal
			await openMovieModal(item);
		}
	}

	// Recently added items (movies + series) - backend now returns combined list
	let recentItems = $derived(data.library.recent ?? []);

	// Filter to only movies for hero carousel
	let recentMovies = $derived(recentItems.filter((m) => m.media_type === 'movie'));

	// Get up to 5 recent items for the hero carousel (movies only)
	let heroItems = $derived(recentMovies.slice(0, 5));

	// Get all MOVIE items only (exclude episodes)
	let allMovies = $derived.by(() => {
		const items = [...recentMovies];
		const seenIds = items.map((m) => m.id);

		// Add items from categories that aren't already in recent
		for (const categoryItems of Object.values(data.library.categories)) {
			for (const item of categoryItems) {
				if (!seenIds.includes(item.id) && item.media_type === 'movie') {
					items.push(item);
					seenIds.push(item.id);
				}
			}
		}
		return items;
	});

	// Top 10 Movies (sorted by rating)
	let topMovies = $derived(
		[...allMovies].sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0)).slice(0, 10)
	);

	// Continue Watching (from dedicated backend endpoint)
	let continueWatching = $derived(data.library.continue_watching ?? []);

	// Collections from API (TMDB-based timeline collections)
	let collections = $derived.by(() => data.collections ?? []);
</script>

<!-- Video Player (fullscreen overlay when playing) -->
{#if playingMedia}
	<div class="fixed inset-0 z-[1000] bg-black">
		<VideoPlayer
			mediaId={playingMedia?.id ?? 0}
			title={playingMedia?.title ?? ''}
			posterUrl={playingMedia?.poster_url ? getImageUrl(playingMedia.poster_url) : ''}
			initialPosition={playFromStart ? 0 : (playingMedia?.current_position ?? 0)}
			contentRating={playingMedia?.content_rating ?? undefined}
			contentWarnings={playingMedia?.content_warnings ?? undefined}
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

<main
	id="main-content"
	class="min-h-screen overflow-x-hidden bg-[#141414] pb-20 font-sans text-white"
>
	<h1 class="sr-only">Homeflix - Browse Your Media Library</h1>

	<Hero
		items={heroItems}
		onMoreInfo={openMovieModal}
		onPlay={playMovie}
		onPlayFromStart={playMovieFromStart}
	/>

	<div class="relative z-20 mt-4 space-y-0 lg:-mt-32">
		<!-- Continue Watching (first row for easy access) -->
		{#if continueWatching.length > 0}
			<MovieRow
				title="Continue Watching"
				items={continueWatching}
				onMovieClick={handleContinueWatchingClick}
				onPlay={playMovie}
			/>
		{/if}

		<!-- Recently Added (Movies + Shows) -->
		{#if recentItems.length > 0}
			<RecentlyAddedRow
				title="Recently Added"
				items={recentItems}
				series={data.series}
				onMovieClick={openMovieModal}
				onSeriesClick={openSeriesModal}
				onPlay={playMovie}
			/>
		{/if}

		<!-- Shows Row -->
		{#if data.series && data.series.length > 0}
			<SeriesRow title="Shows" items={data.series} onSeriesClick={openSeriesModal} />
		{/if}

		<!-- Top 10 Movies -->
		{#if topMovies.length > 0}
			<Top10Row
				title="Top 10 Movies"
				items={topMovies}
				onMovieClick={openMovieModal}
				onPlay={playMovie}
			/>
		{/if}

		<!-- Collections -->
		{#if collections.length > 0}
			<section class="relative py-4" aria-label="Collections section">
				<div class="mb-2 flex items-center justify-between px-4 md:px-[60px]">
					<h2 class="text-base font-medium text-white md:text-[1.4vw]">Collections</h2>
					<a href="/collections" class="text-sm text-gray-400 transition hover:text-white"
						>See All</a
					>
				</div>
				<div
					class="grid grid-cols-2 gap-3 px-4 md:grid-cols-3 md:px-[60px] lg:grid-cols-4 xl:grid-cols-5"
				>
					{#each collections.slice(0, 5) as collection (collection.id)}
						<a
							href="/collections/{collection.id}"
							class="group relative aspect-video overflow-hidden rounded-lg bg-gray-800 transition-all hover:ring-2 hover:ring-white"
						>
							{#if collection.backdrop_url || collection.poster_url}
								<img
									src={collection.backdrop_url ?? collection.poster_url}
									alt={collection.name}
									class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
									loading="lazy"
								/>
							{:else}
								<div class="h-full w-full bg-gradient-to-br from-gray-700 to-gray-900"></div>
							{/if}
							<div
								class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent"
							></div>
							<div class="absolute right-0 bottom-0 left-0 p-3">
								<h3 class="line-clamp-1 text-sm font-semibold text-white md:text-base">
									{collection.name}
								</h3>
								<div class="mt-1 flex items-center justify-between text-xs">
									<span class="text-gray-400"
										>{collection.available_items}/{collection.total_items} items</span
									>
									<span
										class={collection.completion_percentage >= 100
											? 'text-green-400'
											: 'text-yellow-400'}>{Math.round(collection.completion_percentage)}%</span
									>
								</div>
								<!-- Progress Bar -->
								<div class="mt-2 h-1 overflow-hidden rounded-full bg-gray-700">
									<div
										class="h-full {collection.completion_percentage >= 100
											? 'bg-green-500'
											: 'bg-red-500'}"
										style="width: {collection.completion_percentage}%"
									></div>
								</div>
							</div>
						</a>
					{/each}
				</div>
			</section>
		{/if}
	</div>
</main>

<!-- Series Modal -->
{#if selectedSeries}
	<SeriesModal
		series={selectedSeries}
		onClose={closeSeriesModal}
		onPlay={playEpisode}
		{initialEpisodeId}
	/>
{/if}

<!-- Movie Modal -->
{#if selectedMovie}
	<MovieModal
		media={selectedMovie}
		onClose={closeMovieModal}
		onPlay={playMovie}
		onPlayFromStart={playMovieFromStart}
		{similarMedia}
	/>
{/if}
