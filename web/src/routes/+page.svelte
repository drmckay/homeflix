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
	let allMovies = $derived(() => {
		const items = [...recentMovies];
		const seen = new Set(items.map((m) => m.id));

		// Add items from categories that aren't already in recent
		for (const categoryItems of Object.values(data.library.categories)) {
			for (const item of categoryItems) {
				if (!seen.has(item.id) && item.media_type === 'movie') {
					items.push(item);
					seen.add(item.id);
				}
			}
		}
		return items;
	});

	// Top 10 Movies (sorted by rating)
	let topMovies = $derived(
		allMovies()
			.sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))
			.slice(0, 10)
	);

	// Continue Watching (from dedicated backend endpoint)
	let continueWatching = $derived(data.library.continue_watching ?? []);

	// Collections from API (TMDB-based timeline collections)
	let collections = $derived(() => data.collections ?? []);
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
	class="bg-[#141414] min-h-screen text-white font-sans overflow-x-hidden pb-20"
>
	<h1 class="sr-only">Homeflix - Browse Your Media Library</h1>

	<Hero items={heroItems} onMoreInfo={openMovieModal} onPlay={playMovie} onPlayFromStart={playMovieFromStart} />

	<div class="relative z-20 -mt-24 md:-mt-32 space-y-0">
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
			<Top10Row title="Top 10 Movies" items={topMovies} onMovieClick={openMovieModal} onPlay={playMovie} />
		{/if}

		<!-- Collections -->
		{#if collections().length > 0}
			<section class="relative py-4" aria-label="Collections section">
				<div class="px-4 md:px-[60px] mb-2 flex items-center justify-between">
					<h2 class="text-white text-base md:text-[1.4vw] font-medium">Collections</h2>
					<a href="/collections" class="text-gray-400 text-sm hover:text-white transition"
						>See All</a
					>
				</div>
				<div
					class="px-4 md:px-[60px] grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3"
				>
					{#each collections().slice(0, 5) as collection (collection.id)}
						<a
							href="/collections/{collection.id}"
							class="group relative aspect-video rounded-lg overflow-hidden bg-gray-800 hover:ring-2 hover:ring-white transition-all"
						>
							{#if collection.backdrop_url || collection.poster_url}
								<img
									src={collection.backdrop_url ?? collection.poster_url}
									alt={collection.name}
									class="h-full w-full object-cover group-hover:scale-105 transition-transform duration-300"
									loading="lazy"
								/>
							{:else}
								<div class="h-full w-full bg-gradient-to-br from-gray-700 to-gray-900"></div>
							{/if}
							<div
								class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent"
							></div>
							<div class="absolute bottom-0 left-0 right-0 p-3">
								<h3 class="text-white font-semibold text-sm md:text-base line-clamp-1">
									{collection.name}
								</h3>
								<div class="flex items-center justify-between text-xs mt-1">
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
								<div class="h-1 bg-gray-700 rounded-full mt-2 overflow-hidden">
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
	<SeriesModal series={selectedSeries} onClose={closeSeriesModal} onPlay={playEpisode} {initialEpisodeId} />
{/if}

<!-- Movie Modal -->
{#if selectedMovie}
	<MovieModal media={selectedMovie} onClose={closeMovieModal} onPlay={playMovie} onPlayFromStart={playMovieFromStart} {similarMedia} />
{/if}
