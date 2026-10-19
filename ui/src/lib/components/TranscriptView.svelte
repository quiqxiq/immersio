<script lang="ts">
	import { untrack } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { SubtitleIcon } from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { playback, shadowing, videoTranscript, reconstructSentences } from '$lib/player.svelte';
	import WordPopup from '$lib/components/WordPopup.svelte';
	import AiWordPopup from '$lib/components/AiWordPopup.svelte';
	import { vocab } from '$lib/vocab.svelte';
	import { cleanWordToken } from '$lib/lemma';

	// Memory cache across component renders for 0ms instant subtitle display
	const transcriptMemoryCache = new Map<string, api.Lyrics>();

	let transcript = $state<api.Lyrics | null>(null);
	let loading = $state(true);
	let activeIndex = $state(-1);
	let scroller = $state<HTMLElement | null>(null);
	let userScrolled = $state(false);
	let scrollTimeout: any = null;
	let lastLoadedVideoId = '';

	// Selected word popup state for dictionary lookups
	let selectedWord = $state<string | null>(null);
	let selectedContext = $state<string | null>(null);
	let selectedAnchorRect = $state<DOMRect | null>(null);

	// AI Immersion Tutor lookup state
	let selectedAiWord = $state<string | null>(null);
	let selectedAiContext = $state<string | null>(null);
	let selectedAiAnchorRect = $state<DOMRect | null>(null);

	// Memoized comprehension and vocabulary stats computed only when transcript changes
	let textStats = $derived.by(() => {
		if (!transcript?.lines || transcript.lines.length === 0) return null;
		const allText = transcript.lines.map((l) => l.text).join(' ');
		return vocab.computeTextStats(allText);
	});

	async function fetchTranscript(videoId: string) {
		if (!videoId) return;

		// 1. Instant memory cache hit: 0ms UI response
		const cached = transcriptMemoryCache.get(videoId);
		if (cached && cached.lines && cached.lines.length > 0) {
			transcript = cached;
			videoTranscript.lines = cached.lines;
			videoTranscript.sentences =
				cached.sentences && cached.sentences.length > 0
					? cached.sentences
					: reconstructSentences(cached.lines);
			videoTranscript.source = cached.source || '';
			loading = false;
			videoTranscript.loading = false;
			return;
		}

		loading = true;
		videoTranscript.loading = true;
		transcript = null;
		videoTranscript.lines = [];
		videoTranscript.sentences = [];
		activeIndex = -1;
		videoTranscript.activeIndex = -1;
		videoTranscript.activeSentenceIndex = -1;
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

			if (res && res.lines && res.lines.length > 0) {
				transcript = res;
				transcriptMemoryCache.set(videoId, res);
				const lines = res.lines;
				videoTranscript.lines = lines;
				videoTranscript.sentences =
					res.sentences && res.sentences.length > 0
						? res.sentences
						: reconstructSentences(lines);
				videoTranscript.source = res.source || '';
			} else {
				transcript = null;
				videoTranscript.lines = [];
				videoTranscript.sentences = [];
			}
		} catch (e) {
			console.error('Error fetching video transcript:', e);
			transcript = null;
			videoTranscript.lines = [];
			videoTranscript.sentences = [];
		} finally {
			loading = false;
			videoTranscript.loading = false;
		}
	}

	// Watch current time and update active subtitle index & sentence index
	$effect(() => {
		const posMs = playback.position * 1000;
		if (!transcript || !transcript.lines || transcript.lines.length === 0) {
			if (activeIndex !== -1) activeIndex = -1;
			if (videoTranscript.activeIndex !== -1) videoTranscript.activeIndex = -1;
			if (videoTranscript.activeSentenceIndex !== -1) videoTranscript.activeSentenceIndex = -1;
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

		// Also update active sentence index, but lock it while waiting in shadowing mode
		if (!shadowing.isWaiting) {
			const sentences = videoTranscript.sentences;
			let foundSentence = -1;
			if (sentences && sentences.length > 0) {
				for (let s = 0; s < sentences.length; s++) {
					const st = sentences[s];
					if (posMs >= st.start_ms && posMs < st.end_ms + 100) {
						foundSentence = s;
						break;
					} else if (posMs >= st.start_ms) {
						foundSentence = s;
					}
				}
			}

			// Only mutate if index changed to avoid thrashing Svelte's reactive graph
			if (videoTranscript.activeSentenceIndex !== foundSentence) {
				videoTranscript.activeSentenceIndex = foundSentence;
			}
		}

		if (found !== activeIndex) {
			activeIndex = found;
			videoTranscript.activeIndex = found;
			// Smoothly center the active subtitle cue in the transcript list
			if (!userScrolled && found >= 0 && scroller) {
				const el = scroller.querySelector(`[data-index="${found}"]`) as HTMLElement;
				if (el) {
					const targetTop = el.offsetTop - scroller.clientHeight / 2 + el.clientHeight / 2;
					scroller.scrollTo({ top: Math.max(0, targetTop), behavior: 'smooth' });
				}
			}
		}
	});

	// Watch track change - STRICTLY guarded so pause / seek never refetches or wipes transcript
	$effect(() => {
		const vid = playback.now?.videoId;
		if (vid && vid !== lastLoadedVideoId) {
			lastLoadedVideoId = vid;
			untrack(() => {
				fetchTranscript(vid);
			});
		}
	});

	function seekTo(ms?: number | null) {
		if (ms == null) return;
		api.seek(ms / 1000);
		userScrolled = false;
		if (!playback.paused) {
			api.togglePause();
		}
	}

	function handleUserScroll() {
		userScrolled = true;
		clearTimeout(scrollTimeout);
		scrollTimeout = setTimeout(() => {
			userScrolled = false;
		}, 4000);
	}

	function handleWordClick(word: string, fullSentence: string, e: MouseEvent, timeMs?: number | null) {
		e.stopPropagation();
		const clean = cleanWordToken(word);
		if (!clean) return;

		// 1. Auto-pause video when clicking on transcript word
		if (!playback.paused) {
			api.togglePause();
		}

		// 2. Align video position to the spoken line if requested
		if (timeMs != null) {
			api.seek(timeMs / 1000);
			userScrolled = false;
		}

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
	<div class="flex items-center justify-between border-b border-border/60 px-6 py-3 flex-wrap gap-2">
		<div class="flex items-center gap-2 text-foreground">
			<HugeiconsIcon icon={SubtitleIcon} class="h-4 w-4 text-primary" />
			<span class="text-xs font-semibold tracking-wider uppercase text-muted-foreground">Transkrip Video</span>
		</div>

		{#if textStats}
			<div class="flex items-center gap-2">
				<span
					class="rounded px-2 py-0.5 text-[10px] font-bold {textStats.comprehensionPct >= 80
						? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30'
						: textStats.comprehensionPct >= 60
							? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30'
							: 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border border-rose-500/30'}"
					title="{textStats.knownWords} dari {textStats.totalWords} kata dalam transkrip sudah Anda kuasai"
				>
					{textStats.comprehensionPct}% Comprehension
				</span>
				<span class="rounded bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 text-[10px] font-medium text-rose-500 dark:text-rose-400">
					{textStats.newWordsCount} Kata Baru
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
			<!-- YouTube Subtitle Lines Mode -->
			<div class="space-y-2.5 pb-[50vh] pt-2">
				{#each transcript.lines as line, idx (idx)}
					{@const isActive = idx === activeIndex}
					{@const isPast = idx < activeIndex}
					<div
						role="button"
						tabindex="0"
						data-index={idx}
						onclick={() => seekTo(line.time_ms)}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								e.preventDefault();
								seekTo(line.time_ms);
							}
						}}
						class="block w-full origin-left cursor-pointer text-left font-heading font-semibold leading-relaxed transition-all duration-200 ease-out py-2.5 px-3 rounded-2xl text-base md:text-lg hover:text-foreground hover:opacity-100 {isActive
							? 'bg-primary/10 text-foreground font-bold opacity-100 shadow-sm border border-primary/20 scale-[1.01]'
							: isPast
								? 'text-muted-foreground opacity-35 hover:bg-foreground/5'
								: 'text-muted-foreground opacity-70 hover:bg-foreground/5'}"
					>
						<span>
							{#each line.text.split(' ') as word}
								{@const clean = cleanWordToken(word)}
								{#if clean}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<span
										role="button"
										tabindex="0"
										onclick={(e) => handleWordClick(word, line.text, e, line.time_ms)}
										class="inline-block px-0.5 rounded hover:bg-primary/20 hover:text-primary transition-colors cursor-pointer {vocab.getWordClass(clean)}"
										title="Klik untuk arti kamus"
									>
										{word}
									</span>{' '}
								{:else}
									<span>{word} </span>
								{/if}
							{/each}
						</span>
					</div>
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
