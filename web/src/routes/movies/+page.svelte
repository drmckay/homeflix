<script lang="ts">
	import type { PageData } from './$types';
	import type { Media } from '$lib/types';
	import MovieCard from '$lib/components/MovieCard.svelte';
	import MovieRow from '$lib/components/MovieRow.svelte';
	import MovieModal from '$lib/components/MovieModal.svelte';
	import VideoPlayer from '$lib/components/VideoPlayer.svelte';
	import { getImageUrl } from '$lib/api';

	let { data }: { data: PageData } = $props();

	// View mode: 'list' or 'grid'
	let viewMode = $state<'list' | 'grid'>('list');

	// Selected genre filter
	let selectedGenre = $state<string | null>(null);

	// Sort option (for grid view)
	type SortOption = 'suggestions' | 'year' | 'az' | 'za';
	let sortBy = $state<SortOption>('suggestions');

	// Dropdown states
	let genreDropdownOpen = $state(false);
	let sortDropdownOpen = $state(false);

	// Movie modal state
	let selectedMovie: Media | null = $state(null);

	// Video player state
	let playingMedia: Media | null = $state(null);
	let playFromStart: boolean = $state(false);

	function openMovieModal(movie: Media) {
		selectedMovie = movie;
	}

	function closeMovieModal() {
		selectedMovie = null;
	}

	function playMovie(movie: Media) {
		closeMovieModal();
		playFromStart = false;
		playingMedia = movie;
	}

	function playMovieFromStart(movie: Media) {
		closeMovieModal();
		playFromStart = true;
		playingMedia = movie;
	}

	function closePlayer() {
		playingMedia = null;
		playFromStart = false;
	}

	// Group movies by genre for row display
	let moviesByGenre = $derived(() => {
		const genreMap = new Map<string, Media[]>();

		for (const movie of data.movies) {
			if (!movie.genres || movie.genres.trim() === '') {
				// Uncategorized movies
				const existing = genreMap.get('Uncategorized') ?? [];
				existing.push(movie);
				genreMap.set('Uncategorized', existing);
			} else {
				// Add to each genre the movie belongs to
				const genres = movie.genres.split(',').map((g) => g.trim()).filter(Boolean);
				for (const genre of genres) {
					const existing = genreMap.get(genre) ?? [];
					existing.push(movie);
					genreMap.set(genre, existing);
				}
			}
		}

		// Sort movies within each genre by rating
		for (const [genre, movies] of genreMap) {
			genreMap.set(genre, movies.sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0)));
		}

		return genreMap;
	});

	// Get genres to display (filtered or all)
	let genresToShow = $derived(() => {
		if (selectedGenre) {
			return [selectedGenre];
		}
		// Show genres in a sensible order (by movie count, descending)
		return [...moviesByGenre().entries()]
			.sort((a, b) => b[1].length - a[1].length)
			.map(([genre]) => genre);
	});

	// Filter movies by genre (for grid view)
	let filteredMovies = $derived(() => {
		let movies = data.movies;
		if (selectedGenre) {
			if (selectedGenre === 'Uncategorized') {
				movies = movies.filter(
					(m) => !m.genres || m.genres.trim() === '' || m.genres.split(',').every((g) => g.trim() === '')
				);
			} else {
				movies = movies.filter(
					(m) => m.genres && m.genres.split(',').some((g) => g.trim() === selectedGenre)
				);
			}
		}
		return movies;
	});

	// Sort movies (for grid view)
	let sortedMovies = $derived(() => {
		const movies = [...filteredMovies()];
		switch (sortBy) {
			case 'year':
				return movies.sort((a, b) => {
					const yearA = a.release_date ? parseInt(a.release_date.split('-')[0]) : 0;
					const yearB = b.release_date ? parseInt(b.release_date.split('-')[0]) : 0;
					return yearB - yearA;
				});
			case 'az':
				return movies.sort((a, b) => a.title.localeCompare(b.title));
			case 'za':
				return movies.sort((a, b) => b.title.localeCompare(a.title));
			case 'suggestions':
			default:
				return movies.sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0));
		}
	});

	function handleGenreSelect(genre: string | null) {
		selectedGenre = genre;
		genreDropdownOpen = false;
	}

	function handleSortSelect(option: SortOption) {
		sortBy = option;
		sortDropdownOpen = false;
	}

	const sortLabels: Record<SortOption, string> = {
		suggestions: 'Suggestions For You',
		year: 'Year Released',
		az: 'A-Z',
		za: 'Z-A'
	};
</script>

<svelte:head>
	<title>Movies - Homeflix</title>
</svelte:head>

<main class="min-h-screen bg-[#141414] text-white pt-20">
	<!-- Filter Bar (scrolls with content, like Netflix) -->
	<div class="relative z-10 bg-[#141414] pb-4">
		<div class="px-4 md:px-[60px] flex items-center gap-4 flex-wrap">
			<!-- Title -->
			<h1 class="text-2xl md:text-4xl font-bold text-white">
				{selectedGenre ? `${selectedGenre} Movies` : 'Movies'}
			</h1>

			<!-- Genres Dropdown -->
			<div class="relative">
				<button
					class="flex items-center gap-2 px-4 py-2 bg-black/60 border border-gray-600 rounded text-white text-sm hover:bg-black/80 transition"
					onclick={() => (genreDropdownOpen = !genreDropdownOpen)}
				>
					<span>Genres</span>
					<svg
						class="w-4 h-4 transition-transform {genreDropdownOpen ? 'rotate-180' : ''}"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
					>
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
					</svg>
				</button>

				{#if genreDropdownOpen}
					<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
					<div
						class="fixed inset-0 z-40"
						onclick={() => (genreDropdownOpen = false)}
					></div>
					<div class="absolute top-full left-0 mt-1 z-50 bg-black/95 border border-gray-700 rounded-md shadow-2xl min-w-[400px] max-h-[400px] overflow-y-auto">
						<div class="grid grid-cols-2 md:grid-cols-3 gap-1 p-2">
							<button
								class="text-left px-3 py-2 text-sm rounded hover:bg-white/10 transition {selectedGenre === null ? 'text-white font-semibold' : 'text-gray-300'}"
								onclick={() => handleGenreSelect(null)}
							>
								All Movies
							</button>
							{#each data.genres as genre}
								<button
									class="text-left px-3 py-2 text-sm rounded hover:bg-white/10 transition {selectedGenre === genre ? 'text-white font-semibold' : 'text-gray-300'}"
									onclick={() => handleGenreSelect(genre)}
								>
									{genre}
								</button>
							{/each}
						</div>
					</div>
				{/if}
			</div>

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
			<!-- List View: Genre rows -->
			{#each genresToShow() as genre (genre)}
				{@const genreMovies = moviesByGenre().get(genre) ?? []}
				{#if genreMovies.length > 0}
					<MovieRow
						title="{genre} Movies"
						items={genreMovies}
						onMovieClick={openMovieModal}
						onPlay={playMovie}
					/>
				{/if}
			{/each}

			{#if genresToShow().length === 0}
				<div class="px-4 md:px-[60px] text-center py-20 text-gray-400">
					<p class="text-xl">No movies found</p>
					{#if selectedGenre}
						<button
							class="mt-4 text-white underline hover:no-underline"
							onclick={() => (selectedGenre = null)}
						>
							Clear filter
						</button>
					{/if}
				</div>
			{/if}
		{:else}
			<!-- Grid View -->
			<div class="px-4 md:px-[60px]">
				<div class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-7 gap-2">
					{#each sortedMovies() as movie, index (movie.id)}
						<button
							class="movie-grid-item group relative text-left w-full"
							onclick={() => openMovieModal(movie)}
							aria-label="View details for {movie.title}"
						>
							<div class="relative aspect-[2/3] rounded-md overflow-hidden bg-gray-800 transition-all duration-200 group-hover:ring-2 group-hover:ring-white/40">
								{#if movie.poster_url}
									<img
										src={getImageUrl(movie.poster_url)}
										alt={movie.title}
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
										<h3 class="text-white text-xs font-semibold line-clamp-2">{movie.title}</h3>
										{#if movie.release_date}
											<p class="text-gray-400 text-xs">{movie.release_date.split('-')[0]}</p>
										{/if}
									</div>
								</div>

								<!-- Progress bar -->
								{#if movie.current_position > 0 && movie.duration}
									{@const progress = Math.min((movie.current_position / movie.duration) * 100, 100)}
									<div class="absolute bottom-0 left-0 right-0 h-1 bg-gray-700/80">
										<div class="h-full bg-red-600" style="width: {progress}%"></div>
									</div>
								{/if}
							</div>
						</button>
					{/each}
				</div>

				{#if sortedMovies().length === 0}
					<div class="text-center py-20 text-gray-400">
						<p class="text-xl">No movies found</p>
						{#if selectedGenre}
							<button
								class="mt-4 text-white underline hover:no-underline"
								onclick={() => (selectedGenre = null)}
							>
								Clear filter
							</button>
						{/if}
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
			initialPosition={playFromStart ? 0 : (playingMedia?.current_position ?? 0)}
			contentRating={playingMedia?.content_rating ?? undefined}
			contentWarnings={playingMedia?.content_warnings ?? undefined}
			onClose={closePlayer}
		/>
	</div>
{/if}

<!-- Movie Modal -->
{#if selectedMovie}
	<MovieModal media={selectedMovie} onClose={closeMovieModal} onPlay={playMovie} onPlayFromStart={playMovieFromStart} />
{/if}
