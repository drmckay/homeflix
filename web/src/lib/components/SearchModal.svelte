<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { apiUrl } from '$lib/stores/apiUrl';
	import type { Media } from '$lib/types';

	let API_BASE = $state($apiUrl);
	
	// Update API_BASE when store changes
	$effect(() => {
		API_BASE = $apiUrl;
	});

	interface Props {
		isOpen: boolean;
		onClose: () => void;
		onSelect: (media: Media) => void;
	}

	let { isOpen, onClose, onSelect }: Props = $props();

	let searchQuery = $state('');
	let searchResults = $state<Media[]>([]);
	let isLoading = $state(false);
	let searchTimeout: ReturnType<typeof setTimeout> | null = null;
	let inputElement: HTMLInputElement | null = $state(null);

	// Focus input when modal opens
	$effect(() => {
		if (isOpen && inputElement && browser) {
			setTimeout(() => {
				inputElement?.focus();
			}, 100);
		}
	});

	// Reset when modal closes
	$effect(() => {
		if (!isOpen) {
			searchQuery = '';
			searchResults = [];
			if (searchTimeout) {
				clearTimeout(searchTimeout);
				searchTimeout = null;
			}
		}
	});

	async function performSearch(query: string) {
		if (!query.trim() || query.length < 2) {
			searchResults = [];
			return;
		}

		isLoading = true;
		try {
			const response = await fetch(`${API_BASE}/search?q=${encodeURIComponent(query)}`);
			if (response.ok) {
				const results = await response.json();
				searchResults = results;
			} else {
				searchResults = [];
			}
		} catch (error) {
			console.error('Search error:', error);
			searchResults = [];
		} finally {
			isLoading = false;
		}
	}

	function handleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		searchQuery = target.value;

		// Debounce search
		if (searchTimeout) {
			clearTimeout(searchTimeout);
		}

		searchTimeout = setTimeout(() => {
			performSearch(searchQuery);
		}, 300);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}

	function handleSelect(media: Media) {
		onSelect(media);
		onClose();
	}

	function getImageUrl(posterPath: string | null | undefined): string {
		if (!posterPath) return '';
		if (posterPath.startsWith('http')) return posterPath;
		return `https://image.tmdb.org/t/p/w500${posterPath}`;
	}

	function formatDuration(seconds: number | null | undefined): string {
		if (!seconds) return '';
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		if (h > 0) {
			return `${h}h ${m}m`;
		}
		return `${m}m`;
	}
</script>

{#if isOpen}
	<div
		class="fixed inset-0 z-[2000] bg-black/80 backdrop-blur-sm"
		onclick={(e) => {
			if (e.target === e.currentTarget) onClose();
		}}
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Search"
	>
		<div class="absolute top-20 left-1/2 -translate-x-1/2 w-full max-w-2xl px-4">
			<div class="bg-[#141414] rounded-lg shadow-2xl overflow-hidden">
				<!-- Search Input -->
				<div class="p-4 border-b border-gray-800">
					<div class="relative">
						<input
							bind:this={inputElement}
							type="text"
							placeholder="Search for movies and shows..."
							class="w-full bg-gray-900 text-white px-4 py-3 pl-12 rounded-lg border border-gray-700 focus:border-white focus:outline-none"
							value={searchQuery}
							oninput={handleInput}
							autocomplete="off"
						/>
						<svg
							class="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-gray-400"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
							/>
						</svg>
						{#if searchQuery}
							<button
								onclick={() => {
									searchQuery = '';
									searchResults = [];
									inputElement?.focus();
								}}
								class="absolute right-4 top-1/2 -translate-y-1/2 text-gray-400 hover:text-white"
								aria-label="Clear search"
							>
								<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M6 18L18 6M6 6l12 12"
									/>
								</svg>
							</button>
						{/if}
					</div>
				</div>

				<!-- Search Results -->
				<div class="max-h-[60vh] overflow-y-auto">
					{#if isLoading}
						<div class="p-8 text-center text-gray-400">
							<div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
							<p class="mt-4">Searching...</p>
						</div>
					{:else if searchQuery.length < 2}
						<div class="p-8 text-center text-gray-400">
							<p>Type at least 2 characters to search</p>
						</div>
					{:else if searchResults.length === 0 && searchQuery.length >= 2}
						<div class="p-8 text-center text-gray-400">
							<p>No results found for "{searchQuery}"</p>
						</div>
					{:else if searchResults.length > 0}
						<ul class="divide-y divide-gray-800">
							{#each searchResults as media (media.id)}
								<li>
									<button
										onclick={() => handleSelect(media)}
										class="w-full p-4 hover:bg-gray-800/50 transition-colors flex items-center gap-4 text-left"
									>
										{#if media.poster_url}
											<img
												src={getImageUrl(media.poster_url)}
												alt={media.title}
												class="w-16 h-24 object-cover rounded"
												loading="lazy"
											/>
										{:else}
											<div class="w-16 h-24 bg-gray-800 rounded flex items-center justify-center">
												<svg class="h-8 w-8 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
													<path
														stroke-linecap="round"
														stroke-linejoin="round"
														stroke-width="2"
														d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
													/>
												</svg>
											</div>
										{/if}
										<div class="flex-1 min-w-0">
											<h3 class="text-white font-semibold text-lg truncate">{media.title}</h3>
											<div class="flex items-center gap-2 mt-1 text-sm text-gray-400">
												{#if media.release_date}
													<span>{new Date(media.release_date).getFullYear()}</span>
												{/if}
												{#if media.duration}
													<span>•</span>
													<span>{formatDuration(media.duration)}</span>
												{/if}
												{#if media.media_type === 'episode'}
													<span>•</span>
													<span>Episode</span>
												{/if}
											</div>
											{#if media.overview}
												<p class="text-gray-500 text-sm mt-2 line-clamp-2">{media.overview}</p>
											{/if}
										</div>
										<svg
											class="h-6 w-6 text-gray-400 flex-shrink-0"
											fill="none"
											stroke="currentColor"
											viewBox="0 0 24 24"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M9 5l7 7-7 7"
											/>
										</svg>
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

