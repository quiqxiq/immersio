<script lang="ts">
	import { onMount, onDestroy, untrack } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		BookOpenTextIcon,
		SparklesIcon,
		Layers01Icon,
		TranslateIcon,
		Cancel01Icon,
		Copy01Icon,
		Tick02Icon,
		RefreshIcon,
		Settings02Icon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import { vocab } from '$lib/vocab.svelte';
	import { getPrimaryLemma, cleanWordToken } from '$lib/lemma';
	import { DEFAULT_PROMPTS, interpolatePrompt, type PromptCategory } from '$lib/prompts';
	import { copyText } from '$lib/clipboard';
	import { toast, ui } from '$lib/player.svelte';
	import { Button } from '$lib/components/ui/button';
	import { toBody } from '$lib/menu';

	let {
		word,
		sentence = '',
		sourceTitle = '',
		sourceUrl = '',
		anchorRect = null,
		onclose
	}: {
		word: string;
		sentence?: string;
		sourceTitle?: string;
		sourceUrl?: string;
		anchorRect?: DOMRect | { left: number; top: number; right: number; bottom: number; width: number; height: number } | null;
		onclose: () => void;
	} = $props();

	// 4 Icon-Only Tabs matching prompt.md
	const TABS: { id: PromptCategory; icon: any; title: string }[] = [
		{
			id: 'learning_dictionary',
			icon: BookOpenTextIcon,
			title: 'Kamus Pembelajar (Learning Dictionary)'
		},
		{
			id: 'word_explanation',
			icon: SparklesIcon,
			title: 'Nuansa Kata dalam Konteks (Word Nuance)'
		},
		{
			id: 'sentence_breakdown',
			icon: Layers01Icon,
			title: 'Bedah Struktur Kalimat (Sentence Breakdown)'
		},
		{
			id: 'contextual_translation',
			icon: TranslateIcon,
			title: 'Terjemahan Kontekstual (Contextual Translation)'
		}
	];

	let activeTab = $state<PromptCategory>('learning_dictionary');
	let hasToken = $state(true);
	let copied = $state(false);
	let mining = $state(false);

	// Concurrent per-tab cache & loading states so all 4 tabs load in parallel
	let tabCache = $state<Partial<Record<PromptCategory, { content: string; model: string; cached: boolean; error?: string }>>>({});
	let tabLoading = $state<Partial<Record<PromptCategory, boolean>>>({
		learning_dictionary: true,
		word_explanation: true,
		sentence_breakdown: true,
		contextual_translation: true
	});

	const currentTabData = $derived(tabCache[activeTab]);
	const loading = $derived(tabLoading[activeTab] ?? false);
	const content = $derived(currentTabData?.content || '');
	const modelUsed = $derived(currentTabData?.model || '');
	const fromCache = $derived(currentTabData?.cached || false);
	const errorMsg = $derived(currentTabData?.error || null);

	const cleanWord = $derived(cleanWordToken(word));
	const primaryLemma = $derived(getPrimaryLemma(cleanWord));

	// Smart dynamic popup positioning matching Limusic standard
	const popupWidth = 530;
	let posStyle = $derived.by(() => {
		if (!anchorRect) {
			return `left: 50%; transform: translateX(-50%); bottom: 4.5rem; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: min(520px, 82vh);`;
		}

		const actualWidth = Math.min(popupWidth, window.innerWidth - 24);
		const targetX = anchorRect.left + anchorRect.width / 2 - actualWidth / 2;
		const x = Math.min(Math.max(12, targetX), window.innerWidth - actualWidth - 12);

		const spaceAbove = anchorRect.top - 12;
		const spaceBelow = window.innerHeight - anchorRect.bottom - 12;
		const placeAbove = spaceBelow < 300 ? spaceAbove >= spaceBelow : spaceAbove > spaceBelow && spaceBelow < 360;

		if (placeAbove) {
			const bottom = window.innerHeight - anchorRect.top + 8;
			const maxHeight = Math.max(220, Math.min(spaceAbove, 540));
			return `left: ${x}px; bottom: ${bottom}px; top: auto; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: ${maxHeight}px;`;
		} else {
			const top = anchorRect.bottom + 8;
			const maxHeight = Math.max(220, Math.min(spaceBelow, 540));
			return `left: ${x}px; top: ${top}px; bottom: auto; width: min(${popupWidth}px, calc(100vw - 24px)); max-height: ${maxHeight}px;`;
		}
	});

	async function loadSingleTab(category: PromptCategory, force = false) {
		if (!force && tabCache[category] && !tabCache[category]?.error) {
			tabLoading[category] = false;
			return;
		}

		tabLoading[category] = true;

		try {
			const config = DEFAULT_PROMPTS[category];
			const vars = {
				word: cleanWord,
				sentence: sentence || cleanWord,
				targetLang: vocab.targetLang || 'id',
				interfaceLang: 'Indonesian'
			};

			const sysPrompt = interpolatePrompt(config.systemPrompt, vars);
			const usrPrompt = interpolatePrompt(config.userPrompt, vars);

			const res = await api.aiExplainWord(
				cleanWord,
				sentence,
				category,
				sysPrompt,
				usrPrompt
			);

			tabCache[category] = {
				content: res.content,
				model: res.model_used,
				cached: res.from_cache
			};
		} catch (e: any) {
			const msg = e?.message || String(e);
			tabCache[category] = {
				content: '',
				model: '',
				cached: false,
				error: msg
			};
			if (msg.includes('Token OpenRouter belum diatur') || msg.includes('401')) {
				hasToken = false;
			}
		} finally {
			tabLoading[category] = false;
		}
	}

	async function loadAllTabs(force = false) {
		const token = await api.getOpenRouterToken();
		if (!token || !token.trim()) {
			hasToken = false;
			TABS.forEach((t) => (tabLoading[t.id] = false));
			return;
		}
		hasToken = true;

		// Mark tabs as loading
		TABS.forEach((t) => {
			if (force || !tabCache[t.id] || tabCache[t.id]?.error) {
				tabLoading[t.id] = true;
			}
		});

		// Trigger all 4 tabs concurrently in parallel!
		await Promise.allSettled(TABS.map((t) => loadSingleTab(t.id, force)));
	}

	function switchTab(cat: PromptCategory) {
		activeTab = cat;
	}

	async function handleCopy() {
		if (!content) return;
		await copyText(content);
		copied = true;
		toast('Penjelasan AI disalin ke papan klip');
		setTimeout(() => {
			copied = false;
		}, 2000);
	}

	async function handleMine() {
		if (mining || !content) return;
		mining = true;
		try {
			// Format clean AI translation for Anki card
			const formattedContent = content
				.replace(/\n\n/g, '<br><br>')
				.replace(/\n/g, '<br>')
				.replace(/\*\*(.*?)\*\*/g, '<b>$1</b>');

			await vocab.mineCard({
				word: cleanWord,
				lemma: primaryLemma || cleanWord,
				sentence,
				translation: formattedContent,
				source_title: sourceTitle,
				source_url: sourceUrl
			});
		} finally {
			mining = false;
		}
	}

	function openSettingsImmersion() {
		onclose();
		ui.settingsFocus = 'immersion';
		ui.settingsOpen = true;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
			return;
		}
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		} else if (e.key === '1') {
			switchTab('learning_dictionary');
		} else if (e.key === '2') {
			switchTab('word_explanation');
		} else if (e.key === '3') {
			switchTab('sentence_breakdown');
		} else if (e.key === '4') {
			switchTab('contextual_translation');
		} else if (e.key === 'a' || e.key === 'A') {
			e.preventDefault();
			handleMine();
		}
	}

	let lastLoadedWord = '';
	$effect(() => {
		const currentWord = word;
		untrack(() => {
			if (currentWord && currentWord !== lastLoadedWord) {
				lastLoadedWord = currentWord;
				tabCache = {};
				loadAllTabs();
			}
		});
	});

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
	});

	// Safe, lightweight markdown renderer for clean AI outputs
	function renderMarkdown(md: string): string {
		if (!md) return '';
		return md
			// Escape html tags to avoid XSS
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			// Headers
			.replace(/^### (.*$)/gim, '<h4 class="text-xs font-bold text-primary tracking-wide uppercase mt-3 mb-1.5">$1</h4>')
			.replace(/^## (.*$)/gim, '<h3 class="text-sm font-bold text-foreground mt-3 mb-1.5">$1</h3>')
			.replace(/^# (.*$)/gim, '<h2 class="text-base font-bold text-primary mt-2 mb-1">$1</h2>')
			// Bold and italics
			.replace(/\*\*(.*?)\*\*/g, '<strong class="font-bold text-foreground">$1</strong>')
			.replace(/\*(.*?)\*/g, '<em class="italic text-foreground/90">$1</em>')
			// Inline code / keywords
			.replace(/`([^`]+)`/g, '<code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px] text-primary">$1</code>')
			// Bullet points with primary styled bullets
			.replace(/^[•\-\*] (.*$)/gim, '<li class="flex items-start gap-1.5 my-1 text-xs text-foreground/90"><span class="text-primary font-bold select-none">•</span><span>$1</span></li>')
			// Paragraph breaks
			.replace(/\n\n/g, '<div class="h-2"></div>')
			.replace(/\n/g, '<br/>');
	}
</script>

<!-- Backdrop click outside detector -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 z-[9998] bg-transparent"
	onclick={onclose}
	{@attach toBody}
></div>

<!-- Floating AI Immersion Card -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_interactive_supports_focus -->
<div
	tabindex="-1"
	style="position: fixed; z-index: 9999; {posStyle}"
	class="overflow-hidden rounded-2xl border border-primary/30 bg-card/95 p-4 shadow-2xl backdrop-blur-2xl transition-all flex flex-col focus:outline-none animate-in fade-in-0 zoom-in-95 duration-150"
	role="dialog"
	aria-label="AI Immersion Tutor"
	onclick={(e) => e.stopPropagation()}
	{@attach toBody}
>
	<!-- Top Bar: Target Word + Lemma, 4 Icon-Only Tabs with Tooltips, Close Button -->
	<div class="flex items-center justify-between gap-2 border-b border-border/50 pb-2.5 shrink-0">
		<!-- Left: Target Word & Lemma -->
		<div class="flex items-baseline gap-2 min-w-0">
			<h2 class="text-lg font-bold tracking-tight text-primary truncate max-w-[170px]" title={cleanWord}>
				{cleanWord}
			</h2>

			{#if primaryLemma && primaryLemma.toLowerCase() !== cleanWord.toLowerCase()}
				<span
					class="rounded bg-primary/10 border border-primary/20 px-1.5 py-0.5 text-[10px] font-semibold text-primary"
					title="Bentuk dasar / lemma kata ini"
				>
					Lemma: {primaryLemma}
				</span>
			{/if}
		</div>

		<!-- Center: 4 Icon-Only Tabs with Tooltip & Live Loading Indicator -->
		<div class="flex items-center gap-1 rounded-xl bg-muted/50 p-1 border border-border/50 shrink-0">
			{#each TABS as tab}
				{@const isActive = activeTab === tab.id}
				{@const isTabLoading = tabLoading[tab.id]}
				{@const hasError = !!tabCache[tab.id]?.error}
				{@const isReady = !isTabLoading && !!tabCache[tab.id]?.content}
				<button
					type="button"
					onclick={() => switchTab(tab.id)}
					title="{tab.title}{isTabLoading ? ' (Sedang diproses...)' : isReady ? ' (Siap)' : ''}"
					aria-label={tab.title}
					class="relative inline-flex h-7 w-7 items-center justify-center rounded-lg transition cursor-pointer {isActive
						? 'bg-primary text-primary-foreground shadow-xs'
						: 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				>
					<HugeiconsIcon icon={tab.icon} size={15} />

					<!-- Status dot indicator for parallel background processing -->
					{#if isTabLoading}
						<span class="absolute -top-0.5 -right-0.5 flex h-2 w-2">
							<span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-purple-400 opacity-75"></span>
							<span class="relative inline-flex rounded-full h-2 w-2 bg-purple-500"></span>
						</span>
					{:else if hasError}
						<span class="absolute -top-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-destructive" title="Gagal memuat"></span>
					{:else if isReady && !isActive}
						<span class="absolute -top-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-emerald-500/80" title="Siap dibaca"></span>
					{/if}
				</button>
			{/each}
		</div>

		<!-- Right: Close Button -->
		<button
			type="button"
			onclick={onclose}
			class="inline-flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground hover:bg-muted hover:text-foreground transition cursor-pointer shrink-0"
			title="Tutup (Esc)"
		>
			<HugeiconsIcon icon={Cancel01Icon} size={14} />
		</button>
	</div>

	<!-- Body Content -->
	<div class="flex-1 overflow-y-auto my-3 pr-1 min-h-[140px] max-h-[380px] scrollbar-thin text-xs text-foreground leading-relaxed select-text">
		{#if !hasToken}
			<!-- Missing Token State -->
			<div class="flex flex-col items-center justify-center gap-3 py-6 px-3 text-center">
				<div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10 text-primary">
					<HugeiconsIcon icon={Settings02Icon} size={20} />
				</div>
				<div class="space-y-1 max-w-sm">
					<h3 class="text-sm font-bold text-foreground">Token OpenRouter Belum Diatur</h3>
					<p class="text-xs text-muted-foreground leading-relaxed">
						Untuk menggunakan fitur AI Tutor (Ctrl + Klik), cukup tempelkan API Key OpenRouter gratis Anda di menu Pengaturan. Model gratis terbaik dan fallback otomatis dikelola oleh Limusic.
					</p>
				</div>
				<Button
					size="sm"
					onclick={openSettingsImmersion}
					class="gap-1.5 mt-1 text-xs cursor-pointer"
				>
					<HugeiconsIcon icon={Settings02Icon} size={14} />
					<span>Buka Pengaturan & Tempel Token</span>
				</Button>
			</div>
		{:else if loading}
			<!-- Animated Skeleton Loading State -->
			<div class="space-y-3 py-3 animate-pulse">
				<div class="flex items-center gap-2 text-muted-foreground text-xs font-medium">
					<HugeiconsIcon icon={RefreshIcon} size={13} class="animate-spin text-primary" />
					<span>Menganalisis dengan AI (multi-model fallback)...</span>
				</div>
				<div class="h-4 bg-muted rounded w-2/3"></div>
				<div class="h-14 bg-muted/60 rounded w-full"></div>
				<div class="h-4 bg-muted rounded w-4/5"></div>
				<div class="h-10 bg-muted/60 rounded w-full"></div>
			</div>
		{:else if errorMsg}
			<!-- Error State -->
			<div class="space-y-3 py-4 text-center">
				<p class="text-xs text-destructive font-medium leading-relaxed">
					{errorMsg}
				</p>
				<Button
					variant="outline"
					size="sm"
					onclick={() => loadSingleTab(activeTab, true)}
					class="gap-1.5 text-xs cursor-pointer"
				>
					<HugeiconsIcon icon={RefreshIcon} size={13} />
					<span>Coba Lagi</span>
				</Button>
			</div>
		{:else}
			<!-- Clean Markdown Rendered Content -->
			<div class="ai-content space-y-1">
				{@html renderMarkdown(content)}
			</div>
		{/if}
	</div>

	<!-- Footer Bar: Model Used Badge, Copy Button, Mine to Anki Button -->
	<div class="flex items-center justify-between border-t border-border/50 pt-2 shrink-0 text-[11px] text-muted-foreground">
		<!-- Left: Model info -->
		<div class="flex items-center gap-1.5">
			{#if modelUsed}
				<span
					class="inline-flex items-center gap-1 rounded bg-primary/10 px-2 py-0.5 text-[10px] font-semibold text-primary"
					title={fromCache ? 'Dimuat instan dari cache lokal SQLite' : `Merespon dari model ${modelUsed}`}
				>
					<span>⚡ {modelUsed.split('/').pop()?.replace(':free', '') || modelUsed}</span>
					{#if fromCache}
						<span class="text-[9px] opacity-75">• Cached</span>
					{/if}
				</span>
			{/if}
		</div>

		<!-- Right: Action Buttons -->
		<div class="flex items-center gap-1.5">
			<button
				type="button"
				onclick={handleCopy}
				disabled={loading || !content}
				class="inline-flex h-7 items-center gap-1 rounded-lg px-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition cursor-pointer disabled:opacity-40"
				title="Salin penjelasan ini ke clipboard"
			>
				<HugeiconsIcon icon={copied ? Tick02Icon : Copy01Icon} size={13} class={copied ? 'text-emerald-500' : ''} />
				<span>{copied ? 'Tersalin' : 'Salin'}</span>
			</button>

			<Button
				size="sm"
				variant="default"
				onclick={handleMine}
				disabled={loading || !content || mining}
				class="h-7 text-xs gap-1.5 px-2.5 rounded-lg cursor-pointer"
				title="Simpan penjelasan AI ini ke Flashcard Anki (Shortcut: A)"
			>
				<HugeiconsIcon icon={SparklesIcon} size={13} />
				<span>{mining ? 'Menyimpan...' : 'Simpan ke Anki'}</span>
			</Button>
		</div>
	</div>
</div>
