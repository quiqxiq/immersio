<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ArrowDown01Icon,
		SubtitleIcon,
		SparklesIcon,
		Comment01Icon,
		Maximize01Icon,
		Film02Icon,
		PlayIcon,
		PauseIcon,
		RepeatIcon,
		ArrowLeft01Icon,
		ArrowRight01Icon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { np, playback, prefs, shadowing, ui, videoTranscript } from '$lib/player.svelte';
	import { claimVideo, parkVideo, setHole, showVideo, video } from '$lib/video.svelte';
	import { cleanWordToken } from '$lib/lemma';
	import { vocab } from '$lib/vocab.svelte';
	import WordPopup from '$lib/components/WordPopup.svelte';
	import AiWordPopup from '$lib/components/AiWordPopup.svelte';
	import TranscriptView from './TranscriptView.svelte';
	import RecommendedVideos from './RecommendedVideos.svelte';
	import CommentsView from './CommentsView.svelte';

	let activeTab = $state<'transcript' | 'recommended' | 'comments'>('transcript');
	let videoContainer = $state<HTMLElement | null>(null);
	let currentSpeed = $state(1.0);

	// User voice recording practice state (Web Audio API)
	let isRecording = $state(false);
	let recSeconds = $state(0);
	let recTimer: any = null;
	let recordingError = $state<string | null>(null);
	let userAudioUrl = $state<string | null>(null);
	let mediaRecorder: MediaRecorder | null = null;
	let audioChunks: Blob[] = [];

	// Dictionary popup state inside sentence focus card
	let selectedWord = $state<string | null>(null);
	let selectedContext = $state<string | null>(null);
	let selectedAnchorRect = $state<DOMRect | null>(null);

	// AI popup state
	let selectedAiWord = $state<string | null>(null);
	let selectedAiContext = $state<string | null>(null);
	let selectedAiAnchorRect = $state<DOMRect | null>(null);

	let autoTimerTimeout: any = null;
	let autoTimerInterval: any = null;

	function clearShadowTimer() {
		clearTimeout(autoTimerTimeout);
		clearInterval(autoTimerInterval);
		shadowing.pauseRemainingMs = 0;
		shadowing.pauseTotalMs = 0;
	}

	function toggleFullscreen() {
		if (!videoContainer) return;
		if (!document.fullscreenElement) {
			videoContainer.requestFullscreen().catch(() => {});
		} else {
			document.exitFullscreen().catch(() => {});
		}
	}

	function setSpeed(spd: number) {
		currentSpeed = spd;
		api.setPlaybackParams(spd, 0).catch((e) => console.error('Failed to set speed:', e));
	}

	/**
	 * Cleans non-verbal sounds, musical tags, audio cues, and speaker change symbols
	 * like [music], [laughter], (applause), ♪ notes, and ">>" from shadowing dialogue cards.
	 */
	function cleanShadowingDialogue(text: string | null | undefined): string {
		if (!text) return '';
		return text
			.replace(/\[[^\]]*\]/g, '') // remove [music], [laughter], [cheering], etc.
			.replace(/\([^\)]*\)/g, '') // remove (applause), (screaming), etc.
			.replace(/[♪♫]+/g, '')      // remove musical note symbols
			.replace(/&gt;&gt;?/g, '')  // remove HTML entity >> or >
			.replace(/>+ */g, '')       // remove >> or >>> speaker indicators
			.replace(/(?:^|\s)-+\s*/g, ' ') // clean leading dashes from speaker turns
			.replace(/\s+/g, ' ')       // collapse multiple spaces
			.trim();
	}

	function resumeNext() {
		clearShadowTimer();
		shadowing.isWaiting = false;
		if (playback.paused) {
			api.togglePause();
		}
	}

	function replayCurrentSentence() {
		clearShadowTimer();
		const sentences = videoTranscript.sentences;
		const sIdx = videoTranscript.activeSentenceIndex >= 0 ? videoTranscript.activeSentenceIndex : 0;
		if (sentences && sentences.length > 0 && sIdx >= 0 && sIdx < sentences.length) {
			const startMs = sentences[sIdx].start_ms;
			api.seek(Math.max(0, startMs / 1000));
			shadowing.lastPausedSentenceId = -1;
			shadowing.isWaiting = false;
			if (playback.paused) {
				api.togglePause();
			}
			return;
		}

		const lines = videoTranscript.lines;
		const idx = videoTranscript.activeIndex >= 0 ? videoTranscript.activeIndex : 0;
		if (lines && lines.length > 0 && idx >= 0 && idx < lines.length) {
			const startMs = lines[idx].time_ms ?? 0;
			api.seek(Math.max(0, startMs / 1000));
			shadowing.lastPausedSentenceId = -1;
			shadowing.isWaiting = false;
			if (playback.paused) {
				api.togglePause();
			}
		}
	}

	function jumpSentence(delta: number) {
		clearShadowTimer();
		const sentences = videoTranscript.sentences;
		if (sentences && sentences.length > 0) {
			const cur = videoTranscript.activeSentenceIndex >= 0 ? videoTranscript.activeSentenceIndex : 0;
			const nextIdx = Math.max(0, Math.min(sentences.length - 1, cur + delta));
			const startMs = sentences[nextIdx].start_ms;
			api.seek(Math.max(0, startMs / 1000));
			shadowing.lastPausedSentenceId = -1;
			shadowing.isWaiting = false;
			if (playback.paused) {
				api.togglePause();
			}
			return;
		}

		const lines = videoTranscript.lines;
		if (!lines || lines.length === 0) return;
		const cur = videoTranscript.activeIndex >= 0 ? videoTranscript.activeIndex : 0;
		const nextIdx = Math.max(0, Math.min(lines.length - 1, cur + delta));
		const startMs = lines[nextIdx].time_ms ?? 0;
		api.seek(Math.max(0, startMs / 1000));
		shadowing.lastPausedSentenceId = -1;
		shadowing.isWaiting = false;
		if (playback.paused) {
			api.togglePause();
		}
	}

	async function toggleRecording() {
		recordingError = null;
		if (isRecording) {
			if (mediaRecorder && mediaRecorder.state !== 'inactive') {
				mediaRecorder.stop();
			}
			clearInterval(recTimer);
			isRecording = false;
		} else {
			if (!navigator?.mediaDevices?.getUserMedia) {
				recordingError = 'Fitur mikrofon tidak didukung webview ini.';
				return;
			}
			try {
				const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
				audioChunks = [];
				let options: MediaRecorderOptions = {};
				if (typeof MediaRecorder.isTypeSupported === 'function') {
					if (MediaRecorder.isTypeSupported('audio/webm;codecs=opus')) {
						options = { mimeType: 'audio/webm;codecs=opus' };
					} else if (MediaRecorder.isTypeSupported('audio/ogg;codecs=opus')) {
						options = { mimeType: 'audio/ogg;codecs=opus' };
					} else if (MediaRecorder.isTypeSupported('audio/mp4')) {
						options = { mimeType: 'audio/mp4' };
					}
				}

				mediaRecorder = options.mimeType ? new MediaRecorder(stream, options) : new MediaRecorder(stream);
				mediaRecorder.ondataavailable = (e) => {
					if (e.data && e.data.size > 0) audioChunks.push(e.data);
				};
				mediaRecorder.onstop = () => {
					const mime = mediaRecorder?.mimeType || 'audio/webm';
					const audioBlob = new Blob(audioChunks, { type: mime });
					if (userAudioUrl) URL.revokeObjectURL(userAudioUrl);
					userAudioUrl = URL.createObjectURL(audioBlob);
					stream.getTracks().forEach((t) => t.stop());
				};
				mediaRecorder.start();
				isRecording = true;
				recSeconds = 0;
				clearInterval(recTimer);
				recTimer = setInterval(() => {
					recSeconds += 1;
				}, 1000);
			} catch (err: any) {
				console.error('Mikrofon tidak dapat diakses:', err);
				recordingError = err?.message || 'Izin mikrofon ditolak.';
				isRecording = false;
				clearInterval(recTimer);
			}
		}
	}

	function playUserAudio() {
		if (userAudioUrl) {
			const a = new Audio(userAudioUrl);
			a.play().catch((err) => {
				console.error('Gagal memutar rekaman suara:', err);
				recordingError = 'Format rekaman tidak dapat diputar.';
			});
		}
	}

	function handleWordClick(word: string, fullSentence: string, e: MouseEvent) {
		e.stopPropagation();
		const clean = cleanWordToken(word);
		if (!clean || clean.length < 2) return;
		const rect = (e.currentTarget as HTMLElement)?.getBoundingClientRect?.() ?? null;
		selectedAiWord = null;
		selectedWord = clean;
		selectedContext = fullSentence;
		selectedAnchorRect = rect;
	}

	// Keyboard listener for Zero-Friction Shadowing (Space, R, M, Arrows, 1/2/3)
	function handleKeydown(e: KeyboardEvent) {
		const target = e.target as HTMLElement;
		if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) {
			return;
		}

		if (shadowing.active) {
			if (e.code === 'Space') {
				e.preventDefault();
				if (shadowing.isWaiting) {
					resumeNext();
				} else {
					api.togglePause();
				}
			} else if (e.code === 'KeyR' || e.code === 'ArrowUp') {
				e.preventDefault();
				replayCurrentSentence();
			} else if (e.code === 'ArrowLeft') {
				e.preventDefault();
				jumpSentence(-1);
			} else if (e.code === 'ArrowRight') {
				e.preventDefault();
				jumpSentence(1);
			} else if (e.code === 'KeyM') {
				e.preventDefault();
				toggleRecording();
			} else if (e.code === 'Digit1') {
				setSpeed(0.75);
			} else if (e.code === 'Digit2') {
				setSpeed(0.85);
			} else if (e.code === 'Digit3') {
				setSpeed(1.0);
			}
		}
	}

	// Keep activeSentenceIndex and activeIndex updated whenever video is playing
	$effect(() => {
		const posMs = playback.position * 1000;
		if (shadowing.isWaiting) return;

		const sentences = videoTranscript.sentences;
		if (sentences && sentences.length > 0) {
			let foundSentence = -1;
			for (let s = 0; s < sentences.length; s++) {
				const st = sentences[s];
				if (posMs >= st.start_ms && posMs < st.end_ms + 100) {
					foundSentence = s;
					break;
				} else if (posMs >= st.start_ms) {
					foundSentence = s;
				}
			}
			if (videoTranscript.activeSentenceIndex !== foundSentence) {
				videoTranscript.activeSentenceIndex = foundSentence;
			}
		}

		const lines = videoTranscript.lines;
		if (lines && lines.length > 0) {
			let foundLine = -1;
			for (let i = 0; i < lines.length; i++) {
				const t = lines[i].time_ms ?? 0;
				if (posMs >= t) {
					foundLine = i;
				} else {
					break;
				}
			}
			if (videoTranscript.activeIndex !== foundLine) {
				videoTranscript.activeIndex = foundLine;
			}
		}
	});

	// Smart Sentence-Boundary Auto-Pause & Dynamic Timer Engine
	$effect(() => {
		if (!shadowing.active || !shadowing.autoPause) return;
		const sentences = videoTranscript.sentences;
		const sIdx = videoTranscript.activeSentenceIndex;
		const posMs = playback.position * 1000;

		// 1. Primary: Use full reconstructed sentences
		if (sentences && sentences.length > 0 && sIdx >= 0 && sIdx < sentences.length) {
			const curSentence = sentences[sIdx];
			const cleanedSpeech = cleanShadowingDialogue(curSentence.text);

			// Do not pause on non-verbal tags ([Music], [Applause]) or empty lines
			if (curSentence.skip_pause || !cleanedSpeech) {
				return;
			}

			// Preemptive pause timing: stop 50-80ms before next cue or at sentence end
			// so the video NEVER leaks the beginning of the next sentence!
			const nextStartMs = (sIdx + 1 < sentences.length) ? sentences[sIdx + 1].start_ms : (curSentence.end_ms + 800);
			const targetPauseMs = Math.min(curSentence.end_ms - 40, nextStartMs - 70);

			if (posMs >= targetPauseMs && posMs <= nextStartMs + 500) {
				if (!playback.paused && shadowing.lastPausedSentenceId !== curSentence.id) {
					shadowing.lastPausedSentenceId = curSentence.id;
					shadowing.isWaiting = true;
					api.togglePause();

					if (shadowing.pauseMode === 'timer') {
						clearShadowTimer();
						const totalPause = Math.round(
							curSentence.suggested_pause_ms * (shadowing.paceMultiplier || 1.0)
						);
						shadowing.pauseTotalMs = totalPause;
						shadowing.pauseRemainingMs = totalPause;

						const startTick = Date.now();
						autoTimerInterval = setInterval(() => {
							const elapsed = Date.now() - startTick;
							const remaining = Math.max(0, totalPause - elapsed);
							shadowing.pauseRemainingMs = remaining;
							if (remaining <= 0) {
								clearInterval(autoTimerInterval);
							}
						}, 200);

						autoTimerTimeout = setTimeout(() => {
							clearInterval(autoTimerInterval);
							if (shadowing.isWaiting && playback.paused) {
								resumeNext();
							}
						}, totalPause);
					}
				}
			}
			return;
		}

		// 2. Fallback: Raw subtitle lines if sentences are not ready
		const lines = videoTranscript.lines;
		const idx = videoTranscript.activeIndex;
		if (!lines || lines.length === 0 || idx < 0 || idx >= lines.length) return;

		const curLine = lines[idx];
		const cleanedLine = cleanShadowingDialogue(curLine.text);
		if (!cleanedLine) return;

		const startMs = curLine.time_ms ?? 0;
		let endMs: number = curLine.end_time_ms ?? 0;
		if (endMs <= startMs) {
			if (idx + 1 < lines.length && lines[idx + 1].time_ms != null) {
				endMs = lines[idx + 1].time_ms!;
			} else {
				endMs = startMs + 3500;
			}
		}

		const nextStartMs = (idx + 1 < lines.length && lines[idx + 1].time_ms != null)
			? lines[idx + 1].time_ms!
			: endMs + 800;
		const targetPauseMs = Math.min(endMs - 40, nextStartMs - 70);

		if (posMs >= targetPauseMs && posMs <= nextStartMs + 500) {
			if (!playback.paused && shadowing.lastPausedSentenceId !== idx) {
				shadowing.lastPausedSentenceId = idx;
				shadowing.isWaiting = true;
				api.togglePause();

				if (shadowing.pauseMode === 'timer') {
					clearShadowTimer();
					const cueDuration = Math.max(1200, endMs - startMs);
					const suggestedPause = Math.max(2200, Math.min(8000, Math.round(cueDuration * 1.2)));
					const totalPause = Math.round(suggestedPause * (shadowing.paceMultiplier || 1.0));
					shadowing.pauseTotalMs = totalPause;
					shadowing.pauseRemainingMs = totalPause;

					const startTick = Date.now();
					autoTimerInterval = setInterval(() => {
						const elapsed = Date.now() - startTick;
						const remaining = Math.max(0, totalPause - elapsed);
						shadowing.pauseRemainingMs = remaining;
						if (remaining <= 0) {
							clearInterval(autoTimerInterval);
						}
					}, 200);

					autoTimerTimeout = setTimeout(() => {
						clearInterval(autoTimerInterval);
						if (shadowing.isWaiting && playback.paused) {
							resumeNext();
						}
					}, totalPause);
				}
			}
		}
	});

	onMount(() => {
		video.want = true;
		window.addEventListener('keydown', handleKeydown);
		return () => {
			window.removeEventListener('keydown', handleKeydown);
			if (userAudioUrl) URL.revokeObjectURL(userAudioUrl);
			clearShadowTimer();
			clearInterval(recTimer);
		};
	});

	function holeFor(el: HTMLElement) {
		let lastX = 0, lastY = 0, lastW = 0, lastH = 0;
		const measure = () => {
			if (document.hidden) {
				if (lastW !== 0 || lastH !== 0) {
					lastX = 0; lastY = 0; lastW = 0; lastH = 0;
					setHole(null);
				}
				return;
			}
			const r = el.getBoundingClientRect();
			if (r.width < 10 || r.height < 10) return;

			// If the video has scrolled completely offscreen, clear hole so it doesn't punch text
			if (r.bottom <= 0 || r.top >= window.innerHeight) {
				if (lastW !== 0 || lastH !== 0) {
					lastX = 0; lastY = 0; lastW = 0; lastH = 0;
					setHole(null);
				}
				return;
			}

			if (
				Math.abs(r.left - lastX) < 1 &&
				Math.abs(r.top - lastY) < 1 &&
				Math.abs(r.width - lastW) < 1 &&
				Math.abs(r.height - lastH) < 1
			) {
				return;
			}
			lastX = r.left;
			lastY = r.top;
			lastW = r.width;
			lastH = r.height;
			setHole({ x: r.left, y: r.top, w: r.width, h: r.height });
		};
		const ro = new ResizeObserver(measure);
		ro.observe(el);
		ro.observe(document.documentElement);
		document.addEventListener('visibilitychange', measure);
		window.addEventListener('resize', measure);
		window.addEventListener('scroll', measure, { capture: true, passive: true });
		requestAnimationFrame(measure);
		const timer = setTimeout(measure, 60);
		return () => {
			ro.disconnect();
			clearTimeout(timer);
			document.removeEventListener('visibilitychange', measure);
			window.removeEventListener('resize', measure);
			window.removeEventListener('scroll', measure, true);
			setHole(null);
		};
	}
</script>

<div
	transition:fade={{ duration: 180 }}
	onoutrostart={() => setHole(null)}
	class="absolute inset-y-0 left-16 right-0 z-20 flex flex-col overflow-y-auto {ui.sidebarCollapsed
		? ''
		: 'lg:left-60'} {video.hole ? '' : 'bg-background'} text-foreground select-none pb-28"
>
	<!-- Floating Back / Minimize Button -->
	<button
		type="button"
		onclick={() => {
			setHole(null);
			np.open = false;
		}}
		class="fixed top-3 left-20 z-40 flex h-9 w-9 items-center justify-center rounded-xl bg-background/85 text-foreground backdrop-blur-md transition-all hover:bg-background hover:scale-105 cursor-pointer shadow-md border border-border/60"
		title="Tutup / Perkecil Video"
		aria-label="Tutup player"
	>
		<HugeiconsIcon icon={ArrowDown01Icon} class="h-5 w-5" />
	</button>

	<!-- Main Centered Immersion Container -->
	<div class="mx-auto w-full max-w-5xl px-4 pt-3 sm:px-6">
		<!-- Top Video Stage (16:9) -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			bind:this={videoContainer}
			role="button"
			tabindex="0"
			onclick={() => api.togglePause()}
			ondblclick={toggleFullscreen}
			class="relative aspect-video w-full max-h-[70vh] cursor-pointer overflow-hidden rounded-2xl ring-1 ring-border/20 shadow-2xl {video.hole ? '' : 'bg-black'} mx-auto"
			title="Klik untuk Jeda/Putar, Klik dua kali untuk Layar Penuh"
		>
			{#if prefs.nativeVideo}
				{#if showVideo()}
					<div
						class="h-full w-full rounded-2xl {video.hole ? '' : 'bg-black'}"
						{@attach holeFor}
					></div>
				{:else}
					<!-- Clean poster thumbnail & spinner while new video stream is attaching -->
					<div class="relative h-full w-full flex items-center justify-center bg-black rounded-2xl overflow-hidden">
						{#if playback.now?.thumbnail}
							<img
								src={playback.now.thumbnail}
								alt=""
								class="h-full w-full object-cover blur-sm opacity-40 scale-105 transition-all duration-300"
							/>
						{/if}
						<div class="absolute inset-0 bg-black/40 flex flex-col items-center justify-center gap-3 text-white/90">
							<div class="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent shadow-lg"></div>
							<span class="text-xs font-medium tracking-wide drop-shadow-md">Menyiapkan video...</span>
						</div>
					</div>
				{/if}
			{:else}
				<div
					class="contents"
					{@attach (box: HTMLElement) => {
						claimVideo(box);
						return () => parkVideo(box);
					}}
				></div>
			{/if}
		</div>

		<!-- Video Meta & Immersion Controls (Directly Below Video) -->
		<div class="mt-4 flex flex-col gap-3.5 border-b border-border/50 pb-4">
			<!-- Title & Channel Info -->
			<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
				<div class="space-y-1 min-w-0 flex-1">
					<h1 class="text-base sm:text-xl font-bold leading-snug text-foreground tracking-tight line-clamp-2">
						{playback.now?.title || 'Memutar Video'}
					</h1>
					<div class="flex items-center gap-2 text-xs text-muted-foreground flex-wrap">
						<span class="font-semibold text-foreground/90 px-2.5 py-0.5 rounded-full bg-foreground/5 border border-border/40 inline-flex items-center gap-1.5">
							<span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
							{playback.now?.artists || 'YouTube'}
						</span>
						{#if playback.now?.album}
							<span>•</span>
							<span class="truncate">{playback.now.album}</span>
						{/if}
					</div>
				</div>

				<!-- Immersion Action Controls -->
				<div class="flex items-center gap-2 shrink-0 flex-wrap">
					<!-- Shadowing Mode Toggle Button -->
					<button
						type="button"
						onclick={() => {
							shadowing.active = !shadowing.active;
							shadowing.lastPausedIndex = -1;
							shadowing.isWaiting = false;
						}}
						class="flex h-8 items-center gap-1.5 rounded-xl px-3 text-xs font-semibold transition-all cursor-pointer border {shadowing.active
							? 'bg-primary text-primary-foreground border-primary shadow-sm'
							: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground border-border/40'}"
						title="Aktifkan/Nonaktifkan Auto-Pause per Kalimat untuk Shadowing"
					>
						<HugeiconsIcon icon={Film02Icon} class="h-4 w-4" />
						<span>{shadowing.active ? 'Shadowing Aktif' : 'Mode Shadowing'}</span>
					</button>

					<!-- Speed Selector (0.75x, 0.85x, 1.0x) -->
					<div class="flex items-center rounded-xl bg-foreground/5 p-0.5 text-xs border border-border/40">
						{#each [0.75, 0.85, 1.0] as spd}
							<button
								type="button"
								onclick={() => setSpeed(spd)}
								class="rounded-lg px-2.5 py-1 text-xs font-semibold transition-all cursor-pointer {currentSpeed === spd
									? 'bg-primary text-primary-foreground shadow-sm'
									: 'text-muted-foreground hover:text-foreground'}"
							>
								{spd}x
							</button>
						{/each}
					</div>

					<!-- Fullscreen Button -->
					<button
						type="button"
						onclick={toggleFullscreen}
						class="flex h-8 items-center gap-1.5 rounded-xl bg-foreground/5 px-3 text-xs font-medium text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer border border-border/40"
						title="Layar Penuh"
					>
						<HugeiconsIcon icon={Maximize01Icon} class="h-4 w-4" />
						<span class="hidden sm:inline">Layar Penuh</span>
					</button>
				</div>
			</div>

			<!-- SHADOWING ARENA: Focus Sentence Card & Rapid Replay Controls -->
			{#if shadowing.active}
				{@const sentences = videoTranscript.sentences}
				{@const activeSentenceIdx = videoTranscript.activeSentenceIndex}
				{@const curSentence = (sentences && activeSentenceIdx >= 0 && activeSentenceIdx < sentences.length) ? sentences[activeSentenceIdx] : null}
				{@const nextSentence = (sentences && activeSentenceIdx + 1 < sentences.length) ? sentences[activeSentenceIdx + 1] : null}

				{@const lines = videoTranscript.lines}
				{@const activeLineIdx = videoTranscript.activeIndex}
				{@const curLine = (lines && activeLineIdx >= 0 && activeLineIdx < lines.length) ? lines[activeLineIdx] : null}
				{@const nextLine = (lines && activeLineIdx + 1 < lines.length) ? lines[activeLineIdx + 1] : null}

				{@const currentDisplaySentenceText = cleanShadowingDialogue(curSentence ? curSentence.text : (curLine ? curLine.text : ''))}
				{@const nextDisplaySentenceText = cleanShadowingDialogue(nextSentence ? nextSentence.text : (nextLine ? nextLine.text : ''))}
				{@const timerPct = shadowing.pauseTotalMs > 0 ? Math.round((shadowing.pauseRemainingMs / shadowing.pauseTotalMs) * 100) : 0}

				<div class="rounded-2xl border-2 {shadowing.isWaiting ? 'border-primary bg-primary/5 shadow-md shadow-primary/10' : 'border-border/60 bg-card/70'} p-4.5 backdrop-blur-md transition-all">
					<!-- Header Status & Settings Ribbon -->
					<div class="flex items-center justify-between gap-3 border-b border-border/40 pb-3 flex-wrap">
						<div class="flex items-center gap-2">
							{#if shadowing.isWaiting}
								<span class="flex h-2.5 w-2.5 rounded-full bg-primary animate-ping"></span>
								<span class="text-xs font-bold text-primary tracking-wide">
									⏸️ Jeda Shadowing: Giliran Anda Menirukan!
								</span>
								{#if shadowing.pauseMode === 'timer'}
									<div class="flex items-center gap-1.5 ml-2">
										<span class="font-mono text-xs font-bold text-primary">
											{(shadowing.pauseRemainingMs / 1000).toFixed(1)}s
										</span>
										<div class="h-2 w-20 sm:w-28 bg-primary/20 rounded-full overflow-hidden">
											<div class="h-full bg-primary transition-all duration-75" style="width: {timerPct}%"></div>
										</div>
									</div>
								{/if}
							{:else}
								<span class="flex h-2 w-2 rounded-full bg-emerald-500"></span>
								<span class="text-xs font-semibold text-muted-foreground">
									● Mendengarkan Dialog... (Otomatis jeda di akhir kalimat)
								</span>
							{/if}
						</div>

						<!-- Auto-Pause Settings Options -->
						<div class="flex items-center gap-2 text-xs flex-wrap">
							<!-- Auto-Pause Toggle -->
							<button
								type="button"
								onclick={() => (shadowing.autoPause = !shadowing.autoPause)}
								class="rounded-lg px-2.5 py-1 font-semibold transition-all border cursor-pointer {shadowing.autoPause
									? 'bg-primary/15 text-primary border-primary/30'
									: 'bg-muted text-muted-foreground border-border/40'}"
								title="Otomatis pause di setiap akhir kalimat dialog"
							>
								Auto-Pause: {shadowing.autoPause ? 'ON' : 'OFF'}
							</button>

							<!-- Pace Multiplier Pills -->
							<div class="inline-flex items-center rounded-lg bg-foreground/5 p-0.5 border border-border/40 text-[11px]">
								{#each [{ label: 'Santai', val: 1.3 }, { label: 'Normal', val: 1.0 }, { label: 'Cepat', val: 0.8 }] as pace}
									<button
										type="button"
										onclick={() => (shadowing.paceMultiplier = pace.val)}
										class="rounded px-2 py-0.5 font-semibold transition cursor-pointer {shadowing.paceMultiplier === pace.val
											? 'bg-primary text-primary-foreground shadow-xs'
											: 'text-muted-foreground hover:text-foreground'}"
										title="Tempo Jeda {pace.label} ({pace.val}x)"
									>
										{pace.label}
									</button>
								{/each}
							</div>

							<!-- Manual vs Timer Toggle -->
							<button
								type="button"
								onclick={() => {
									shadowing.pauseMode = shadowing.pauseMode === 'manual' ? 'timer' : 'manual';
									clearShadowTimer();
								}}
								class="rounded-lg px-2.5 py-1 font-medium transition-all border border-border/40 bg-foreground/5 hover:bg-foreground/10 text-muted-foreground cursor-pointer"
								title="Pilih apakah video menunggu Spasi atau lanjut otomatis setelah jeda adaptif"
							>
								Mode: {shadowing.pauseMode === 'manual' ? 'Manual [Spasi]' : 'Auto Jeda'}
							</button>
						</div>
					</div>

					<!-- Current Sentence Typography Display -->
					<div class="py-4 space-y-2">
						{#if currentDisplaySentenceText}
							<div class="text-lg sm:text-2xl font-bold leading-relaxed text-foreground tracking-tight">
								{#each currentDisplaySentenceText.split(' ') as word}
									{@const clean = cleanWordToken(word)}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<span
										role="button"
										tabindex="0"
										onclick={(e) => handleWordClick(word, currentDisplaySentenceText, e)}
										class="inline-block px-1 py-0.5 rounded hover:bg-primary/20 hover:text-primary transition-colors cursor-pointer {vocab.getWordClass(clean)}"
										title="Klik kata untuk melihat arti & kamus"
									>
										{word}{' '}
									</span>
								{/each}
							</div>

							{#if nextDisplaySentenceText}
								<div class="text-xs text-muted-foreground/75 flex items-center gap-1.5 pt-1">
									<span class="font-semibold text-muted-foreground">Kalimat Berikutnya:</span>
									<span class="line-clamp-1 italic">"{nextDisplaySentenceText}"</span>
								</div>
							{/if}
						{:else}
							<div class="text-sm font-medium text-muted-foreground italic py-2">
								{videoTranscript.loading ? 'Memuat subtitel dialog...' : 'Menunggu dialog pertama dimulai...'}
							</div>
						{/if}
					</div>

					<!-- Shadowing Action Buttons Ribbon -->
					<div class="flex items-center justify-between gap-3 pt-2 border-t border-border/40 flex-wrap">
						<!-- Primary Replay & Next Buttons -->
						<div class="flex items-center gap-2">
							<!-- Replay Sentence Button -->
							<button
								type="button"
								onclick={replayCurrentSentence}
								class="flex items-center gap-1.5 rounded-xl bg-foreground/10 hover:bg-foreground/15 text-foreground px-3.5 py-2 text-xs font-bold transition-all cursor-pointer border border-border/50 shadow-xs"
								title="Putar ulang kalimat ini (Shortcut: R atau Panah Atas)"
							>
								<HugeiconsIcon icon={RepeatIcon} class="h-4 w-4" />
								<span>Ulangi (R)</span>
							</button>

							<!-- Resume / Next Button -->
							<button
								type="button"
								onclick={resumeNext}
								class="flex items-center gap-1.5 rounded-xl bg-primary text-primary-foreground px-4 py-2 text-xs font-bold transition-all cursor-pointer shadow-md hover:scale-[1.02]"
								title="Lanjutkan video ke kalimat berikutnya (Shortcut: Spasi)"
							>
								<HugeiconsIcon icon={PlayIcon} class="h-4 w-4 fill-current" />
								<span>Lanjut (Spasi)</span>
							</button>

							<!-- Prev & Next Sentence Navigation -->
							<div class="flex items-center rounded-xl bg-foreground/5 border border-border/40 p-0.5 ml-1">
								<button
									type="button"
									onclick={() => jumpSentence(-1)}
									class="p-1.5 rounded-lg text-muted-foreground hover:text-foreground cursor-pointer"
									title="Kalimat Sebelumnya (Panah Kiri)"
								>
									<HugeiconsIcon icon={ArrowLeft01Icon} class="h-3.5 w-3.5" />
								</button>
								<button
									type="button"
									onclick={() => jumpSentence(1)}
									class="p-1.5 rounded-lg text-muted-foreground hover:text-foreground cursor-pointer"
									title="Kalimat Berikutnya (Panah Kanan)"
								>
									<HugeiconsIcon icon={ArrowRight01Icon} class="h-3.5 w-3.5" />
								</button>
							</div>
						</div>

						<!-- Microphone Voice Practice Widget -->
						<div class="flex items-center gap-2 flex-wrap">
							{#if recordingError}
								<span class="text-[11px] font-medium text-rose-500 bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 rounded-lg">
									{recordingError}
								</span>
							{/if}

							<button
								type="button"
								onclick={toggleRecording}
								class="flex items-center gap-1.5 rounded-xl px-3 py-1.5 text-xs font-bold transition-all cursor-pointer border {isRecording
									? 'bg-rose-500 text-white border-rose-600 animate-pulse'
									: 'bg-foreground/5 hover:bg-foreground/10 text-muted-foreground hover:text-foreground border-border/40'}"
								title="Rekam suara Anda menirukan aktor (Shortcut: M)"
							>
								<span class="h-2 w-2 rounded-full {isRecording ? 'bg-white' : 'bg-rose-500'}"></span>
								<span>{isRecording ? `Merekam... (${recSeconds}s) [M]` : 'Latih Suara (M)'}</span>
							</button>

							{#if userAudioUrl}
								<button
									type="button"
									onclick={playUserAudio}
									class="flex items-center gap-1.5 rounded-xl bg-emerald-500/15 border border-emerald-500/30 text-emerald-600 dark:text-emerald-400 px-3 py-1.5 text-xs font-semibold transition-all hover:bg-emerald-500/25 cursor-pointer"
									title="Putar rekaman suara Anda untuk dibandingkan dengan aktor"
								>
									<HugeiconsIcon icon={PlayIcon} class="h-3.5 w-3.5 fill-current" />
									<span>Dengar Rekaman Anda</span>
								</button>
							{/if}
						</div>
					</div>
				</div>
			{/if}

			<!-- Tab Switcher (Transkrip, Rekomendasi, Komentar) -->
			<div class="flex border-b border-border/40 gap-3 pt-1">
				<button
					type="button"
					onclick={() => (activeTab = 'transcript')}
					class="flex items-center gap-2 pb-2.5 px-2 text-xs sm:text-sm font-semibold transition-all border-b-2 cursor-pointer {activeTab === 'transcript'
						? 'border-primary text-primary'
						: 'border-transparent text-muted-foreground hover:text-foreground'}"
				>
					<HugeiconsIcon icon={SubtitleIcon} class="h-4 w-4" />
					<span>Transkrip Interaktif</span>
				</button>
				<button
					type="button"
					onclick={() => (activeTab = 'recommended')}
					class="flex items-center gap-2 pb-2.5 px-2 text-xs sm:text-sm font-semibold transition-all border-b-2 cursor-pointer {activeTab === 'recommended'
						? 'border-primary text-primary'
						: 'border-transparent text-muted-foreground hover:text-foreground'}"
				>
					<HugeiconsIcon icon={SparklesIcon} class="h-4 w-4" />
					<span>Rekomendasi Terkait</span>
				</button>
				<button
					type="button"
					onclick={() => (activeTab = 'comments')}
					class="flex items-center gap-2 pb-2.5 px-2 text-xs sm:text-sm font-semibold transition-all border-b-2 cursor-pointer {activeTab === 'comments'
						? 'border-primary text-primary'
						: 'border-transparent text-muted-foreground hover:text-foreground'}"
				>
					<HugeiconsIcon icon={Comment01Icon} class="h-4 w-4" />
					<span>Komentar</span>
				</button>
			</div>
		</div>

		<!-- Tab Contents (Directly Under Video & Meta) -->
		<div class="mt-4">
			<div class="h-[520px] rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm overflow-hidden shadow-sm {activeTab === 'transcript' ? '' : 'hidden'}">
				<TranscriptView />
			</div>
			{#if activeTab === 'recommended'}
				<div class="rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm p-4 overflow-hidden shadow-sm">
					<RecommendedVideos />
				</div>
			{:else if activeTab === 'comments'}
				<div class="h-[520px] rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm overflow-hidden shadow-sm">
					<CommentsView />
				</div>
			{/if}
		</div>
	</div>

	<!-- Interactive Dictionary & Anki Word Popup -->
	{#if selectedWord}
		<WordPopup
			word={selectedWord}
			sentence={selectedContext || ''}
			sourceTitle={playback.now?.title || 'Video YouTube'}
			sourceUrl={playback.now?.videoId ? `https://youtube.com/watch?v=${playback.now.videoId}` : ''}
			anchorRect={selectedAnchorRect}
			onclose={() => {
				selectedWord = null;
				selectedAnchorRect = null;
			}}
			onopenAi={() => {
				selectedAiWord = selectedWord;
				selectedAiContext = selectedContext;
				selectedAiAnchorRect = selectedAnchorRect;
				selectedWord = null;
			}}
		/>
	{/if}

	<!-- AI Immersion Tutor Popup -->
	{#if selectedAiWord}
		<AiWordPopup
			word={selectedAiWord}
			sentence={selectedAiContext || ''}
			sourceTitle={playback.now?.title || 'Video YouTube'}
			sourceUrl={playback.now?.videoId ? `https://youtube.com/watch?v=${playback.now.videoId}` : ''}
			anchorRect={selectedAiAnchorRect}
			onclose={() => {
				selectedAiWord = null;
				selectedAiAnchorRect = null;
			}}
		/>
	{/if}
</div>
