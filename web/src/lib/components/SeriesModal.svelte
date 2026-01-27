<script lang="ts">
    import type { Series, SeriesDetails, SeasonGroup, Media, Credits } from '$lib/types';
    import { fetchSeriesDetails, fetchMediaCredits, getImageUrl, fetchSubtitleCapabilities, type ServiceCapabilities } from '$lib/api';
    import { onMount, tick } from 'svelte';
    import BatchSubtitleGenerator from './BatchSubtitleGenerator.svelte';

    let { series, onClose, onPlay, initialEpisodeId }: {
        series: Series;
        onClose: () => void;
        onPlay: (episode: Media) => void;
        initialEpisodeId?: number; // Episode ID from Continue Watching
    } = $props();

    let details: SeriesDetails | null = $state(null);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let selectedSeason: SeasonGroup | null = $state(null);
    let credits: Credits | null = $state(null);

    // Subtitle generation state
    let showBatchGenerator = $state(false);
    let subtitleCapabilities = $state<ServiceCapabilities | null>(null);

    // Track the episode to highlight (from Continue Watching or next unwatched)
    let highlightEpisodeId: number | null = $state(null);
    let episodeListRef: HTMLDivElement | null = $state(null);

    // Find the next episode to watch (first unwatched or in-progress)
    function findNextEpisode(seasons: SeasonGroup[]): { season: SeasonGroup; episode: Media } | null {
        for (const season of seasons) {
            for (const episode of season.episodes) {
                // In progress episode
                if (episode.current_position > 0 && !episode.is_watched) {
                    return { season, episode };
                }
            }
        }
        // If no in-progress, find first unwatched
        for (const season of seasons) {
            for (const episode of season.episodes) {
                if (!episode.is_watched) {
                    return { season, episode };
                }
            }
        }
        return null;
    }

    onMount(async () => {
        try {
            details = await fetchSeriesDetails(series.id);
            if (details.seasons.length > 0) {
                // If we have an initial episode ID (from Continue Watching), find its season
                if (initialEpisodeId) {
                    for (const season of details.seasons) {
                        const found = season.episodes.find(e => e.id === initialEpisodeId);
                        if (found) {
                            selectedSeason = season;
                            highlightEpisodeId = initialEpisodeId;
                            break;
                        }
                    }
                }

                // If no initial episode or not found, try to find next episode
                if (!selectedSeason) {
                    const next = findNextEpisode(details.seasons);
                    if (next) {
                        selectedSeason = next.season;
                        highlightEpisodeId = next.episode.id;
                    } else {
                        // Fall back to first season
                        selectedSeason = details.seasons[0];
                    }
                }

                // Fetch credits using the first episode's ID
                const firstEpisode = details.seasons[0]?.episodes[0];
                if (firstEpisode) {
                    credits = await fetchMediaCredits(firstEpisode.id);
                }

                // Scroll to the highlighted episode after DOM updates
                await tick();
                scrollToHighlightedEpisode();
            }

            // Fetch subtitle generation capabilities (non-blocking)
            fetchSubtitleCapabilities().then(caps => {
                subtitleCapabilities = caps;
            }).catch(err => {
                console.warn('Failed to fetch subtitle capabilities:', err);
            });
        } catch (e) {
            error = 'Failed to load series details';
            console.error(e);
        } finally {
            loading = false;
        }
    });

    // Scroll to the highlighted episode
    function scrollToHighlightedEpisode() {
        if (!highlightEpisodeId) return;

        // Wait a bit for the episode list to render
        setTimeout(() => {
            const highlightedElement = document.querySelector(`[data-episode-id="${highlightEpisodeId}"]`);
            if (highlightedElement) {
                highlightedElement.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        }, 100);
    }

    // When season changes, update highlight and scroll
    function handleSeasonChange(seasonNum: number) {
        selectedSeason = details?.seasons.find(s => s.season_number === seasonNum) ?? null;

        // Find next episode in this season
        if (selectedSeason) {
            const inProgress = selectedSeason.episodes.find(e => e.current_position > 0 && !e.is_watched);
            const firstUnwatched = selectedSeason.episodes.find(e => !e.is_watched);
            highlightEpisodeId = inProgress?.id ?? firstUnwatched?.id ?? null;

            tick().then(() => scrollToHighlightedEpisode());
        }
    }

    function handleBackdropClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            onClose();
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            onClose();
        }
    }

    function formatDuration(seconds: number | null): string {
        if (!seconds) return '';
        const mins = Math.floor(seconds / 60);
        return `${mins}m`;
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div 
    class="fixed inset-0 z-[500] bg-black/80 flex items-start justify-center overflow-y-auto py-8 backdrop-blur-sm"
    onclick={handleBackdropClick}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
>
    <div class="relative bg-[#181818] rounded-lg w-full max-w-4xl mx-4 shadow-2xl overflow-hidden">
        <!-- Close Button -->
        <button 
            class="absolute top-4 right-4 z-10 bg-[#181818] rounded-full p-2 hover:bg-gray-700 transition"
            onclick={onClose}
            aria-label="Close"
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
        </button>

        <!-- Hero Section -->
        <div class="relative h-64 md:h-80">
            {#if series.backdrop_url || series.poster_url}
                <img 
                    src={getImageUrl(series.backdrop_url || series.poster_url)} 
                    alt={series.title} 
                    class="h-full w-full object-cover"
                />
            {:else}
                <div class="h-full w-full bg-gradient-to-br from-gray-700 to-gray-900"></div>
            {/if}
            <div class="absolute inset-0 bg-gradient-to-t from-[#181818] via-[#181818]/60 to-transparent"></div>
            
            <div class="absolute bottom-4 left-4 right-4">
                <h1 class="text-3xl md:text-4xl font-black text-white drop-shadow-lg">{series.title}</h1>
                <div class="flex items-center gap-3 mt-2 text-sm text-gray-300">
                    <span class="font-medium">{series.total_seasons ?? 0} Season{series.total_seasons !== 1 ? 's' : ''}</span>
                    <span>•</span>
                    <span>{series.total_episodes ?? 0} Episode{series.total_episodes !== 1 ? 's' : ''}</span>
                </div>
            </div>
        </div>

        <!-- Content -->
        <div class="p-6">
            {#if loading}
                <div class="flex items-center justify-center py-12">
                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
                </div>
            {:else if error}
                <p class="text-red-500 text-center py-8">{error}</p>
            {:else if details}
                <!-- Overview -->
                {#if details.series.overview}
                    <p class="text-gray-300 text-sm md:text-base mb-4 line-clamp-3">{details.series.overview}</p>
                {/if}

                <!-- Cast Section -->
                {#if credits && credits.cast.length > 0}
                    <div class="mb-6 pt-4 border-t border-gray-700/50">
                        <h3 class="text-white text-lg font-semibold mb-3">Cast</h3>
                        <div class="flex flex-wrap gap-4">
                            {#each credits.cast.slice(0, 8) as member (member.id)}
                                <div class="flex items-center gap-2 bg-gray-800/50 rounded-lg p-2 pr-3">
                                    <div class="w-10 h-10 rounded-full overflow-hidden bg-gray-700 flex-shrink-0">
                                        {#if member.profile_url}
                                            <img
                                                src={getImageUrl(member.profile_url)}
                                                alt={member.name}
                                                class="w-full h-full object-cover"
                                                loading="lazy"
                                            />
                                        {:else}
                                            <div class="w-full h-full flex items-center justify-center text-gray-500">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1">
                                                    <path stroke-linecap="round" stroke-linejoin="round" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                                                </svg>
                                            </div>
                                        {/if}
                                    </div>
                                    <div class="min-w-0">
                                        <p class="text-white text-sm font-medium truncate">{member.name}</p>
                                        <p class="text-gray-400 text-xs truncate">{member.character}</p>
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </div>
                {/if}

                <!-- Season Selector and Actions -->
                <div class="mb-4 flex items-center gap-3">
                    <label for="season-select" class="sr-only">Select Season</label>
                    <div class="relative inline-block">
                        <select
                            id="season-select"
                            class="appearance-none bg-[#242424] text-white border border-gray-600 rounded pl-4 pr-10 py-2 text-sm font-bold cursor-pointer hover:bg-[#333] transition"
                            value={selectedSeason?.season_number}
                            onchange={(e) => {
                                const seasonNum = parseInt((e.target as HTMLSelectElement).value);
                                handleSeasonChange(seasonNum);
                            }}
                        >
                            {#each details.seasons as season}
                                <option value={season.season_number}>
                                    Season {season.season_number}
                                </option>
                            {/each}
                        </select>
                        <!-- Custom dropdown arrow -->
                        <div class="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-white" viewBox="0 0 20 20" fill="currentColor">
                                <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                            </svg>
                        </div>
                    </div>

                    <!-- Generate Subtitles Button -->
                    {#if subtitleCapabilities?.whisper_available && subtitleCapabilities?.whisper_model_exists}
                        <button
                            class="flex items-center gap-2 bg-gray-700 hover:bg-gray-600 text-white text-sm font-medium px-3 py-2 rounded transition"
                            onclick={() => (showBatchGenerator = true)}
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
                            </svg>
                            Generate Subtitles
                        </button>
                    {/if}
                </div>

                <!-- Episodes List -->
                {#if selectedSeason}
                    <div class="space-y-3" bind:this={episodeListRef}>
                        {#each selectedSeason.episodes as episode, index}
                            {@const isHighlighted = episode.id === highlightEpisodeId}
                            {@const isInProgress = episode.current_position > 0 && !episode.is_watched}
                            <button
                                data-episode-id={episode.id}
                                class="w-full flex items-start gap-4 p-3 rounded-md transition text-left group
                                    {isHighlighted ? 'bg-[#333] ring-2 ring-red-600' : 'bg-[#242424] hover:bg-[#333]'}"
                                onclick={() => onPlay(episode)}
                            >
                                <!-- Episode Number -->
                                <div class="flex-shrink-0 w-8 text-2xl font-bold transition
                                    {isHighlighted ? 'text-red-500' : 'text-gray-500 group-hover:text-white'}">
                                    {episode.episode_number ?? index + 1}
                                </div>

                                <!-- Thumbnail -->
                                <div class="flex-shrink-0 w-32 aspect-video bg-gray-700 rounded overflow-hidden relative">
                                    {#if episode.poster_url}
                                        <img
                                            src={getImageUrl(episode.poster_url)}
                                            alt={episode.title}
                                            class="h-full w-full object-cover"
                                            loading="lazy"
                                        />
                                    {/if}
                                    <!-- Watched Indicator -->
                                    {#if episode.is_watched}
                                        <div class="absolute top-1 right-1 bg-green-600 rounded-full p-0.5">
                                            <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 text-white" viewBox="0 0 20 20" fill="currentColor">
                                                <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
                                            </svg>
                                        </div>
                                    {/if}
                                    <!-- Play Icon Overlay -->
                                    <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition bg-black/40">
                                        <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-white" viewBox="0 0 20 20" fill="currentColor">
                                            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                                        </svg>
                                    </div>
                                    <!-- Progress Bar (for in-progress episodes) -->
                                    {#if episode.current_position > 0 && episode.duration && !episode.is_watched}
                                        <div class="absolute bottom-0 left-0 right-0 h-1 bg-gray-600">
                                            <div class="h-full bg-red-600" style="width: {Math.min((episode.current_position / episode.duration) * 100, 100)}%"></div>
                                        </div>
                                    {/if}
                                </div>

                                <!-- Episode Info -->
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center justify-between gap-2">
                                        <div class="flex items-center gap-2 min-w-0">
                                            <h3 class="text-white font-semibold text-sm truncate">{episode.title}</h3>
                                            {#if isHighlighted && isInProgress}
                                                <span class="flex-shrink-0 px-2 py-0.5 bg-red-600 text-white text-xs font-medium rounded">
                                                    Continue
                                                </span>
                                            {:else if isHighlighted && !episode.is_watched}
                                                <span class="flex-shrink-0 px-2 py-0.5 bg-gray-600 text-white text-xs font-medium rounded">
                                                    Up Next
                                                </span>
                                            {/if}
                                        </div>
                                        <span class="text-gray-400 text-xs flex-shrink-0 ml-2">{formatDuration(episode.duration)}</span>
                                    </div>
                                    {#if episode.overview}
                                        <p class="text-gray-400 text-xs mt-1 line-clamp-2">{episode.overview}</p>
                                    {/if}
                                </div>
                            </button>
                        {/each}
                    </div>
                {/if}
            {/if}
        </div>
    </div>
</div>

<!-- Batch Subtitle Generator Modal -->
{#if showBatchGenerator && details}
    {@const seasons = details.seasons.map(s => ({ number: s.season_number, episodeCount: s.episodes.length }))}
    <div class="fixed inset-0 z-[600] bg-black/80 flex items-center justify-center backdrop-blur-sm">
        <BatchSubtitleGenerator
            seriesId={series.id}
            seriesTitle={series.title}
            {seasons}
            onComplete={() => (showBatchGenerator = false)}
            onClose={() => (showBatchGenerator = false)}
        />
    </div>
{/if}
