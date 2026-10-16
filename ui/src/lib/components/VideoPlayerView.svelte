<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { fade, fly } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ArrowDown01Icon,
		SubtitleIcon,
		SparklesIcon,
		Comment01Icon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { np, playback, prefs, ui } from '$lib/player.svelte';
	import { claimVideo, parkVideo, setHole, showVideo, video } from '$lib/video.svelte';
	import TranscriptView from './TranscriptView.svelte';
	import RecommendedVideos from './RecommendedVideos.svelte';
	import CommentsView from './CommentsView.svelte';

	let activeTab = $state<'transcript' | 'recommended' | 'comments'>('transcript');
	let videoContainer = $state<HTMLElement | null>(null);

	function toggleFullscreen() {
		if (!videoContainer) return;
		if (!document.fullscreenElement) {
			videoContainer.requestFullscreen().catch(() => {});
		} else {
			document.exitFullscreen().catch(() => {});
		}
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
		// Measure immediately and on subsequent tick to ensure settled layout
		requestAnimationFrame(measure);
		const timer = setTimeout(measure, 60);
		return () => {
			ro.disconnect();
			clearTimeout(timer);
			document.removeEventListener('visibilitychange', measure);
			window.removeEventListener('resize', measure);
			setHole(null);
		};
	}
</script>

<div
	transition:fade={{ duration: 180 }}
	onoutrostart={() => setHole(null)}
	class="absolute inset-y-0 left-16 right-0 z-20 flex flex-col overflow-hidden {ui.sidebarCollapsed
		? ''
		: 'lg:left-60'} {video.hole ? '' : 'bg-background'} text-foreground select-none"
>
	<!-- Top Navigation Bar -->
	<header class="flex h-14 shrink-0 items-center justify-between border-b border-border bg-card px-6">
		<div class="flex items-center gap-4">
			<button
				type="button"
				onclick={() => {
					setHole(null);
					np.open = false;
				}}
				class="flex h-9 w-9 items-center justify-center rounded-xl bg-foreground/5 text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				aria-label="Tutup player"
			>
				<HugeiconsIcon icon={ArrowDown01Icon} class="h-5 w-5" />
			</button>

			<div class="flex items-center gap-2">
				<span class="rounded-lg bg-primary/10 px-2.5 py-1 text-xs font-semibold text-primary border border-primary/20">
					Mode Video
				</span>
				<div class="flex flex-col">
					<h2 class="max-w-md truncate text-sm font-bold text-foreground">
						{playback.now?.title || 'Memutar Video'}
					</h2>
					<p class="truncate text-xs text-muted-foreground">
						{playback.now?.artists || 'YouTube'}
					</p>
				</div>
			</div>
		</div>
	</header>

	<!-- Main Workspace: Video on Left, Tabs on Right -->
	<div class="flex min-h-0 flex-1 flex-col md:flex-row">
		<!-- Left: Video Screen Stage (16:9) -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			bind:this={videoContainer}
			class="relative flex flex-1 flex-col items-center justify-center p-4 md:p-8"
		>
			<!-- Video Frame: click to pause/play, double click for fullscreen -->
			<div
				role="button"
				tabindex="0"
				onclick={() => api.togglePause()}
				ondblclick={toggleFullscreen}
				class="relative aspect-video w-full max-h-[82vh] cursor-pointer overflow-hidden rounded-2xl ring-1 ring-border/20 {video.hole ? '' : 'bg-black'}"
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
		</div>

		<!-- Right: Dedicated Video Tabs (Transkrip, Rekomendasi, Komentar) -->
		<aside class="flex w-full flex-col border-l border-border bg-card md:w-[24rem] lg:w-[28rem]">
			<!-- Tab Switcher -->
			<div class="flex border-b border-border p-2 gap-1">
				<button
					type="button"
					onclick={() => (activeTab = 'transcript')}
					class="flex flex-1 items-center justify-center gap-1.5 rounded-xl py-2 text-xs font-semibold transition-all {activeTab ===
					'transcript'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				>
					<HugeiconsIcon icon={SubtitleIcon} class="h-4 w-4" />
					Transkrip
				</button>
				<button
					type="button"
					onclick={() => (activeTab = 'recommended')}
					class="flex flex-1 items-center justify-center gap-1.5 rounded-xl py-2 text-xs font-semibold transition-all {activeTab ===
					'recommended'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				>
					<HugeiconsIcon icon={SparklesIcon} class="h-4 w-4" />
					Rekomendasi
				</button>
				<button
					type="button"
					onclick={() => (activeTab = 'comments')}
					class="flex flex-1 items-center justify-center gap-1.5 rounded-xl py-2 text-xs font-semibold transition-all {activeTab ===
					'comments'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				>
					<HugeiconsIcon icon={Comment01Icon} class="h-4 w-4" />
					Komentar
				</button>
			</div>

			<!-- Tab Contents -->
			<div class="flex-1 overflow-hidden">
				{#if activeTab === 'transcript'}
					<TranscriptView />
				{:else if activeTab === 'recommended'}
					<RecommendedVideos />
				{:else if activeTab === 'comments'}
					<CommentsView />
				{/if}
			</div>
		</aside>
	</div>
</div>
