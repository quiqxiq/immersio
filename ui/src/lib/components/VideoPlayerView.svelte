<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ArrowDown01Icon,
		SubtitleIcon,
		SparklesIcon,
		Comment01Icon,
		Maximize01Icon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { np, playback, prefs, ui } from '$lib/player.svelte';
	import { claimVideo, parkVideo, setHole, showVideo, video } from '$lib/video.svelte';
	import TranscriptView from './TranscriptView.svelte';
	import RecommendedVideos from './RecommendedVideos.svelte';
	import CommentsView from './CommentsView.svelte';

	let activeTab = $state<'transcript' | 'recommended' | 'comments'>('transcript');
	let videoContainer = $state<HTMLElement | null>(null);
	let currentSpeed = $state(1.0);

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

	onMount(() => {
		video.want = true;
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
		// Measure immediately and on subsequent tick to ensure settled layout
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
			class="relative aspect-video w-full max-h-[72vh] cursor-pointer overflow-hidden rounded-2xl ring-1 ring-border/20 shadow-2xl {video.hole ? '' : 'bg-black'} mx-auto"
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
					<!-- Speed Selector (0.75x, 1.0x, 1.25x) -->
					<div class="flex items-center rounded-xl bg-foreground/5 p-0.5 text-xs border border-border/40">
						{#each [0.75, 1.0, 1.25] as spd}
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
			{#if activeTab === 'transcript'}
				<div class="h-[540px] rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm overflow-hidden shadow-sm">
					<TranscriptView />
				</div>
			{:else if activeTab === 'recommended'}
				<div class="rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm p-4 overflow-hidden shadow-sm">
					<RecommendedVideos />
				</div>
			{:else if activeTab === 'comments'}
				<div class="h-[540px] rounded-2xl border border-border/60 bg-card/60 backdrop-blur-sm overflow-hidden shadow-sm">
					<CommentsView />
				</div>
			{/if}
		</div>
	</div>
</div>
