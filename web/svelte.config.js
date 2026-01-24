import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// adapter-node creates a standalone Node server.
		// See https://svelte.dev/docs/kit/adapter-node for more information about adapters.
		adapter: adapter()
	}
};

export default config;
