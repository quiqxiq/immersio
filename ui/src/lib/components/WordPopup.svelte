<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		VolumeHighIcon,
		Tick02Icon,
		BookOpenTextIcon,
		ViewOffSlashIcon,
		Cancel01Icon,
		ExternalLinkIcon,
		SparklesIcon,
		HelpCircleIcon
	} from '@hugeicons/core-free-icons';
	import { vocab, normalizeLemma, type DictLookupResult } from '$lib/vocab.svelte';
	import * as api from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { tts } from '$lib/tts.svelte';
	import { toBody } from '$lib/menu';

	let {
		word,
		sentence = '',
		sourceTitle = '',
		sourceUrl = '',
		anchorRect = null,
		onclose,
		onopenAi
	}: {
		word: string;
		sentence?: string;
		sourceTitle?: string;
		sourceUrl?: string;
		anchorRect?: DOMRect | { left: number; top: number; right: number; bottom: number; width: number; height: number } | null;
		onclose: () => void;
		onopenAi?: () => void;
	} = $props();

	let loading = $state(true);
	let data = $state<DictLookupResult | null>(null);
	let mining = $state(false);

	const status = $derived(vocab.getStatus(word));

	// Smart Dynamic Positioning: above or below anchor element
	const popupWidth = 490;
	let posStyle = $derived.by(() => {
		if (!anchorRect) {
			return `left: 50%; transform: translateX(-50%); bottom: 4.5rem; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: min(480px, 80vh);`;
		}

		const actualWidth = Math.min(popupWidth, window.innerWidth - 24);
		const targetX = anchorRect.left + anchorRect.width / 2 - actualWidth / 2;
		const x = Math.min(Math.max(12, targetX), window.innerWidth - actualWidth - 12);

		const spaceAbove = anchorRect.top - 12;
		const spaceBelow = window.innerHeight - anchorRect.bottom - 12;

		// Determine if popup should appear above or below the anchor:
		// If space below is tight (< 280px) and there's more room above, flip above.
		// Also if space above is significantly larger and below is limited (< 340px), flip above.
		const placeAbove = spaceBelow < 280 ? (spaceAbove >= spaceBelow) : (spaceAbove > spaceBelow && spaceBelow < 340);

		if (placeAbove) {
			const bottom = window.innerHeight - anchorRect.top + 8;
			const maxHeight = Math.max(180, Math.min(spaceAbove, 490));
			return `left: ${x}px; bottom: ${bottom}px; top: auto; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: ${maxHeight}px;`;
		} else {
			const top = anchorRect.bottom + 8;
			const maxHeight = Math.max(180, Math.min(spaceBelow, 490));
			return `left: ${x}px; top: ${top}px; bottom: auto; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: ${maxHeight}px;`;
		}
	});

	async function loadDict(targetWord: string) {
		loading = true;
		try {
			data = await vocab.lookup(targetWord, sentence);
		} catch (e) {
			console.error('Dict lookup error:', e);
		} finally {
			loading = false;
		}
	}

	function playAudio() {
		if (data?.audio_url) {
			const a = new Audio(data.audio_url);
			a.play().catch(() => playSpeech());
		} else {
			playSpeech();
		}
	}

	function playSpeech() {
		tts.speakSingle(word);
	}

	async function handleMine() {
		if (mining) return;
		mining = true;
		try {
			const idTr = data?.translation_id ? `<b>${data.translation_id}</b>` : '';
			const enDef = data?.senses?.[0]?.definition
				? `<div style="font-size:0.88em;color:#666;margin-top:4px;">(${data.senses[0].pos}) ${data.senses[0].definition}</div>`
				: '';
			const translation = [idTr, enDef].filter(Boolean).join('<br>') || data?.translation_id || word;
			const primaryLemma = data?.lemma || normalizeLemma(word);
			await vocab.mineCard({
				word,
				lemma: primaryLemma,
				sentence,
				translation,
				source_title: sourceTitle,
				source_url: sourceUrl,
				audio_url: data?.audio_url
			});
		} finally {
			mining = false;
		}
	}

	function openExternalDictionary() {
		const clean = encodeURIComponent(word.toLowerCase().trim());
		let url = `https://dictionary.cambridge.org/dictionary/english/${clean}`;
		if (data?.provider === 'oxford') {
			url = `https://www.oxfordlearnersdictionaries.com/definition/english/${clean}`;
		} else if (data?.provider === 'wiktionary') {
			url = `https://en.wiktionary.org/wiki/${clean}`;
		}
		api.openExternal(url).catch((e) => console.error(e));
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
			return;
		}
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		} else if (e.key === '0' || e.key === 'u' || e.key === 'U') {
			e.preventDefault();
			vocab.setStatus(word, 'unknown');
		} else if (e.key === '1' || e.key === 'l' || e.key === 'L') {
			e.preventDefault();
			vocab.setStatus(word, 'learning');
		} else if (e.key === '2' || e.key === 'k' || e.key === 'K') {
			e.preventDefault();
			vocab.setStatus(word, 'known');
		} else if (e.key === '3' || e.key === 'i' || e.key === 'I') {
			e.preventDefault();
			vocab.setStatus(word, 'ignored');
		} else if (e.key === 'a' || e.key === 'A') {
			e.preventDefault();
			handleMine();
		}
	}

	$effect(() => {
		if (word) {
			loadDict(word);
		}
	});

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});
</script>

<!-- Transparent click-outside backdrop detector -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 z-[9998] bg-transparent"
	onclick={onclose}
	{@attach toBody}
></div>

<!-- Dynamically Positioned Floating Immersion Popup Card -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_interactive_supports_focus -->
<div
	tabindex="-1"
	style="position: fixed; z-index: 9999; {posStyle}"
	class="overflow-hidden rounded-2xl border border-border/80 bg-card/95 p-4 shadow-2xl backdrop-blur-2xl transition-all flex flex-col focus:outline-none animate-in fade-in-0 zoom-in-95 duration-150"
	role="dialog"
	aria-label="Kamus & Flashcard"
	onclick={(e) => e.stopPropagation()}
	{@attach toBody}
>
	<!-- Top Bar: Target Word, Lemma, IPA, Audio, Cambridge, Close -->
	<div class="flex items-center justify-between gap-2 border-b border-border/50 pb-2.5 shrink-0">
		<div class="flex items-baseline gap-2 flex-wrap min-w-0">
			<h2 class="text-xl font-bold tracking-tight text-primary truncate max-w-[200px]">
				{word}
			</h2>

			{#if (data?.lemma && data.lemma.toLowerCase() !== word.toLowerCase()) || (normalizeLemma(word).toLowerCase() !== word.toLowerCase())}
				{@const lemmaWord = data?.lemma || normalizeLemma(word)}
				<span
					class="rounded bg-primary/10 border border-primary/20 px-1.5 py-0.5 text-[11px] font-semibold text-primary"
					title="Bentuk dasar / lemma kata ini"
				>
					Lemma: {lemmaWord}
				</span>
			{/if}

			{#if data?.phonetic}
				<span class="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground font-mono">
					{data.phonetic}
				</span>
			{/if}

			<button
				type="button"
				onclick={playAudio}
				class="inline-flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-primary transition hover:bg-primary hover:text-primary-foreground cursor-pointer"
				title="Dengarkan Pelafalan Asli"
			>
				<HugeiconsIcon icon={VolumeHighIcon} size={13} />
			</button>
		</div>

		<div class="flex items-center gap-1.5 shrink-0">
			<!-- Provider Badge -->
			{#if data?.provider}
				<span
					class="inline-flex items-center gap-1 rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-primary capitalize"
					title={data.from_cache ? 'Diambil instan dari SQLite Cache lokal' : 'Diambil langsung secara native'}
				>
					<span>{data.provider}</span>
					{#if data.from_cache}
						<span class="text-[9px] opacity-75" title="Cached">⚡</span>
					{/if}
				</span>
			{/if}

			<!-- AI Immersion Tutor Launcher -->
			{#if onopenAi}
				<button
					type="button"
					onclick={() => {
						onclose();
						onopenAi?.();
					}}
					class="inline-flex h-6 items-center gap-1 rounded-md px-2 text-[11px] font-semibold text-purple-600 dark:text-purple-400 bg-purple-500/10 hover:bg-purple-500/20 transition cursor-pointer"
					title="Buka AI Immersion Tutor (Ctrl+Klik)"
				>
					<HugeiconsIcon icon={SparklesIcon} size={12} />
					<span>AI</span>
				</button>
			{/if}

			<!-- External Dictionary Quick Link Button -->
			<button
				type="button"
				onclick={openExternalDictionary}
				class="inline-flex h-6 items-center gap-1 rounded-md px-1.5 text-[11px] font-medium text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Buka entri web di {data?.provider || 'kamus'}"
			>
				<HugeiconsIcon icon={ExternalLinkIcon} size={12} />
			</button>

			<!-- Close Button -->
			<button
				type="button"
				onclick={onclose}
				class="inline-flex h-6 w-6 items-center justify-center rounded-full text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Tutup (Esc)"
			>
				<HugeiconsIcon icon={Cancel01Icon} size={14} />
			</button>
		</div>
	</div>

	<!-- Status Toggles & Quick Mining Bar -->
	<div class="flex items-center justify-between gap-1 py-2 border-b border-border/40 shrink-0">
		<div class="flex items-center gap-1 flex-wrap">
			<!-- Unknown (Rose) -->
			<button
				type="button"
				onclick={() => vocab.setStatus(word, 'unknown')}
				class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium transition cursor-pointer {status === 'unknown'
					? 'bg-rose-500/20 text-rose-600 dark:text-rose-400 border border-rose-500/40 shadow-xs font-semibold'
					: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
				title="Tandai Belum Tahu (0/U)"
			>
				<HugeiconsIcon icon={HelpCircleIcon} size={11} />
				<span>Unknown</span>
				<span class="text-[9px] opacity-60">0</span>
			</button>

			<!-- Learning (Amber) -->
			<button
				type="button"
				onclick={() => vocab.setStatus(word, 'learning')}
				class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium transition cursor-pointer {status === 'learning'
					? 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/40 shadow-xs font-semibold'
					: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
				title="Tandai Sedang Dipelajari (1/L)"
			>
				<HugeiconsIcon icon={BookOpenTextIcon} size={11} />
				<span>Learning</span>
				<span class="text-[9px] opacity-60">1</span>
			</button>

			<!-- Known (Emerald) -->
			<button
				type="button"
				onclick={() => vocab.setStatus(word, 'known')}
				class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium transition cursor-pointer {status === 'known'
					? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border border-emerald-500/40 shadow-xs font-semibold'
					: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
				title="Tandai Sudah Tahu (2/K)"
			>
				<HugeiconsIcon icon={Tick02Icon} size={11} />
				<span>Known</span>
				<span class="text-[9px] opacity-60">2</span>
			</button>

			<!-- Ignore (Muted) -->
			<button
				type="button"
				onclick={() => vocab.setStatus(word, 'ignored')}
				class="inline-flex items-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] font-medium transition cursor-pointer {status === 'ignored'
					? 'bg-muted text-foreground border border-border shadow-xs'
					: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
				title="Abaikan Kata (3/I)"
			>
				<HugeiconsIcon icon={ViewOffSlashIcon} size={11} />
				<span>Ignore</span>
				<span class="text-[9px] opacity-60">3</span>
			</button>
		</div>

		<!-- Quick Mine to Anki Button -->
		<Button
			size="sm"
			onclick={handleMine}
			disabled={mining}
			class="h-6 gap-1 rounded-full px-2.5 text-[11px] font-semibold shadow-xs cursor-pointer"
			title="Simpan Kartu ke Anki (Tekan A)"
		>
			<HugeiconsIcon icon={SparklesIcon} size={11} class={mining ? 'animate-spin' : ''} />
			<span>+ Anki</span>
			<span class="rounded bg-primary-foreground/20 px-1 text-[9px]">A</span>
		</Button>
	</div>

	<!-- Scrollable Content Area: Word Translation, English Dictionary, Sentence Context -->
	<div class="overflow-y-auto space-y-2.5 py-2.5 pr-1 min-h-[90px] text-xs">
		{#if loading && !data}
			<div class="space-y-2 py-1">
				<Skeleton class="h-4 w-3/4 rounded" />
				<Skeleton class="h-4 w-full rounded" />
				<Skeleton class="h-3 w-1/2 rounded" />
			</div>
		{:else}
			<!-- 1. Terjemahan Kata (Bahasa Indonesia / Target Language) -->
			{#if data?.translation_id}
				<div class="rounded-xl bg-primary/10 border border-primary/20 px-3 py-2 flex items-center justify-between gap-3">
					<div class="min-w-0 flex-1">
						<span class="text-[10px] uppercase font-bold text-primary tracking-wider block mb-0.5">
							Arti Kata ({vocab.targetLang.toUpperCase()})
						</span>
						<p class="font-semibold text-foreground text-sm truncate">
							{data.translation_id}
						</p>
					</div>
				</div>
			{/if}

			<!-- 2. Definisi Kamus Bahasa Inggris (English Dictionary / Senses) -->
			{#if data?.senses && data.senses.length > 0}
				<div class="space-y-1.5">
					<span class="text-[10px] uppercase font-bold text-muted-foreground tracking-wider block">
						Definisi Bahasa Inggris ({data?.provider || 'Dictionary'})
					</span>
					{#each data.senses as sense}
						<div class="rounded-xl border border-border/50 bg-muted/20 p-2.5 space-y-1 transition-colors hover:bg-muted/30">
							<div class="flex items-start gap-2">
								<span class="rounded bg-muted px-1.5 py-0.5 text-[9px] font-bold text-muted-foreground uppercase shrink-0 mt-0.5">
									{sense.pos}
								</span>
								<p class="text-foreground/90 font-medium leading-snug flex-1">
									{sense.definition}
								</p>
							</div>
							{#if sense.example}
								<p class="italic text-muted-foreground text-[11px] pl-2.5 border-l-2 border-primary/40 leading-relaxed">
									"{sense.example}"
								</p>
							{:else if sense.examples && sense.examples.length > 0}
								<p class="italic text-muted-foreground text-[11px] pl-2.5 border-l-2 border-primary/40 leading-relaxed">
									"{sense.examples[0]}"
								</p>
							{/if}
						</div>
					{/each}
				</div>
			{:else if !loading}
				<div class="rounded-xl border border-border/40 bg-muted/20 p-2.5 text-center text-muted-foreground">
					<p class="text-[11px]">Tidak ada definisi langsung dari {data?.provider || 'kamus'}. Klik ikon web di atas untuk leksikon online.</p>
				</div>
			{/if}

			<!-- 3. Konteks Kalimat & Terjemahannya (Sentence Context & Translation) -->
			{#if sentence}
				<div class="rounded-xl border border-border/60 bg-muted/30 p-2.5 space-y-1">
					<div class="flex items-center justify-between gap-2">
						<span class="text-[10px] uppercase font-bold text-muted-foreground tracking-wider block">
							Konteks Kalimat
						</span>
						<button
							type="button"
							onclick={() => tts.speakSingle(sentence)}
							class="inline-flex h-5 w-5 items-center justify-center rounded-full bg-primary/10 text-primary transition hover:bg-primary hover:text-primary-foreground cursor-pointer"
							title="Dengarkan Kalimat Utuh (Kokoro TTS)"
						>
							<HugeiconsIcon icon={VolumeHighIcon} size={11} />
						</button>
					</div>
					<p class="text-foreground font-medium leading-relaxed">
						"{sentence}"
					</p>
					{#if data?.sentence_translation_id}
						<p class="italic text-muted-foreground text-[11px] leading-relaxed pt-1 border-t border-border/40">
							→ {data.sentence_translation_id}
						</p>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
</div>
