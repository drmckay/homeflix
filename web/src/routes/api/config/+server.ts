import { json } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
import type { RequestHandler } from './$types';

/**
 * Runtime configuration endpoint
 * 
 * Returns the API URL that should be used by the client.
 * This allows the API URL to be configured at runtime via environment variables
 * instead of being baked into the build.
 */
export const GET: RequestHandler = async () => {
	const apiUrl = env.PUBLIC_API_URL || import.meta.env.VITE_API_URL || 'http://localhost:3000';
	
	return json({
		apiUrl
	});
};
