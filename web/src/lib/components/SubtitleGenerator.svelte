<script lang="ts">
    import { onDestroy } from 'svelte';
    import type { AudioTrack, ServiceCapabilities, JobStatus } from '$lib/api';
    import {
        fetchSubtitleCapabilities,
        generateSubtitle,
        fetchJobStatus,
        cancelJob
    } from '$lib/api';

    interface Props {
        mediaId: number;
        audioTracks: AudioTrack[];
        onComplete?: () => void;
        onClose?: () => void;
    }

    let { mediaId, audioTracks, onComplete, onClose }: Props = $props();

    // State
    let capabilities = $state<ServiceCapabilities | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // Form state
    let selectedAudioTrack = $state(0);
    let sourceLanguage = $state<string>('auto');
    let targetLanguage = $state<string>('');

    // Job tracking
    let status = $state<'idle' | 'processing' | 'completed' | 'failed'>('idle');
    let jobId = $state<string | null>(null);
    let progress = $state(0);
    let statusMessage = $state<string | null>(null);
    let jobResult = $state<JobStatus | null>(null);

    // Polling interval
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    // Language options
    const languages = [
        { code: 'auto', name: 'Auto-detect' },
        { code: 'en', name: 'English' },
        { code: 'hu', name: 'Magyar' },
        { code: 'de', name: 'Deutsch' },
        { code: 'es', name: 'Espanol' },
        { code: 'fr', name: 'Francais' },
        { code: 'it', name: 'Italiano' },
        { code: 'ru', name: 'Russian' },
        { code: 'ja', name: 'Japanese' },
        { code: 'ko', name: 'Korean' },
        { code: 'zh', name: 'Chinese' }
    ];

    const targetLanguages = [
        { code: '', name: 'No translation' },
        ...languages.filter((l) => l.code !== 'auto')
    ];

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

    async function startGeneration() {
        try {
            error = null;
            status = 'processing';
            progress = 0;
            statusMessage = 'Starting...';

            const response = await generateSubtitle(mediaId, {
                audio_track_index: selectedAudioTrack,
                source_language: sourceLanguage === 'auto' ? null : sourceLanguage,
                target_language: targetLanguage || null
            });

            jobId = response.job_id;
            startPolling();
        } catch (e) {
            error = e instanceof Error ? e.message : 'Failed to start generation';
            status = 'failed';
        }
    }

    function startPolling() {
        if (pollInterval) clearInterval(pollInterval);

        pollInterval = setInterval(async () => {
            if (!jobId) return;

            try {
                const jobStatus = await fetchJobStatus(jobId);
                progress = jobStatus.progress;
                statusMessage = jobStatus.message;
                jobResult = jobStatus;

                if (jobStatus.state === 'completed') {
                    status = 'completed';
                    stopPolling();
                    onComplete?.();
                } else if (jobStatus.state === 'failed') {
                    status = 'failed';
                    error = jobStatus.error ?? 'Generation failed';
                    stopPolling();
                } else if (jobStatus.state === 'cancelled') {
                    status = 'idle';
                    stopPolling();
                }
            } catch (e) {
                console.error('Failed to poll job status:', e);
            }
        }, 2000);
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
            await cancelJob(jobId);
            status = 'idle';
            stopPolling();
        } catch (e) {
            console.error('Failed to cancel job:', e);
        }
    }

    function handleClose() {
        stopPolling();
        onClose?.();
    }

    function reset() {
        status = 'idle';
        jobId = null;
        progress = 0;
        statusMessage = null;
        jobResult = null;
        error = null;
    }

    onDestroy(() => {
        stopPolling();
    });
</script>

<div class="bg-zinc-900 rounded-lg p-6 max-w-md w-full">
    <div class="flex justify-between items-center mb-4">
        <h2 class="text-xl font-semibold text-white">Generate Subtitle</h2>
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
            <p class="text-gray-500 text-sm mt-2">
                Please ensure whisper-cli and the model are installed.
            </p>
        </div>
    {:else if status === 'idle'}
        <form onsubmit={(e) => { e.preventDefault(); startGeneration(); }}>
            <!-- Audio Track Selection -->
            <div class="mb-4">
                <label class="block text-sm font-medium text-gray-300 mb-2">
                    Audio Track
                    <select
                        bind:value={selectedAudioTrack}
                        class="w-full mt-1 bg-zinc-800 text-white rounded-md px-3 py-2 border border-zinc-700 focus:border-red-500 focus:outline-none"
                    >
                        {#each audioTracks as track, i}
                            <option value={i}>
                                {track.language ?? 'Unknown'}
                                {track.codec ? `(${track.codec})` : ''}
                                {track.channels ? `${track.channels}ch` : ''}
                                {track.is_default ? '(default)' : ''}
                            </option>
                        {/each}
                    </select>
                </label>
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

            <button
                type="submit"
                class="w-full bg-red-600 hover:bg-red-700 text-white font-semibold py-3 px-4 rounded-md transition-colors"
            >
                Generate Subtitle
            </button>
        </form>
    {:else if status === 'processing'}
        <div class="py-6">
            <div class="mb-4">
                <div class="flex justify-between text-sm text-gray-400 mb-1">
                    <span>Progress</span>
                    <span>{Math.round(progress)}%</span>
                </div>
                <div class="w-full bg-zinc-800 rounded-full h-2.5">
                    <div
                        class="bg-red-600 h-2.5 rounded-full transition-all duration-300"
                        style="width: {progress}%"
                    ></div>
                </div>
            </div>

            {#if statusMessage}
                <p class="text-gray-400 text-sm text-center mb-4">{statusMessage}</p>
            {/if}

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
            <h3 class="text-lg font-semibold text-white mb-2">Subtitle Generated!</h3>
            {#if jobResult?.result}
                <p class="text-gray-400 text-sm">
                    Language: {jobResult.result.language}
                    {#if jobResult.result.was_translated}
                        (translated)
                    {/if}
                </p>
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
            <h3 class="text-lg font-semibold text-white mb-2">Generation Failed</h3>
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
