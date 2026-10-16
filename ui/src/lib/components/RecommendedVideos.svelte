<script lang="ts">
	import { onMount } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { PlayIcon, SparklesIcon, Video01Icon } from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import type { SongItem } from '$lib/api';
	import { playback, playSong } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { vocab } from '$lib/vocab.svelte';

	let recommended = $state<SongItem[]>([]);
	let loading = $state(true);

	async function loadRecommendations(title: string, videoId: string) {
		loading = true;
		try {
			// Extract primary keywords from title for recommendation
			const cleanTitle = title.replace(/\(.*?\)|\[.*?\]/g, '').trim();
			const res = await api.searchVideos(cleanTitle);
			// Filter out current video
			recommended = (res || []).filter((v) => v.video_id !== videoId);
		} catch (e) {
			recommended = [];
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const vid = playback.now?.videoId;
		const title = playback.now?.title;
		if (vid && title) {
			loadRecommendations(title, vid);
		}
	});

	function playNextVideo(v: SongItem) {
		const item: SongItem = { ...v, is_video: true };
		playSong(item);
	}
</script>

<div class="flex h-full min-h-0 flex-col bg-card">
	<div class="flex items-center justify-between border-b border-border px-5 py-3">
		<div class="flex items-center gap-2 text-foreground">
			<HugeiconsIcon icon={SparklesIcon} class="h-4 w-4 text-primary" />
			<span class="text-xs font-semibold tracking-wider uppercase">Video Terkait & Rekomendasi</span>
		</div>
	</div>

	<div class="flex-1 overflow-y-auto p-4 scrollbar-thin scrollbar-thumb-foreground/10">
		{#if loading}
			<div class="flex flex-col gap-3">
				{#each Array(6) as _, i (i)}
					<div class="flex gap-3">
						<Skeleton class="aspect-video w-32 shrink-0 rounded-lg" />
						<div class="flex flex-1 flex-col gap-2">
							<Skeleton class="h-3 w-full rounded" />
							<Skeleton class="h-3 w-2/3 rounded" />
						</div>
					</div>
				{/each}
			</div>
		{:else if recommended.length === 0}
			<div class="flex flex-col items-center justify-center gap-2 py-20 text-center text-muted-foreground">
				<HugeiconsIcon icon={Video01Icon} class="h-10 w-10 opacity-30" />
				<p class="text-xs">Tidak ada video rekomendasi lain.</p>
			</div>
		{:else}
			<div class="flex flex-col gap-2">
				{#each recommended as v}
					<div
						role="button"
						tabindex="0"
						onclick={() => playNextVideo(v)}
						onkeydown={(e) => e.key === 'Enter' && playNextVideo(v)}
						class="group flex cursor-pointer gap-3 rounded-xl p-2 transition-colors hover:bg-muted/60"
					>
						<!-- 16:9 Thumbnail with duration -->
						<div class="relative aspect-video w-32 shrink-0 overflow-hidden rounded-lg bg-muted shadow-sm">
							{#if v.thumbnail}
								<img
									src={thumb(v.thumbnail, 240)}
									alt={v.title}
									class="h-full w-full object-cover transition-transform group-hover:scale-105"
									loading="lazy"
								/>
							{:else}
								<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
									<HugeiconsIcon icon={Video01Icon} class="h-6 w-6" />
								</div>
							{/if}

							{#if v.duration}
								<span class="absolute bottom-1 right-1 rounded bg-black/80 px-1 py-0.5 text-[9px] font-semibold text-white/90">
									{v.duration}
								</span>
							{/if}

							<!-- Hover Play Icon -->
							<div class="absolute inset-0 flex items-center justify-center bg-black/40 opacity-0 transition-opacity group-hover:opacity-100">
								<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-white text-white" />
							</div>
						</div>

						<!-- Video details -->
						<div class="flex min-w-0 flex-1 flex-col justify-center gap-1">
							<h4 class="line-clamp-2 text-xs font-medium leading-snug text-foreground group-hover:text-primary">
								{v.title}
							</h4>
							<p class="truncate text-[11px] text-muted-foreground">
								{v.artists || 'YouTube Video'}
							</p>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
