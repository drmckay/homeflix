<script lang="ts">
    import type { Media, Credits } from '$lib/types';
    import { getImageUrl, fetchMediaCredits, fetchSubtitleCapabilities, fetchMediaTracks, type ServiceCapabilities, type AudioTrack } from '$lib/api';
    import { onMount } from 'svelte';
    import SubtitleGenerator from './SubtitleGenerator.svelte';

    let { media, onClose, onPlay, onPlayFromStart, similarMedia = [] }: {
        media: Media;
        onClose: () => void;
        onPlay: (media: Media) => void;
        onPlayFromStart?: (media: Media) => void;
        similarMedia?: Media[];
    } = $props();

    let credits: Credits | null = $state(null);

    // Subtitle generation state
    let showSubtitleGenerator = $state(false);
    let subtitleCapabilities = $state<ServiceCapabilities | null>(null);
    let audioTracks = $state<AudioTrack[]>([]);

    onMount(async () => {
        // Fetch credits for the media
        credits = await fetchMediaCredits(media.id);

        // Fetch tracks for audio track info (needed for subtitle generator)
        try {
            const tracks = await fetchMediaTracks(media.id);
            audioTracks = tracks.audio_tracks || [];
        } catch (e) {
            console.warn('Failed to fetch media tracks:', e);
        }

        // Fetch subtitle generation capabilities (non-blocking)
        fetchSubtitleCapabilities().then(caps => {
            subtitleCapabilities = caps;
        }).catch(err => {
            console.warn('Failed to fetch subtitle capabilities:', err);
        });
    });

    // Check if media has watch progress (at least 10 seconds)
    function hasWatchProgress(): boolean {
        return (media.current_position ?? 0) > 10;
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
        const hours = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        if (hours > 0) {
            return `${hours}h ${mins}m`;
        }
        return `${mins}m`;
    }

    function formatYear(date: string | null): string {
        if (!date) return '';
        return new Date(date).getFullYear().toString();
    }

    function formatRating(rating: number | null): string {
        if (!rating) return '';
        return rating.toFixed(1);
    }

    function getProgressPercent(): number {
        if (!media.current_position || !media.duration) return 0;
        return Math.min((media.current_position / media.duration) * 100, 100);
    }

    function formatTimeRemaining(): string {
        if (!media.duration) return '';
        const remaining = media.duration - (media.current_position ?? 0);
        const mins = Math.floor(remaining / 60);
        if (mins >= 60) {
            const hours = Math.floor(mins / 60);
            const remMins = mins % 60;
            return `${hours}h ${remMins}m remaining`;
        }
        return `${mins}m remaining`;
    }

    let genres = $derived(media.genres?.split(',').map(g => g.trim()).filter(Boolean) ?? []);
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
    <div class="relative bg-[#181818] rounded-lg w-full max-w-3xl mx-4 shadow-2xl overflow-hidden animate-modal-in">
        <!-- Close Button -->
        <button 
            class="absolute top-4 right-4 z-20 bg-[#181818]/80 rounded-full p-2 hover:bg-gray-700 transition-colors"
            onclick={onClose}
            aria-label="Close"
        >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
        </button>

        <!-- Hero Section with Backdrop -->
        <div class="relative h-[20rem] md:h-[24rem]">
            {#if media.backdrop_url || media.poster_url}
                <img 
                    src={getImageUrl(media.backdrop_url || media.poster_url)} 
                    alt={media.title} 
                    class="h-full w-full object-cover"
                />
            {:else}
                <div class="h-full w-full bg-gradient-to-br from-gray-700 to-gray-900 flex items-center justify-center">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-24 w-24 text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
                    </svg>
                </div>
            {/if}
            
            <!-- Gradient Overlay -->
            <div class="absolute inset-0 bg-gradient-to-t from-[#181818] via-[#181818]/40 to-transparent"></div>
            
            <!-- Title & Info Overlay -->
            <div class="absolute bottom-0 left-0 right-0 p-6">
                <h1 class="text-3xl md:text-4xl font-black text-white drop-shadow-lg mb-3">{media.title}</h1>
                
                <!-- Metadata Row -->
                <div class="flex flex-wrap items-center gap-3 text-sm text-gray-300 mb-4">
                    {#if media.rating}
                        <span class="text-yellow-400 font-bold flex items-center gap-1">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                                <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                            </svg>
                            {formatRating(media.rating)}
                        </span>
                    {/if}
                    {#if media.release_date}
                        <span>{formatYear(media.release_date)}</span>
                    {/if}
                    {#if media.duration}
                        <span>{formatDuration(media.duration)}</span>
                    {/if}
                    {#if media.resolution}
                        {@const isHD = media.resolution?.includes('1080') || media.resolution?.includes('720')}
                        {@const is4K = media.resolution?.includes('2160') || media.resolution?.includes('4K') || media.resolution?.includes('UHD')}
                        {#if is4K}
                            <span class="bg-white/20 text-white px-1.5 py-0.5 text-xs font-bold rounded">4K</span>
                            <span class="border border-gray-500 px-1.5 py-0.5 text-xs font-medium rounded">HDR</span>
                        {:else if isHD}
                            <span class="bg-white/20 text-white px-1.5 py-0.5 text-xs font-bold rounded">HD</span>
                        {:else}
                            <span class="border border-gray-500 px-1.5 py-0.5 text-xs font-medium rounded">{media.resolution}</span>
                        {/if}
                    {/if}
                </div>

                <!-- Action Buttons -->
                <div class="flex flex-wrap items-center gap-3">
                    {#if hasWatchProgress()}
                        <!-- Resume button -->
                        <button 
                            onclick={() => onPlay(media)}
                            class="flex items-center gap-2 bg-white hover:bg-gray-200 text-black font-bold px-6 py-2.5 rounded-md transition-colors"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" viewBox="0 0 20 20" fill="currentColor">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                            </svg>
                            Resume
                        </button>
                        <!-- Start from Beginning button -->
                        <button 
                            onclick={() => onPlayFromStart?.(media)}
                            class="flex items-center gap-2 bg-gray-600 hover:bg-gray-500 text-white font-bold px-5 py-2.5 rounded-md transition-colors"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                            Start from Beginning
                        </button>
                    {:else}
                        <!-- Normal Play button -->
                        <button 
                            onclick={() => onPlay(media)}
                            class="flex items-center gap-2 bg-white hover:bg-gray-200 text-black font-bold px-6 py-2.5 rounded-md transition-colors"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" viewBox="0 0 20 20" fill="currentColor">
                                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                            </svg>
                            Play
                        </button>
                    {/if}
                    
                    <button 
                        class="flex items-center justify-center w-10 h-10 border-2 border-gray-400 hover:border-white rounded-full transition-colors group"
                        aria-label="Add to My List"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-gray-400 group-hover:text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4v16m8-8H4" />
                        </svg>
                    </button>
                    
                    <button
                        class="flex items-center justify-center w-10 h-10 border-2 border-gray-400 hover:border-white rounded-full transition-colors group"
                        aria-label="Like"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-gray-400 group-hover:text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M14 10h4.764a2 2 0 011.789 2.894l-3.5 7A2 2 0 0115.263 21h-4.017c-.163 0-.326-.02-.485-.06L7 20m7-10V5a2 2 0 00-2-2h-.095c-.5 0-.905.405-.905.905 0 .714-.211 1.412-.608 2.006L7 11v9m7-10h-2M7 20H5a2 2 0 01-2-2v-6a2 2 0 012-2h2.5" />
                        </svg>
                    </button>

                    <!-- Generate Subtitles Button -->
                    {#if subtitleCapabilities?.whisper_available && subtitleCapabilities?.whisper_model_exists}
                        <button
                            onclick={() => showSubtitleGenerator = true}
                            class="flex items-center justify-center w-10 h-10 border-2 border-gray-400 hover:border-white rounded-full transition-colors group"
                            aria-label="Generate Subtitles"
                            title="Generate Subtitles"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-gray-400 group-hover:text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
                            </svg>
                        </button>
                    {/if}
                </div>

                <!-- Progress Bar (if watching) -->
                {#if media.current_position > 0 && media.duration}
                    <div class="mt-4">
                        <div class="h-1 bg-gray-600 rounded-full overflow-hidden">
                            <div class="h-full bg-red-600 transition-all" style="width: {getProgressPercent()}%"></div>
                        </div>
                        <p class="text-xs text-gray-400 mt-1">{formatTimeRemaining()}</p>
                    </div>
                {/if}
            </div>
        </div>

        <!-- Content Section -->
        <div class="p-6 pt-4">
            <!-- Two-column layout for overview and details -->
            <div class="flex flex-col md:flex-row gap-6">
                <!-- Left column: Overview -->
                <div class="flex-1">
                    {#if media.overview}
                        <p class="text-gray-300 text-sm md:text-base leading-relaxed">{media.overview}</p>
                    {/if}
                </div>

                <!-- Right column: Details -->
                <div class="md:w-[280px] flex-shrink-0 space-y-2 text-sm">
                    <!-- Genres -->
                    {#if genres.length > 0}
                        <div>
                            <span class="text-gray-500">Genres: </span>
                            <span class="text-white">{genres.join(', ')}</span>
                        </div>
                    {/if}

                    <!-- Original Title -->
                    {#if media.original_title && media.original_title !== media.title}
                        <div>
                            <span class="text-gray-500">Original Title: </span>
                            <span class="text-white">{media.original_title}</span>
                        </div>
                    {/if}

                    <!-- Trailer link -->
                    {#if media.trailer_url}
                        <div class="pt-2">
                            <a
                                href={media.trailer_url}
                                target="_blank"
                                rel="noopener noreferrer"
                                class="inline-flex items-center gap-2 text-gray-300 hover:text-white transition-colors text-sm"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" viewBox="0 0 20 20" fill="currentColor">
                                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                                </svg>
                                Watch Trailer
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                    <path stroke-linecap="round" stroke-linejoin="round" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                </svg>
                            </a>
                        </div>
                    {/if}
                </div>
            </div>

            <!-- Cast Section -->
            {#if credits && credits.cast.length > 0}
                <div class="mt-6 pt-4 border-t border-gray-700/50">
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
        </div>

        <!-- More Like This Section -->
        {#if similarMedia.length > 0}
            <div class="p-6 pt-0 border-t border-gray-700/50">
                <h3 class="text-white text-lg font-semibold mt-6 mb-4">More Like This</h3>
                <div class="grid grid-cols-3 gap-3">
                    {#each similarMedia.slice(0, 6) as item (item.id)}
                        <button
                            class="group relative rounded-md overflow-hidden bg-gray-800 hover:ring-1 hover:ring-white transition-all"
                            onclick={() => onPlay(item)}
                        >
                            <div class="aspect-[2/3] w-full">
                                {#if item.poster_url}
                                    <img
                                        src={getImageUrl(item.poster_url)}
                                        alt={item.title}
                                        class="h-full w-full object-cover"
                                        loading="lazy"
                                    />
                                {:else}
                                    <div class="h-full w-full flex items-center justify-center text-gray-500 text-xs p-2 text-center">
                                        {item.title}
                                    </div>
                                {/if}

                                <!-- Play overlay on hover -->
                                <div class="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                                    <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 text-white" viewBox="0 0 20 20" fill="currentColor">
                                        <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd" />
                                    </svg>
                                </div>

                                <!-- Progress bar if watching -->
                                {#if item.current_position > 0 && item.duration}
                                    {@const percent = Math.min((item.current_position / item.duration) * 100, 100)}
                                    <div class="absolute bottom-0 left-0 right-0 h-1 bg-gray-700">
                                        <div class="h-full bg-red-600" style="width: {percent}%"></div>
                                    </div>
                                {/if}
                            </div>

                            <!-- Title -->
                            <div class="p-2 bg-[#2f2f2f]">
                                <p class="text-white text-xs font-medium line-clamp-1">{item.title}</p>
                                {#if item.release_date}
                                    <p class="text-gray-400 text-xs">{new Date(item.release_date).getFullYear()}</p>
                                {/if}
                            </div>
                        </button>
                    {/each}
                </div>
            </div>
        {/if}
    </div>
</div>

<!-- Subtitle Generator Modal -->
{#if showSubtitleGenerator}
    <div class="fixed inset-0 z-[600] bg-black/80 flex items-center justify-center backdrop-blur-sm">
        <SubtitleGenerator
            mediaId={media.id}
            {audioTracks}
            onComplete={() => {
                showSubtitleGenerator = false;
            }}
            onClose={() => showSubtitleGenerator = false}
        />
    </div>
{/if}

<style>
    @keyframes modal-in {
        from {
            opacity: 0;
            transform: scale(0.95) translateY(20px);
        }
        to {
            opacity: 1;
            transform: scale(1) translateY(0);
        }
    }

    .animate-modal-in {
        animation: modal-in 0.25s ease-out forwards;
    }
</style>

