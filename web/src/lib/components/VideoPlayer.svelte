<script lang="ts">
	/**
	 * VideoPlayer.svelte - Netflix-style MP4 streaming player
	 *
	 * Features:
	 * - Direct MP4 streaming via FFmpeg transcode
	 * - Video copy, audio transcode to AAC
	 * - Hungarian audio auto-selection
	 * - Watch progress persistence
	 * - Netflix-style controls layout
	 * - Automatic subtitle generation (Whisper + Ollama)
	 */

	import { onMount, onDestroy, untrack } from 'svelte';
	import { browser } from '$app/environment';
	import SubtitleGenerator from './SubtitleGenerator.svelte';
	import { fetchSubtitleCapabilities, fetchSeriesDetails, getImageUrl, getApiBase, type ServiceCapabilities } from '$lib/api';
	import type { SeriesDetails, Media } from '$lib/types';

	// Props using Svelte 5 runes
	interface Props {
		mediaId: number;
		title?: string;
		posterUrl?: string;
		initialPosition?: number;
		contentRating?: string;
		contentWarnings?: string;
		seriesId?: number;
		seasonNumber?: number;
		episodeNumber?: number;
		onClose?: () => void;
		onEpisodeChange?: (episode: Media) => void;
	}

	let {
		mediaId,
		title = 'Unknown',
		posterUrl = '',
		initialPosition = 0,
		contentRating = '',
		contentWarnings = '',
		seriesId,
		seasonNumber,
		episodeNumber,
		onClose,
		onEpisodeChange
	}: Props = $props();

	// State
	let videoElement: HTMLVideoElement | null = $state(null);
	let containerElement: HTMLDivElement | null = $state(null);

	let isLoading = $state(true);
	let error: string | null = $state(null);
	let currentTime = $state(0);
	let duration = $state(0);
	let isPlaying = $state(false);
	let volume = $state(1);
	let isMuted = $state(false);
	let isFullscreen = $state(false);
	let showControls = $state(true);
	let controlsTimeout: ReturnType<typeof setTimeout> | null = null;

	// Initial info overlay state (shows for first 6 seconds)
	let showInitialInfo = $state(true);
	let initialInfoTimeout: ReturnType<typeof setTimeout> | null = null;
	let lastStreamOffset = $state(untrack(() => initialPosition));
	
	// Reset showInitialInfo when starting a new stream
	$effect(() => {
		// Only reset if stream offset actually changed (new stream started)
		if (streamStartOffset !== lastStreamOffset) {
			lastStreamOffset = streamStartOffset;
			showInitialInfo = true;
			if (initialInfoTimeout) {
				clearTimeout(initialInfoTimeout);
				initialInfoTimeout = null;
			}
		}
	});

	// Stream offset tracking - the position where the current stream started
	let streamStartOffset = $state(untrack(() => initialPosition));
	let isSeeking = $state(false);
	let lastSavedTime = $state(0); // Track last saved time to avoid saving too frequently

	// Audio track state
	interface AudioTrack {
		index: number;
		language: string | null;
		codec: string | null;
		channels: number | null;
		title: string | null;
		is_default: boolean;
	}
	let audioTracks = $state<AudioTrack[]>([]);
	let selectedAudioTrack = $state(0);
	let showAudioMenu = $state(false);

	// Subtitle track state
	interface SubtitleTrack {
		index: number;
		language: string | null;
		language_name: string | null;
		source: 'external' | 'embedded';
		is_default: boolean;
	}
	let subtitleTracks = $state<SubtitleTrack[]>([]);
	let selectedSubtitleIndex = $state<number | null>(null);
	let subtitleFontSize = $state<'small' | 'default' | 'large'>('default');
	let showSubtitleMenu = $state(false);

	// Subtitle generation state
	let showSubtitleGenerator = $state(false);
	let subtitleCapabilities = $state<ServiceCapabilities | null>(null);

	// Chromecast state
	let isCasting = $state(false);
	let castSession: any = $state(null);
	let castPlayer: any = $state(null);
	let castController: any = $state(null);
	let castContext: any = $state(null);
	let showCastButton = $state(false);
	
	// UI state
	let isDragging = $state(false);

	// Credits detection state
	let creditsCanvas: HTMLCanvasElement | null = null;
	let creditsContext: CanvasRenderingContext2D | null = null;
	let creditsDetected = $state(false);
	let consecutiveCreditsFrames = $state(0);
	let lastCheckTime = $state(0);

	// Initialize Chromecast
	function initializeCast() {
		if (!browser) return;
		
		if (!window.cast) {
			// If API not loaded yet, retry in a bit
			if (window.chrome && !window.cast) {
				setTimeout(initializeCast, 500);
			} else {
				// Define callback for when script loads
				window.__onGCastApiAvailable = (isAvailable: boolean) => {
					if (isAvailable) {
						initializeCast();
					}
				};
			}
			return;
		}

		try {
			castContext = window.cast.framework.CastContext.getInstance();
			
			castContext.setOptions({
				receiverApplicationId: window.chrome.cast.media.DEFAULT_MEDIA_RECEIVER_APP_ID,
				autoJoinPolicy: window.chrome.cast.AutoJoinPolicy.ORIGIN_SCOPED
			});

			// Listen for state changes
			castContext.addEventListener(
				window.cast.framework.CastContextEventType.CAST_STATE_CHANGED,
				(event: any) => {
					showCastButton = event.castState !== window.cast.framework.CastState.NO_DEVICES_AVAILABLE;
				}
			);

			castContext.addEventListener(
				window.cast.framework.CastContextEventType.SESSION_STATE_CHANGED,
				handleCastSessionChange
			);
			
			// Check initial state
			const state = castContext.getCastState();
			showCastButton = state !== window.cast.framework.CastState.NO_DEVICES_AVAILABLE;
			
			// If already connected (rejoined), setup session
			if (castContext.getCurrentSession()) {
				handleCastSessionChange({
					sessionState: window.cast.framework.SessionState.SESSION_STARTED,
					session: castContext.getCurrentSession()
				});
			}

		} catch (e) {
			console.error('Failed to initialize Cast SDK:', e);
		}
	}

	function handleCastSessionChange(event: any) {
		switch (event.sessionState) {
			case window.cast.framework.SessionState.SESSION_STARTED:
			case window.cast.framework.SessionState.SESSION_RESUMED:
				castSession = event.session;
				isCasting = true;
				setupRemotePlayer();
				// If we just started, load the media
				if (event.sessionState === window.cast.framework.SessionState.SESSION_STARTED) {
					loadMediaToCast();
				}
				break;
			case window.cast.framework.SessionState.SESSION_ENDED:
				isCasting = false;
				castSession = null;
				castPlayer = null;
				castController = null;
				// Sync back to local player
				if (videoElement) {
					videoElement.currentTime = currentTime;
					videoElement.play();
				}
				break;
		}
	}

	function setupRemotePlayer() {
		if (!castSession) return;
		
		castPlayer = new window.cast.framework.RemotePlayer();
		castController = new window.cast.framework.RemotePlayerController(castPlayer);
		
		castController.addEventListener(
			window.cast.framework.RemotePlayerEventType.CURRENT_TIME_CHANGED,
			() => {
				if (isCasting && castPlayer) {
					currentTime = castPlayer.currentTime;
					// Save progress periodically
					if (currentTime > 0 && currentTime - lastSavedTime >= 30) {
						saveProgress();
					}
				}
			}
		);
		
		castController.addEventListener(
			window.cast.framework.RemotePlayerEventType.IS_PAUSED_CHANGED,
			() => {
				if (isCasting && castPlayer) {
					isPlaying = !castPlayer.isPaused;
				}
			}
		);

		castController.addEventListener(
			window.cast.framework.RemotePlayerEventType.IS_MUTED_CHANGED,
			() => {
				if (isCasting && castPlayer) {
					isMuted = castPlayer.isMuted;
				}
			}
		);

		castController.addEventListener(
			window.cast.framework.RemotePlayerEventType.VOLUME_LEVEL_CHANGED,
			() => {
				if (isCasting && castPlayer) {
					volume = castPlayer.volumeLevel;
				}
			}
		);
	}

	function loadMediaToCast() {
		if (!castSession) return;

		// Add metadata
		const metadata = new window.chrome.cast.media.GenericMediaMetadata();
		metadata.title = title;
		if (posterUrl) {
			metadata.images = [new window.chrome.cast.Image(getImageUrl(posterUrl))];
		}
		if (seriesDetails && episodeNumber) {
			metadata.subtitle = `${seriesDetails.series.title} S${seasonNumber}E${episodeNumber}`;
		}

		// Stream full file to Chromecast (allow it to handle range requests)
		const fullStreamUrl = `${getApiBase()}/v2/stream/web/${mediaId}?start=0&audio=${selectedAudioTrack}`;
		const mediaInfoFull = new window.chrome.cast.media.MediaInfo(fullStreamUrl, 'video/mp4');
		mediaInfoFull.metadata = metadata;
		mediaInfoFull.streamType = window.chrome.cast.media.StreamType.BUFFERED;
		mediaInfoFull.duration = duration;
		
		if (subtitleTracks.length > 0) {
			const tracks = subtitleTracks.map((track, index) => {
				const trackInfo = new window.chrome.cast.media.Track(index + 1, window.chrome.cast.media.TrackType.TEXT);
				trackInfo.trackContentId = `${getApiBase()}/v2/subtitles/${mediaId}/${index}`;
				trackInfo.trackContentType = 'text/vtt';
				trackInfo.subtype = window.chrome.cast.media.TextTrackType.SUBTITLES;
				trackInfo.name = getSubtitleTrackLabel(track);
				trackInfo.language = track.language || 'en';
				return trackInfo;
			});
			mediaInfoFull.tracks = tracks;
		}

		const requestFull = new window.chrome.cast.media.LoadRequest(mediaInfoFull);
		requestFull.currentTime = currentTime;
		requestFull.autoplay = true;

		// If subtitle selected
		if (selectedSubtitleIndex !== null) {
			requestFull.activeTrackIds = [selectedSubtitleIndex + 1];
		}

		castSession.loadMedia(requestFull).then(
			() => { console.log('Cast media loaded'); },
			(e: any) => { console.error('Cast load error:', e); }
		);
		
		// Pause local video
		if (videoElement) {
			videoElement.pause();
		}
	}

	// Episode selector state
	let seriesDetails = $state<SeriesDetails | null>(null);
	let showEpisodeSelector = $state(false);
	let currentSeasonEpisodes = $state<Media[]>([]);
	let selectedSeasonInModal = $state<number | null>(null);

	// Next episode logic
	let nextEpisode = $derived.by(() => {
		if (!seriesDetails || seasonNumber === undefined || episodeNumber === undefined) return null;

		// 1. Try to find next episode in current season
		const currentSeason = seriesDetails.seasons.find(s => s.season_number === seasonNumber);
		if (currentSeason) {
			const nextEp = currentSeason.episodes.find(e => e.episode_number === episodeNumber! + 1);
			if (nextEp) return nextEp;
		}

		// 2. If not found, try first episode of next season
		// Sort seasons to be sure we get the immediate next one
		const sortedSeasons = [...seriesDetails.seasons].sort((a, b) => a.season_number - b.season_number);
		const nextSeason = sortedSeasons.find(s => s.season_number > seasonNumber!);
		
		if (nextSeason && nextSeason.episodes.length > 0) {
			// Return the first episode of the next season
			// Sort episodes just in case
			const sortedEpisodes = [...nextSeason.episodes].sort((a, b) => (a.episode_number ?? 0) - (b.episode_number ?? 0));
			return sortedEpisodes[0];
		}

		return null;
	});

	let showNextEpisodeButton = $state(false);

	// Progress percentage for styling
	let progressPercent = $derived(duration > 0 ? (currentTime / duration) * 100 : 0);

	// Language code to name mapping
	const languageNames: Record<string, string> = {
		hun: 'Magyar',
		hu: 'Magyar',
		eng: 'English',
		en: 'English',
		ger: 'Deutsch',
		de: 'Deutsch',
		deu: 'Deutsch',
		fra: 'Français',
		fr: 'Français',
		fre: 'Français',
		spa: 'Español',
		es: 'Español',
		ita: 'Italiano',
		it: 'Italiano',
		jpn: 'Japanese',
		ja: 'Japanese',
		kor: 'Korean',
		ko: 'Korean',
		rus: 'Russian',
		ru: 'Russian',
		por: 'Português',
		pt: 'Português',
		pol: 'Polish',
		pl: 'Polish',
		cze: 'Czech',
		cs: 'Czech',
		und: 'Unknown'
	};

	// Get display name for audio track
	function getAudioTrackLabel(track: AudioTrack): string {
		const langCode = track.language?.toLowerCase() || 'und';
		const lang = languageNames[langCode] || track.language?.toUpperCase() || 'Unknown';
		const channels = track.channels ? ` (${track.channels}ch)` : '';
		return `${lang}${channels}`;
	}

	// Get display name for subtitle track
	function getSubtitleTrackLabel(track: SubtitleTrack): string {
		// Use language_name if available, otherwise map from code
		if (track.language_name) {
			return track.language_name;
		}
		if (track.language) {
			const langCode = track.language.toLowerCase();
			return languageNames[langCode] || track.language.toUpperCase();
		}
		return 'Felirat';
	}

	// Start streaming from a specific position with optional audio track
	async function startStreamFrom(position: number, audioTrack?: number) {
		if (!videoElement) return;

		isLoading = true;
		// Ensure position is a valid number (floor to integer for cleaner URLs)
		const safePosition = Math.max(0, Math.floor(position || 0));
		streamStartOffset = safePosition;

		const audio = audioTrack ?? selectedAudioTrack;
		const url = `${getApiBase()}/v2/stream/web/${mediaId}?start=${safePosition}&audio=${audio}`;

		// Stop any current playback and clear source first
		videoElement.pause();
		videoElement.removeAttribute('src');
		videoElement.load(); // Reset the element

		// Set new source
		videoElement.src = url;
		videoElement.volume = volume;
		videoElement.muted = isMuted;

		// Wait for sufficient buffer and video rendering to ensure A/V sync
		await new Promise<void>((resolve) => {
			let resolved = false;
			let loadedDataFired = false;
			let canPlayThroughFired = false;

			const doResolve = () => {
				if (resolved) return;
				resolved = true;
				resolve();
			};

			// Check if we have sufficient buffer (at least 3-5 seconds)
			const checkBuffer = (): boolean => {
				if (!videoElement || !videoElement.buffered.length) return false;
				const bufferedEnd = videoElement.buffered.end(0);
				const currentTime = videoElement.currentTime;
				return bufferedEnd - currentTime >= 3; // At least 3 seconds buffered
			};

			// Check if video is actually rendering (has valid dimensions)
			const checkVideoRendering = (): boolean => {
				if (!videoElement) return false;
				return videoElement.videoWidth > 0 && videoElement.videoHeight > 0;
			};

			// Check if all conditions are met for synchronized playback
			// Don't require events - check actual state instead
			const checkAllConditions = (): boolean => {
				if (!videoElement) return false;
				return (
					checkBuffer() &&
					checkVideoRendering() &&
					videoElement.readyState >= 3 // HAVE_FUTURE_DATA or better
					// Note: We don't require events here - we check actual state
					// Events are just triggers to start checking, not requirements
				);
			};

			// Wait for all conditions and then add sync delay
			const waitForSync = () => {
				if (resolved) return;

				// Check conditions with retries
				const checkWithRetries = (retries: number) => {
					if (resolved) return;
					if (checkAllConditions() || retries <= 0) {
						// Additional sync delay (2.5-3 seconds) to ensure FFmpeg has fully synchronized streams
						// This gives FFmpeg time to align audio and video timestamps and output synchronized frames
						// We wait longer to ensure the first few frames are properly synchronized
						setTimeout(doResolve, 3000);
					} else {
						setTimeout(() => checkWithRetries(retries - 1), 100);
					}
				};

				checkWithRetries(50); // Up to 5 seconds of checking (increased from 40)
			};

			// Handle loadeddata event (first frame loaded)
			const handleLoadedData = () => {
				if (resolved) return;
				loadedDataFired = true;
				waitForSync();
			};

			// Handle canplaythrough event
			const handleCanPlayThrough = () => {
				if (resolved) return;
				canPlayThroughFired = true;
				waitForSync();
			};

			// Progress event can help detect when more data is available
			// Also start checking periodically even without events
			const handleProgress = () => {
				if (resolved) return;
				if (checkAllConditions()) {
					waitForSync();
				}
			};

			// Periodic check even if events don't fire
			// This ensures we don't wait forever if events are delayed
			const periodicCheck = setInterval(() => {
				if (resolved) {
					clearInterval(periodicCheck);
					return;
				}
				if (checkAllConditions()) {
					clearInterval(periodicCheck);
					waitForSync();
				}
			}, 200); // Check every 200ms

			if (videoElement) {
				videoElement.addEventListener('loadeddata', handleLoadedData, { once: true });
				videoElement.addEventListener('canplaythrough', handleCanPlayThrough, { once: true });
				videoElement.addEventListener('progress', handleProgress);

				// Timeout fallback - wait up to 10 seconds
				setTimeout(() => {
					if (!resolved) {
						videoElement?.removeEventListener('progress', handleProgress);
						clearInterval(periodicCheck);
						doResolve();
					}
				}, 10000);
			} else {
				doResolve();
			}
		});

		// Hide initial info overlay after 6 seconds (regardless of play state)
		if (showInitialInfo) {
			if (initialInfoTimeout) {
				clearTimeout(initialInfoTimeout);
			}
			initialInfoTimeout = setTimeout(() => {
				showInitialInfo = false;
			}, 6000);
		}

		// Additional wait to ensure video is fully ready before playing
		// This helps ensure the first frame is properly synchronized with audio
		await new Promise(resolve => setTimeout(resolve, 500));

		try {
			await videoElement.play();
			isPlaying = true;
		} catch (e) {
			console.log('Autoplay blocked, waiting for user interaction');
		}

		isLoading = false;

		// Reload subtitle if one was selected (to sync with new offset)
		// Add delay to ensure video playback is stable before loading subtitles
		if (selectedSubtitleIndex !== null) {
			setTimeout(() => {
				if (selectedSubtitleIndex !== null) {
					enableSubtitle(selectedSubtitleIndex);
				}
			}, 300);
		}
	}

	// Load media details and start playback
	async function loadMedia(id: number) {
		if (!videoElement) return;

		isLoading = true;
		error = null;
		showNextEpisodeButton = false;
		
		// Reset player state
		currentTime = 0;
		duration = 0;
		audioTracks = [];
		subtitleTracks = [];
		selectedAudioTrack = 0;
		selectedSubtitleIndex = null;
		creditsDetected = false;
		consecutiveCreditsFrames = 0;
		lastCheckTime = 0;

		try {
			// Fetch media tracks info (duration, position, audio tracks)
			const tracksResponse = await fetch(`${getApiBase()}/v2/media/${id}/tracks`);
			if (tracksResponse.ok) {
				const tracksData = await tracksResponse.json();
				duration = tracksData.duration;
				audioTracks = tracksData.audio_tracks || [];
				subtitleTracks = tracksData.subtitle_tracks || [];

				// Use initialPosition if explicitly provided (including 0 for "start from beginning")
				// Only fall back to saved position if initialPosition was not set
				// Note: When switching episodes, initialPosition prop might be stale if not updated by parent yet,
				// but usually we want to start from 0 or saved position for the new episode.
				// If we are just mounting, use prop. If switching, use saved position or 0.
				const startPos = tracksData.current_position || 0;
				streamStartOffset = startPos;

				// Select Hungarian audio track by default if available
				const hunIndex = audioTracks.findIndex(
					(t) => t.language?.toLowerCase() === 'hun' || t.language?.toLowerCase() === 'hu'
				);
				if (hunIndex >= 0) {
					selectedAudioTrack = hunIndex;
				} else if (audioTracks.length > 0) {
					// Fall back to default track or first track
					const defaultIndex = audioTracks.findIndex((t) => t.is_default);
					selectedAudioTrack = defaultIndex >= 0 ? defaultIndex : 0;
				}

				// Start streaming from saved/initial position
				await startStreamFrom(startPos, selectedAudioTrack);
			} else {
				// Fallback to basic media info
				const mediaResponse = await fetch(`${getApiBase()}/v2/media/${id}`);
				if (mediaResponse.ok) {
					const mediaData = await mediaResponse.json();
					if (mediaData.duration) {
						duration = mediaData.duration;
					}
				}
				await startStreamFrom(0);
			}
			// Fetch subtitle generation capabilities (non-blocking)
			fetchSubtitleCapabilities().then(caps => {
				subtitleCapabilities = caps;
			}).catch(err => {
				console.warn('Failed to fetch subtitle capabilities:', err);
			});
		} catch (e) {
			console.error('Player initialization error:', e);
			error = e instanceof Error ? e.message : 'Unknown error';
			isLoading = false;
		}
	}

	// Initialize player on mount
	onMount(async () => {
		if (!browser) return;

		initializeCast();

		if (!videoElement) return;

		// Prevent body scroll when player is open
		const scrollY = window.scrollY;
		document.body.style.position = 'fixed';
		document.body.style.top = `-${scrollY}px`;
		document.body.style.width = '100%';
		document.body.style.overflow = 'hidden';
	});

	// React to mediaId changes
	$effect(() => {
		if (videoElement && mediaId) {
			// Use untrack to avoid re-running if internal state changes, 
			// but we want to run when mediaId changes.
			untrack(() => loadMedia(mediaId));
		}
	});

	// React to seriesId changes
	$effect(() => {
		if (seriesId) {
			fetchSeriesDetails(seriesId).then(details => {
				seriesDetails = details;
				// Filter episodes for current season
				if (seasonNumber !== undefined) {
					const season = details.seasons.find(s => s.season_number === seasonNumber);
					if (season) {
						currentSeasonEpisodes = season.episodes;
						selectedSeasonInModal = seasonNumber;
					}
				} else if (details.seasons.length > 0) {
					// Fallback to first season if no season number provided
					currentSeasonEpisodes = details.seasons[0].episodes;
					selectedSeasonInModal = details.seasons[0].season_number;
				}
			}).catch(err => {
				console.warn('Failed to fetch series details:', err);
			});
		}
	});

	// Cleanup on destroy
	onDestroy(() => {
		// Save progress before closing (force save)
		if (currentTime > 0 && duration > 0) {
			saveProgress(true);
		}

		if (controlsTimeout) {
			clearTimeout(controlsTimeout);
		}

		if (initialInfoTimeout) {
			clearTimeout(initialInfoTimeout);
		}

		// Clean up video element
		if (videoElement) {
			videoElement.pause();
			videoElement.src = '';
		}

		// Restore body scroll
		if (browser) {
			const scrollY = document.body.style.top;
			document.body.style.position = '';
			document.body.style.top = '';
			document.body.style.width = '';
			document.body.style.overflow = '';
			if (scrollY) {
				window.scrollTo(0, parseInt(scrollY || '0') * -1);
			}
		}
	});

	// Save progress to server
	function saveProgress(force: boolean = false) {
		if (!browser) return;
		
		// Don't save if less than 5 seconds (unless forced)
		if (!force && currentTime < 5) return;
		
		// Don't save if we just saved recently (unless forced)
		if (!force && Math.abs(currentTime - lastSavedTime) < 5) return;

		const isWatched = duration > 0 && currentTime >= duration - 60;

		const data = JSON.stringify({
			current_position_seconds: Math.floor(currentTime),
			is_watched: isWatched
		});

		// Use sendBeacon with Blob to set Content-Type header
		const blob = new Blob([data], { type: 'application/json' });
		const success = navigator.sendBeacon(`${getApiBase()}/v2/progress/${mediaId}`, blob);
		
		if (success) {
			lastSavedTime = currentTime;
		}
	}

	// Format time as MM:SS or H:MM:SS
	function formatTime(seconds: number): string {
		if (!isFinite(seconds) || isNaN(seconds)) return '0:00';

		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = Math.floor(seconds % 60);

		if (h > 0) {
			return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
		}
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	// Playback controls
	function togglePlay() {
		if (isCasting && castController) {
			castController.playOrPause();
			return;
		}

		if (!videoElement) return;

		if (isPlaying) {
			videoElement.pause();
		} else {
			videoElement.play();
		}
	}

	function toggleMute() {
		if (isCasting && castController) {
			castController.muteOrUnmute();
			return;
		}

		if (!videoElement) return;
		isMuted = !isMuted;
		videoElement.muted = isMuted;
	}

	function setVolume(value: number) {
		if (isCasting && castPlayer && castController) {
			castPlayer.volumeLevel = value;
			castController.setVolumeLevel();
			volume = value;
			return;
		}

		if (!videoElement) return;
		volume = value;
		videoElement.volume = value;
		if (value > 0 && isMuted) {
			isMuted = false;
			videoElement.muted = false;
		}
	}

	// Seek to absolute position in video - restarts stream from new position
	async function seek(absoluteTime: number) {
		if (isCasting && castPlayer && castController) {
			castPlayer.currentTime = absoluteTime;
			castController.seek();
			return;
		}

		if (!videoElement || isSeeking) return;

		// Clamp to valid range
		const seekTarget = Math.max(0, Math.min(absoluteTime, duration));

		// Don't restart stream for small seeks (within buffered range)
		const relativeTime = seekTarget - streamStartOffset;
		if (relativeTime >= 0 && relativeTime <= videoElement.duration) {
			// Can seek within current stream
			videoElement.currentTime = relativeTime;
			return;
		}

		// Need to restart stream from new position
		isSeeking = true;
		saveProgress(true); // Save current progress before seeking (force save)

		await startStreamFrom(seekTarget);
		isSeeking = false;
	}

	function skip(seconds: number) {
		if (!videoElement) return;
		// Use current absolute time + offset
		seek(currentTime + seconds);
	}

	// Switch audio track - saves position and restarts stream with new audio
	async function switchAudioTrack(trackIndex: number) {
		if (trackIndex === selectedAudioTrack) {
			showAudioMenu = false;
			return;
		}

		saveProgress(true); // Save current position before switching
		selectedAudioTrack = trackIndex;
		showAudioMenu = false;

		// Restart stream from current position with new audio track
		await startStreamFrom(currentTime, trackIndex);
	}

	// Enable subtitle track
	function enableSubtitle(index: number) {
		if (!videoElement) return;

		// First, completely clear all existing subtitle state
		// Disable and remove all text tracks
		for (let i = 0; i < videoElement.textTracks.length; i++) {
			videoElement.textTracks[i].mode = 'disabled';
		}

		// Remove all existing track elements
		const existingTracks = videoElement.querySelectorAll('track');
		existingTracks.forEach((track) => track.remove());

		// Wait a frame to ensure browser has cleared the old cues
		requestAnimationFrame(() => {
			if (!videoElement) return;

			// Add the new subtitle track with offset for sync
			// The offset matches the stream start position so timestamps align
			const track = document.createElement('track');
			track.kind = 'subtitles';
			// Add cache-busting timestamp to force reload
			track.src = `${getApiBase()}/v2/subtitles/${mediaId}/${index}?offset=${streamStartOffset}&_t=${Date.now()}`;
			track.default = true;

			// Set label from track info
			const subtitleInfo = subtitleTracks[index];
			if (subtitleInfo) {
				track.label = getSubtitleTrackLabel(subtitleInfo);
				track.srclang = subtitleInfo.language || 'und';
			}

			videoElement.appendChild(track);

			// Wait for track to load, then enable it
			const enableTrack = () => {
				if (videoElement?.textTracks[0]) {
					videoElement.textTracks[0].mode = 'showing';
				}
			};

			track.addEventListener('load', enableTrack);

			// Fallback: enable after short delay
			setTimeout(enableTrack, 200);
		});

		selectedSubtitleIndex = index;
		showSubtitleMenu = false;
	}

	// Disable all subtitles
	function disableSubtitles() {
		if (!videoElement) return;

		// Hide all text tracks
		for (let i = 0; i < videoElement.textTracks.length; i++) {
			videoElement.textTracks[i].mode = 'hidden';
		}

		// Remove track elements
		const existingTracks = videoElement.querySelectorAll('track');
		existingTracks.forEach(track => track.remove());

		selectedSubtitleIndex = null;
		showSubtitleMenu = false;
	}

	// Toggle subtitle (for single subtitle track)
	function toggleSubtitle() {
		if (selectedSubtitleIndex !== null) {
			disableSubtitles();
		} else if (subtitleTracks.length > 0) {
			enableSubtitle(0);
		}
	}

	// Set subtitle font size
	function setSubtitleFontSize(size: 'small' | 'default' | 'large') {
		subtitleFontSize = size;
		showSubtitleMenu = false;
	}

	// Open subtitle generator
	function openSubtitleGenerator() {
		showSubtitleMenu = false;
		showSubtitleGenerator = true;
	}

	// Episode selector functions
	function formatDuration(seconds: number | null): string {
		if (!seconds || !isFinite(seconds)) return '0:00';
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		const s = Math.floor(seconds % 60);
		if (h > 0) {
			return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
		}
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	function changeSeason(seasonNum: number) {
		if (!seriesDetails) return;
		const season = seriesDetails.seasons.find(s => s.season_number === seasonNum);
		if (season) {
			currentSeasonEpisodes = season.episodes;
			selectedSeasonInModal = seasonNum;
		}
	}

	async function switchEpisode(episode: Media) {
		if (episode.id === mediaId) {
			showEpisodeSelector = false;
			return;
		}

		// Save current progress before switching
		saveProgress(true);

		// Close episode selector
		showEpisodeSelector = false;

		// Call callback to switch episode
		if (onEpisodeChange) {
			onEpisodeChange(episode);
		}
	}

	// Handle subtitle generation complete
	async function handleSubtitleGenerated() {
		showSubtitleGenerator = false;
		// Reload tracks to get the new subtitle
		try {
			const tracksResponse = await fetch(`${getApiBase()}/v2/media/${mediaId}/tracks`);
			if (tracksResponse.ok) {
				const tracksData = await tracksResponse.json();
				subtitleTracks = tracksData.subtitle_tracks || [];
				// Auto-enable the first subtitle if we just generated one
				if (subtitleTracks.length > 0) {
					enableSubtitle(0);
				}
			}
		} catch (e) {
			console.error('Failed to reload tracks after subtitle generation:', e);
		}
	}

	function toggleFullscreen() {
		if (!containerElement) return;

		if (!isFullscreen) {
			containerElement.requestFullscreen?.();
		} else {
			document.exitFullscreen?.();
		}
	}

	// Credits detection logic
	function checkCredits() {
		if (!videoElement || !nextEpisode) return;

		// Initialize canvas if needed
		if (!creditsCanvas) {
			creditsCanvas = document.createElement('canvas');
			creditsCanvas.width = 50;
			creditsCanvas.height = 50;
			creditsContext = creditsCanvas.getContext('2d', { willReadFrequently: true });
		}

		if (!creditsContext) return;

		try {
			// Draw current frame to small canvas
			creditsContext.drawImage(videoElement, 0, 0, 50, 50);
			const frameData = creditsContext.getImageData(0, 0, 50, 50);
			const data = frameData.data;
			let darkPixels = 0;
			let lightPixels = 0;
			const totalPixels = data.length / 4;

			for (let i = 0; i < data.length; i += 4) {
				const r = data[i];
				const g = data[i + 1];
				const b = data[i + 2];
				
				// Check for dark pixel (black background)
				// Relaxed to < 60 to handle compression artifacts and washed out blacks
				if (r < 60 && g < 60 && b < 60) {
					darkPixels++;
				}
				// Check for light pixel (text)
				// Relaxed to > 100 because downsampling blends white text with black bg, making it grey
				// Also changed to OR to catch colored logos/text easier
				else if (r > 100 || g > 100 || b > 100) {
					lightPixels++;
				}
			}

			// Heuristic: Mostly dark (>90%) but with some light content (>0.05%)
			const darkRatio = darkPixels / totalPixels;
			const lightRatio = lightPixels / totalPixels;

			// console.log(`Credits detection: Dark=${darkRatio.toFixed(3)}, Light=${lightRatio.toFixed(4)}`);

			if (darkRatio > 0.95 && lightRatio > 0.00005) {
				consecutiveCreditsFrames++;
				if (consecutiveCreditsFrames >= 3) {
					creditsDetected = true;
				}
			} else {
				consecutiveCreditsFrames = 0;
				// Don't reset detected state immediately to avoid flickering off
				// creditsDetected = false; 
			}
		} catch (e) {
			// Canvas might be tainted or other error
			console.warn('Error checking credits:', e);
		}
	}

	// Controls visibility
	function showControlsTemporarily() {
		showControls = true;
		if (controlsTimeout) {
			clearTimeout(controlsTimeout);
		}
		controlsTimeout = setTimeout(() => {
			if (isPlaying) {
				showControls = false;
			}
		}, 3000);
	}

	// Event handlers
	function handleTimeUpdate(e: Event) {
		// Don't update time if user is dragging
		if (isDragging) return;

		const video = e.target as HTMLVideoElement;
		// Add stream offset to get actual position in the full video
		currentTime = streamStartOffset + video.currentTime;

		// Save progress every 30 seconds (check if at least 30 seconds have passed since last save)
		if (currentTime > 0 && currentTime - lastSavedTime >= 30) {
			saveProgress();
		}

		// Credits detection and Next Episode button logic
		if (nextEpisode && duration > 0) {
			const timeRemaining = duration - currentTime;
			const now = Date.now();

			// Run credits check if within last 5 minutes (300 seconds) and not checked recently (throttle 1s)
			if (timeRemaining <= 300 && now - lastCheckTime >= 1000) {
				lastCheckTime = now;
				checkCredits();
			} else if (timeRemaining > 300) {
				// Reset detection if we seeked back or are too far from end
				creditsDetected = false;
				consecutiveCreditsFrames = 0;
			}

			// Show button if within 15s OR credits detected within last 5 minutes
			if (timeRemaining <= 15 || (timeRemaining <= 300 && creditsDetected)) {
				showNextEpisodeButton = true;
			} else {
				showNextEpisodeButton = false;
			}
		} else {
			showNextEpisodeButton = false;
		}
	}

	function handleLoadedMetadata(e: Event) {
		const video = e.target as HTMLVideoElement;
		// Only update duration from video if we don't already have it from API
		// Fragmented MP4 duration is initially incomplete
		if (!duration && video.duration && isFinite(video.duration)) {
			duration = video.duration;
		}
	}

	function handlePlay() {
		isPlaying = true;
	}

	function handlePause() {
		isPlaying = false;
		saveProgress(true); // Force save on pause
	}

	function handleEnded() {
		isPlaying = false;
		saveProgress(true); // Force save on end
	}

	function handleError(e: Event) {
		const video = e.target as HTMLVideoElement;
		if (video.error) {
			error = `Playback error: ${video.error.message || 'Unknown error'}`;
		}
		isLoading = false;
	}

	function handleWaiting() {
		isLoading = true;
	}

	function handleCanPlay() {
		isLoading = false;
	}

	// Handle container click to toggle play/pause
	function handleContainerClick(e: MouseEvent) {
		// Don't toggle if clicking on interactive elements (buttons, inputs, controls)
		const target = e.target as HTMLElement;
		
		// Check if clicking on a button, input, or control element
		if (
			target.tagName === 'BUTTON' ||
			target.tagName === 'INPUT' ||
			target.closest('button') ||
			target.closest('input') ||
			target.closest('.control-button') ||
			target.closest('.center-play') ||
			target.closest('.progress-bar') ||
			target.closest('.volume-slider') ||
			target.closest('.audio-menu')
		) {
			return;
		}
		
		// Toggle play/pause for any other click (video area, overlay, etc.)
		togglePlay();
	}

	function handleKeydown(e: KeyboardEvent) {
		switch (e.key) {
			case ' ':
			case 'k':
				e.preventDefault();
				togglePlay();
				break;
			case 'ArrowLeft':
				e.preventDefault();
				skip(-10);
				break;
			case 'ArrowRight':
				e.preventDefault();
				skip(10);
				break;
			case 'ArrowUp':
				e.preventDefault();
				setVolume(Math.min(1, volume + 0.1));
				break;
			case 'ArrowDown':
				e.preventDefault();
				setVolume(Math.max(0, volume - 0.1));
				break;
			case 'm':
				toggleMute();
				break;
			case 'f':
				toggleFullscreen();
				break;
			case 'Escape':
				if (isFullscreen) {
					toggleFullscreen();
				} else {
					onClose?.();
				}
				break;
		}
		showControlsTemporarily();
	}

	// Listen for fullscreen changes
	$effect(() => {
		if (!browser) return;

		const handleFullscreenChange = () => {
			isFullscreen = !!document.fullscreenElement;
		};

		document.addEventListener('fullscreenchange', handleFullscreenChange);

		return () => {
			document.removeEventListener('fullscreenchange', handleFullscreenChange);
		};
	});
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	bind:this={containerElement}
	class="video-player"
	data-subtitle-size={subtitleFontSize}
	role="application"
	aria-label="Video player"
	onclick={handleContainerClick}
	onmousemove={showControlsTemporarily}
	onkeydown={handleKeydown}
	tabindex="0"
>
	<!-- Video Element -->
	<video
		bind:this={videoElement}
		class="video-element"
		poster={posterUrl}
		playsinline
		crossorigin="anonymous"
		autoplay
		ontimeupdate={handleTimeUpdate}
		onloadedmetadata={handleLoadedMetadata}
		onplay={handlePlay}
		onpause={handlePause}
		onended={handleEnded}
		onerror={handleError}
		onwaiting={handleWaiting}
		oncanplay={handleCanPlay}
	>
		Your browser does not support video playback.
	</video>

	<!-- Loading Spinner -->
	{#if isLoading}
		<div class="loading-overlay">
			<div class="spinner"></div>
		</div>
	{/if}

	<!-- Error Display -->
	{#if error}
		<div class="error-overlay">
			<svg class="error-icon" viewBox="0 0 24 24" fill="currentColor">
				<path
					d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
				/>
			</svg>
			<p class="error-message">{error}</p>
			<button class="retry-button" onclick={() => window.location.reload()}>Retry</button>
		</div>
	{/if}

	<!-- Casting Overlay -->
	{#if isCasting}
		<div class="casting-overlay">
			<svg class="casting-icon" viewBox="0 0 24 24" fill="currentColor">
				<path d="M1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11zm9 5.27l5.14 5.14c.38.39 1.02.39 1.41 0l5.14-5.14c.39-.38.39-1.02 0-1.41l-5.14-5.14c-.39-.39-1.02-.39-1.41 0l-5.14 5.14c-.39.38-.39 1.02 0 1.41z"/>
			</svg>
			<p class="casting-text">Casting to TV</p>
		</div>
	{/if}

	<!-- Next Episode Button -->
	{#if showNextEpisodeButton && nextEpisode}
		<button class="next-episode-button" onclick={() => switchEpisode(nextEpisode!)}>
			<div class="next-episode-icon">
				<svg viewBox="0 0 24 24" width="24" height="24" aria-hidden="true" fill="none" role="img">
					<path fill="currentColor" d="M5 2.7a1 1 0 0 1 1.48-.88l16.93 9.3a1 1 0 0 1 0 1.76l-16.93 9.3A1 1 0 0 1 5 21.31z"></path>
				</svg>
			</div>
			<span class="next-episode-text">Next Episode</span>
		</button>
	{/if}

	<!-- Initial Rating Overlay (Netflix-style, shows first 6 seconds) -->
	{#if showInitialInfo}
		<div class="initial-info-overlay">
			<div class="rating-badge">
				<div class="rating-bar"></div>
				<div class="rating-content">
					{#if contentRating}
						<p class="rating-label">RATED</p>
						<p class="rating-value">{contentRating}</p>
					{/if}
					{#if contentWarnings}
						<p class="rating-warnings">{contentWarnings}</p>
					{/if}
					{#if !contentRating && !contentWarnings}
						<p class="rating-label">PLAYING</p>
					{/if}
				</div>
			</div>
		</div>
	{/if}

	<!-- Controls Overlay -->
	<div class="controls-overlay" class:visible={showControls || !isPlaying}>
		<!-- Top bar - Back button only (Netflix style) -->
		<div class="top-bar">
			<button class="back-button" onclick={() => onClose?.()} aria-label="Go back">
				<svg viewBox="0 0 24 24" fill="currentColor">
					<path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z" />
				</svg>
			</button>
			<div class="flex-1"></div>
		</div>

		<!-- Center play button -->
		{#if !isPlaying && !isLoading}
			<button class="center-play" onclick={togglePlay} aria-label="Play">
				<svg viewBox="0 0 24 24" width="24" height="24" fill="none" role="img">
					<path fill="currentColor" d="M5 2.7a1 1 0 0 1 1.48-.88l16.93 9.3a1 1 0 0 1 0 1.76l-16.93 9.3A1 1 0 0 1 5 21.31z"></path>
				</svg>
			</button>
		{/if}

		<!-- Bottom controls -->
		<div class="bottom-controls">
			<!-- Progress bar -->
			<div class="progress-container">
				<input
					type="range"
					class="progress-bar"
					min="0"
					max={duration || 100}
					value={currentTime}
					disabled={isSeeking}
					oninput={(e) => {
						isDragging = true;
						currentTime = parseFloat((e.target as HTMLInputElement).value);
					}}
					onchange={(e) => {
						isDragging = false;
						seek(parseFloat((e.target as HTMLInputElement).value));
					}}
					aria-label="Video progress"
					style="--progress: {progressPercent}%;"
				/>
				<span class="progress-time">{formatTime(Math.max(0, duration - currentTime))}</span>
			</div>

			<div class="controls-row">
				<div class="left-controls">
					<!-- Play/Pause -->
					<button class="control-button" onclick={togglePlay} aria-label={isPlaying ? 'Pause' : 'Play'}>
						{#if isPlaying}
							<svg viewBox="0 0 24 24" fill="currentColor">
								<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
							</svg>
						{:else}
							<svg viewBox="0 0 24 24" width="24" height="24" fill="none" role="img">
								<path fill="currentColor" d="M5 2.7a1 1 0 0 1 1.48-.88l16.93 9.3a1 1 0 0 1 0 1.76l-16.93 9.3A1 1 0 0 1 5 21.31z"></path>
							</svg>
						{/if}
					</button>

					<!-- Rewind -->
					<button class="control-button skip-button" onclick={() => skip(-10)} aria-label="Rewind 10 seconds">
						<svg viewBox="0 0 24 24" width="24" height="24" fill="none" role="img">
							<path fill="currentColor" fill-rule="evenodd" d="M11.02 2.05A10 10 0 1 1 2 12H0a12 12 0 1 0 5-9.75V1H3v4a1 1 0 0 0 1 1h4V4H6a10 10 0 0 1 5.02-1.95M2 4v3h3v2H1a1 1 0 0 1-1-1V4zm12.13 12q-.88 0-1.53-.42-.64-.44-1-1.22a5 5 0 0 1-.35-1.86q0-1.05.35-1.85.36-.79 1-1.22A2.7 2.7 0 0 1 14.13 9a2.65 2.65 0 0 1 2.52 1.65q.35.79.35 1.85 0 1.07-.35 1.86a3 3 0 0 1-1.01 1.22 2.7 2.7 0 0 1-1.52.42m0-1.35q.59 0 .91-.56.34-.56.34-1.59 0-1.01-.34-1.58-.33-.57-.91-.57-.6 0-.92.57-.34.56-.34 1.58t.34 1.6q.33.54.91.55m-5.53 1.2v-5.13l-1.6.42V9.82l3.2-.8v6.84z" clip-rule="evenodd"></path>
						</svg>
					</button>

					<!-- Forward -->
					<button class="control-button skip-button" onclick={() => skip(10)} aria-label="Forward 10 seconds">
						<svg viewBox="0 0 24 24" width="24" height="24" fill="none" role="img">
							<path fill="currentColor" fill-rule="evenodd" d="M6.44 3.69A10 10 0 0 1 18 4h-2v2h4a1 1 0 0 0 1-1V1h-2v1.25A12 12 0 1 0 24 12h-2A10 10 0 1 1 6.44 3.69M22 4v3h-3v2h4a1 1 0 0 0 1-1V4zm-9.4 11.58q.66.42 1.53.42a2.7 2.7 0 0 0 1.5-.42q.67-.44 1.02-1.22.35-.8.35-1.86 0-1.05-.35-1.85A2.65 2.65 0 0 0 14.13 9a2.7 2.7 0 0 0-1.53.43q-.64.44-1 1.22a4.5 4.5 0 0 0-.35 1.85q0 1.07.35 1.86.36.78 1 1.22m2.44-1.49q-.33.56-.91.56-.6 0-.92-.56-.34-.56-.34-1.59 0-1.01.34-1.58.33-.57.91-.57.6 0 .92.57.34.56.34 1.58t-.34 1.6M8.6 10.72v5.14h1.6V9.02l-3.2.8v1.32z" clip-rule="evenodd"></path>
						</svg>
					</button>

					<!-- Volume -->
					<div class="volume-control">
						<button class="control-button" onclick={toggleMute} aria-label={isMuted ? 'Unmute' : 'Mute'}>
							{#if isMuted || volume === 0}
								<svg viewBox="0 0 24 24" fill="currentColor">
									<path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z" />
								</svg>
							{:else if volume < 0.5}
								<svg viewBox="0 0 24 24" fill="currentColor">
									<path d="M18.5 12c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM5 9v6h4l5 5V4L9 9H5z" />
								</svg>
							{:else}
								<svg viewBox="0 0 24 24" fill="currentColor">
									<path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z" />
								</svg>
							{/if}
						</button>
						<input
							type="range"
							class="volume-slider"
							min="0"
							max="1"
							step="0.05"
							value={volume}
							oninput={(e) => setVolume(parseFloat((e.target as HTMLInputElement).value))}
							aria-label="Volume"
						/>
					</div>
				</div>

				<!-- Center: Title -->
				<div class="center-title">
					<span class="video-title">
						{#if seriesId && seriesDetails && episodeNumber !== undefined}
							{@const isGenericTitle = /^Episode\s+(One|Two|Three|Four|Five|Six|Seven|Eight|Nine|Ten|\d+)$/i.test(title)}
							{seriesDetails.series.title} E{episodeNumber.toString().padStart(2, '0')}{#if !isGenericTitle} {title}{/if}
						{:else}
							{title}
						{/if}
					</span>
				</div>

				<div class="right-controls">
					<!-- Subtitle Selector or Generate Button -->
					{#if subtitleTracks.length === 0 && subtitleCapabilities?.whisper_available && subtitleCapabilities?.whisper_model_exists}
						<!-- No subtitles available, but can generate -->
						<button
							class="control-button generate-subtitle-btn"
							onclick={openSubtitleGenerator}
							aria-label="Generate subtitle"
							title="Generate subtitle with AI"
						>
							<svg viewBox="0 0 24 24" fill="currentColor">
								<path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zM4 12h4v2H4v-2zm10 6H4v-2h10v2zm6 0h-4v-2h4v2zm0-4H10v-2h10v2z" />
							</svg>
							<span class="generate-badge">+</span>
						</button>
					{:else if subtitleTracks.length > 0}
						{#if subtitleTracks.length === 1}
							<!-- Single subtitle: ON/OFF toggle + font size menu -->
							<div class="subtitle-selector">
								<button
									class="control-button"
									class:active={selectedSubtitleIndex !== null}
									onclick={() => (showSubtitleMenu = !showSubtitleMenu)}
									aria-label="Subtitle settings"
								>
									<svg viewBox="0 0 24 24" fill="currentColor">
										<path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zM4 12h4v2H4v-2zm10 6H4v-2h10v2zm6 0h-4v-2h4v2zm0-4H10v-2h10v2z" />
									</svg>
								</button>
								{#if showSubtitleMenu}
									<div class="subtitle-menu" data-subtitle-size={subtitleFontSize}>
										<!-- Toggle ON/OFF -->
										<button
											class="subtitle-menu-item"
											onclick={toggleSubtitle}
										>
											{#if selectedSubtitleIndex !== null}
												<span class="check-mark">✓</span>
											{/if}
											{selectedSubtitleIndex !== null ? 'Felirat ki' : 'Felirat be'}
										</button>
										<!-- Font size section -->
										<div class="subtitle-menu-divider"></div>
										<div class="subtitle-font-sizes">
											<span class="font-size-label">Méret:</span>
											<button
												class="font-size-btn"
												class:active={subtitleFontSize === 'small'}
												onclick={() => setSubtitleFontSize('small')}
											>A</button>
											<button
												class="font-size-btn medium"
												class:active={subtitleFontSize === 'default'}
												onclick={() => setSubtitleFontSize('default')}
											>A</button>
											<button
												class="font-size-btn large"
												class:active={subtitleFontSize === 'large'}
												onclick={() => setSubtitleFontSize('large')}
											>A</button>
										</div>
										<!-- Generate new subtitle option -->
										{#if subtitleCapabilities?.whisper_available && subtitleCapabilities?.whisper_model_exists}
											<div class="subtitle-menu-divider"></div>
											<button
												class="subtitle-menu-item generate-option"
												onclick={() => { showSubtitleMenu = false; openSubtitleGenerator(); }}
											>
												<span class="generate-icon">+</span>
												Új felirat generálása...
											</button>
										{/if}
									</div>
								{/if}
							</div>
						{:else}
							<!-- Dropdown menu for multiple subtitles -->
							<div class="subtitle-selector">
								<button
									class="control-button"
									class:active={selectedSubtitleIndex !== null}
									onclick={() => (showSubtitleMenu = !showSubtitleMenu)}
									aria-label="Subtitle settings"
								>
									<svg viewBox="0 0 24 24" fill="currentColor">
										<path d="M20 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zM4 12h4v2H4v-2zm10 6H4v-2h10v2zm6 0h-4v-2h4v2zm0-4H10v-2h10v2z" />
									</svg>
								</button>
								{#if showSubtitleMenu}
									<div class="subtitle-menu" data-subtitle-size={subtitleFontSize}>
										<!-- Off option -->
										<button
											class="subtitle-menu-item"
											class:selected={selectedSubtitleIndex === null}
											onclick={disableSubtitles}
										>
											{#if selectedSubtitleIndex === null}
												<span class="check-mark">✓</span>
											{/if}
											Ki
										</button>
										<!-- Subtitle tracks -->
										{#each subtitleTracks as track, i}
											<button
												class="subtitle-menu-item"
												class:selected={i === selectedSubtitleIndex}
												onclick={() => enableSubtitle(i)}
											>
												{#if i === selectedSubtitleIndex}
													<span class="check-mark">✓</span>
												{/if}
												{getSubtitleTrackLabel(track)}
											</button>
										{/each}
										<!-- Font size section -->
										<div class="subtitle-menu-divider"></div>
										<div class="subtitle-font-sizes">
											<span class="font-size-label">Méret:</span>
											<button
												class="font-size-btn"
												class:active={subtitleFontSize === 'small'}
												onclick={() => setSubtitleFontSize('small')}
											>A</button>
											<button
												class="font-size-btn medium"
												class:active={subtitleFontSize === 'default'}
												onclick={() => setSubtitleFontSize('default')}
											>A</button>
											<button
												class="font-size-btn large"
												class:active={subtitleFontSize === 'large'}
												onclick={() => setSubtitleFontSize('large')}
											>A</button>
										</div>
										<!-- Generate new subtitle option -->
										{#if subtitleCapabilities?.whisper_available && subtitleCapabilities?.whisper_model_exists}
											<div class="subtitle-menu-divider"></div>
											<button
												class="subtitle-menu-item generate-option"
												onclick={() => { showSubtitleMenu = false; openSubtitleGenerator(); }}
											>
												<span class="generate-icon">+</span>
												Új felirat generálása...
											</button>
										{/if}
									</div>
								{/if}
							</div>
						{/if}
					{/if}

					<!-- Episode Selector (only for series episodes) -->
					{#if seriesId && currentSeasonEpisodes.length > 0}
						<button
							class="control-button"
							onclick={() => (showEpisodeSelector = !showEpisodeSelector)}
							aria-label="Episodes"
						>
							<svg viewBox="0 0 24 24" width="24" height="24" xmlns="http://www.w3.org/2000/svg" fill="none" role="img">
								<path fill="currentColor" fill-rule="evenodd" d="M8 5h14v8h2V5a2 2 0 0 0-2-2H8zm10 4H4V7h14a2 2 0 0 1 2 2v8h-2zM0 13c0-1.1.9-2 2-2h12a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm14 6v-6H2v6z" clip-rule="evenodd"></path>
							</svg>
						</button>
					{/if}

					<!-- Audio Track Selector -->
					{#if audioTracks.length > 0}
						<div class="audio-selector">
							<button
								class="control-button"
								onclick={() => (showAudioMenu = !showAudioMenu)}
								aria-label="Audio track"
							>
								<svg viewBox="0 0 24 24" fill="currentColor">
									<path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
								</svg>
							</button>
							{#if showAudioMenu}
								<div class="audio-menu">
									{#each audioTracks as track, i}
										<button
											class="audio-menu-item"
											class:selected={i === selectedAudioTrack}
											onclick={() => switchAudioTrack(i)}
										>
											{#if i === selectedAudioTrack}
												<span class="check-mark">✓</span>
											{/if}
											{getAudioTrackLabel(track)}
										</button>
									{/each}
								</div>
							{/if}
						</div>
					{/if}

					<!-- Cast Button -->
					{#if showCastButton}
						<button
							class="control-button"
							class:active={isCasting}
							onclick={() => castContext && castContext.requestSession()}
							aria-label="Cast"
						>
							<svg viewBox="0 0 24 24" fill="currentColor">
								<path d="M1 18v3h3c0-1.66-1.34-3-3-3zm0-4v2c2.76 0 5 2.24 5 5h2c0-3.87-3.13-7-7-7zm0-4v2c4.97 0 9 4.03 9 9h2c0-6.08-4.93-11-11-11zm9 5.27l5.14 5.14c.38.39 1.02.39 1.41 0l5.14-5.14c.39-.38.39-1.02 0-1.41l-5.14-5.14c-.39-.39-1.02-.39-1.41 0l-5.14 5.14c-.39.38-.39 1.02 0 1.41z"/>
							</svg>
						</button>
					{/if}

					<!-- Fullscreen -->
					<button
						class="control-button"
						onclick={toggleFullscreen}
						aria-label={isFullscreen ? 'Exit fullscreen' : 'Fullscreen'}
					>
						{#if isFullscreen}
							<svg viewBox="0 0 24 24" fill="currentColor">
								<path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z" />
							</svg>
						{:else}
							<svg viewBox="0 0 24 24" fill="currentColor">
								<path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z" />
							</svg>
						{/if}
					</button>
				</div>
			</div>
		</div>
	</div>
</div>

<!-- Subtitle Generator Modal -->
{#if showSubtitleGenerator}
	<div class="subtitle-generator-overlay">
		<SubtitleGenerator
			{mediaId}
			{audioTracks}
			onComplete={handleSubtitleGenerated}
			onClose={() => (showSubtitleGenerator = false)}
		/>
	</div>
{/if}

<!-- Episode Selector Modal -->
{#if showEpisodeSelector && currentSeasonEpisodes.length > 0}
	<div
		class="episode-selector-overlay"
		role="dialog"
		aria-modal="true"
		aria-label="Episode Selector"
		tabindex="-1"
		onclick={(e) => { if (e.target === e.currentTarget) showEpisodeSelector = false; }}
		onkeydown={(e) => { if (e.key === 'Escape') showEpisodeSelector = false; }}
	>
		<div class="episode-selector-modal">
			<div class="episode-selector-header">
				<div class="episode-selector-title-section">
					<h3 class="episode-selector-title">
						{#if seriesDetails}
							{seriesDetails.series.title}
						{:else}
							Episodes
						{/if}
					</h3>
					{#if seriesDetails && seriesDetails.seasons.length > 1}
						<div class="season-selector">
							<label for="season-select" class="season-select-label">Season:</label>
							<select
								id="season-select"
								class="season-select"
								value={selectedSeasonInModal ?? seasonNumber ?? seriesDetails.seasons[0]?.season_number}
								onchange={(e) => changeSeason(parseInt((e.target as HTMLSelectElement).value))}
							>
								{#each seriesDetails.seasons as season}
									<option value={season.season_number}>
										Season {season.season_number} ({season.episodes.length} episodes)
									</option>
								{/each}
							</select>
						</div>
					{/if}
				</div>
				<button class="episode-selector-close" onclick={() => (showEpisodeSelector = false)} aria-label="Close">
					<svg viewBox="0 0 24 24" fill="currentColor">
						<path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
					</svg>
				</button>
			</div>
			<div class="episode-selector-list">
				{#each currentSeasonEpisodes as episode}
					{@const isCurrentEpisode = episode.id === mediaId}
					{@const isWatched = episode.is_watched}
					{@const isInProgress = episode.current_position > 0 && !episode.is_watched}
					<button
						class="episode-card"
						class:current={isCurrentEpisode}
						onclick={() => switchEpisode(episode)}
					>
						<div class="episode-thumbnail">
							{#if episode.poster_url}
								<img src={getImageUrl(episode.poster_url)} alt={episode.title} />
							{:else}
								<div class="episode-thumbnail-placeholder">
									<span>{episode.episode_number ?? '?'}</span>
								</div>
							{/if}
							{#if isWatched}
								<div class="episode-watched-badge">
									<svg viewBox="0 0 24 24" fill="currentColor">
										<path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
									</svg>
								</div>
							{/if}
							{#if isCurrentEpisode}
								<div class="episode-current-badge">Playing</div>
							{/if}
							{#if isInProgress && episode.duration}
								<div class="episode-progress-bar">
									<div
										class="episode-progress-fill"
										style="width: {Math.min((episode.current_position / episode.duration) * 100, 100)}%"
									></div>
								</div>
							{/if}
						</div>
						<div class="episode-info">
							<div class="episode-number">Episode {episode.episode_number ?? '?'}</div>
							<h4 class="episode-title">{episode.title}</h4>
							{#if episode.duration}
								<div class="episode-duration">{formatDuration(episode.duration)}</div>
							{/if}
							{#if episode.overview}
								<p class="episode-overview">{episode.overview}</p>
							{/if}
						</div>
					</button>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	/* Subtitle Generator Overlay */
	.subtitle-generator-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1100;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
	}

	/* Generate subtitle button badge */
	.generate-subtitle-btn {
		position: relative;
	}

	.generate-badge {
		position: absolute;
		top: 4px;
		right: 4px;
		width: 14px;
		height: 14px;
		background: #e50914;
		border-radius: 50%;
		font-size: 12px;
		font-weight: bold;
		line-height: 14px;
		text-align: center;
	}
	.video-player {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: #000;
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		outline: none;
	}

	.video-element {
		width: 100%;
		height: 100%;
		object-fit: contain;
		cursor: pointer;
	}

	/* Loading spinner */
	.loading-overlay {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
	}

	.spinner {
		width: 48px;
		height: 48px;
		border: 4px solid rgba(255, 255, 255, 0.3);
		border-top-color: #e50914;
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	/* Error display */
	.error-overlay {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		text-align: center;
		color: white;
	}

	.error-icon {
		width: 64px;
		height: 64px;
		color: #e50914;
		margin-bottom: 16px;
	}

	.error-message {
		font-size: 16px;
		margin-bottom: 16px;
	}

	.retry-button {
		padding: 10px 24px;
		background: #e50914;
		color: white;
		border: none;
		border-radius: 4px;
		font-size: 14px;
		cursor: pointer;
	}

	.retry-button:hover {
		background: #f40612;
	}

	/* Casting overlay */
	.casting-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: black;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		z-index: 50;
	}

	.casting-icon {
		width: 64px;
		height: 64px;
		color: #e50914;
		margin-bottom: 16px;
		opacity: 0.8;
		animation: pulse 2s infinite;
	}

	.casting-text {
		color: white;
		font-size: 18px;
		opacity: 0.8;
	}

	@keyframes pulse {
		0% { opacity: 0.6; }
		50% { opacity: 1; }
		100% { opacity: 0.6; }
	}

	/* Initial info overlay (Netflix-style rating badge) */
	.initial-info-overlay {
		position: absolute;
		top: 80px;
		left: 24px;
		z-index: 100;
		pointer-events: none; /* Don't block clicks */
		animation: fade-in 0.5s ease-out;
	}

	@keyframes fade-in {
		from { opacity: 0; transform: translateX(-10px); }
		to { opacity: 1; transform: translateX(0); }
	}

	.rating-badge {
		display: flex;
		align-items: stretch;
		gap: 0;
	}

	.rating-bar {
		width: 4px;
		background: #e50914;
		border-radius: 2px 0 0 2px;
	}

	.rating-content {
		background: rgba(0, 0, 0, 0.7);
		padding: 8px 16px;
		border-radius: 0 4px 4px 0;
	}

	.rating-label {
		font-size: 11px;
		color: #999;
		margin: 0;
		letter-spacing: 0.5px;
	}

	.rating-value {
		font-size: 18px;
		font-weight: bold;
		color: white;
		margin: 2px 0;
	}

	.rating-warnings {
		font-size: 12px;
		color: #aaa;
		margin: 0;
	}

	/* Controls overlay */
	.controls-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: linear-gradient(
			to top,
			rgba(0, 0, 0, 0.8) 0%,
			transparent 25%,
			transparent 85%,
			rgba(0, 0, 0, 0.6) 100%
		);
		opacity: 0;
		transition: opacity 0.3s ease;
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		pointer-events: none;
	}

	.controls-overlay.visible {
		opacity: 1;
		pointer-events: auto;
	}

	/* Top bar */
	.top-bar {
		display: flex;
		align-items: center;
		padding: 16px 24px;
	}

	.back-button {
		width: 44px;
		height: 44px;
		padding: 10px;
		background: transparent;
		border: none;
		border-radius: 50%;
		cursor: pointer;
		color: white;
		transition: background 0.2s;
	}

	.back-button:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.back-button svg {
		width: 100%;
		height: 100%;
	}

	.flex-1 {
		flex: 1;
	}

	/* Center play button */
	.center-play {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 80px;
		height: 80px;
		padding: 0;
		background: rgba(0, 0, 0, 0.6);
		border: none;
		border-radius: 50%;
		cursor: pointer;
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.center-play:hover {
		background: rgba(229, 9, 20, 0.9);
	}

	.center-play svg {
		width: 40px;
		height: 40px;
	}

	/* Bottom controls */
	.bottom-controls {
		padding: 0 24px 16px;
	}

	/* Progress container */
	.progress-container {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-bottom: 8px;
	}

	/* Progress bar - Netflix style thin bar */
	.progress-bar {
		flex: 1;
		height: 3px;
		appearance: none;
		border-radius: 1.5px;
		cursor: pointer;
		outline: none;
		background: linear-gradient(to right, #e50914 var(--progress, 0%), rgba(255,255,255,0.3) var(--progress, 0%));
		transition: height 0.15s ease;
	}

	.progress-bar:hover {
		height: 5px;
	}

	.progress-bar::-webkit-slider-runnable-track {
		height: 100%;
		border-radius: 1.5px;
		background: transparent;
	}

	.progress-bar::-webkit-slider-thumb {
		appearance: none;
		width: 12px;
		height: 12px;
		background: #e50914;
		border-radius: 50%;
		cursor: pointer;
		margin-top: -4px;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.progress-bar:hover::-webkit-slider-thumb {
		opacity: 1;
	}

	.progress-bar::-moz-range-track {
		height: 100%;
		border-radius: 1.5px;
		background: transparent;
	}

	.progress-bar::-moz-range-thumb {
		width: 12px;
		height: 12px;
		background: #e50914;
		border-radius: 50%;
		cursor: pointer;
		border: none;
		opacity: 0;
		transition: opacity 0.15s;
	}

	.progress-bar:hover::-moz-range-thumb {
		opacity: 1;
	}

	.progress-time {
		color: rgba(255, 255, 255, 0.8);
		font-size: 13px;
		font-family: 'SF Mono', 'Monaco', 'Consolas', monospace;
		min-width: 50px;
		text-align: right;
	}

	/* Controls row */
	.controls-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.left-controls,
	.right-controls {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.center-title {
		flex: 1;
		text-align: center;
		padding: 0 16px;
	}

	.video-title {
		color: white;
		font-size: 16px;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Control buttons */
	.control-button {
		width: 40px;
		height: 40px;
		padding: 8px;
		background: none;
		border: none;
		cursor: pointer;
		color: white;
		opacity: 0.9;
		position: relative;
	}

	.control-button:hover {
		opacity: 1;
	}

	.control-button svg {
		width: 100%;
		height: 100%;
	}

	/* Skip buttons with label */
	.skip-button {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.skip-button svg {
		width: 24px;
		height: 24px;
	}

	/* Volume control */
	.volume-control {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.volume-slider {
		width: 70px;
		height: 3px;
		appearance: none;
		background: rgba(255, 255, 255, 0.3);
		border-radius: 1.5px;
		cursor: pointer;
	}

	.volume-slider::-webkit-slider-thumb {
		appearance: none;
		width: 10px;
		height: 10px;
		background: white;
		border-radius: 50%;
		cursor: pointer;
	}

	.volume-slider::-moz-range-thumb {
		width: 10px;
		height: 10px;
		background: white;
		border-radius: 50%;
		cursor: pointer;
		border: none;
	}

	/* Audio selector */
	.audio-selector {
		position: relative;
	}

	.audio-menu {
		position: absolute;
		bottom: 100%;
		right: 0;
		margin-bottom: 8px;
		background: rgba(20, 20, 20, 0.95);
		border-radius: 4px;
		padding: 8px 0;
		min-width: 200px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
		z-index: 100;
	}

	.audio-menu-item {
		display: flex;
		align-items: center;
		width: 100%;
		padding: 10px 16px;
		background: none;
		border: none;
		color: white;
		font-size: 14px;
		text-align: left;
		cursor: pointer;
		transition: background 0.15s;
	}

	.audio-menu-item:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.audio-menu-item.selected {
		color: #e50914;
	}

	.check-mark {
		margin-right: 8px;
		font-weight: bold;
	}

	/* Subtitle selector */
	.subtitle-selector {
		position: relative;
	}

	.control-button.active {
		color: #e50914;
	}

	.subtitle-menu {
		position: absolute;
		bottom: 100%;
		right: 0;
		margin-bottom: 8px;
		background: rgba(20, 20, 20, 0.95);
		border-radius: 4px;
		padding: 8px 0;
		min-width: 200px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
		z-index: 100;
	}

	.subtitle-menu-item {
		display: flex;
		align-items: center;
		width: 100%;
		padding: 10px 16px;
		background: none;
		border: none;
		color: white;
		font-size: 14px;
		text-align: left;
		cursor: pointer;
		transition: background 0.15s;
	}

	.subtitle-menu-item:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.subtitle-menu-item.selected {
		color: #e50914;
	}

	.subtitle-menu-item.generate-option {
		color: rgba(255, 255, 255, 0.8);
		font-size: 13px;
	}

	.subtitle-menu-item.generate-option:hover {
		color: white;
	}

	.generate-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		margin-right: 8px;
		background: rgba(229, 9, 20, 0.8);
		border-radius: 50%;
		font-size: 14px;
		font-weight: bold;
		color: white;
	}

	.subtitle-menu-divider {
		height: 1px;
		background: rgba(255, 255, 255, 0.2);
		margin: 8px 0;
	}

	.subtitle-font-sizes {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
	}

	.font-size-label {
		color: rgba(255, 255, 255, 0.7);
		font-size: 12px;
		margin-right: 4px;
	}

	.font-size-btn {
		width: 28px;
		height: 28px;
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 4px;
		color: white;
		cursor: pointer;
		font-weight: bold;
		font-size: 11px;
		transition: all 0.15s;
	}

	.font-size-btn.medium {
		font-size: 14px;
	}

	.font-size-btn.large {
		font-size: 17px;
	}

	.font-size-btn:hover {
		background: rgba(255, 255, 255, 0.2);
	}

	.font-size-btn.active {
		background: #e50914;
		border-color: #e50914;
	}

	/* Subtitle cue styling (WebVTT) */
	.video-player video::cue {
		background: rgba(0, 0, 0, 0.8);
		color: white;
		font-size: 1.4em;
		padding: 0.2em 0.5em;
		border-radius: 4px;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	}

	/* Font size variants */
	.video-player[data-subtitle-size='small'] video::cue {
		font-size: 1.1em;
	}

	.video-player[data-subtitle-size='default'] video::cue {
		font-size: 1.4em;
	}

	.video-player[data-subtitle-size='large'] video::cue {
		font-size: 1.8em;
	}

	/* Episode Selector Modal */
	.episode-selector-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1100;
		background: rgba(0, 0, 0, 0.9);
		display: flex;
		align-items: center;
		justify-content: center;
		backdrop-filter: blur(4px);
		animation: fade-in 0.2s ease-out;
	}

	.episode-selector-modal {
		width: 90%;
		max-width: 1200px;
		max-height: 90vh;
		background: #141414;
		border-radius: 8px;
		overflow: hidden;
		display: flex;
		flex-direction: column;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
	}

	.episode-selector-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 24px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.episode-selector-title-section {
		display: flex;
		align-items: center;
		gap: 24px;
		flex: 1;
	}

	.episode-selector-title {
		color: white;
		font-size: 24px;
		font-weight: 600;
		margin: 0;
	}

	.season-selector {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.season-select-label {
		color: rgba(255, 255, 255, 0.8);
		font-size: 16px;
		font-weight: 500;
	}

	.season-select {
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 4px;
		color: white;
		font-size: 16px;
		padding: 8px 16px;
		cursor: pointer;
		transition: all 0.2s;
		min-width: 200px;
	}

	.season-select:hover {
		background: rgba(255, 255, 255, 0.15);
		border-color: rgba(255, 255, 255, 0.3);
	}

	.season-select:focus {
		outline: none;
		border-color: #e50914;
		background: rgba(255, 255, 255, 0.15);
	}

	.season-select option {
		background: #141414;
		color: white;
	}

	.episode-selector-close {
		width: 40px;
		height: 40px;
		padding: 8px;
		background: transparent;
		border: none;
		border-radius: 50%;
		cursor: pointer;
		color: white;
		transition: background 0.2s;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.episode-selector-close:hover {
		background: rgba(255, 255, 255, 0.1);
	}

	.episode-selector-close svg {
		width: 24px;
		height: 24px;
	}

	.episode-selector-list {
		flex: 1;
		overflow-y: auto;
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.episode-card {
		display: flex;
		gap: 16px;
		padding: 16px;
		background: #1a1a1a;
		border: 2px solid transparent;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		width: 100%;
	}

	.episode-card:hover {
		background: #242424;
		border-color: rgba(255, 255, 255, 0.2);
	}

	.episode-card.current {
		border-color: #e50914;
		background: rgba(229, 9, 20, 0.1);
	}

	.episode-thumbnail {
		position: relative;
		flex-shrink: 0;
		width: 160px;
		height: 90px;
		border-radius: 4px;
		overflow: hidden;
		background: #2a2a2a;
	}

	.episode-thumbnail img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.episode-thumbnail-placeholder {
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: rgba(255, 255, 255, 0.5);
		font-size: 32px;
		font-weight: bold;
	}

	.episode-watched-badge {
		position: absolute;
		top: 8px;
		right: 8px;
		width: 24px;
		height: 24px;
		background: rgba(0, 0, 0, 0.8);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: #e50914;
	}

	.episode-watched-badge svg {
		width: 16px;
		height: 16px;
	}

	.episode-current-badge {
		position: absolute;
		bottom: 8px;
		left: 8px;
		right: 8px;
		background: #e50914;
		color: white;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 12px;
		font-weight: 600;
		text-align: center;
	}

	.episode-progress-bar {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 3px;
		background: rgba(255, 255, 255, 0.2);
	}

	.episode-progress-fill {
		height: 100%;
		background: #e50914;
		transition: width 0.2s;
	}

	.episode-info {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 8px;
		min-width: 0;
	}

	.episode-number {
		color: rgba(255, 255, 255, 0.7);
		font-size: 14px;
		font-weight: 500;
	}

	.episode-title {
		color: white;
		font-size: 18px;
		font-weight: 600;
		margin: 0;
	}

	.episode-card.current .episode-title {
		color: #e50914;
	}

	.episode-duration {
		color: rgba(255, 255, 255, 0.6);
		font-size: 14px;
	}

	.episode-overview {
		color: rgba(255, 255, 255, 0.7);
		font-size: 14px;
		line-height: 1.5;
		margin: 0;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.next-episode-button {
		position: absolute;
		bottom: 120px;
		right: 24px;
		background: white;
		color: black;
		padding: 8px 16px 8px 12px;
		border: none;
		border-radius: 4px;
		font-weight: bold;
		cursor: pointer;
		z-index: 200;
		display: flex;
		align-items: center;
		box-shadow: 0 4px 12px rgba(0,0,0,0.5);
		transition: transform 0.2s;
		animation: slide-up 0.3s ease-out;
		gap: 12px;
	}

	.next-episode-button:hover {
		transform: scale(1.05);
		background: #e6e6e6;
	}

	.next-episode-icon {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.next-episode-text {
		font-size: 16px;
		font-weight: bold;
		white-space: nowrap;
	}

	@keyframes slide-up {
		from { transform: translateY(20px); opacity: 0; }
		to { transform: translateY(0); opacity: 1; }
	}
</style>
