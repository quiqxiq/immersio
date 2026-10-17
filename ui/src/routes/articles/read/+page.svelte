<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ArrowLeft01Icon,
		ExternalLinkIcon,
		CopyLinkIcon,
		Copy01Icon,
		BookOpenTextIcon,
		VolumeHighIcon,
		Cancel01Icon,
		ArrowReloadHorizontalIcon,
		HeadphonesIcon,
		PlayIcon,
		PauseIcon,
		SparklesIcon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import type { ArticleContent } from '$lib/api';
	import { copyText } from '$lib/clipboard';
	import { toast } from '$lib/player.svelte';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import WordPopup from '$lib/components/WordPopup.svelte';
	import AiWordPopup from '$lib/components/AiWordPopup.svelte';
	import ArticleTtsBar from '$lib/components/ArticleTtsBar.svelte';
	import { vocab } from '$lib/vocab.svelte';
	import { cleanWordToken } from '$lib/lemma';
	import { tts } from '$lib/tts.svelte';

	let loading = $state(true);
	let article = $state<ArticleContent | null>(null);
	let processedHtml = $state('');
	let articleSentences = $state<string[]>([]);
	let error = $state<string | null>(null);

	// Reading customization preferences
	let fontSize = $state<'normal' | 'large' | 'xlarge'>('large');
	let fontSerif = $state(true);

	// Word Immersion / Dictionary lookup state
	let selectedWord = $state<string | null>(null);
	let selectedSentence = $state<string>('');
	let selectedAnchorRect = $state<DOMRect | null>(null);

	// AI Immersion Tutor lookup state
	let selectedAiWord = $state<string | null>(null);
	let selectedAiSentence = $state<string>('');
	let selectedAiAnchorRect = $state<DOMRect | null>(null);

	const targetUrl = $derived(page.url.searchParams.get('url') || '');
	const fallbackTitle = $derived(page.url.searchParams.get('title') || '');
	const fallbackSource = $derived(page.url.searchParams.get('source') || '');

	/**
	 * Transform article HTML into interactive sentence and word tokens
	 * so every sentence and word can be hovered with background highlight
	 * matching transcript / lyrics experience.
	 */
	function enhanceHtmlWithTokens(rawHtml: string): string {
		if (!rawHtml) return '';
		try {
			const parser = new DOMParser();
			const doc = parser.parseFromString(rawHtml, 'text/html');

			const containers = doc.querySelectorAll('p, li, blockquote, h1, h2, h3, h4');
			let sentenceCounter = 0;
			const collected: string[] = [];

			containers.forEach((container) => {
				const walker = doc.createTreeWalker(container, NodeFilter.SHOW_TEXT);
				const textNodes: Text[] = [];
				let node = walker.nextNode();
				while (node) {
					if (node.nodeValue && node.nodeValue.trim().length > 0) {
						textNodes.push(node as Text);
					}
					node = walker.nextNode();
				}

				for (const textNode of textNodes) {
					const parent = textNode.parentNode;
					if (!parent || parent.nodeName === 'SCRIPT' || parent.nodeName === 'STYLE') continue;

					const text = textNode.nodeValue || '';
					// Split into sentences using punctuation boundaries
					const sentenceRegex = /([^.!?]+[.!?]+(?:\s+|$)|[^.!?]+$)/g;
					const sentences = text.match(sentenceRegex) || [text];

					const frag = doc.createDocumentFragment();

					for (const sentence of sentences) {
						if (!sentence.trim()) {
							frag.appendChild(doc.createTextNode(sentence));
							continue;
						}

						const curIdx = sentenceCounter++;
						collected.push(sentence.trim());

						const sentenceSpan = doc.createElement('span');
						sentenceSpan.className = 'reader-sentence';
						sentenceSpan.setAttribute('data-sentence-index', String(curIdx));

						// Split sentence into words and whitespace
						const words = sentence.split(/(\s+)/);
						for (const token of words) {
							if (!token) continue;
							if (/^\s+$/.test(token)) {
								sentenceSpan.appendChild(doc.createTextNode(token));
							} else {
								const wordSpan = doc.createElement('span');
								const clean = cleanWordToken(token);
								if (clean) {
									wordSpan.setAttribute('data-word', clean);
									const st = vocab.getStatus(clean);
									wordSpan.className = `reader-word status-${st}`;
								} else {
									wordSpan.className = 'reader-word';
								}
								wordSpan.textContent = token;
								sentenceSpan.appendChild(wordSpan);
							}
						}
						frag.appendChild(sentenceSpan);
					}

					parent.replaceChild(frag, textNode);
				}
			});

			articleSentences = collected;
			return doc.body.innerHTML;
		} catch (e) {
			console.error('Failed to enhance HTML with reader tokens:', e);
			return rawHtml;
		}
	}

	async function loadArticle() {
		if (!targetUrl) {
			error = 'URL artikel tidak ditemukan.';
			loading = false;
			return;
		}

		loading = true;
		error = null;
		selectedWord = null;

		try {
			const res = await api.extractArticle(targetUrl);
			article = res;
			processedHtml = enhanceHtmlWithTokens(res.html);
		} catch (e: any) {
			console.error('Failed to extract article:', e);
			error = e?.message || 'Gagal mengekstrak isi artikel. Website mungkin memblokir akses atau memerlukan login.';
		} finally {
			loading = false;
		}
	}

	function handleBack() {
		if (window.history.length > 1) {
			window.history.back();
		} else {
			goto('/articles');
		}
	}

	async function copyArticleLink() {
		if (!targetUrl) return;
		try {
			await copyText(targetUrl);
			toast('Tautan artikel disalin ke clipboard');
		} catch {
			// ignore
		}
	}

	async function copyMarkdown() {
		if (!article?.markdown) {
			toast('Konten Markdown belum tersedia');
			return;
		}
		try {
			await copyText(article.markdown);
			toast('Artikel disalin dalam format Markdown (Obsidian / Notion)');
		} catch {
			toast('Gagal menyalin Markdown');
		}
	}

	function openInBrowser() {
		if (!targetUrl) return;
		api.openExternal(targetUrl).catch((e) => console.error(e));
	}

	function speakWord(word: string) {
		tts.speakSingle(word);
	}

	function openCambridgeDictionary(word: string) {
		const dictUrl = `https://dictionary.cambridge.org/dictionary/english/${encodeURIComponent(word.toLowerCase())}`;
		api.openExternal(dictUrl).catch((e) => console.error(e));
	}

	function handleContentClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (!target) return;

		// Check if user clicked an interactive word
		const wordEl = target.closest('.reader-word') as HTMLElement | null;
		if (wordEl) {
			const word = wordEl.getAttribute('data-word');
			if (word && word.length > 0) {
				const sentEl = wordEl.closest('.reader-sentence') || wordEl.closest('p') || wordEl.closest('li');
				const sentText = sentEl?.textContent?.trim() || '';
				const rect = wordEl.getBoundingClientRect();

				if (e.ctrlKey || e.metaKey) {
					// Ctrl + Click -> Open AI Immersion Popup
					selectedWord = null;
					selectedAiWord = word;
					selectedAiSentence = sentText;
					selectedAiAnchorRect = rect;
					return;
				}

				selectedAiWord = null;
				selectedWord = word;
				selectedSentence = sentText;
				selectedAnchorRect = rect;
				return;
			}
		}

		// Check if user clicked a sentence directly to play TTS
		const sentEl = target.closest('.reader-sentence') as HTMLElement | null;
		if (sentEl && !wordEl && !target.closest('a')) {
			const idxStr = sentEl.getAttribute('data-sentence-index');
			if (idxStr !== null) {
				const idx = parseInt(idxStr, 10);
				if (!isNaN(idx) && articleSentences.length > 0) {
					tts.playArticle(articleSentences, idx);
					return;
				}
			}
		}

		// Handle links inside article
		const linkEl = target.closest('a') as HTMLAnchorElement | null;
		if (linkEl) {
			const href = linkEl.getAttribute('href');
			if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
				e.preventDefault();
				api.openExternal(href).catch((err) => console.error(err));
			}
		}
	}

	function handleMouseUp(e: MouseEvent) {
		const selection = window.getSelection();
		const text = selection?.toString().trim();
		if (text && text.length > 1 && text.length < 40 && !text.includes('\n')) {
			const clean = text.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, '').toLowerCase();
			if (clean.length > 1) {
				const anchor = selection?.anchorNode?.parentElement;
				const sentEl = anchor?.closest('.reader-sentence') || anchor?.closest('p');
				const sentText = sentEl?.textContent?.trim() || '';
				const rect = (selection && selection.rangeCount > 0) ? selection.getRangeAt(0).getBoundingClientRect() : null;

				if (e.ctrlKey || e.metaKey) {
					selectedWord = null;
					selectedAiWord = clean;
					selectedAiSentence = sentText;
					selectedAiAnchorRect = rect;
					return;
				}

				selectedAiWord = null;
				selectedWord = clean;
				selectedSentence = sentText;
				selectedAnchorRect = rect;
			}
		}
	}

	let contentNode = $state<HTMLElement | null>(null);

	$effect(() => {
		// Re-evaluate word classes across the article whenever vocab.statuses or vocab.highlightEnabled updates
		const _statuses = vocab.statuses;
		const _highlight = vocab.highlightEnabled;
		if (!contentNode) return;

		const wordEls = contentNode.querySelectorAll<HTMLElement>('.reader-word[data-word]');
		wordEls.forEach((el) => {
			const w = el.getAttribute('data-word');
			if (!w) return;
			const st = vocab.getStatus(w);
			el.classList.remove('status-unknown', 'status-learning', 'status-known', 'status-ignored');
			el.classList.add(`status-${st}`);
		});
	});

	function contentAction(node: HTMLElement) {
		contentNode = node;
		node.addEventListener('click', handleContentClick);
		node.addEventListener('mouseup', handleMouseUp);
		return {
			destroy() {
				contentNode = null;
				node.removeEventListener('click', handleContentClick);
				node.removeEventListener('mouseup', handleMouseUp);
			}
		};
	}

	function formatDate(dateStr?: string): string {
		if (!dateStr) return '';
		try {
			const d = new Date(dateStr);
			if (isNaN(d.getTime())) return dateStr;
			return d.toLocaleDateString('id-ID', {
				weekday: 'long',
				day: 'numeric',
				month: 'long',
				year: 'numeric'
			});
		} catch {
			return dateStr;
		}
	}

	function estimatedReadTime(chars: number): string {
		const minutes = Math.max(1, Math.ceil(chars / 1000));
		return `${minutes} menit baca`;
	}

	onMount(() => {
		loadArticle();
	});

	onDestroy(() => {
		tts.stop();
	});

	$effect(() => {
		const activeIdx = tts.activeSentenceIndex;
		if (typeof document === 'undefined') return;

		const prev = document.querySelectorAll('.reader-sentence.active-tts-sentence');
		prev.forEach((el) => el.classList.remove('active-tts-sentence'));

		if (activeIdx !== null) {
			const targetEl = document.querySelector(`.reader-sentence[data-sentence-index="${activeIdx}"]`);
			if (targetEl) {
				targetEl.classList.add('active-tts-sentence');
				targetEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
			}
		}
	});
</script>

<div class="relative min-h-screen pb-36 pt-1">
	<!-- Sticky Control Toolbar matching Limusic layout -->
	<header class="sticky top-0 z-30 mb-6 flex items-center justify-between border-b border-border/60 bg-background/85 px-4 sm:px-6 py-2.5 backdrop-blur-md">
		<div class="flex items-center gap-3 min-w-0">
			<button
				type="button"
				onclick={handleBack}
				class="inline-flex h-8 w-8 items-center justify-center rounded-full bg-foreground/5 text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Kembali ke Daftar Artikel"
			>
				<HugeiconsIcon icon={ArrowLeft01Icon} size={16} />
			</button>

			<div class="hidden sm:flex flex-col min-w-0">
				<span class="truncate text-xs font-semibold text-foreground">
					{article?.source || fallbackSource || 'Reader Mode'}
				</span>
				<span class="truncate text-[10px] text-muted-foreground">
					Bebas Iklan • Sentuh kata untuk kamus
				</span>
			</div>
		</div>

		<!-- Action Controls -->
		<div class="flex items-center gap-2">
			<!-- Dengarkan Artikel (Kokoro TTS) Button -->
			<button
				type="button"
				onclick={() => {
					if (tts.playing && !tts.paused) {
						tts.pause();
					} else if (tts.paused) {
						tts.resume();
					} else if (articleSentences.length > 0) {
						tts.playArticle(articleSentences, tts.activeSentenceIndex ?? 0);
					}
				}}
				class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold transition shadow-xs cursor-pointer {tts.playing && !tts.paused
					? 'bg-primary text-primary-foreground shadow-sm'
					: tts.generating
						? 'bg-primary/20 text-primary animate-pulse'
						: 'bg-primary/15 text-primary hover:bg-primary/25'}"
				title="Dengarkan artikel dibacakan dengan suara alami"
			>
				{#if tts.generating}
					<HugeiconsIcon icon={ArrowReloadHorizontalIcon} size={13} class="animate-spin" />
					<span>Memproses Suara...</span>
				{:else if tts.playing && !tts.paused}
					<HugeiconsIcon icon={PauseIcon} size={13} />
					<span>Jeda Suara</span>
				{:else if tts.paused}
					<HugeiconsIcon icon={PlayIcon} size={13} />
					<span>Lanjutkan</span>
				{:else}
					<HugeiconsIcon icon={HeadphonesIcon} size={13} />
					<span class="hidden sm:inline">Dengarkan Artikel</span>
					<span class="sm:hidden">Dengar</span>
				{/if}
			</button>

			<!-- Font Serif / Sans Toggle -->
			<button
				type="button"
				onclick={() => (fontSerif = !fontSerif)}
				class="rounded-full bg-foreground/5 px-2.5 py-1 text-xs font-medium text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Ubah Gaya Huruf (Serif / Sans)"
			>
				<span class={fontSerif ? 'font-serif font-bold text-primary' : 'font-sans'}>
					{fontSerif ? 'Serif' : 'Sans'}
				</span>
			</button>

			<!-- Font Size Switcher -->
			<div class="inline-flex items-center rounded-full bg-foreground/5 p-0.5 text-xs">
				<button
					type="button"
					onclick={() => (fontSize = 'normal')}
					class="rounded-full px-2 py-0.5 text-[11px] font-medium transition cursor-pointer {fontSize === 'normal'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground'}"
					title="Ukuran Font Normal"
				>
					A
				</button>
				<button
					type="button"
					onclick={() => (fontSize = 'large')}
					class="rounded-full px-2 py-0.5 text-xs font-semibold transition cursor-pointer {fontSize === 'large'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground'}"
					title="Ukuran Font Sedang"
				>
					A+
				</button>
				<button
					type="button"
					onclick={() => (fontSize = 'xlarge')}
					class="rounded-full px-2 py-0.5 text-xs font-bold transition cursor-pointer {fontSize === 'xlarge'
						? 'bg-primary text-primary-foreground shadow-sm'
						: 'text-muted-foreground hover:text-foreground'}"
					title="Ukuran Font Besar"
				>
					A++
				</button>
			</div>

			<!-- Copy Markdown (Obsidian / Notion) -->
			<button
				type="button"
				onclick={copyMarkdown}
				class="inline-flex items-center gap-1.5 rounded-full bg-foreground/5 px-2.5 py-1 text-xs font-medium text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Salin Artikel sebagai Markdown (Obsidian / Notion)"
			>
				<HugeiconsIcon icon={Copy01Icon} size={13} />
				<span class="hidden sm:inline">Salin MD</span>
			</button>

			<!-- Copy Link -->
			<button
				type="button"
				onclick={copyArticleLink}
				class="inline-flex h-7.5 w-7.5 items-center justify-center rounded-full bg-foreground/5 text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Salin Tautan Artikel Asli"
			>
				<HugeiconsIcon icon={CopyLinkIcon} size={14} />
			</button>

			<!-- Open in Real Browser -->
			<button
				type="button"
				onclick={openInBrowser}
				class="inline-flex items-center gap-1.5 rounded-full bg-foreground/5 px-3 py-1 text-xs font-medium text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
				title="Buka Website Asli di Browser"
			>
				<span>Web Asli</span>
				<HugeiconsIcon icon={ExternalLinkIcon} size={12} />
			</button>
		</div>
	</header>

	<!-- Main Article Container -->
	<main class="mx-auto max-w-3xl px-4 sm:px-6">
		{#if loading}
			<!-- Reading Skeleton -->
			<div class="space-y-6 pt-4">
				<div class="space-y-3">
					<Skeleton class="h-4 w-28 rounded-full" />
					<Skeleton class="h-8 w-full rounded" />
					<Skeleton class="h-8 w-4/5 rounded" />
					<div class="flex gap-4 pt-2">
						<Skeleton class="h-4 w-32 rounded" />
						<Skeleton class="h-4 w-24 rounded" />
					</div>
				</div>
				<Skeleton class="aspect-[16/9] w-full rounded-xl" />
				<div class="space-y-4 pt-4">
					<Skeleton class="h-4.5 w-full rounded" />
					<Skeleton class="h-4.5 w-full rounded" />
					<Skeleton class="h-4.5 w-3/4 rounded" />
					<Skeleton class="h-4.5 w-full rounded" />
					<Skeleton class="h-4.5 w-5/6 rounded" />
				</div>
			</div>
		{:else if error}
			<div class="mt-8 rounded-2xl border border-border bg-card p-8 text-center space-y-4 shadow-sm">
				<div class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10 text-destructive">
					<HugeiconsIcon icon={BookOpenTextIcon} size={24} />
				</div>
				<div class="space-y-1">
					<h2 class="text-base font-bold text-foreground">Artikel Tidak Dapat Dimuat</h2>
					<p class="text-xs text-muted-foreground max-w-md mx-auto">{error}</p>
				</div>
				<div class="flex flex-wrap items-center justify-center gap-2.5 pt-2">
					<Button variant="outline" size="sm" onclick={loadArticle} class="rounded-full text-xs">
						<HugeiconsIcon icon={ArrowReloadHorizontalIcon} size={13} class="mr-1.5" />
						Coba Lagi
					</Button>
					<Button size="sm" onclick={openInBrowser} class="rounded-full text-xs">
						Buka di Web Asli
						<HugeiconsIcon icon={ExternalLinkIcon} size={13} class="ml-1.5" />
					</Button>
				</div>
			</div>
		{:else if article}
			{@const articleStats = vocab.computeTextStats((article.html || '').replace(/<[^>]*>/g, ' ') || article.title)}
			<article class="space-y-6 pt-2">
				<!-- Paywall / Bot-block Alert Banner if detected -->
				{#if article.is_paywall_blocked}
					<div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs text-amber-600 dark:text-amber-400 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shadow-sm">
						<div class="space-y-0.5">
							<span class="font-bold flex items-center gap-1.5">
								⚠️ Proteksi Paywall / Akses Terbatas Terdeteksi
							</span>
							<p class="text-muted-foreground text-[11px]">
								Situs web ini membatasi akses pembaca otomatis (paywall atau verifikasi browser). Sebagian isi artikel mungkin terpotong.
							</p>
						</div>
						<Button size="sm" variant="outline" onclick={openInBrowser} class="shrink-0 rounded-full text-xs">
							Buka di Web Asli
							<HugeiconsIcon icon={ExternalLinkIcon} size={12} class="ml-1.5" />
						</Button>
					</div>
				{/if}

				<!-- Article Header & Meta -->
				<header class="space-y-3 border-b border-border/40 pb-5">
					<div class="flex flex-wrap items-center gap-2 text-xs">
						<span class="rounded-full bg-primary/15 px-2.5 py-0.5 font-semibold text-primary text-[11px]">
							{article.source}
						</span>
						{#if article.published}
							<span class="text-muted-foreground">•</span>
							<time class="text-muted-foreground">{formatDate(article.published)}</time>
						{/if}
						<span class="text-muted-foreground">•</span>
						<span class="text-muted-foreground">
							{article.reading_time_minutes ? `${article.reading_time_minutes} menit baca` : estimatedReadTime(article.text_length)}
						</span>
					</div>

					<h1
						class="text-2xl font-bold tracking-tight text-foreground sm:text-3xl leading-snug {fontSerif
							? 'font-serif'
							: 'font-sans'}"
					>
						{article.title || fallbackTitle}
					</h1>

					{#if article.byline}
						<p class="text-xs font-medium text-muted-foreground">
							{article.byline}
						</p>
					{/if}

					<!-- Article Difficulty & Comprehension Stats -->
					<div class="flex items-center gap-2 pt-1 flex-wrap">
						<span
							class="rounded px-2 py-0.5 text-xs font-bold {articleStats.comprehensionPct >= 80
								? 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30'
								: articleStats.comprehensionPct >= 60
									? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30'
									: 'bg-rose-500/15 text-rose-600 dark:text-rose-400 border border-rose-500/30'}"
							title="{articleStats.knownWords} dari {articleStats.totalWords} kata dalam artikel ini sudah Anda kuasai"
						>
							{articleStats.comprehensionPct}% Comprehension
						</span>
						<span class="rounded bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 text-xs font-medium text-rose-500 dark:text-rose-400">
							{articleStats.newWordsCount} Kata Baru
						</span>
						<span class="text-xs text-muted-foreground">
							• {articleStats.totalWords} Total Kata
						</span>
					</div>
				</header>

				<!-- Hero Image (if provided and not duplicate in HTML) -->
				{#if article.hero_image && !article.html.includes(article.hero_image)}
					<figure class="overflow-hidden rounded-xl border border-border/60 bg-muted shadow-sm">
						<img
							src={article.hero_image}
							alt={article.title}
							class="w-full max-h-[420px] object-cover"
						/>
					</figure>
				{/if}

				<!-- Interactive Editorial HTML Content with Word & Sentence Hover -->
				<div
					use:contentAction
					class="article-body {vocab.highlightEnabled ? 'highlight-active' : ''} {fontSerif ? 'font-serif' : 'font-sans'} {fontSize === 'normal'
						? 'text-base leading-relaxed'
						: fontSize === 'large'
							? 'text-lg leading-loose'
							: 'text-xl leading-loose'} text-foreground/90 selection:bg-primary/20 selection:text-foreground"
				>
					{@html processedHtml || article.html}
				</div>

				<!-- Article Footer & Source Attribution -->
				<footer class="mt-10 rounded-xl border border-border/60 bg-card/60 p-5 backdrop-blur">
					<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
						<div class="space-y-0.5">
							<span class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
								Sumber Asli
							</span>
							<p class="text-xs font-semibold text-foreground">
								{article.source}
							</p>
						</div>
						<div class="flex items-center gap-2">
							<Button variant="outline" size="sm" onclick={copyArticleLink} class="rounded-full text-xs">
								<HugeiconsIcon icon={CopyLinkIcon} size={13} class="mr-1.5" />
								Salin Tautan
							</Button>
							<Button size="sm" onclick={openInBrowser} class="rounded-full text-xs">
								Kunjungi Situs
								<HugeiconsIcon icon={ExternalLinkIcon} size={13} class="ml-1.5" />
							</Button>
						</div>
					</div>
				</footer>
			</article>
		{/if}
	</main>

	<!-- Universal Immersion Word Popup -->
	{#if selectedWord}
		<WordPopup
			word={selectedWord}
			sentence={selectedSentence}
			sourceTitle={article?.title || fallbackTitle}
			sourceUrl={targetUrl}
			anchorRect={selectedAnchorRect}
			onclose={() => (selectedWord = null)}
			onopenAi={() => {
				selectedAiWord = selectedWord;
				selectedAiSentence = selectedSentence;
				selectedAiAnchorRect = selectedAnchorRect;
			}}
		/>
	{/if}

	<!-- AI Immersion Tutor Popup (Ctrl + Click) -->
	{#if selectedAiWord}
		<AiWordPopup
			word={selectedAiWord}
			sentence={selectedAiSentence}
			sourceTitle={article?.title || fallbackTitle}
			sourceUrl={targetUrl}
			anchorRect={selectedAiAnchorRect}
			onclose={() => (selectedAiWord = null)}
		/>
	{/if}

	<!-- Floating Article Audio Player Bar & Kokoro Manager -->
	<ArticleTtsBar />
</div>

<style>
	/* Scoped Reader Mode HTML Styling */
	:global(.article-body p) {
		margin-bottom: 1.5rem;
		text-align: justify;
		text-justify: inter-word;
	}

	:global(.article-body h1, .article-body h2, .article-body h3, .article-body h4) {
		font-weight: 700;
		color: var(--color-foreground, #ffffff);
		margin-top: 1.75rem;
		margin-bottom: 0.75rem;
		line-height: 1.3;
	}

	:global(.article-body h1) {
		font-size: 1.5em;
	}

	:global(.article-body h2) {
		font-size: 1.3em;
	}

	:global(.article-body h3) {
		font-size: 1.15em;
	}

	:global(.article-body blockquote) {
		margin: 1.5rem 0;
		padding: 0.75rem 1.25rem;
		border-left: 3px solid var(--color-primary, #3b82f6);
		background-color: var(--color-accent, rgba(255, 255, 255, 0.04));
		border-radius: 0 0.5rem 0.5rem 0;
		font-style: italic;
		color: var(--color-muted-foreground, #a1a1aa);
	}

	:global(.article-body ul, .article-body ol) {
		margin-bottom: 1.25rem;
		padding-left: 1.5rem;
	}

	:global(.article-body ul) {
		list-style-type: disc;
	}

	:global(.article-body ol) {
		list-style-type: decimal;
	}

	:global(.article-body li) {
		margin-bottom: 0.35rem;
	}

	:global(.article-body img) {
		display: block;
		max-width: 100%;
		height: auto;
		border-radius: 0.75rem;
		margin: 1.5rem auto;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
	}

	:global(.article-body figure) {
		margin: 1.75rem 0;
		text-align: center;
	}

	:global(.article-body figcaption) {
		margin-top: 0.5rem;
		font-size: 0.85em;
		font-style: italic;
		color: var(--color-muted-foreground, #a1a1aa);
	}

	:global(.article-body a) {
		color: var(--color-primary, #3b82f6);
		text-decoration: underline;
		text-underline-offset: 3px;
	}

	/* Sentence hover background: like transcript line hover */
	:global(.article-body .reader-sentence) {
		border-radius: 0.375rem;
		padding: 0.1rem 0.2rem;
		margin: -0.1rem -0.2rem;
		transition: background-color 0.15s ease, box-shadow 0.15s ease;
		display: inline;
	}
	:global(.article-body .reader-sentence:hover) {
		background-color: rgba(125, 125, 125, 0.08);
	}
	:global(.article-body .reader-sentence.active-tts-sentence) {
		background-color: rgba(59, 130, 246, 0.18) !important;
		box-shadow: 0 0 0 2px var(--color-primary, #3b82f6), 0 2px 10px rgba(59, 130, 246, 0.22);
		border-radius: 0.375rem;
	}

	/* Word hover background & styling: Lingua-Miner square highlight with subtle padding */
	:global(.article-body .reader-word) {
		padding: 0 0.12rem;
		margin: 0 0.04rem;
		cursor: pointer;
		user-select: text;
		transition: background-color 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
		display: inline-block;
		border-bottom: 2px solid transparent;
	}
	:global(.article-body .reader-word:hover) {
		background-color: rgba(59, 130, 246, 0.15);
	}

	/* Word Status Highlighting (Only active when enabled in settings) */
	:global(.article-body.highlight-active .reader-word.status-unknown) {
		border-bottom: 2px solid #f43f5e; /* Rose 500: unknown word */
	}
	:global(.article-body.highlight-active .reader-word.status-unknown:hover) {
		background-color: rgba(244, 63, 94, 0.15);
	}
	:global(.article-body.highlight-active .reader-word.status-learning) {
		border-bottom: 2px solid #f59e0b; /* Amber 500: learning word */
		font-weight: 500;
	}
	:global(.article-body.highlight-active .reader-word.status-learning:hover) {
		background-color: rgba(245, 158, 11, 0.15);
	}
	:global(.article-body.highlight-active .reader-word.status-known) {
		border-bottom: 2px solid transparent;
	}
	:global(.article-body.highlight-active .reader-word.status-ignored) {
		opacity: 0.40;
		border-bottom: 2px solid transparent;
	}
	:global(.article-body.highlight-active .reader-word.status-ignored:hover) {
		opacity: 0.85;
	}
</style>
