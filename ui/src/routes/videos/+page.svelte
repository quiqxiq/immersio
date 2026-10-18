<script lang="ts">
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { HugeiconsIcon, type IconSvgElement } from '@hugeicons/svelte';
	import {
		AiBrain01Icon,
		ArrowLeft01Icon,
		ArrowUpBigIcon,
		Atom02Icon,
		Cancel01Icon,
		CodeIcon,
		DashboardSquareEditIcon,
		LaptopIcon,
		Mic01Icon,
		Notification01Icon,
		PlayIcon,
		RefreshIcon,
		Search01Icon,
		SparklesIcon,
		UserGroup02Icon,
		UserIcon,
		Video01Icon,
		ViewOffSlashIcon
	} from '@hugeicons/core-free-icons';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SectionHeading from '$lib/components/SectionHeading.svelte';
	import VideoShelf from '$lib/components/VideoShelf.svelte';
	import ChannelRail from '$lib/components/ChannelRail.svelte';
	import VideosLayoutDialog from '$lib/components/VideosLayoutDialog.svelte';
	import * as api from '$lib/api';
	import type { HomeChip, SongItem, SubscribedChannel } from '$lib/api';
	import { parseYtLink } from '$lib/ytlink';
	import { auth, np, personal, playback, playSong, shadowing, noteVideoSections } from '$lib/player.svelte';
	import { arrangeVideoSections, hiddenVideoSections } from '$lib/personal';
	import { thumb } from '$lib/thumb';
	import { getCached, putCached } from '$lib/pagecache';

	// Section Keys
	const SEC_REC = '@recommendations';
	const SEC_SUBS = '@subscriptions';
	const SEC_CHANNELS = '@channels';
	const SEC_AI = '@ai';
	const SEC_TECH = '@tech';
	const SEC_SCIENCE = '@science';
	const SEC_PODCASTS = '@podcasts';
	const SEC_CODING = '@coding';

	interface VideoCategoryChip {
		id: string;
		label: string;
		icon?: IconSvgElement;
		sectionKey?: string;
	}

	const FALLBACK_CHIPS: VideoCategoryChip[] = [
		{ id: 'ai', label: 'AI & Machine Learning', icon: AiBrain01Icon, sectionKey: SEC_AI },
		{ id: 'tech', label: 'Teknologi & Gadget', icon: LaptopIcon, sectionKey: SEC_TECH },
		{ id: 'science', label: 'Sains & Edukasi', icon: Atom02Icon, sectionKey: SEC_SCIENCE },
		{ id: 'podcasts', label: 'Podcast & Wawancara', icon: Mic01Icon, sectionKey: SEC_PODCASTS },
		{ id: 'coding', label: 'Coding & Software', icon: CodeIcon, sectionKey: SEC_CODING }
	];

	// Search & navigation states
	let query = $state('');
	let selectedChip = $state<string | null>(null);
	let dynamicChips = $state<HomeChip[]>([]);
	let selectedDynamicChip = $state<HomeChip | null>(null);
	let dynamicVideos = $state<SongItem[]>([]);
	let loadingDynamic = $state(false);
	let editing = $state(false);

	// Channel inspection state
	let selectedChannel = $state<SubscribedChannel | null>(null);
	let channelVideos = $state<SongItem[]>([]);
	let channelLoading = $state(false);
	let channelFilter = $state('');

	// Shelves data states
	let recVideos = $state<SongItem[]>([]);
	let subVideos = $state<SongItem[]>([]);
	let channelsList = $state<SubscribedChannel[]>([]);
	let aiVideos = $state<SongItem[]>([]);
	let techVideos = $state<SongItem[]>([]);
	let scienceVideos = $state<SongItem[]>([]);
	let podcastVideos = $state<SongItem[]>([]);
	let codingVideos = $state<SongItem[]>([]);

	// Loading flags per section
	let loadingRec = $state(false);
	let loadingSubs = $state(false);
	let loadingChannels = $state(false);
	let loadingAi = $state(false);
	let loadingTech = $state(false);
	let loadingScience = $state(false);
	let loadingPodcasts = $state(false);
	let loadingCoding = $state(false);

	// Search state
	let searchVideos = $state<SongItem[]>([]);
	let searching = $state(false);
	let searchError = $state<string | null>(null);

	// Scroll observer
	let scroller = $state<HTMLElement | null>(null);
	let scrolled = $state(false);
	let stuck = $state(false);

	function stickWatch(node: HTMLElement) {
		const io = new IntersectionObserver(
			([e]) => (stuck = !e.isIntersecting && e.boundingClientRect.top < (e.rootBounds?.top ?? 0)),
			{ root: node.closest('main') }
		);
		io.observe(node);
		return () => io.disconnect();
	}

	function watchScroll(node: HTMLElement) {
		const el = node.closest('main');
		if (!el) return;
		scroller = el;
		const onScroll = () => (scrolled = el.scrollTop > 400);
		el.addEventListener('scroll', onScroll, { passive: true });
		return () => el.removeEventListener('scroll', onScroll);
	}

	// Layout blocks definition
	interface VideoBlock {
		key: string;
		title: string;
		icon?: any;
		items?: SongItem[];
		channels?: SubscribedChannel[];
		loading?: boolean;
		onMore?: () => void;
	}

	const allBlocks = $derived<VideoBlock[]>([
		{
			key: SEC_REC,
			title: 'Rekomendasi Untuk Anda',
			icon: SparklesIcon,
			items: recVideos,
			loading: loadingRec,
			onMore: () => selectCategory('rec')
		},
		{
			key: SEC_SUBS,
			title: 'Langganan Terbaru',
			icon: Notification01Icon,
			items: subVideos,
			loading: loadingSubs,
			onMore: () => selectCategory('subs')
		},
		{
			key: SEC_CHANNELS,
			title: 'Channel Langganan Saya',
			icon: UserGroup02Icon,
			channels: channelsList,
			loading: loadingChannels,
			onMore: () => selectCategory('channels')
		},
		{
			key: SEC_AI,
			title: 'AI & Machine Learning',
			icon: AiBrain01Icon,
			items: aiVideos,
			loading: loadingAi,
			onMore: () => selectCategory('ai')
		},
		{
			key: SEC_TECH,
			title: 'Teknologi & Gadget',
			icon: LaptopIcon,
			items: techVideos,
			loading: loadingTech,
			onMore: () => selectCategory('tech')
		},
		{
			key: SEC_SCIENCE,
			title: 'Sains & Edukasi',
			icon: Atom02Icon,
			items: scienceVideos,
			loading: loadingScience,
			onMore: () => selectCategory('science')
		},
		{
			key: SEC_PODCASTS,
			title: 'Podcast & Wawancara',
			icon: Mic01Icon,
			items: podcastVideos,
			loading: loadingPodcasts,
			onMore: () => selectCategory('podcasts')
		},
		{
			key: SEC_CODING,
			title: 'Coding & Software',
			icon: CodeIcon,
			items: codingVideos,
			loading: loadingCoding,
			onMore: () => selectCategory('coding')
		}
	]);

	const hiddenKeys = $derived(hiddenVideoSections(personal));
	const orderedBlocks = $derived(arrangeVideoSections(allBlocks, personal));
	const visibleBlocks = $derived(orderedBlocks.filter((b) => !hiddenKeys.has(b.key)));

	const knownSections = $derived(
		allBlocks.map((b) => ({ key: b.key, title: b.title }))
	);

	// Remember sections for Edit dialog
	$effect(() => {
		const titles = allBlocks.map((b) => b.title);
		if (titles.length) noteVideoSections(titles);
	});

	// Playback helper
	function playVideoTrack(v: SongItem) {
		shadowing.active = false;
		shadowing.lastPausedIndex = -1;
		shadowing.isWaiting = false;
		const item: SongItem = { ...v, is_video: true };
		playSong(item);
		np.open = true;
	}

	// Fetchers
	async function loadRecommendations(force = false) {
		const key = 'videos:recommendations';
		const cached = getCached<SongItem[]>(key);
		if (cached && !force) {
			recVideos = cached;
		}
		loadingRec = true;
		try {
			const res = await api.getYoutubeVideoFeed();
			recVideos = res.videos || [];
			if (res.chips && res.chips.length > 0) {
				dynamicChips = res.chips.filter(
					(c) => c.params && c.params.trim().length > 0 && c.title.toLowerCase() !== 'all'
				);
			}
			putCached(key, recVideos);
		} catch {
			if (!cached) recVideos = [];
		} finally {
			loadingRec = false;
		}
	}

	async function loadSubscriptions(force = false) {
		const key = 'videos:subscriptions';
		const cached = getCached<SongItem[]>(key);
		if (cached && !force) {
			subVideos = cached;
			return;
		}
		loadingSubs = true;
		try {
			const res = await api.getYoutubeSubscriptions();
			subVideos = res || [];
			putCached(key, subVideos);
		} catch {
			subVideos = [];
		} finally {
			loadingSubs = false;
		}
	}

	async function loadChannels(force = false) {
		const key = 'videos:channels:list';
		const cached = getCached<SubscribedChannel[]>(key);
		if (cached && !force) {
			channelsList = cached;
			return;
		}
		loadingChannels = true;
		try {
			const res = await api.getSubscribedChannels();
			channelsList = res || [];
			putCached(key, channelsList);
		} catch {
			channelsList = [];
		} finally {
			loadingChannels = false;
		}
	}

	async function loadTrendingCategory(category: string, setter: (items: SongItem[]) => void, loadingSetter: (l: boolean) => void, force = false) {
		const key = `videos:trending:${category}`;
		const cached = getCached<SongItem[]>(key);
		if (cached && !force) {
			setter(cached);
			return;
		}
		loadingSetter(true);
		try {
			const res = await api.getTrendingVideos(category);
			const items = res || [];
			setter(items);
			putCached(key, items);
		} catch {
			setter([]);
		} finally {
			loadingSetter(false);
		}
	}

	async function loadAllShelves(force = false) {
		await Promise.allSettled([
			loadRecommendations(force),
			loadSubscriptions(force),
			loadChannels(force),
			loadTrendingCategory('ai', (v) => (aiVideos = v), (l) => (loadingAi = l), force),
			loadTrendingCategory('tech', (v) => (techVideos = v), (l) => (loadingTech = l), force),
			loadTrendingCategory('science', (v) => (scienceVideos = v), (l) => (loadingScience = l), force),
			loadTrendingCategory('podcasts', (v) => (podcastVideos = v), (l) => (loadingPodcasts = l), force),
			loadTrendingCategory('coding', (v) => (codingVideos = v), (l) => (loadingCoding = l), force)
		]);
	}

	async function loadChannelDetail(channel: SubscribedChannel, force = false) {
		selectedChannel = channel;
		const key = `videos:channel:${channel.id}`;
		const cached = getCached<SongItem[]>(key);
		if (cached && !force) {
			channelVideos = cached;
			channelLoading = false;
			return;
		}
		channelLoading = true;
		try {
			const res = await api.getChannelVideos(channel.id);
			channelVideos = (res || []).map((v) => ({
				...v,
				artists: v.artists && v.artists !== 'YouTube Creator' && v.artists !== 'YouTube Channel' ? v.artists : channel.title
			}));
			putCached(key, channelVideos);
		} catch {
			channelVideos = [];
		} finally {
			channelLoading = false;
		}
	}

	// Chip selection
	function selectCategory(chipId: string | null) {
		query = '';
		selectedChannel = null;
		selectedDynamicChip = null;
		dynamicVideos = [];
		selectedChip = chipId === 'all' ? null : chipId;

		if (selectedChip === 'rec') loadRecommendations();
		else if (selectedChip === 'subs') loadSubscriptions();
		else if (selectedChip === 'channels') loadChannels();
		else if (selectedChip === 'ai') loadTrendingCategory('ai', (v) => (aiVideos = v), (l) => (loadingAi = l));
		else if (selectedChip === 'tech') loadTrendingCategory('tech', (v) => (techVideos = v), (l) => (loadingTech = l));
		else if (selectedChip === 'science') loadTrendingCategory('science', (v) => (scienceVideos = v), (l) => (loadingScience = l));
		else if (selectedChip === 'podcasts') loadTrendingCategory('podcasts', (v) => (podcastVideos = v), (l) => (loadingPodcasts = l));
		else if (selectedChip === 'coding') loadTrendingCategory('coding', (v) => (codingVideos = v), (l) => (loadingCoding = l));
	}

	async function selectDynamicChip(chip: HomeChip) {
		query = '';
		selectedChannel = null;
		selectedChip = null;

		if (selectedDynamicChip?.params === chip.params) {
			selectedDynamicChip = null;
			dynamicVideos = [];
			return;
		}

		selectedDynamicChip = chip;
		loadingDynamic = true;
		dynamicVideos = [];

		const key = `videos:dynamic:${chip.params}`;
		const cached = getCached<SongItem[]>(key);
		if (cached) {
			dynamicVideos = cached;
		}

		try {
			const res = await api.getYoutubeVideoFeed(chip.params);
			dynamicVideos = res.videos || [];
			putCached(key, dynamicVideos);
		} catch (err) {
			console.error('Failed to load dynamic category feed:', err);
			if (!cached) dynamicVideos = [];
		} finally {
			loadingDynamic = false;
		}
	}

	function selectAll() {
		query = '';
		selectedChannel = null;
		selectedChip = null;
		selectedDynamicChip = null;
		dynamicVideos = [];
	}

	// Search or direct link handler
	async function handleSearchOrPaste(e?: Event) {
		if (e) e.preventDefault();
		const q = query.trim();
		if (!q) {
			searchVideos = [];
			return;
		}

		// Direct YouTube video link
		const link = parseYtLink(q);
		if (link && link.kind === 'song') {
			playVideoTrack({
				video_id: link.id,
				title: 'Memutar Video...',
				artists: 'YouTube Video',
				is_video: true
			});
			query = '';
			return;
		}

		searching = true;
		searchError = null;
		try {
			const res = await api.searchVideos(q);
			searchVideos = res || [];
		} catch (err: any) {
			searchError = err?.message || String(err);
			searchVideos = [];
		} finally {
			searching = false;
		}
	}

	// Active single category data for filtered view
	const activeFilteredData = $derived.by(() => {
		if (selectedDynamicChip) {
			return {
				title: selectedDynamicChip.title,
				icon: SparklesIcon,
				items: dynamicVideos,
				loading: loadingDynamic
			};
		}
		if (!selectedChip) return null;
		if (selectedChip === 'rec') return { title: 'Rekomendasi Video Untuk Anda', icon: SparklesIcon, items: recVideos, loading: loadingRec };
		if (selectedChip === 'subs') return { title: 'Video Terbaru Dari Langganan', icon: Notification01Icon, items: subVideos, loading: loadingSubs };
		if (selectedChip === 'ai') return { title: 'Video AI & Machine Learning', icon: AiBrain01Icon, items: aiVideos, loading: loadingAi };
		if (selectedChip === 'tech') return { title: 'Video Teknologi & Gadget', icon: LaptopIcon, items: techVideos, loading: loadingTech };
		if (selectedChip === 'science') return { title: 'Video Sains & Edukasi', icon: Atom02Icon, items: scienceVideos, loading: loadingScience };
		if (selectedChip === 'podcasts') return { title: 'Video Podcast & Wawancara', icon: Mic01Icon, items: podcastVideos, loading: loadingPodcasts };
		if (selectedChip === 'coding') return { title: 'Video Coding & Software', icon: CodeIcon, items: codingVideos, loading: loadingCoding };
		return null;
	});

	const filteredChannelList = $derived.by(() => {
		const q = channelFilter.trim().toLowerCase();
		if (!q) return channelsList;
		return channelsList.filter((c) => c.title.toLowerCase().includes(q));
	});

	onMount(() => {
		loadAllShelves();
	});
</script>

<div class="relative isolate" {@attach watchScroll}>
	<!-- Top Radial Gradient Wash matching Home -->
	<div
		class="pointer-events-none absolute inset-x-0 top-0 -z-10 h-80 overflow-hidden"
		aria-hidden="true"
	>
		<div
			class="absolute inset-0 opacity-[0.16]"
			style="background:radial-gradient(90% 75% at 10% 0%, var(--primary) 0%, transparent 70%)"
		></div>
		<div class="absolute inset-0 bg-gradient-to-b from-transparent to-background"></div>
	</div>

	<!-- Page Heading -->
	<div class="px-6 pt-10">
		<div class="flex flex-col sm:flex-row sm:items-baseline justify-between gap-4">
			<div>
				<h1 class="font-heading text-3xl sm:text-4xl font-bold tracking-tight text-foreground">Videos Hub</h1>
				<p class="mt-1 text-xs sm:text-sm text-muted-foreground">
					Tonton video edukasi, sains, AI, dan podcast dengan teks interaktif dan translasi imersif tanpa gangguan.
				</p>
			</div>
			<Button
				variant="ghost"
				size="sm"
				onclick={() => {
					if (selectedChannel) loadChannelDetail(selectedChannel, true);
					else loadAllShelves(true);
				}}
				class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer self-start sm:self-auto"
				title="Segarkan konten video"
			>
				<HugeiconsIcon icon={RefreshIcon} size={14} class={loadingRec || loadingSubs ? 'animate-spin' : ''} />
				<span>Perbarui</span>
			</Button>
		</div>

		<!-- Search Form -->
		<form class="relative mt-5 max-w-2xl" onsubmit={handleSearchOrPaste}>
			<HugeiconsIcon
				icon={Search01Icon}
				class="pointer-events-none absolute left-4 top-1/2 z-10 h-5 w-5 -translate-y-1/2 text-muted-foreground"
			/>
			<input
				type="text"
				bind:value={query}
				placeholder="Cari video atau tempel tautan YouTube (https://youtube.com/watch?v=...)..."
				class="h-11 w-full rounded-full border border-border/50 bg-card pl-11 pr-24 text-sm text-foreground placeholder:text-muted-foreground shadow-xs transition-all focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
			/>
			{#if query}
				<button
					type="button"
					class="absolute right-18 top-1/2 z-10 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					onclick={() => {
						query = '';
						searchVideos = [];
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
	</div>

	<!-- Sticky Control Row: Edit Videos Button + Category Chips -->
	<div class="h-px" {@attach stickWatch}></div>
	<div
		class="sticky top-0 z-20 flex items-start gap-2 px-6 py-2.5 transition-colors duration-200 {stuck
			? 'bg-background shadow-xs'
			: ''}"
	>
		<!-- Edit Videos Button (identical to Home's Edit button) -->
		<button
			onclick={() => (editing = true)}
			title={personal.videos.chips ? 'Tata Letak Videos' : undefined}
			aria-label={personal.videos.chips ? 'Tata Letak Videos' : undefined}
			class="-mt-px flex shrink-0 cursor-pointer items-center justify-center gap-1.5 rounded-full border border-border py-1.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground {personal.videos.chips
				? 'px-2'
				: 'pl-3 pr-3.5 text-sm font-medium'}"
		>
			<HugeiconsIcon icon={DashboardSquareEditIcon} class="h-5 w-5" />
			{#if !personal.videos.chips}Tata Letak Videos{/if}
		</button>

		<!-- Category Chips Rail -->
		{#if personal.videos.chips || selectedChip || selectedDynamicChip}
			<div class="min-w-0 flex-1">
				<div class="rail flex gap-2 overflow-x-auto pb-0.5">
					<button
						onclick={selectAll}
						class="shrink-0 cursor-pointer rounded-full px-3.5 py-1.5 text-xs sm:text-sm font-medium transition-colors {!selectedChip && !selectedDynamicChip && !query && !selectedChannel
							? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
							: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'}"
					>
						Semua
					</button>

					<button
						onclick={() => selectCategory(selectedChip === 'subs' ? null : 'subs')}
						class="flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full px-3.5 py-1.5 text-xs sm:text-sm font-medium transition-colors {selectedChip === 'subs' && !query && !selectedChannel
							? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
							: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'}"
					>
						<HugeiconsIcon icon={Notification01Icon} class="h-3.5 w-3.5 shrink-0" />
						<span>Langganan</span>
					</button>

					<button
						onclick={() => selectCategory(selectedChip === 'channels' ? null : 'channels')}
						class="flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full px-3.5 py-1.5 text-xs sm:text-sm font-medium transition-colors {selectedChip === 'channels' && !query && !selectedChannel
							? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
							: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'}"
					>
						<HugeiconsIcon icon={UserGroup02Icon} class="h-3.5 w-3.5 shrink-0" />
						<span>Channel Saya</span>
					</button>

					{#if dynamicChips.length > 0}
						{#each dynamicChips as chip (chip.params)}
							{@const active = selectedDynamicChip?.params === chip.params && !query && !selectedChannel}
							<button
								onclick={() => selectDynamicChip(chip)}
								class="shrink-0 cursor-pointer rounded-full px-3.5 py-1.5 text-xs sm:text-sm font-medium transition-colors {active
									? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
									: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'}"
							>
								{chip.title}
							</button>
						{/each}
					{:else}
						{#each FALLBACK_CHIPS as chip (chip.id)}
							{@const active = selectedChip === chip.id && !query && !selectedChannel}
							<button
								onclick={() => selectCategory(active ? null : chip.id)}
								class="flex shrink-0 cursor-pointer items-center gap-1.5 rounded-full px-3.5 py-1.5 text-xs sm:text-sm font-medium transition-colors {active
									? 'bg-primary text-primary-foreground font-semibold shadow-2xs'
									: 'bg-foreground/5 text-foreground/75 hover:bg-foreground/10 hover:text-foreground'}"
							>
								{#if chip.icon}
									<HugeiconsIcon icon={chip.icon} class="h-3.5 w-3.5 shrink-0" />
								{/if}
								<span>{chip.label}</span>
							</button>
						{/each}
					{/if}
				</div>
			</div>
		{/if}
	</div>

	<!-- Main Content Area -->
	<div class="px-6 pb-20 pt-4">
		{#if query}
			<!-- Search Results Mode -->
			<div class="content-in flex flex-col gap-5">
				<SectionHeading title={`Hasil Pencarian: "${query}"`} icon={Search01Icon} />

				{#if searching}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _}
							<div class="flex flex-col gap-2 animate-pulse">
								<Skeleton class="aspect-video w-full rounded-xl" />
								<Skeleton class="h-4 w-3/4 rounded" />
								<Skeleton class="h-3 w-1/2 rounded" />
							</div>
						{/each}
					</div>
				{:else if searchError}
					<ErrorState message={searchError} onRetry={handleSearchOrPaste} />
				{:else if searchVideos.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Video01Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada video yang cocok dengan kata kunci tersebut.</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each searchVideos as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playVideoTrack(v)}
								onkeydown={(e) => e.key === 'Enter' && playVideoTrack(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
							>
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
											<HugeiconsIcon icon={Video01Icon} class="h-8 w-8" />
										</div>
									{/if}
									{#if v.duration}
										<span class="absolute bottom-1.5 right-1.5 rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white/95">
											{v.duration}
										</span>
									{/if}
									<div class="absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover:opacity-100">
										<div class="flex h-11 w-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
											<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
										</div>
									</div>
								</div>
								<div class="flex flex-col gap-1">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
										<span class="line-clamp-1">{v.artists || 'YouTube Creator'}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		{:else if selectedChannel}
			<!-- Single Subscribed Channel Explorer -->
			<div class="content-in flex flex-col gap-6">
				<div class="flex items-center gap-3">
					<Button
						variant="ghost"
						size="sm"
						onclick={() => (selectedChannel = null)}
						class="gap-1.5 text-xs cursor-pointer text-muted-foreground hover:text-foreground"
					>
						<HugeiconsIcon icon={ArrowLeft01Icon} class="h-4 w-4" />
						<span>Kembali ke Semua Video</span>
					</Button>
				</div>

				<!-- Channel Header Banner -->
				<div class="flex flex-col sm:flex-row items-center gap-4 rounded-2xl border border-border/40 bg-card/60 p-5">
					<div class="h-20 w-20 overflow-hidden rounded-full border-2 border-primary/50 shadow-md shrink-0">
						{#if selectedChannel.avatar_url}
							<img src={selectedChannel.avatar_url} alt={selectedChannel.title} class="h-full w-full object-cover" />
						{:else}
							<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
								<HugeiconsIcon icon={UserIcon} class="h-8 w-8" />
							</div>
						{/if}
					</div>
					<div class="flex flex-col text-center sm:text-left gap-1 min-w-0 flex-1">
						<h2 class="font-heading text-2xl font-bold text-foreground">{selectedChannel.title}</h2>
						<p class="text-xs text-muted-foreground">{selectedChannel.subscriber_count || 'Channel Langganan'}</p>
						{#if selectedChannel.description}
							<p class="mt-1 line-clamp-2 text-xs text-muted-foreground/80 leading-relaxed">{selectedChannel.description}</p>
						{/if}
					</div>
				</div>

				<!-- Channel Uploaded Videos Grid -->
				<SectionHeading title={`Video Terbaru dari ${selectedChannel.title}`} icon={Video01Icon} />

				{#if channelLoading}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _}
							<div class="flex flex-col gap-2 animate-pulse">
								<Skeleton class="aspect-video w-full rounded-xl" />
								<Skeleton class="h-4 w-3/4 rounded" />
								<Skeleton class="h-3 w-1/2 rounded" />
							</div>
						{/each}
					</div>
				{:else if channelVideos.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Video01Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Belum ada video yang ditemukan dari channel ini.</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each channelVideos as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playVideoTrack(v)}
								onkeydown={(e) => e.key === 'Enter' && playVideoTrack(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
							>
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
											<HugeiconsIcon icon={Video01Icon} class="h-8 w-8" />
										</div>
									{/if}
									{#if v.duration}
										<span class="absolute bottom-1.5 right-1.5 rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white/95">
											{v.duration}
										</span>
									{/if}
									<div class="absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover:opacity-100">
										<div class="flex h-11 w-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
											<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
										</div>
									</div>
								</div>
								<div class="flex flex-col gap-1">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
										<span class="line-clamp-1">{v.artists || selectedChannel.title}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		{:else if selectedChip === 'channels'}
			<!-- All Subscribed Channels Full View -->
			<div class="content-in flex flex-col gap-5">
				<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
					<SectionHeading title="Semua Channel Langganan Anda" icon={UserGroup02Icon} />
					<input
						type="text"
						bind:value={channelFilter}
						placeholder="Filter nama channel..."
						class="h-9 w-full sm:w-64 rounded-full border border-border/50 bg-card px-4 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/20"
					/>
				</div>

				{#if loadingChannels}
					<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
						{#each Array(12) as _}
							<div class="flex flex-col items-center gap-2 p-3 animate-pulse">
								<Skeleton class="h-20 w-20 rounded-full" />
								<Skeleton class="h-4 w-24 rounded" />
							</div>
						{/each}
					</div>
				{:else if filteredChannelList.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={UserGroup02Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada channel yang ditemukan.</p>
					</div>
				{:else}
					<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
						{#each filteredChannelList as ch (ch.id)}
							<button
								type="button"
								onclick={() => loadChannelDetail(ch)}
								class="group flex flex-col items-center gap-2.5 rounded-2xl p-3 text-center transition-all hover:bg-accent/10 cursor-pointer"
							>
								<div class="h-20 w-20 overflow-hidden rounded-full border-2 border-border/60 bg-muted shadow-xs transition-transform duration-200 group-hover:scale-105 group-hover:border-primary">
									{#if ch.avatar_url}
										<img src={ch.avatar_url} alt={ch.title} class="h-full w-full object-cover" />
									{:else}
										<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
											<HugeiconsIcon icon={UserIcon} class="h-8 w-8" />
										</div>
									{/if}
								</div>
								<div class="flex w-full flex-col items-center gap-0.5">
									<span class="w-full truncate text-xs font-semibold text-foreground group-hover:text-primary transition-colors">
										{ch.title}
									</span>
									{#if ch.subscriber_count}
										<span class="w-full truncate text-[11px] text-muted-foreground/80">{ch.subscriber_count}</span>
									{/if}
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>

		{:else if activeFilteredData}
			<!-- Single Category Filtered Full View (Grid) -->
			<div class="content-in flex flex-col gap-5">
				<SectionHeading title={activeFilteredData.title} icon={activeFilteredData.icon} />

				{#if activeFilteredData.loading && activeFilteredData.items.length === 0}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _}
							<div class="flex flex-col gap-2 animate-pulse">
								<Skeleton class="aspect-video w-full rounded-xl" />
								<Skeleton class="h-4 w-3/4 rounded" />
								<Skeleton class="h-3 w-1/2 rounded" />
							</div>
						{/each}
					</div>
				{:else if activeFilteredData.items.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Video01Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada video yang ditemukan pada kategori ini.</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each activeFilteredData.items as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playVideoTrack(v)}
								onkeydown={(e) => e.key === 'Enter' && playVideoTrack(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
							>
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
											<HugeiconsIcon icon={Video01Icon} class="h-8 w-8" />
										</div>
									{/if}
									{#if v.duration}
										<span class="absolute bottom-1.5 right-1.5 rounded bg-black/85 px-1.5 py-0.5 text-[10px] font-semibold text-white/95">
											{v.duration}
										</span>
									{/if}
									<div class="absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover:opacity-100">
										<div class="flex h-11 w-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
											<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
										</div>
									</div>
								</div>
								<div class="flex flex-col gap-1">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
										<span class="line-clamp-1">{v.artists || 'YouTube Creator'}</span>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		{:else}
			<!-- Full Multi-Shelf Feed (Ordered by user via Edit Videos dialog) -->
			<div class="content-in flex flex-col gap-12">
				{#each visibleBlocks as block (block.key)}
					{#if block.key === SEC_CHANNELS}
						<ChannelRail
							channels={block.channels || []}
							onSelectChannel={(ch) => loadChannelDetail(ch)}
							onMore={block.onMore}
						/>
					{:else if block.items && block.items.length > 0}
						<VideoShelf
							title={block.title}
							icon={block.icon}
							items={block.items}
							size={personal.videos.cards}
							onMore={block.onMore}
							onPlay={playVideoTrack}
						/>
					{:else if block.loading}
						<section aria-hidden="true" class="animate-pulse">
							<Skeleton class="mb-4 h-6 w-48 rounded" />
							<div class="flex gap-3 overflow-hidden pb-2">
								{#each Array(5) as _}
									<div class="w-64 shrink-0 flex flex-col gap-2">
										<Skeleton class="aspect-video w-full rounded-xl" />
										<Skeleton class="h-4 w-3/4 rounded" />
										<Skeleton class="h-3 w-1/2 rounded" />
									</div>
								{/each}
							</div>
						</section>
					{/if}
				{/each}

				{#if visibleBlocks.length === 0}
					<div class="flex flex-col items-center gap-3 py-20 text-center">
						<HugeiconsIcon icon={ViewOffSlashIcon} class="h-8 w-8 text-muted-foreground/40" />
						<p class="max-w-sm text-sm text-muted-foreground">Semua rak video sedang disembunyikan.</p>
						<Button variant="outline" size="sm" onclick={() => (editing = true)}>
							Tata Letak Videos
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

{#if scrolled}
	<button
		transition:fade={{ duration: 150 }}
		onclick={() => scroller?.scrollTo({ top: 0, behavior: 'smooth' })}
		aria-label="Kembali ke atas"
		class="fixed right-6 z-10 flex h-11 w-11 cursor-pointer items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform hover:scale-110 {playback.now
			? 'bottom-24'
			: 'bottom-6'}"
	>
		<HugeiconsIcon icon={ArrowUpBigIcon} class="h-5 w-5" />
	</button>
{/if}

<VideosLayoutDialog bind:open={editing} sections={knownSections} />
