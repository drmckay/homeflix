import type { PageLoad } from './$types';
import { fetchGroupedLibrary } from '$lib/api';
import type { Media } from '$lib/types';

export const load: PageLoad = async ({ fetch }) => {
	const library = await fetchGroupedLibrary(fetch);

	const items = [...(library.recent ?? [])];
	const seen = new Set(items.map((item) => item.id));
	for (const categoryItems of Object.values(library.categories ?? {})) {
		for (const item of categoryItems) {
			if (!seen.has(item.id)) {
				items.push(item);
				seen.add(item.id);
			}
		}
	}

	const allMovies: Media[] = items.filter((item) => item.media_type === 'movie');

	// Extract unique genres from all movies
	const genreSet = new Set<string>();
	let hasUncategorized = false;
	
	for (const movie of allMovies) {
		if (movie.genres && movie.genres.trim()) {
			const genres = movie.genres.split(',').map((g) => g.trim()).filter((g) => g.length > 0);
			if (genres.length > 0) {
				genres.forEach((g) => genreSet.add(g));
			} else {
				hasUncategorized = true;
			}
		} else {
			hasUncategorized = true;
		}
	}

	const genres = Array.from(genreSet).sort();
	
	// Add "Uncategorized" at the end if there are movies without genres
	if (hasUncategorized) {
		genres.push('Uncategorized');
	}

	return {
		movies: allMovies,
		genres,
		categories: library.categories
	};
};
