import type { PageLoad } from './$types';
import { fetchCollectionDetails } from '$lib/api';

export const load: PageLoad = async ({ params, fetch }) => {
	const id = parseInt(params.id, 10);
	const collection = await fetchCollectionDetails(id, fetch);
	return { collection };
};

