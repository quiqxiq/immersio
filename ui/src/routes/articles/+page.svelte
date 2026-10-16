<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		News01Icon,
		Link01Icon,
		Cancel01Icon,
		ArrowReloadHorizontalIcon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import type { ArticleCard } from '$lib/api';
	import { getCached, putCached } from '$lib/pagecache';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import SectionHeading from '$lib/components/SectionHeading.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import { vocab } from '$lib/vocab.svelte';

	let activeSource = $state('all');
	let loading = $state(false);
	let articles = $state<ArticleCard[]>([]);
	let error = $state<string | null>(null);
	let pastedUrl = $state('');

	const SOURCES = [
		{ id: 'all', label: 'Semua' },
		{ id: 'devto', label: 'Dev.to' },
		{ id: 'hackernews', label: 'Hacker News' },
		{ id: 'freecodecamp', label: 'FreeCodeCamp' },
		{ id: 'npr', label: 'NPR News' },
		{ id: 'guardian', label: 'The Guardian' },
		{ id: 'bbc', label: 'BBC English' }
	];

	const chipClass = (active: boolean) =>
		`shrink-0 cursor-pointer rounded-full px-3.5 py-1.5 text-sm font-medium transition-colors ${
			active
				? 'bg-primary text-primary-foreground shadow-sm'
				: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'
		}`;

	async function loadFeed(sourceId: string, force = false) {
		activeSource = sourceId;
		const cacheKey = `articles:feed:${sourceId}`;
		const cached = getCached<ArticleCard[]>(cacheKey);
		if (cached && !force) {
			articles = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const sourceParam = sourceId === 'all' ? undefined : sourceId;
			const res = await api.getArticlesFeed(sourceParam);
			articles = res || [];
			putCached(cacheKey, articles);
		} catch (e: any) {
			console.error('Failed to load articles feed:', e);
			error = e?.message || 'Gagal memuat artikel. Silakan periksa koneksi internet Anda.';
		} finally {
			loading = false;
		}
	}

	function handleOpenUrl(e?: Event) {
		if (e) e.preventDefault();
		const trimmed = pastedUrl.trim();
		if (!trimmed) return;
		if (!trimmed.startsWith('http://') && !trimmed.startsWith('https://')) {
			alert('Mohon masukkan URL yang valid diawali http:// atau https://');
			return;
		}
		goto(`/articles/read?url=${encodeURIComponent(trimmed)}`);
	}

	function openArticle(item: ArticleCard) {
		const params = new URLSearchParams({
			url: item.link,
			title: item.title,
			source: item.source
		});
		goto(`/articles/read?${params.toString()}`);
	}

	function formatDate(dateStr?: string): string {
		if (!dateStr) return '';
		try {
			const d = new Date(dateStr);
			if (isNaN(d.getTime())) return dateStr;
			return d.toLocaleDateString('id-ID', {
				day: 'numeric',
				month: 'short'
			});
		} catch {
			return dateStr;
		}
	}

	function formatRelativeTime(dateStr?: string, timestamp?: number): string {
		if (timestamp) {
			const now = Math.floor(Date.now() / 1000);
			const diff = now - timestamp;
			if (diff < 60) return 'Baru saja';
			if (diff < 3600) return `${Math.floor(diff / 60)} mnt lalu`;
			if (diff < 86400) return `${Math.floor(diff / 3600)} jam lalu`;
			if (diff < 172800) return 'Kemarin';
			if (diff < 604800) return `${Math.floor(diff / 86400)} hari lalu`;
		}
		if (!dateStr) return '';
		return formatDate(dateStr);
	}

	function getSourceBadgeClass(source: string): string {
		const s = source.toLowerCase();
		if (s.includes('dev.to') || s.includes('devto')) return 'bg-purple-500/20 text-purple-600 dark:text-purple-400 border border-purple-500/30';
		if (s.includes('hacker news')) return 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/30';
		if (s.includes('freecodecamp')) return 'bg-indigo-500/20 text-indigo-600 dark:text-indigo-400 border border-indigo-500/30';
		if (s.includes('npr')) return 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30';
		if (s.includes('guardian')) return 'bg-blue-500/20 text-blue-600 dark:text-blue-400 border border-blue-500/30';
		if (s.includes('bbc')) return 'bg-rose-500/20 text-rose-600 dark:text-rose-400 border border-rose-500/30';
		return 'bg-primary/20 text-primary border border-primary/30';
	}

	onMount(() => {
		loadFeed('all');
	});
</script>

<div class="min-h-full space-y-6 px-6 pb-28 pt-4">
	<!-- Page Heading matching Home -->
	<SectionHeading title="Artikel & Bacaan" icon={News01Icon}>
		<!-- Paste URL Bar in the header action slot -->
		<form
			onsubmit={handleOpenUrl}
			class="relative flex w-64 sm:w-80 md:w-96 items-center"
		>
			<HugeiconsIcon
				icon={Link01Icon}
				class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
				size={15}
			/>
			<input
				type="url"
				bind:value={pastedUrl}
				placeholder="Tempel URL artikel bebas iklan..."
				class="h-8 w-full rounded-full border border-border/80 bg-card/80 pl-9 pr-16 text-xs text-foreground shadow-sm backdrop-blur transition-all placeholder:text-muted-foreground/70 focus:border-primary focus:bg-background focus:outline-none focus:ring-1 focus:ring-primary/20"
			/>
			{#if pastedUrl}
				<button
					type="button"
					onclick={() => (pastedUrl = '')}
					class="absolute right-12 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
					title="Hapus"
				>
					<HugeiconsIcon icon={Cancel01Icon} size={12} />
				</button>
			{/if}
			<Button
				type="submit"
				size="sm"
				disabled={!pastedUrl.trim()}
				class="absolute right-0.5 top-1/2 -translate-y-1/2 h-6.5 rounded-full px-2.5 text-[11px] font-medium"
			>
				Buka
			</Button>
		</form>
	</SectionHeading>

	<!-- Source Filter Chips matching Home -->
	<div class="flex items-center justify-between gap-3">
		<div class="rail flex items-center gap-2 overflow-x-auto pb-1">
			{#each SOURCES as src}
				<button
					type="button"
					onclick={() => loadFeed(src.id)}
					class={chipClass(activeSource === src.id)}
				>
					{src.label}
				</button>
			{/each}
		</div>

		<button
			type="button"
			onclick={() => loadFeed(activeSource, true)}
			disabled={loading}
			title="Segarkan feed"
			class="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-foreground/5 px-3 py-1.5 text-xs text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground disabled:opacity-50"
		>
			<HugeiconsIcon
				icon={ArrowReloadHorizontalIcon}
				size={13}
				class={loading ? 'animate-spin' : ''}
			/>
			<span>Segarkan</span>
		</button>
	</div>

	<!-- Main Content Grid -->
	{#if error}
		<ErrorState message={error} onRetry={() => loadFeed(activeSource, true)} />
	{:else if loading && articles.length === 0}
		<!-- Compact Skeletons matching Home MediaCardSkeleton -->
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
			{#each Array(12) as _}
				<div class="flex flex-col gap-1.5 rounded-xl p-2">
					<Skeleton class="aspect-[16/10] w-full rounded-lg" />
					<Skeleton class="h-3.5 w-full rounded mt-1" />
					<Skeleton class="h-3 w-3/4 rounded" />
					<Skeleton class="h-2.5 w-1/2 rounded" />
				</div>
			{/each}
		</div>
	{:else if articles.length === 0}
		<div class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border py-16 text-center">
			<HugeiconsIcon icon={News01Icon} size={40} class="text-muted-foreground/40 mb-3" />
			<h3 class="text-sm font-semibold text-foreground">Tidak Ada Artikel</h3>
			<p class="text-xs text-muted-foreground mt-1 max-w-sm">
				Coba segarkan atau pilih kategori sumber berita lainnya.
			</p>
			<Button
				variant="outline"
				size="sm"
				onclick={() => loadFeed(activeSource, true)}
				class="mt-3 rounded-full text-xs"
			>
				Segarkan Feed
			</Button>
		</div>
	{:else}
		<!-- Compact Article Cards Grid matching Home MediaCard styling -->
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
			{#each articles as item (item.id || item.link)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					role="button"
					tabindex="0"
					onclick={() => openArticle(item)}
					onkeydown={(e) => e.key === 'Enter' && openArticle(item)}
					class="group flex cursor-pointer flex-col text-left transition-colors hover:bg-accent/10 gap-1.5 rounded-xl p-2 focus:outline-none focus:ring-1 focus:ring-primary/30"
				>
					<!-- Compact Cover Image (16:10) -->
					<div class="relative aspect-[16/10] w-full overflow-hidden bg-muted rounded-lg">
						{#if item.image_url}
							<img
								src={item.image_url}
								alt={item.title}
								loading="lazy"
								class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-103"
								onerror={(e) => {
									(e.currentTarget as HTMLElement).style.display = 'none';
								}}
							/>
						{:else}
							<div class="flex h-full w-full items-center justify-center bg-card text-muted-foreground/30">
								<HugeiconsIcon icon={News01Icon} size={28} />
							</div>
						{/if}

						<!-- Compact Source Badge -->
						<div class="absolute left-1.5 top-1.5">
							<span class="rounded px-1.5 py-0.5 text-[10px] font-semibold backdrop-blur-md shadow-sm {getSourceBadgeClass(item.source)}">
								{item.source}
							</span>
						</div>
					</div>

					<!-- Compact Card Details -->
					<div class="flex flex-col gap-0.5 pt-0.5">
						<h3
							class="line-clamp-2 text-xs md:text-sm font-semibold leading-snug text-foreground transition-colors group-hover:text-primary"
							title={item.title}
						>
							{item.title}
						</h3>

						<div class="flex items-center justify-between text-[11px] text-muted-foreground pt-0.5">
							<span class="truncate">{formatRelativeTime(item.published, item.published_at)}</span>
							<div class="flex items-center gap-1.5 truncate max-w-[55%] text-right text-[10px] text-muted-foreground/80">
								{#if item.reading_time}
									<span class="shrink-0 font-medium text-foreground/60">{item.reading_time} mnt</span>
								{/if}
								{#if item.category}
									<span class="truncate">{item.category}</span>
								{/if}
							</div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
