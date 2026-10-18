<script lang="ts">
	import { onMount } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		Film02Icon,
		Search01Icon,
		Cancel01Icon,
		RefreshIcon,
		PlayIcon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import type { SongItem } from '$lib/api';
	import { np, playback, playSong, shadowing } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';
	import { Button } from '$lib/components/ui/button';

	interface Category {
		id: string;
		label: string;
	}

	const CATEGORIES: Category[] = [
		{ id: 'starter', label: 'Rekomendasi' },
		{ id: 'criticpicks', label: 'Critic Picks' },
		{ id: 'clipempire', label: 'Clip Empire' },
		{ id: 'dollartheater', label: 'The Dollar Theater' },
		{ id: 'binge', label: 'Binge Society' },
		{ id: 'firstslasts', label: 'Firsts and Lasts' },
		{ id: 'movieclips', label: 'Movieclips' },
		{ id: 'scenecity', label: 'Scene City' },
		{ id: 'omeleto', label: 'Omeleto' },
		{ id: 'all', label: 'Jelajah Semua' }
	];

	let activeCategory = $state('starter');
	let scenes = $state<SongItem[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let query = $state('');

	// In-memory cache for fast tab switching
	const cache = new Map<string, SongItem[]>();

	async function loadScenes(category: string, customQuery?: string, force = false) {
		const key = customQuery ? `q:${customQuery.trim().toLowerCase()}` : `c:${category}`;
		if (!force && cache.has(key)) {
			scenes = cache.get(key)!;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.getShadowingScenes(category, customQuery);
			scenes = res || [];
			cache.set(key, scenes);
		} catch (e: any) {
			error = e?.message || String(e);
			scenes = [];
		} finally {
			loading = false;
		}
	}

	function handleCategoryClick(catId: string) {
		activeCategory = catId;
		query = '';
		loadScenes(catId);
	}

	function handleSearch(e?: Event) {
		if (e) e.preventDefault();
		const q = query.trim();
		if (!q) {
			loadScenes(activeCategory);
			return;
		}
		loadScenes(activeCategory, q);
	}

	function playShadowingScene(v: SongItem) {
		shadowing.active = true;
		shadowing.lastPausedIndex = -1;
		shadowing.isWaiting = false;

		const item: SongItem = { ...v, is_video: true };
		playSong(item);
		np.open = true;
	}

	onMount(() => {
		loadScenes(activeCategory);
	});
</script>

<div class="relative isolate min-h-screen">
	<!-- Top Radial Gradient Wash matching Limusic Aesthetic -->
	<div
		class="pointer-events-none absolute inset-x-0 top-0 -z-10 h-80 overflow-hidden"
		aria-hidden="true"
	>
		<div
			class="absolute inset-0 opacity-[0.18]"
			style="background:radial-gradient(90% 75% at 15% 0%, var(--primary) 0%, transparent 70%)"
		></div>
		<div class="absolute inset-0 bg-gradient-to-b from-transparent to-background"></div>
	</div>

	<!-- Page Heading -->
	<div class="px-6 pt-10">
		<div class="flex flex-col sm:flex-row sm:items-baseline justify-between gap-4">
			<div>
				<div class="flex items-center gap-2.5">
					<span class="flex h-8 w-8 items-center justify-center rounded-xl bg-primary/15 text-primary">
						<HugeiconsIcon icon={Film02Icon} size={18} />
					</span>
					<h1 class="font-heading text-3xl sm:text-4xl font-bold tracking-tight text-foreground">
						Movie Scene Shadowing
					</h1>
				</div>
				<p class="mt-1.5 text-xs sm:text-sm text-muted-foreground max-w-2xl">
					Tonton adegan film & drama pilihan (<span class="text-primary font-semibold">2–15 menit</span>). 
					Video otomatis berhenti di setiap akhir kalimat agar Anda dapat menirukan intonasi dan pengucapan aktor.
				</p>
			</div>

			<!-- Refresh Button -->
			<Button
				variant="ghost"
				size="sm"
				onclick={() => loadScenes(activeCategory, query || undefined, true)}
				class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer self-start sm:self-auto"
				title="Segarkan daftar adegan"
			>
				<HugeiconsIcon icon={RefreshIcon} size={14} class={loading ? 'animate-spin' : ''} />
				<span>Perbarui</span>
			</Button>
		</div>

		<!-- Pro-tip Immersion Ribbon -->
		<div class="mt-4 flex flex-wrap items-center gap-2 rounded-xl bg-primary/10 border border-primary/20 px-3.5 py-2 text-xs text-foreground/90">
			<span class="font-semibold text-primary">Kontrol Cepat:</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">Spasi</kbd> untuk lanjut</span>
			<span>•</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">R</kbd> putar ulang kalimat</span>
			<span>•</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">M</kbd> rekam suara latihan</span>
		</div>
	</div>

	<!-- Sticky Filter & Search Bar -->
	<div class="sticky top-0 z-20 px-6 pb-3.5 pt-4 bg-background/95 backdrop-blur-md border-b border-border/40 space-y-3">
		<!-- Search Form -->
		<form class="relative max-w-2xl" onsubmit={handleSearch}>
			<HugeiconsIcon
				icon={Search01Icon}
				class="pointer-events-none absolute left-4 top-1/2 z-10 h-5 w-5 -translate-y-1/2 text-muted-foreground"
			/>
			<input
				type="text"
				bind:value={query}
				placeholder="Cari adegan film, klip drama, atau aktor (misal: Good Will Hunting, Friends, Al Pacino)..."
				class="h-11 w-full rounded-full border border-border/50 bg-card pl-11 pr-24 text-sm text-foreground placeholder:text-muted-foreground shadow-xs transition-all focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
			/>
			{#if query}
				<button
					type="button"
					class="absolute right-18 top-1/2 z-10 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					onclick={() => {
						query = '';
						loadScenes(activeCategory);
					}}
					aria-label="Bersihkan pencarian"
				>
					<HugeiconsIcon icon={Cancel01Icon} class="h-4 w-4" />
				</button>
			{/if}
			<Button
				type="submit"
				size="sm"
				class="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-full px-4 text-xs font-semibold cursor-pointer"
			>
				Cari
			</Button>
		</form>

		<!-- Category Chips Rail -->
		<div class="flex items-center gap-2 overflow-x-auto pt-1 pb-1 scrollbar-none">
			{#each CATEGORIES as cat (cat.id)}
				{@const isActive = activeCategory === cat.id && !query}
				<button
					type="button"
					onclick={() => handleCategoryClick(cat.id)}
					class="inline-flex shrink-0 items-center justify-center rounded-xl px-4 py-1.5 text-xs font-semibold transition-all cursor-pointer {isActive
						? 'bg-primary text-primary-foreground shadow-xs'
						: 'bg-muted/60 text-muted-foreground hover:bg-muted hover:text-foreground border border-border/30'}"
				>
					<span>{cat.label}</span>
				</button>
			{/each}
		</div>
	</div>

	<!-- Main Content Grid -->
	<div class="px-6 pb-20 pt-5">
		{#if loading}
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
				{#each Array(8) as _}
					<div class="flex flex-col gap-2 rounded-xl p-2 animate-pulse">
						<div class="aspect-video w-full rounded-lg bg-muted/60"></div>
						<div class="h-4 w-3/4 rounded bg-muted/60"></div>
						<div class="h-3 w-1/2 rounded bg-muted/40"></div>
					</div>
				{/each}
			</div>
		{:else if error && scenes.length === 0}
			<div class="flex flex-col items-center justify-center gap-3 py-14 px-4 text-center rounded-2xl border border-border/40 bg-card/60">
				<HugeiconsIcon icon={Film02Icon} class="h-10 w-10 text-muted-foreground opacity-60" />
				<div class="space-y-1 max-w-md">
					<h3 class="text-sm font-bold text-foreground">Gagal Memuat Adegan Film</h3>
					<p class="text-xs text-muted-foreground leading-relaxed">{error}</p>
				</div>
				<Button variant="outline" size="sm" onclick={() => loadScenes(activeCategory, query || undefined, true)} class="mt-2 text-xs cursor-pointer">
					Coba Lagi
				</Button>
			</div>
		{:else if scenes.length === 0}
			<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
				<HugeiconsIcon icon={Film02Icon} class="h-12 w-12 opacity-40" />
				<p class="text-sm font-medium">Tidak ada klip adegan film yang ditemukan.</p>
				<p class="text-xs text-muted-foreground/80 max-w-sm">
					Coba gunakan kata kunci lain seperti "Friends", "Al Pacino", atau pilih kategori "Rekomendasi".
				</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
				{#each scenes as v (v.video_id)}
					<div
						role="button"
						tabindex="0"
						onclick={() => playShadowingScene(v)}
						onkeydown={(e) => e.key === 'Enter' && playShadowingScene(v)}
						class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
					>
						<!-- 16:9 Thumbnail with duration, CEFR tag, and play overlay -->
						<div class="relative aspect-video w-full overflow-hidden rounded-lg bg-muted shadow-xs">
							{#if v.thumbnail}
								<img
									src={thumb(v.thumbnail, 400)}
									alt={v.title}
									class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
									loading="lazy"
									onerror={(e) => {
										// If high-res thumbnail 404s, step down to mqdefault or fallback placeholder
										const target = e.currentTarget as HTMLImageElement;
										if (target.src.includes('hqdefault.jpg')) {
											target.src = target.src.replace('hqdefault.jpg', 'mqdefault.jpg');
										} else if (!target.src.includes('default.jpg')) {
											target.src = `https://i.ytimg.com/vi/${v.video_id}/default.jpg`;
										} else {
											target.style.display = 'none';
										}
									}}
								/>
							{:else}
								<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
									<HugeiconsIcon icon={Film02Icon} class="h-8 w-8" />
								</div>
							{/if}

							<!-- Duration Badge (2-15 mins) -->
							{#if v.duration}
								<span class="absolute bottom-1.5 right-1.5 rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white/95">
									{v.duration}
								</span>
							{/if}

							<!-- Hover Play Overlay -->
							<div class="absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover:opacity-100">
								<div class="flex h-11 w-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
									<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
								</div>
							</div>
						</div>

						<!-- Scene Title and Channel Info -->
						<div class="flex flex-col gap-1">
							<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
								{v.title}
							</h3>
							<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
								<span class="line-clamp-1">{v.artists || 'Movie Scene'}</span>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
