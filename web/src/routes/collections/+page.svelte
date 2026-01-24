<script lang="ts">
	import type { PageData } from './$types';
	import type { CollectionSummary } from '$lib/types';
	import { getImageUrl } from '$lib/api';

	let { data }: { data: PageData } = $props();

	// Sort options
	type SortOption = 'completion' | 'name' | 'items';
	let sortBy = $state<SortOption>('completion');

	let sortedCollections = $derived(() => {
		const collections = [...data.collections];
		switch (sortBy) {
			case 'name':
				return collections.sort((a, b) => a.name.localeCompare(b.name));
			case 'items':
				return collections.sort((a, b) => b.total_items - a.total_items);
			case 'completion':
			default:
				return collections.sort((a, b) => b.completion_percentage - a.completion_percentage);
		}
	});

	function getCompletionBadge(percentage: number): { text: string; class: string } {
		if (percentage >= 100) return { text: 'Complete!', class: 'bg-green-600 text-white' };
		if (percentage >= 90) return { text: 'Almost There', class: 'bg-yellow-600 text-black' };
		if (percentage >= 50) return { text: 'In Progress', class: 'bg-blue-600 text-white' };
		return { text: 'Getting Started', class: 'bg-gray-600 text-white' };
	}

	function getProgressBarColor(percentage: number): string {
		if (percentage >= 100) return 'bg-green-500';
		if (percentage >= 75) return 'bg-yellow-500';
		if (percentage >= 50) return 'bg-blue-500';
		return 'bg-red-500';
	}
</script>

<svelte:head>
	<title>Collections - Homeflix</title>
</svelte:head>

<main class="min-h-screen bg-[#141414] text-white pt-20">
	<!-- Header -->
	<div class="px-4 md:px-[60px] pb-6">
		<div class="flex items-center justify-between mb-6">
			<h1 class="text-2xl md:text-4xl font-bold">Collections</h1>

			<!-- Sort Dropdown -->
			<div class="flex items-center gap-2 text-sm">
				<span class="text-gray-400">Sort by:</span>
				<select
					bind:value={sortBy}
					class="bg-black/60 border border-gray-600 rounded px-3 py-2 text-white cursor-pointer hover:bg-black/80"
				>
					<option value="completion">Completion %</option>
					<option value="name">Name</option>
					<option value="items">Total Items</option>
				</select>
			</div>
		</div>

		<!-- Stats Summary -->
		<div class="flex items-center gap-6 text-sm text-gray-400 mb-8">
			<span>{data.collections.length} collections available</span>
			<span>•</span>
			<span>
				{data.collections.filter((c) => c.completion_percentage >= 100).length} complete
			</span>
		</div>
	</div>

	<!-- Collections Grid -->
	<div class="px-4 md:px-[60px] pb-20">
		{#if sortedCollections().length === 0}
			<div class="text-center py-20 text-gray-400">
				<p class="text-xl">No collections found</p>
				<p class="mt-2 text-sm">Collections will appear here when you add movies that belong to a series.</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
				{#each sortedCollections() as collection (collection.id)}
					{@const badge = getCompletionBadge(collection.completion_percentage)}
					<a
						href="/collections/{collection.id}"
						class="group relative rounded-lg overflow-hidden bg-gray-900 hover:ring-2 hover:ring-white/60 transition-all duration-300"
					>
						<!-- Backdrop Image -->
						<div class="relative aspect-video">
							{#if collection.backdrop_url || collection.poster_url}
								<img
									src={getImageUrl(collection.backdrop_url ?? collection.poster_url)}
									alt={collection.name}
									class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-500"
									loading="lazy"
								/>
							{:else}
								<div class="w-full h-full bg-gradient-to-br from-gray-700 to-gray-900"></div>
							{/if}

							<!-- Gradient Overlay -->
							<div class="absolute inset-0 bg-gradient-to-t from-black via-black/40 to-transparent"></div>

							<!-- Completion Badge -->
							<div class="absolute top-3 right-3">
								<span class="px-2 py-1 rounded text-xs font-semibold {badge.class}">
									{badge.text}
								</span>
							</div>
						</div>

						<!-- Info Section -->
						<div class="p-4">
							<h3 class="text-lg font-bold text-white mb-1 line-clamp-1 group-hover:text-gray-200">
								{collection.name}
							</h3>

							<!-- Item Count -->
							<div class="flex items-center gap-2 text-sm text-gray-400 mb-3">
								<span>{collection.available_items}/{collection.total_items} items</span>
								<span>•</span>
								<span>{Math.round(collection.completion_percentage)}% complete</span>
							</div>

							<!-- Progress Bar -->
							<div class="h-1.5 bg-gray-700 rounded-full overflow-hidden">
								<div
									class="h-full {getProgressBarColor(collection.completion_percentage)} transition-all duration-300"
									style="width: {collection.completion_percentage}%"
								></div>
							</div>

							<!-- Missing Count -->
							{#if collection.total_items - collection.available_items > 0}
								<p class="mt-2 text-xs text-red-400">
									Missing: {collection.total_items - collection.available_items} item{collection.total_items - collection.available_items !== 1 ? 's' : ''}
								</p>
							{/if}
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>
</main>

