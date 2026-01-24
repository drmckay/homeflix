import type { PageLoad } from './$types';
import { fetchAllSeries } from '$lib/api';

export const load: PageLoad = async ({ fetch }) => {
	const series = await fetchAllSeries(fetch);

	// Extract unique genres from all series (if available in future)
	// For now, we'll group by other criteria or just list all
	const genres: string[] = [];

	return {
		series,
		genres
	};
};

