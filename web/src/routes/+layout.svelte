<script lang="ts">
	import { page } from '$app/state';
	import { locales, localizeHref } from '$lib/paraglide/runtime';
	import Header from '$lib/components/Header.svelte';
	import SearchModal from '$lib/components/SearchModal.svelte';
	import { goto } from '$app/navigation';
	import type { Media } from '$lib/types';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	let searchOpen = $state(false);

	function handleSearchSelect(media: Media) {
		if (media.media_type === 'episode') {
			// Navigate to series page
			if (media.series_id) {
				goto(`/series/${media.series_id}`);
			}
		} else if (media.media_type === 'movie') {
			// Navigate to movies page or show modal
			goto(`/movies`);
		}
	}
</script>

<svelte:head>
	<title>Homeflix - Your Personal Media Server</title>
	<link rel="icon" href={favicon} />
</svelte:head>

<a
	href="#main-content"
	class="sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-[60] focus:bg-white focus:text-black focus:px-4 focus:py-2 focus:rounded"
>
	Skip to main content
</a>

<Header onSearchClick={() => (searchOpen = true)} />

<SearchModal isOpen={searchOpen} onClose={() => (searchOpen = false)} onSelect={handleSearchSelect} />

{@render children()}

<nav aria-label="Language selection" class="sr-only">
	{#each locales as locale}
		<a href={localizeHref(page.url.pathname, { locale })}>
			{locale}
		</a>
	{/each}
</nav>
