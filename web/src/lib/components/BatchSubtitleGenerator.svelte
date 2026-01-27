<script lang="ts">
    import { onDestroy, untrack } from 'svelte';
    import type { ServiceCapabilities, BatchJobStatus } from '$lib/api';
    import {
        fetchSubtitleCapabilities,
        batchGenerateSubtitles,
        fetchBatchJobStatus,
        cancelBatchJob
    } from '$lib/api';

    interface Season {
        number: number;
        episodeCount: number;
    }

    interface Props {
        seriesId: number;
        seriesTitle: string;
        seasons: Season[];
        onComplete?: () => void;
        onClose?: () => void;
    }

    let { seriesId, seriesTitle, seasons, onComplete, onClose }: Props = $props();

    // State
    let capabilities = $state<ServiceCapabilities | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // Form state
    let targetType = $state<'series' | 'season'>('series');
    let selectedSeason = $state(untrack(() => seasons[0]?.number ?? 1));
    let preferredAudioLanguage = $state<string>(''); // Empty = auto/first track
    let sourceLanguage = $state<string>('auto');
    let targetLanguage = $state<string>('');

    // Job tracking
    let status = $state<'idle' | 'processing' | 'completed' | 'failed'>('idle');
    let jobId = $state<string | null>(null);
    let batchStatus = $state<BatchJobStatus | null>(null);

    // Polling interval
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    // Audio language options (for selecting which audio track to use)
    const audioLanguages = [
        { code: '', name: 'Auto (first track)' },
        { code: 'hun', name: 'Hungarian (Magyar)' },
        { code: 'eng', name: 'English' },
        { code: 'jpn', name: 'Japanese' },
        { code: 'ger', name: 'German (Deutsch)' },
        { code: 'spa', name: 'Spanish' },
        { code: 'fra', name: 'French' },
        { code: 'ita', name: 'Italian' },
        { code: 'rus', name: 'Russian' },
        { code: 'kor', name: 'Korean' }
    ];

    // Whisper source language options
    const languages = [
        { code: 'auto', name: 'Auto-detect' },
        { code: 'en', name: 'English' },
        { code: 'hu', name: 'Magyar' },
        { code: 'de', name: 'Deutsch' },
        { code: 'es', name: 'Espanol' },
        { code: 'fr', name: 'Francais' },
        { code: 'ja', name: 'Japanese' }
    ];

    const targetLanguages = [
        { code: '', name: 'No translation' },
        ...languages.filter((l) => l.code !== 'auto')
    ];

    // Computed values
    let totalEpisodes = $derived(
        targetType === 'series'
            ? seasons.reduce((sum, s) => sum + s.episodeCount, 0)
            : seasons.find((s) => s.number === selectedSeason)?.episodeCount ?? 0
    );

    let progressPercent = $derived(
        batchStatus ? (batchStatus.completed / batchStatus.total) * 100 : 0
    );

    // Load capabilities on mount
    $effect(() => {
        loadCapabilities();
    });

    async function loadCapabilities() {
        try {
            loading = true;
            error = null;
            capabilities = await fetchSubtitleCapabilities();
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to load capabilities';
        } finally {
            loading = false;
        }
    }

    async function startBatch() {
        try {
            error = null;
            status = 'processing';

            const response = await batchGenerateSubtitles({
                target_type: targetType,
                target_id: seriesId,
                season_number: targetType === 'season' ? selectedSeason : undefined,
                preferred_audio_language: preferredAudioLanguage || null,
                source_language: sourceLanguage === 'auto' ? null : sourceLanguage,
                target_language: targetLanguage || null
            });

            jobId = response.job_id;
            startPolling();
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to start batch generation';
            status = 'failed';
        }
    }

    function startPolling() {
        if (pollInterval) clearInterval(pollInterval);

        pollInterval = setInterval(async () => {
            if (!jobId) return;

            try {
                const jobStatus = await fetchBatchJobStatus(jobId);
                batchStatus = jobStatus;

                if (jobStatus.state === 'completed') {
                    status = 'completed';
                    stopPolling();
                    onComplete?.();
                } else if (jobStatus.state === 'failed') {
                    status = 'failed';
                    error = 'Batch generation failed';
                    stopPolling();
                } else if (jobStatus.state === 'cancelled') {
                    status = 'idle';
                    stopPolling();
                }
            } catch (e) {
                console.error('Failed to poll batch status:', e);
            }
        }, 3000);
    }

    function stopPolling() {
        if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
        }
    }

    async function handleCancel() {
        if (!jobId) return;

        try {
            await cancelBatchJob(jobId);
            status = 'idle';
            stopPolling();
        } catch (e) {
            console.error('Failed to cancel batch job:', e);
        }
    }

    function handleClose() {
        stopPolling();
        onClose?.();
    }

    function reset() {
        status = 'idle';
        jobId = null;
        batchStatus = null;
        error = null;
    }

    onDestroy(() => {
        stopPolling();
    });
</script>

<div class="bg-zinc-900 rounded-lg p-6 max-w-lg w-full">
    <div class="flex justify-between items-center mb-4">
        <h2 class="text-xl font-semibold text-white">Batch Generate Subtitles</h2>
        <button
            onclick={handleClose}
            class="text-gray-400 hover:text-white transition-colors"
            aria-label="Close"
        >
            <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                />
            </svg>
        </button>
    </div>

    <p class="text-gray-400 text-sm mb-4">{seriesTitle}</p>

    {#if loading}
        <div class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-red-500"></div>
        </div>
    {:else if !capabilities?.whisper_available || !capabilities?.whisper_model_exists}
        <div class="text-center py-6">
            <div class="text-yellow-500 mb-2">
                <svg class="w-12 h-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                    />
                </svg>
            </div>
            <p class="text-gray-400">Whisper speech-to-text is not available.</p>
        </div>
    {:else if status === 'idle'}
        <form onsubmit={(e) => { e.preventDefault(); startBatch(); }}>
            <!-- Target Type -->
            <fieldset class="mb-4">
                <legend class="block text-sm font-medium text-gray-300 mb-2">Generate for</legend>
                <div class="flex gap-4">
                    <label class="flex items-center">
                        <input
                            type="radio"
                            name="targetType"
                            value="series"
                            bind:group={targetType}
                            class="mr-2 accent-red-500"
                        />
                        <span class="text-white">Entire Series</span>
                    </label>
                    <label class="flex items-center">
                        <input
                            type="radio"
                            name="targetType"
                            value="season"
                            bind:group={targetType}
                            class="mr-2 accent-red-500"
                        />
                        <span class="text-white">Single Season</span>
                    </label>
                </div>
            </fieldset>

            <!-- Season Selection (if season mode) -->
            {#if targetType === 'season'}
                <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-300 mb-2">
                        Season
                        <select
                            bind:value={selectedSeason}
                            class="w-full mt-1 bg-zinc-800 text-white rounded-md px-3 py-2 border border-zinc-700 focus:border-red-500 focus:outline-none"
                        >
                            {#each seasons as season}
                                <option value={season.number}>
                                    Season {season.number} ({season.episodeCount} episodes)
                                </option>
                            {/each}
                        </select>
                    </label>
                </div>
            {/if}

            <!-- Audio Track Language -->
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-300 mb-2">
                    Audio Track Language
                    <span class="text-gray-500 text-xs ml-1">(auto-matched for each episode)</span>
                    <select
                        bind:value={preferredAudioLanguage}
                        class="w-full mt-1 bg-zinc-800 text-white rounded-md px-3 py-2 border border-zinc-700 focus:border-red-500 focus:outline-none"
                    >
                        {#each audioLanguages as lang}
                            <option value={lang.code}>{lang.name}</option>
                        {/each}
                    </select>
                </label>
                <p class="text-gray-500 text-xs mt-1">
                    The system will automatically find the matching audio track for each episode.
                </p>
            </div>

            <!-- Source Language -->
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-300 mb-2">
                    Source Language
                    <select
                        bind:value={sourceLanguage}
                        class="w-full mt-1 bg-zinc-800 text-white rounded-md px-3 py-2 border border-zinc-700 focus:border-red-500 focus:outline-none"
                    >
                        {#each languages as lang}
                            <option value={lang.code}>{lang.name}</option>
                        {/each}
                    </select>
                </label>
            </div>

            <!-- Target Language (Translation) -->
            <div class="mb-6">
                <label class="block text-sm font-medium text-gray-300 mb-2">
                    Translate to
                    {#if !capabilities?.ollama_available}
                        <span class="text-yellow-500 text-xs">(Ollama unavailable)</span>
                    {/if}
                    <select
                        bind:value={targetLanguage}
                        disabled={!capabilities?.ollama_available}
                        class="w-full mt-1 bg-zinc-800 text-white rounded-md px-3 py-2 border border-zinc-700 focus:border-red-500 focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {#each targetLanguages as lang}
                            <option value={lang.code}>{lang.name}</option>
                        {/each}
                    </select>
                </label>
            </div>

            {#if error}
                <div class="mb-4 p-3 bg-red-900/50 border border-red-700 rounded-md text-red-300 text-sm">
                    {error}
                </div>
            {/if}

            <!-- Summary -->
            <div class="mb-4 p-3 bg-zinc-800 rounded-md">
                <p class="text-gray-300 text-sm">
                    Will generate subtitles for <strong class="text-white">{totalEpisodes}</strong> episodes.
                </p>
                <p class="text-gray-500 text-xs mt-1">
                    This may take a while. Episodes are processed one at a time.
                </p>
            </div>

            <button
                type="submit"
                class="w-full bg-red-600 hover:bg-red-700 text-white font-semibold py-3 px-4 rounded-md transition-colors"
            >
                Start Batch Generation
            </button>
        </form>
    {:else if status === 'processing'}
        <div class="py-6">
            {#if batchStatus}
                <div class="mb-4">
                    <div class="flex justify-between text-sm text-gray-400 mb-1">
                        <span>Progress</span>
                        <span>{batchStatus.completed} / {batchStatus.total} episodes</span>
                    </div>
                    <div class="w-full bg-zinc-800 rounded-full h-2.5">
                        <div
                            class="bg-red-600 h-2.5 rounded-full transition-all duration-300"
                            style="width: {progressPercent}%"
                        ></div>
                    </div>
                </div>

                {#if batchStatus.failed > 0}
                    <p class="text-yellow-500 text-sm mb-2">
                        {batchStatus.failed} episode(s) failed
                    </p>
                {/if}
            {:else}
                <div class="flex items-center justify-center py-4">
                    <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-red-500"></div>
                </div>
            {/if}

            <p class="text-gray-400 text-sm text-center mb-4">
                Processing episodes sequentially...
            </p>

            <button
                onclick={handleCancel}
                class="w-full bg-zinc-700 hover:bg-zinc-600 text-white font-medium py-2 px-4 rounded-md transition-colors"
            >
                Cancel
            </button>
        </div>
    {:else if status === 'completed'}
        <div class="text-center py-6">
            <div class="text-green-500 mb-4">
                <svg class="w-16 h-16 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                    />
                </svg>
            </div>
            <h3 class="text-lg font-semibold text-white mb-2">Batch Complete!</h3>
            {#if batchStatus}
                <p class="text-gray-400 text-sm">
                    {batchStatus.completed} of {batchStatus.total} episodes completed
                </p>
                {#if batchStatus.failed > 0}
                    <p class="text-yellow-500 text-sm mt-1">
                        {batchStatus.failed} failed
                    </p>
                {/if}
            {/if}
            <button
                onclick={handleClose}
                class="mt-4 bg-red-600 hover:bg-red-700 text-white font-medium py-2 px-6 rounded-md transition-colors"
            >
                Done
            </button>
        </div>
    {:else if status === 'failed'}
        <div class="text-center py-6">
            <div class="text-red-500 mb-4">
                <svg class="w-16 h-16 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M6 18L18 6M6 6l12 12"
                    />
                </svg>
            </div>
            <h3 class="text-lg font-semibold text-white mb-2">Batch Failed</h3>
            {#if error}
                <p class="text-red-400 text-sm mb-4">{error}</p>
            {/if}
            <button
                onclick={reset}
                class="bg-zinc-700 hover:bg-zinc-600 text-white font-medium py-2 px-6 rounded-md transition-colors"
            >
                Try Again
            </button>
        </div>
    {/if}
</div>
