<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { SubtitleIcon } from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { playback, shadowing, videoTranscript } from '$lib/player.svelte';
	import WordPopup from '$lib/components/WordPopup.svelte';
	import AiWordPopup from '$lib/components/AiWordPopup.svelte';
	import { vocab } from '$lib/vocab.svelte';
	import { cleanWordToken } from '$lib/lemma';

	let transcript = $state<api.Lyrics | null>(null);
	let loading = $state(true);
	let activeIndex = $state(-1);
	let scroller = $state<HTMLElement | null>(null);
	let userScrolled = $state(false);
	let scrollTimeout: any = null;

	// Selected word popup state for dictionary lookups
	let selectedWord = $state<string | null>(null);
	let selectedContext = $state<string | null>(null);
	let selectedAnchorRect = $state<DOMRect | null>(null);

	// AI Immersion Tutor lookup state
	let selectedAiWord = $state<string | null>(null);
	let selectedAiContext = $state<string | null>(null);
	let selectedAiAnchorRect = $state<DOMRect | null>(null);

	async function fetchTranscript(videoId: string) {
		loading = true;
		videoTranscript.loading = true;
		transcript = null;
		videoTranscript.lines = [];
		activeIndex = -1;
		videoTranscript.activeIndex = -1;
		userScrolled = false;
		try {
			let res: api.Lyrics | null = null;
			// 1. First try YouTube Subtitles explicitly
			try {
				res = await api.getLyrics({
					videoId,
					title: playback.now?.title || '',
					artists: playback.now?.artists || '',
					album: playback.now?.album || undefined,
					duration: playback.duration > 0 ? playback.duration : undefined,
					source: 'youtube'
				});
			} catch (err) {
				console.warn('YouTube subtitles explicit query failed, trying provider chain', err);
			}

			// 2. If no lines found, try standard provider chain
			if (!res || !res.lines || res.lines.length === 0) {
				res = await api.getLyrics({
					videoId,
					title: playback.now?.title || '',
					artists: playback.now?.artists || '',
					album: playback.now?.album || undefined,
					duration: playback.duration > 0 ? playback.duration : undefined
				});
			}
			transcript = res;
			videoTranscript.lines = res?.lines || [];
			videoTranscript.source = res?.source || '';
		} catch (e) {
			console.error('Error fetching video transcript:', e);
			transcript = null;
			videoTranscript.lines = [];
		} finally {
			loading = false;
			videoTranscript.loading = false;
		}
	}

	// Watch current time and update active subtitle index without moving the outer page
	$effect(() => {
		const posMs = playback.position * 1000;
		if (!transcript || !transcript.lines || transcript.lines.length === 0) {
			activeIndex = -1;
			videoTranscript.activeIndex = -1;
			return;
		}

		const lines = transcript.lines;
		let found = -1;
		for (let i = 0; i < lines.length; i++) {
			const t = lines[i].time_ms ?? 0;
			if (posMs >= t) {
				found = i;
			} else {
				break;
			}
		}

		if (found !== activeIndex) {
			activeIndex = found;
			videoTranscript.activeIndex = found;
			// Strictly prevent auto-scrolling when in Shadowing mode or when paused.
			// Only scroll the local container itself (never window or outer document).
			if (!shadowing.active && !playback.paused && !userScrolled && found >= 0 && scroller) {
				const el = scroller.querySelector(`[data-index="${found}"]`) as HTMLElement;
				if (el) {
					const targetTop = el.offsetTop - scroller.clientHeight / 2;
					scroller.scrollTo({ top: Math.max(0, targetTop), behavior: 'smooth' });
				}
			}
		}
	});

	// Watch track change
	$effect(() => {
		const vid = playback.now?.videoId;
		if (vid) {
			fetchTranscript(vid);
		}
	});

	function seekTo(ms?: number | null) {
		if (ms == null) return;
		api.seek(ms / 1000);
		userScrolled = false;
	}

	function handleUserScroll() {
		userScrolled = true;
		clearTimeout(scrollTimeout);
		scrollTimeout = setTimeout(() => {
			userScrolled = false;
		}, 4000);
	}

	function handleWordClick(word: string, fullSentence: string, e: MouseEvent) {
		e.stopPropagation();
		const clean = cleanWordToken(word);
		if (!clean) return;
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		if (e.ctrlKey || e.metaKey) {
			selectedWord = null;
			selectedAiWord = clean;
			selectedAiContext = fullSentence;
			selectedAiAnchorRect = rect;
			return;
		}
		selectedAiWord = null;
		selectedWord = clean;
		selectedContext = fullSentence;
		selectedAnchorRect = rect;
	}
</script>

<div class="flex h-full min-h-0 flex-col bg-card">
	<!-- Minimalist Top Bar -->
	<div class="flex items-center justify-between border-b border-border/60 px-6 py-3">
		<div class="flex items-center gap-2 text-foreground">
			<HugeiconsIcon icon={SubtitleIcon} class="h-4 w-4 text-primary" />
			<span class="text-xs font-semibold tracking-wider uppercase text-muted-foreground">Transkrip Video</span>
		</div>
		{#if transcript?.lines && transcript.lines.length > 0}
			{@const allText = transcript.lines.map((l) => l.text).join(' ')}
			{@const stats = vocab.computeTextStats(allText)}
			<div class="flex items-center gap-2">
				<span
					class="rounded px-2 py-0.5 text-[10px] font-bold {stats.comprehensionPct >= 80
						? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30'
						: stats.comprehensionPct >= 60
							? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30'
							: 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border border-rose-500/30'}"
					title="{stats.knownWords} dari {stats.totalWords} kata dalam transkrip sudah Anda kuasai"
				>
					{stats.comprehensionPct}% Comprehension
				</span>
				<span class="rounded bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 text-[10px] font-medium text-rose-500 dark:text-rose-400">
					{stats.newWordsCount} Kata Baru
				</span>
			</div>
		{:else if transcript?.source}
			<span class="rounded-md bg-foreground/5 px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
				{transcript.source}
			</span>
		{/if}
	</div>

	<!-- Transcript Content in Music Lyrics Style -->
	<div
		bind:this={scroller}
		onscroll={handleUserScroll}
		class="min-h-0 flex-1 overflow-y-auto px-6 py-6 scrollbar-thin scrollbar-thumb-foreground/10"
	>
		{#if loading}
			<!-- Skeleton placeholder matching music lyrics lines -->
			<div class="space-y-4 pt-4">
				{#each { length: 8 } as _, i}
					<div
						class="h-6 animate-pulse rounded-lg bg-foreground/5"
						style="width: {50 + ((i * 19) % 45)}%"
					></div>
				{/each}
			</div>
		{:else if !transcript || !transcript.lines || transcript.lines.length === 0}
			<div class="flex flex-col items-center justify-center gap-3 py-24 text-center text-muted-foreground">
				<HugeiconsIcon icon={SubtitleIcon} class="h-10 w-10 opacity-30" />
				<p class="text-sm font-medium">Tidak ada transkrip atau subtitel untuk video ini.</p>
				<p class="text-xs text-muted-foreground/70">Pastikan video memiliki Closed Captions (CC) di YouTube.</p>
			</div>
		{:else}
			<!-- Bottom padding allows the final lines to center-scroll smoothly like lyrics -->
			<div class="space-y-2 pb-[50vh] pt-2">
				{#each transcript.lines as line, idx (idx)}
					{@const isActive = idx === activeIndex}
					{@const isPast = idx < activeIndex}
					<button
						data-index={idx}
						type="button"
						onclick={() => seekTo(line.time_ms)}
						class="block w-full origin-left cursor-pointer text-left font-heading font-semibold leading-relaxed transition-[color,opacity,transform] duration-300 ease-out py-2 text-base md:text-lg hover:text-foreground hover:opacity-100 {isActive
							? 'scale-[1.03] text-foreground font-bold opacity-100'
							: isPast
								? 'text-muted-foreground opacity-35'
								: 'text-muted-foreground opacity-65'}"
					>
						<span>
							{#each line.text.split(' ') as word}
								{@const clean = cleanWordToken(word)}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<span
									role="button"
									tabindex="0"
									onclick={(e) => handleWordClick(word, line.text, e)}
									class="inline-block px-0.5 {vocab.getWordClass(clean)}"
								>
									{word}{' '}
								</span>
							{/each}
						</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Universal Immersion Word Popup -->
	{#if selectedWord}
		<WordPopup
			word={selectedWord}
			sentence={selectedContext || ''}
			sourceTitle={playback.now?.title || 'Video YouTube'}
			sourceUrl={playback.now?.videoId ? `https://youtube.com/watch?v=${playback.now.videoId}` : ''}
			anchorRect={selectedAnchorRect}
			onclose={() => (selectedWord = null)}
			onopenAi={() => {
				selectedAiWord = selectedWord;
				selectedAiContext = selectedContext;
				selectedAiAnchorRect = selectedAnchorRect;
			}}
		/>
	{/if}

	<!-- AI Immersion Tutor Popup (Ctrl + Click) -->
	{#if selectedAiWord}
		<AiWordPopup
			word={selectedAiWord}
			sentence={selectedAiContext || ''}
			sourceTitle={playback.now?.title || 'Video YouTube'}
			sourceUrl={playback.now?.videoId ? `https://youtube.com/watch?v=${playback.now.videoId}` : ''}
			anchorRect={selectedAiAnchorRect}
			onclose={() => (selectedAiWord = null)}
		/>
	{/if}
</div>
