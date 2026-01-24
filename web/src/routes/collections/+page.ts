import type { PageLoad } from './$types';
import { fetchCollections } from '$lib/api';

export const load: PageLoad = async ({ fetch }) => {
	const collections = await fetchCollections(fetch);
	return { collections };
};

