import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';

/**
 * API URL store
 * 
 * Stores the API base URL, which can be configured at runtime via PUBLIC_API_URL
 * environment variable instead of being baked into the build.
 */
function createApiUrlStore() {
	// Initialize with server-side value or build-time fallback
	const initialValue = browser 
		? (import.meta.env.VITE_API_URL || 'http://localhost:3000')
		: (env.PUBLIC_API_URL || import.meta.env.VITE_API_URL || 'http://localhost:3000');

	const { subscribe, set } = writable<string>(initialValue);

	// Load runtime config on client-side
	if (browser) {
		fetch('/api/config')
			.then(res => res.json())
			.then(config => {
				if (config.apiUrl) {
					set(config.apiUrl);
				}
			})
			.catch(() => {
				// Keep initial value on error
			});
	}

	return {
		subscribe
	};
}

export const apiUrl = createApiUrlStore();
