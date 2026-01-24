import { fetchGroupedLibrary, fetchAllSeries, fetchCollections } from '$lib/api';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
    const [libraryResult, seriesResult, collectionsResult] = await Promise.allSettled([
        fetchGroupedLibrary(fetch),
        fetchAllSeries(fetch),
        fetchCollections(fetch)
    ]);

    if (libraryResult.status === 'rejected') {
        console.error('Failed to load library', libraryResult.reason);
    }
    if (seriesResult.status === 'rejected') {
        console.error('Failed to load series', seriesResult.reason);
    }
    if (collectionsResult.status === 'rejected') {
        console.error('Failed to load collections', collectionsResult.reason);
    }

    return {
        library: libraryResult.status === 'fulfilled' ? libraryResult.value : { recent: [], continue_watching: [], categories: {} },
        series: seriesResult.status === 'fulfilled' ? seriesResult.value : [],
        collections: collectionsResult.status === 'fulfilled' ? collectionsResult.value : []
    };
};
