<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { HugeiconsIcon, type IconSvgElement } from '@hugeicons/svelte';
	import {
		ArrowLeft01Icon,
		ArrowUpBigIcon,
		CameraSmile02Icon,
		CameraVideoIcon,
		Cancel01Icon,
		ClapperboardIcon,
		DashboardSquareEditIcon,
		Film01Icon,
		Film02Icon,
		ListStartIcon,
		ModernTvIcon,
		PlayIcon,
		RefreshIcon,
		Search01Icon,
		SparklesIcon,
		StarAward01Icon,
		TheaterIcon,
		ViewOffSlashIcon
	} from '@hugeicons/core-free-icons';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import VideoShelf from '$lib/components/VideoShelf.svelte';
	import ShadowingLayoutDialog from '$lib/components/ShadowingLayoutDialog.svelte';
	import * as api from '$lib/api';
	import type { SongItem } from '$lib/api';
	import { np, personal, playSong, shadowing, noteShadowingSections } from '$lib/player.svelte';
	import {
		arrangeShadowingSections,
		hiddenShadowingSections,
		shadowingIsDefault
	} from '$lib/personal';
	import { thumb } from '$lib/thumb';
	import { getCached, putCached } from '$lib/pagecache';

	// Section Keys
	const SEC_STARTER = '@starter';
	const SEC_MOVIECLIPS = '@movieclips';
	const SEC_BINGE = '@binge';
	const SEC_CRITICPICKS = '@criticpicks';
	const SEC_OMELETO = '@omeleto';
	const SEC_KEYANDPEELE = '@keyandpeele';
	const SEC_SCENECITY = '@scenecity';
	const SEC_CLIPEMPIRE = '@clipempire';
	const SEC_DOLLARTHEATER = '@dollartheater';
	const SEC_FIRSTSLASTS = '@firstslasts';

	interface ShadowChip {
		id: string;
		label: string;
		icon?: IconSvgElement;
		sectionKey?: string;
	}

	const CHIPS: ShadowChip[] = [
		{ id: 'all', label: 'Semua Rak', icon: SparklesIcon },
		{ id: 'starter', label: 'Pilihan Editor', icon: SparklesIcon, sectionKey: SEC_STARTER },
		{ id: 'movieclips', label: 'Movieclips', icon: Film02Icon, sectionKey: SEC_MOVIECLIPS },
		{ id: 'binge', label: 'Binge Society', icon: ClapperboardIcon, sectionKey: SEC_BINGE },
		{ id: 'criticpicks', label: 'Critic Picks', icon: StarAward01Icon, sectionKey: SEC_CRITICPICKS },
		{ id: 'omeleto', label: 'Omeleto', icon: TheaterIcon, sectionKey: SEC_OMELETO },
		{ id: 'keyandpeele', label: 'Key & Peele', icon: CameraSmile02Icon, sectionKey: SEC_KEYANDPEELE },
		{ id: 'scenecity', label: 'Scene City', icon: ModernTvIcon, sectionKey: SEC_SCENECITY },
		{ id: 'clipempire', label: 'Clip Empire', icon: CameraVideoIcon, sectionKey: SEC_CLIPEMPIRE },
		{ id: 'dollartheater', label: 'The Dollar Theater', icon: Film01Icon, sectionKey: SEC_DOLLARTHEATER },
		{ id: 'firstslasts', label: 'Firsts & Lasts', icon: ListStartIcon, sectionKey: SEC_FIRSTSLASTS }
	];

	// Navigation & Search state
	let query = $state('');
	let selectedCategory = $state<string | null>(null);
	let editing = $state(false);

	// Shelves data
	let starterScenes = $state<SongItem[]>([]);
	let movieclipsScenes = $state<SongItem[]>([]);
	let bingeScenes = $state<SongItem[]>([]);
	let criticpicksScenes = $state<SongItem[]>([]);
	let omeletoScenes = $state<SongItem[]>([]);
	let keyandpeeleScenes = $state<SongItem[]>([]);
	let scenecityScenes = $state<SongItem[]>([]);
	let clipempireScenes = $state<SongItem[]>([]);
	let dollartheaterScenes = $state<SongItem[]>([]);
	let firstslastsScenes = $state<SongItem[]>([]);

	// Loading flags
	let loadingStarter = $state(false);
	let loadingMovieclips = $state(false);
	let loadingBinge = $state(false);
	let loadingCriticpicks = $state(false);
	let loadingOmeleto = $state(false);
	let loadingKeyandpeele = $state(false);
	let loadingScenecity = $state(false);
	let loadingClipempire = $state(false);
	let loadingDollartheater = $state(false);
	let loadingFirstslasts = $state(false);

	// Search state
	let searchScenes = $state<SongItem[]>([]);
	let loadingSearch = $state(false);
	let searchError = $state<string | null>(null);

	// Focused category grid view state
	let focusedScenes = $state<SongItem[]>([]);
	let loadingFocused = $state(false);

	// Scroll to top
	let scrolled = $state(false);
	let scroller = $state<HTMLElement | null>(null);

	function trackScroll(node: HTMLElement) {
		const el = node.closest('main');
		if (!el) return;
		scroller = el;
		const onScroll = () => (scrolled = el.scrollTop > 400);
		el.addEventListener('scroll', onScroll, { passive: true });
		return () => el.removeEventListener('scroll', onScroll);
	}

	interface ShadowBlock {
		key: string;
		title: string;
		icon: IconSvgElement;
		items: SongItem[];
		loading: boolean;
		onMore: () => void;
	}

	const allBlocks = $derived<ShadowBlock[]>([
		{
			key: SEC_STARTER,
			title: 'Pilihan Editor (Starter Scenes)',
			icon: SparklesIcon,
			items: starterScenes,
			loading: loadingStarter,
			onMore: () => selectCategory('starter')
		},
		{
			key: SEC_MOVIECLIPS,
			title: 'Movieclips (Hollywood Iconic Scenes)',
			icon: Film02Icon,
			items: movieclipsScenes,
			loading: loadingMovieclips,
			onMore: () => selectCategory('movieclips')
		},
		{
			key: SEC_BINGE,
			title: 'Binge Society (Drama & Thriller)',
			icon: ClapperboardIcon,
			items: bingeScenes,
			loading: loadingBinge,
			onMore: () => selectCategory('binge')
		},
		{
			key: SEC_CRITICPICKS,
			title: 'Critic Picks (Dialogue Masterpieces)',
			icon: StarAward01Icon,
			items: criticpicksScenes,
			loading: loadingCriticpicks,
			onMore: () => selectCategory('criticpicks')
		},
		{
			key: SEC_OMELETO,
			title: 'Omeleto (Short Films & Monologues)',
			icon: TheaterIcon,
			items: omeletoScenes,
			loading: loadingOmeleto,
			onMore: () => selectCategory('omeleto')
		},
		{
			key: SEC_KEYANDPEELE,
			title: 'Key & Peele (Conversational Comedy)',
			icon: CameraSmile02Icon,
			items: keyandpeeleScenes,
			loading: loadingKeyandpeele,
			onMore: () => selectCategory('keyandpeele')
		},
		{
			key: SEC_SCENECITY,
			title: 'Scene City (Classics & Biopics)',
			icon: ModernTvIcon,
			items: scenecityScenes,
			loading: loadingScenecity,
			onMore: () => selectCategory('scenecity')
		},
		{
			key: SEC_CLIPEMPIRE,
			title: 'Clip Empire (TV & Sitcom Moments)',
			icon: CameraVideoIcon,
			items: clipempireScenes,
			loading: loadingClipempire,
			onMore: () => selectCategory('clipempire')
		},
		{
			key: SEC_DOLLARTHEATER,
			title: 'The Dollar Theater (Classic Cinema)',
			icon: Film01Icon,
			items: dollartheaterScenes,
			loading: loadingDollartheater,
			onMore: () => selectCategory('dollartheater')
		},
		{
			key: SEC_FIRSTSLASTS,
			title: 'Firsts and Lasts (Iconic Entrances & Exits)',
			icon: ListStartIcon,
			items: firstslastsScenes,
			loading: loadingFirstslasts,
			onMore: () => selectCategory('firstslasts')
		}
	]);

	const hiddenKeys = $derived(hiddenShadowingSections(personal));
	const orderedBlocks = $derived(arrangeShadowingSections(allBlocks, personal));
	const visibleBlocks = $derived(orderedBlocks.filter((b) => !hiddenKeys.has(b.key)));

	const knownSections = $derived(
		allBlocks.map((b) => ({ key: b.key, title: b.title }))
	);

	// Persist seen shelf titles for the layout dialog
	$effect(() => {
		const titles = allBlocks.map((b) => b.title);
		if (titles.length) noteShadowingSections(titles);
	});

	// Playback with full shadowing rehearsal setup
	function playShadowingScene(v: SongItem) {
		shadowing.active = true;
		shadowing.lastPausedIndex = -1;
		shadowing.isWaiting = false;

		const item: SongItem = { ...v, is_video: true };
		playSong(item);
		np.open = true;
	}

	// Fetch individual channel scenes with cache
	async function loadChannelScenes(
		slug: string,
		setter: (items: SongItem[]) => void,
		loadingSetter: (l: boolean) => void,
		force = false
	) {
		const cacheKey = `shadowing:scenes:${slug}`;
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached && !force) {
			setter(cached);
			return;
		}

		loadingSetter(true);
		try {
			const res = await api.getShadowingScenes(slug);
			const items = res || [];
			setter(items);
			putCached(cacheKey, items);
		} catch {
			if (!cached) setter([]);
		} finally {
			loadingSetter(false);
		}
	}

	async function loadAllShelves(force = false) {
		// Starter scenes are instant
		loadChannelScenes('starter', (v) => (starterScenes = v), (l) => (loadingStarter = l), force);

		// Concurrently load primary shelves
		await Promise.allSettled([
			loadChannelScenes('movieclips', (v) => (movieclipsScenes = v), (l) => (loadingMovieclips = l), force),
			loadChannelScenes('binge', (v) => (bingeScenes = v), (l) => (loadingBinge = l), force),
			loadChannelScenes('criticpicks', (v) => (criticpicksScenes = v), (l) => (loadingCriticpicks = l), force),
			loadChannelScenes('omeleto', (v) => (omeletoScenes = v), (l) => (loadingOmeleto = l), force),
			loadChannelScenes('keyandpeele', (v) => (keyandpeeleScenes = v), (l) => (loadingKeyandpeele = l), force),
			loadChannelScenes('scenecity', (v) => (scenecityScenes = v), (l) => (loadingScenecity = l), force),
			loadChannelScenes('clipempire', (v) => (clipempireScenes = v), (l) => (loadingClipempire = l), force),
			loadChannelScenes('dollartheater', (v) => (dollartheaterScenes = v), (l) => (loadingDollartheater = l), force),
			loadChannelScenes('firstslasts', (v) => (firstslastsScenes = v), (l) => (loadingFirstslasts = l), force)
		]);
	}

	// Select focused category view
	async function selectCategory(catId: string) {
		if (catId === 'all') {
			selectedCategory = null;
			return;
		}
		selectedCategory = catId;
		query = '';

		// Check if we already have it loaded in memory
		const foundBlock = allBlocks.find((b) => b.key.replace('@', '') === catId);
		if (foundBlock && foundBlock.items.length > 0) {
			focusedScenes = foundBlock.items;
			loadingFocused = false;
			return;
		}

		loadingFocused = true;
		try {
			const res = await api.getShadowingScenes(catId);
			focusedScenes = res || [];
		} catch {
			focusedScenes = [];
		} finally {
			loadingFocused = false;
		}
	}

	// Search handler
	async function handleSearch(e?: Event) {
		if (e) e.preventDefault();
		const q = query.trim();
		if (!q) {
			searchScenes = [];
			return;
		}

		loadingSearch = true;
		searchError = null;
		selectedCategory = null;

		try {
			const res = await api.getShadowingScenes('starter', q);
			searchScenes = res || [];
		} catch (err: any) {
			searchError = err?.message || String(err);
			searchScenes = [];
		} finally {
			loadingSearch = false;
		}
	}

	function clearSearch() {
		query = '';
		searchScenes = [];
		searchError = null;
	}

	function scrollToTop() {
		scroller?.scrollTo({ top: 0, behavior: 'smooth' });
	}

	onMount(() => {
		loadAllShelves();
	});
</script>

<div class="relative isolate min-h-screen" {@attach trackScroll}>
	<!-- Ambient Limusic Radial Background Wash -->
	<div
		class="pointer-events-none absolute inset-x-0 top-0 -z-10 h-88 overflow-hidden"
		aria-hidden="true"
	>
		<div
			class="absolute inset-0 opacity-[0.16]"
			style="background:radial-gradient(90% 75% at 15% 0%, var(--primary) 0%, transparent 70%)"
		></div>
		<div class="absolute inset-0 bg-gradient-to-b from-transparent to-background"></div>
	</div>

	<!-- Page Heading -->
	<div class="px-6 pt-10">
		<div class="flex flex-col sm:flex-row sm:items-baseline justify-between gap-4">
			<div>
				<div class="flex items-center gap-2.5">
					<span class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary/15 text-primary shadow-xs">
						<HugeiconsIcon icon={Film02Icon} size={20} />
					</span>
					<h1 class="font-heading text-3xl sm:text-4xl font-bold tracking-tight text-foreground">
						Movie Scene Shadowing
					</h1>
				</div>
				<p class="mt-2 text-xs sm:text-sm text-muted-foreground max-w-2xl leading-relaxed">
					Tonton adegan film & drama pilihan (<span class="text-primary font-semibold">2–15 menit</span>).
					Video otomatis berhenti di setiap akhir kalimat dialog agar Anda dapat menirukan intonasi, ritme, dan pengucapan aktor.
				</p>
			</div>

			<div class="flex items-center gap-2 self-start sm:self-auto">
				<!-- Refresh Button -->
				<Button
					variant="ghost"
					size="sm"
					onclick={() => {
						if (selectedCategory) {
							selectCategory(selectedCategory);
						} else if (query.trim()) {
							handleSearch();
						} else {
							loadAllShelves(true);
						}
					}}
					class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
					title="Segarkan adegan"
				>
					<HugeiconsIcon
						icon={RefreshIcon}
						size={14}
						class={loadingStarter || loadingMovieclips || loadingBinge ? 'animate-spin' : ''}
					/>
					<span>Perbarui</span>
				</Button>

				<!-- Floating Layout Drawer Trigger -->
				<Button
					variant="outline"
					size="sm"
					onclick={() => (editing = true)}
					class="relative gap-1.5 text-xs font-semibold cursor-pointer border-border/70 hover:bg-accent/40"
					title="Sesuaikan tata letak rak shadowing"
				>
					<HugeiconsIcon icon={DashboardSquareEditIcon} size={15} />
					<span>Tata Letak</span>
					{#if !shadowingIsDefault(personal)}
						<span
							class="absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full bg-primary ring-2 ring-background"
							aria-label="Tata letak dikustomisasi"
						></span>
					{/if}
				</Button>
			</div>
		</div>

		<!-- Pro-tip Immersion Ribbon -->
		<div class="mt-4 flex flex-wrap items-center gap-2 rounded-xl bg-primary/10 border border-primary/20 px-3.5 py-2 text-xs text-foreground/90">
			<span class="font-semibold text-primary">Kontrol Cepat:</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">Spasi</kbd> untuk lanjut</span>
			<span>•</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">R</kbd> putar ulang kalimat</span>
			<span>•</span>
			<span>Tekan <kbd class="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] border border-border shadow-2xs font-semibold">M</kbd> rekam suara latihan</span>
			<span>•</span>
			<span class="text-primary font-medium">Auto-pause di akhir subtitle aktif</span>
		</div>
	</div>

	<!-- Sticky Action Bar: Search & Dynamic Category Chips -->
	<div class="sticky top-0 z-20 px-6 pb-3 pt-4 bg-background/95 backdrop-blur-md border-b border-border/40 space-y-3">
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
					onclick={clearSearch}
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

		<!-- Category Chips Rail (Toggleable via Personal settings) -->
		{#if personal.shadowing.chips}
			<div class="flex items-center gap-2 overflow-x-auto pt-0.5 pb-1 scrollbar-none">
				{#each CHIPS as chip (chip.id)}
					{@const isActive = (chip.id === 'all' && !selectedCategory && !query) || (selectedCategory === chip.id && !query)}
					<button
						type="button"
						onclick={() => selectCategory(chip.id)}
						class="inline-flex shrink-0 items-center gap-1.5 rounded-xl px-3.5 py-1.5 text-xs font-semibold transition-all cursor-pointer {isActive
							? 'bg-primary text-primary-foreground shadow-xs'
							: 'bg-muted/60 text-muted-foreground hover:bg-muted hover:text-foreground border border-border/30'}"
					>
						{#if chip.icon}
							<HugeiconsIcon icon={chip.icon} size={14} />
						{/if}
						<span>{chip.label}</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Main Body Content -->
	<div class="px-6 pb-24 pt-6">
		<!-- 1. Search Results Mode -->
		{#if query.trim().length > 0}
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2">
						<h2 class="font-heading text-lg font-bold text-foreground">
							Hasil Pencarian untuk "{query.trim()}"
						</h2>
						{#if !loadingSearch}
							<span class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
								{searchScenes.length} adegan
							</span>
						{/if}
					</div>
					<Button
						variant="ghost"
						size="sm"
						onclick={clearSearch}
						class="gap-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
					>
						<HugeiconsIcon icon={Cancel01Icon} size={14} />
						<span>Tutup Pencarian</span>
					</Button>
				</div>

				{#if loadingSearch}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _}
							<div class="flex flex-col gap-2 rounded-xl p-2 animate-pulse">
								<div class="aspect-video w-full rounded-lg bg-muted/60"></div>
								<div class="h-4 w-3/4 rounded bg-muted/60"></div>
								<div class="h-3 w-1/2 rounded bg-muted/40"></div>
							</div>
						{/each}
					</div>
				{:else if searchError}
					<div class="flex flex-col items-center justify-center gap-3 py-14 px-4 text-center rounded-2xl border border-border/40 bg-card/60">
						<HugeiconsIcon icon={Film02Icon} class="h-10 w-10 text-muted-foreground opacity-60" />
						<div class="space-y-1 max-w-md">
							<h3 class="text-sm font-bold text-foreground">Gagal Menjalankan Pencarian</h3>
							<p class="text-xs text-muted-foreground leading-relaxed">{searchError}</p>
						</div>
						<Button variant="outline" size="sm" onclick={handleSearch} class="mt-2 text-xs cursor-pointer">
							Coba Lagi
						</Button>
					</div>
				{:else if searchScenes.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Film02Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada klip adegan film yang cocok dengan pencarian.</p>
						<p class="text-xs text-muted-foreground/80 max-w-sm">
							Coba gunakan kata kunci aktor atau judul film lain, seperti "The Dark Knight", "Good Will Hunting", atau "Al Pacino".
						</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each searchScenes as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playShadowingScene(v)}
								onkeydown={(e) => e.key === 'Enter' && playShadowingScene(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
							>
								<!-- 16:9 Thumbnail -->
								<div class="relative aspect-video w-full overflow-hidden rounded-lg bg-muted shadow-xs">
									{#if v.thumbnail}
										<img
											src={thumb(v.thumbnail, 400)}
											alt={v.title}
											class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
											loading="lazy"
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

								<!-- Scene Title & Channel Info -->
								<div class="flex flex-col gap-1">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<span class="line-clamp-1 text-xs text-muted-foreground">{v.artists || 'Movie Scene'}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		<!-- 2. Single Category Focused Grid View ("Lihat Semua") -->
		{:else if selectedCategory}
			{@const activeChip = CHIPS.find((c) => c.id === selectedCategory)}
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<div class="flex items-center gap-2.5">
						<Button
							variant="ghost"
							size="icon-sm"
							onclick={() => (selectedCategory = null)}
							class="cursor-pointer"
							title="Kembali ke semua rak"
						>
							<HugeiconsIcon icon={ArrowLeft01Icon} size={18} />
						</Button>
						{#if activeChip?.icon}
							<HugeiconsIcon icon={activeChip.icon} size={20} class="text-primary" />
						{/if}
						<h2 class="font-heading text-xl font-bold text-foreground">
							{activeChip?.label || 'Kategori Adegan'}
						</h2>
						{#if !loadingFocused}
							<span class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
								{focusedScenes.length} adegan
							</span>
						{/if}
					</div>

					<Button
						variant="ghost"
						size="sm"
						onclick={() => (selectedCategory = null)}
						class="text-xs text-muted-foreground hover:text-foreground cursor-pointer"
					>
						Semua Rak
					</Button>
				</div>

				{#if loadingFocused}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _}
							<div class="flex flex-col gap-2 rounded-xl p-2 animate-pulse">
								<div class="aspect-video w-full rounded-lg bg-muted/60"></div>
								<div class="h-4 w-3/4 rounded bg-muted/60"></div>
								<div class="h-3 w-1/2 rounded bg-muted/40"></div>
							</div>
						{/each}
					</div>
				{:else if focusedScenes.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Film02Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada adegan film pada kategori ini.</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each focusedScenes as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playShadowingScene(v)}
								onkeydown={(e) => e.key === 'Enter' && playShadowingScene(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
							>
								<!-- 16:9 Thumbnail -->
								<div class="relative aspect-video w-full overflow-hidden rounded-lg bg-muted shadow-xs">
									{#if v.thumbnail}
										<img
											src={thumb(v.thumbnail, 400)}
											alt={v.title}
											class="h-full w-full object-cover transition-transform duration-300 group-hover:scale-105"
											loading="lazy"
										/>
									{:else}
										<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
											<HugeiconsIcon icon={Film02Icon} class="h-8 w-8" />
										</div>
									{/if}

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

								<!-- Scene Title & Info -->
								<div class="flex flex-col gap-1">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<span class="line-clamp-1 text-xs text-muted-foreground">{v.artists || 'Movie Scene'}</span>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		<!-- 3. Multi-Shelf Feed Architecture (Default) -->
		{:else}
			<div class="flex flex-col gap-10">
				{#each visibleBlocks as block (block.key)}
					{#if block.loading && block.items.length === 0}
						<!-- Shelf Skeleton -->
						<div class="space-y-3">
							<div class="flex items-center gap-2.5">
								<Skeleton class="h-5 w-5 rounded" />
								<Skeleton class="h-6 w-48 rounded" />
							</div>
							<div class="flex gap-3 overflow-hidden">
								{#each Array(4) as _}
									<div class="w-72 shrink-0 space-y-2">
										<Skeleton class="aspect-video w-full rounded-lg" />
										<Skeleton class="h-4 w-3/4 rounded" />
										<Skeleton class="h-3 w-1/2 rounded" />
									</div>
								{/each}
							</div>
						</div>
					{:else if block.items.length > 0}
						<VideoShelf
							title={block.title}
							icon={block.icon}
							items={block.items}
							size={personal.shadowing.cards}
							onMore={block.onMore}
							onPlay={playShadowingScene}
						/>
					{/if}
				{/each}

				{#if visibleBlocks.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={ViewOffSlashIcon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Semua rak adegan disembunyikan.</p>
						<p class="text-xs text-muted-foreground/80 max-w-sm">
							Buka menu "Tata Letak" untuk mengaktifkan kembali rak pilihan Anda.
						</p>
						<Button
							variant="outline"
							size="sm"
							onclick={() => (editing = true)}
							class="mt-2 text-xs cursor-pointer"
						>
							Buka Tata Letak
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Scroll-to-Top Floating Button -->
	{#if scrolled}
		<div
			class="fixed bottom-20 right-6 z-30"
			transition:fade={{ duration: 150 }}
		>
			<Button
				variant="secondary"
				size="icon"
				class="h-11 w-11 rounded-full shadow-lg cursor-pointer border border-border/50 hover:bg-primary hover:text-primary-foreground transition-all"
				onclick={scrollToTop}
				aria-label="Kembali ke atas"
			>
				<HugeiconsIcon icon={ArrowUpBigIcon} size={20} />
			</Button>
		</div>
	{/if}

	<!-- Floating Layout Drawer Component -->
	<ShadowingLayoutDialog
		bind:open={editing}
		sections={knownSections}
	/>
</div>
