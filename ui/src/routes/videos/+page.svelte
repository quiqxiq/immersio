<script lang="ts">
	import { onMount } from 'svelte';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		PlayIcon,
		Search01Icon,
		Video01Icon,
		Cancel01Icon,
		Notification01Icon,
		UserGroup02Icon,
		Fire02Icon,
		SparklesIcon,
		ArrowLeft01Icon,
		RefreshIcon
	} from '@hugeicons/core-free-icons';
	import * as api from '$lib/api';
	import type { SongItem, SubscribedChannel } from '$lib/api';
	import { parseYtLink } from '$lib/ytlink';
	import { np, playSong, auth } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';
	import { getCached, putCached } from '$lib/pagecache';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Button } from '$lib/components/ui/button';
	import SectionHeading from '$lib/components/SectionHeading.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';

	type VideoTab = 'subscriptions' | 'channels' | 'trending' | 'recommendations';

	let activeTab = $state<VideoTab>('subscriptions');
	let activeTrendingCategory = $state('ai');
	let query = $state('');

	// Data states
	let loading = $state(false);
	let error = $state<string | null>(null);
	let videos = $state<SongItem[]>([]);

	// Subscribed channels states
	let channels = $state<SubscribedChannel[]>([]);
	let channelFilter = $state('');
	let selectedChannel = $state<SubscribedChannel | null>(null);
	let channelVideos = $state<SongItem[]>([]);
	let channelLoading = $state(false);
	let channelError = $state<string | null>(null);

	const MAIN_TABS: { id: VideoTab; label: string; icon: any; count?: number }[] = [
		{ id: 'subscriptions', label: 'Subscriptions', icon: Notification01Icon },
		{ id: 'channels', label: 'Channel Saya', icon: UserGroup02Icon },
		{ id: 'trending', label: 'Trending & Populer', icon: Fire02Icon },
		{ id: 'recommendations', label: 'Rekomendasi', icon: SparklesIcon }
	];

	const TRENDING_CATEGORIES = [
		{ id: 'all', label: '🌟 Semua Populer' },
		{ id: 'ai', label: '🤖 AI & Machine Learning' },
		{ id: 'tech', label: '💻 Teknologi & Gadget' },
		{ id: 'science', label: '🔬 Sains & Edukasi' },
		{ id: 'podcasts', label: '🎙️ Podcast & Wawancara' },
		{ id: 'coding', label: '⚡ Coding & Software' }
	];

	const filteredChannels = $derived.by(() => {
		const q = channelFilter.trim().toLowerCase();
		if (!q) return channels;
		return channels.filter((c) => c.title.toLowerCase().includes(q));
	});

	// Tab switcher
	async function switchTab(tab: VideoTab, force = false) {
		activeTab = tab;
		selectedChannel = null;
		error = null;

		if (tab === 'subscriptions') {
			await loadSubscriptions(force);
		} else if (tab === 'channels') {
			await loadChannels(force);
		} else if (tab === 'trending') {
			await loadTrending(activeTrendingCategory, force);
		} else if (tab === 'recommendations') {
			await loadRecommendations(force);
		}
	}

	async function loadSubscriptions(force = false) {
		const cacheKey = 'videos:subscriptions';
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached && !force) {
			videos = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.getYoutubeSubscriptions();
			videos = res || [];
			putCached(cacheKey, videos);
		} catch (e: any) {
			const msg = e?.message || String(e);
			error = msg;
		} finally {
			loading = false;
		}
	}

	async function loadChannels(force = false) {
		const cacheKey = 'videos:channels:list';
		const cached = getCached<SubscribedChannel[]>(cacheKey);
		if (cached && !force) {
			channels = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.getSubscribedChannels();
			channels = res || [];
			putCached(cacheKey, channels);
		} catch (e: any) {
			const msg = e?.message || String(e);
			error = msg;
		} finally {
			loading = false;
		}
	}

	async function loadChannelVideos(channel: SubscribedChannel, force = false) {
		selectedChannel = channel;
		const cacheKey = `videos:channel:${channel.id}`;
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached && !force) {
			channelVideos = cached;
			channelLoading = false;
			channelError = null;
			return;
		}

		channelLoading = true;
		channelError = null;
		try {
			const res = await api.getChannelVideos(channel.id);
			channelVideos = (res || []).map((v) => ({
				...v,
				artists: v.artists && v.artists !== 'YouTube Creator' && v.artists !== 'YouTube Channel' ? v.artists : channel.title
			}));
			putCached(cacheKey, channelVideos);
		} catch (e: any) {
			channelError = e?.message || String(e);
		} finally {
			channelLoading = false;
		}
	}

	async function loadTrending(catId: string, force = false) {
		activeTrendingCategory = catId;
		const cacheKey = `videos:trending:${catId}`;
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached && !force) {
			videos = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.getTrendingVideos(catId);
			videos = res || [];
			putCached(cacheKey, videos);
		} catch (e: any) {
			error = e?.message || String(e);
		} finally {
			loading = false;
		}
	}

	async function loadRecommendations(force = false) {
		const cacheKey = 'videos:recommendations';
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached && !force) {
			videos = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.getYoutubeRecommendations();
			videos = res || [];
			putCached(cacheKey, videos);
		} catch (e: any) {
			error = e?.message || String(e);
		} finally {
			loading = false;
		}
	}

	// Search or direct link paste
	async function handleSearchOrPaste(e?: Event) {
		if (e) e.preventDefault();
		const q = query.trim();
		if (!q) return;

		// Direct YouTube video link
		const link = parseYtLink(q);
		if (link && link.kind === 'song') {
			playSong({
				video_id: link.id,
				title: 'Memutar Video...',
				artists: 'YouTube Video',
				is_video: true
			});
			np.open = true;
			query = '';
			return;
		}

		const cacheKey = `videos:search:${q}`;
		const cached = getCached<SongItem[]>(cacheKey);
		if (cached) {
			videos = cached;
			loading = false;
			error = null;
			return;
		}

		loading = true;
		error = null;
		try {
			const res = await api.searchVideos(q);
			videos = res || [];
			putCached(cacheKey, videos);
		} catch (e: any) {
			error = e?.message || String(e);
		} finally {
			loading = false;
		}
	}

	function playVideoTrack(v: SongItem) {
		const item: SongItem = { ...v, is_video: true };
		playSong(item);
		np.open = true;
	}

	onMount(() => {
		switchTab(activeTab);
	});
</script>

<!-- Outer page container matching search/+page.svelte & Home -->
<div class="relative isolate min-h-screen">
	<!-- Top Radial Gradient Wash matching Limusic -->
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
		<div class="flex items-baseline justify-between gap-4">
			<div>
				<h1 class="font-heading text-4xl font-bold tracking-tight text-foreground">Videos Hub</h1>
				<p class="mt-1 text-sm text-muted-foreground">
					Video edukasi, teknologi, dan AI dengan Closed Captions & translasi imersif tanpa gangguan.
				</p>
			</div>
			<Button
				variant="ghost"
				size="sm"
				onclick={() => switchTab(activeTab, true)}
				class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
				title="Segarkan data terbaru"
			>
				<HugeiconsIcon icon={RefreshIcon} size={14} class={loading || channelLoading ? 'animate-spin' : ''} />
				<span>Perbarui</span>
			</Button>
		</div>
	</div>

	<!-- Sticky Filter & Search Bar matching Limusic Search/Home -->
	<div class="sticky top-0 z-20 px-6 pb-3.5 pt-4 bg-background/95 backdrop-blur-md border-b border-border/40 space-y-3">
		<!-- Search Form -->
		<form class="relative max-w-2xl" onsubmit={handleSearchOrPaste}>
			<HugeiconsIcon
				icon={Search01Icon}
				class="pointer-events-none absolute left-4 top-1/2 z-10 h-5 w-5 -translate-y-1/2 text-muted-foreground"
			/>
			<input
				type="text"
				bind:value={query}
				placeholder="Cari video atau tempel tautan YouTube (https://youtube.com/watch?v=...)..."
				class="h-11 w-full rounded-full border border-border/50 bg-card pl-11 pr-24 text-sm text-foreground placeholder:text-muted-foreground shadow-xs transition-all focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20 md:text-sm"
			/>
			{#if query}
				<button
					type="button"
					class="absolute right-18 top-1/2 z-10 flex h-7 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					onclick={() => {
						query = '';
						switchTab(activeTab);
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

		<!-- 4 Major Sub-Tabs Navigation -->
		<div class="flex items-center gap-2 overflow-x-auto pt-1 scrollbar-none">
			{#each MAIN_TABS as tab (tab.id)}
				{@const isActive = activeTab === tab.id && !query}
				<button
					type="button"
					onclick={() => {
						query = '';
						switchTab(tab.id);
					}}
					class="inline-flex shrink-0 items-center gap-2 rounded-xl px-4 py-2 text-xs font-semibold transition-all cursor-pointer {isActive
						? 'bg-primary text-primary-foreground shadow-xs'
						: 'bg-muted/60 text-muted-foreground hover:bg-muted hover:text-foreground border border-border/30'}"
				>
					<HugeiconsIcon icon={tab.icon} size={15} />
					<span>{tab.label}</span>
					{#if tab.id === 'channels' && channels.length > 0}
						<span class="rounded-full px-1.5 py-0.2 text-[10px] {isActive ? 'bg-primary-foreground/20 text-primary-foreground' : 'bg-foreground/10 text-foreground'}">
							{channels.length}
						</span>
					{/if}
				</button>
			{/each}
		</div>

		<!-- Sub-Category Pills for Trending Tab -->
		{#if activeTab === 'trending' && !query}
			<div class="rail flex items-center gap-1.5 overflow-x-auto pt-1 pb-0.5">
				{#each TRENDING_CATEGORIES as cat (cat.id)}
					<button
						type="button"
						onclick={() => loadTrending(cat.id)}
						class="shrink-0 cursor-pointer rounded-full px-3 py-1 text-xs font-medium transition-colors {activeTrendingCategory === cat.id
							? 'bg-foreground/15 text-foreground font-semibold border border-foreground/20'
							: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
					>
						{cat.label}
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Main Content Area -->
	<div class="px-6 pb-16 pt-5">
		{#if query}
			<!-- Search Results Mode -->
			<div class="content-in flex flex-col gap-5">
				<SectionHeading title={`Hasil Pencarian: "${query}"`} icon={Search01Icon} />

				{#if loading}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _, i (i)}
							<div class="flex flex-col gap-2">
								<Skeleton class="aspect-video w-full rounded-xl" />
								<Skeleton class="h-4 w-3/4 rounded" />
								<Skeleton class="h-3 w-1/2 rounded" />
							</div>
						{/each}
					</div>
				{:else if error}
					<ErrorState message={error} onRetry={handleSearchOrPaste} />
				{:else if videos.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Video01Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada video yang cocok dengan pencarian.</p>
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each videos as v (v.video_id)}
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
										<span class="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1.5 py-0.5 text-[10px] font-semibold text-white/90">
											{v.duration}
										</span>
									{/if}
									<div class="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 transition-opacity group-hover:opacity-100">
										<div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
											<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
										</div>
									</div>
								</div>
								<div class="flex flex-col gap-0.5">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary">
										{v.title}
									</h3>
									<p class="line-clamp-1 text-xs text-muted-foreground">
										{v.artists || 'YouTube Video'}
									</p>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

		{:else if activeTab === 'channels'}
			<!-- Subscribed Channels Tab -->
			<div class="content-in flex flex-col gap-5">
				{#if selectedChannel}
					<!-- Single Channel Videos View -->
					<div class="flex flex-col gap-4">
						<div class="flex items-center justify-between border-b border-border/50 pb-4">
							<div class="flex items-center gap-3">
								<Button
									variant="outline"
									size="sm"
									onclick={() => (selectedChannel = null)}
									class="gap-1 text-xs cursor-pointer"
								>
									<HugeiconsIcon icon={ArrowLeft01Icon} size={14} />
									<span>Semua Channel</span>
								</Button>
								<div class="h-4 w-px bg-border"></div>
								<div class="flex items-center gap-3">
									<img
										src={selectedChannel.avatar_url}
										alt={selectedChannel.title}
										class="h-10 w-10 rounded-full border border-border object-cover shadow-xs"
									/>
									<div>
										<h2 class="text-base font-bold text-foreground leading-tight">{selectedChannel.title}</h2>
										<p class="text-xs text-muted-foreground">{selectedChannel.subscriber_count}</p>
									</div>
								</div>
							</div>
							<Button
								variant="ghost"
								size="sm"
								onclick={() => selectedChannel && loadChannelVideos(selectedChannel, true)}
								class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
							>
								<HugeiconsIcon icon={RefreshIcon} size={14} class={channelLoading ? 'animate-spin' : ''} />
								<span>Segarkan Video</span>
							</Button>
						</div>

						{#if channelLoading}
							<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
								{#each Array(8) as _, i (i)}
									<div class="flex flex-col gap-2">
										<Skeleton class="aspect-video w-full rounded-xl" />
										<Skeleton class="h-4 w-3/4 rounded" />
										<Skeleton class="h-3 w-1/2 rounded" />
									</div>
								{/each}
							</div>
						{:else if channelError}
							<ErrorState message={channelError} onRetry={() => selectedChannel && loadChannelVideos(selectedChannel, true)} />
						{:else if channelVideos.length === 0}
							<div class="flex flex-col items-center justify-center gap-2 py-16 text-center text-muted-foreground">
								<HugeiconsIcon icon={Video01Icon} class="h-10 w-10 opacity-40" />
								<p class="text-sm font-medium">Belum ada video dari channel ini.</p>
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
												<span class="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1.5 py-0.5 text-[10px] font-semibold text-white/90">
													{v.duration}
												</span>
											{/if}
											<div class="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 transition-opacity group-hover:opacity-100">
												<div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
													<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
												</div>
											</div>
										</div>
										<div class="flex flex-col gap-0.5">
											<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
												{v.title}
											</h3>
											<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
												<span class="line-clamp-1">{v.artists || selectedChannel?.title || 'YouTube Creator'}</span>
											</div>
											{#if v.album}
												<p class="line-clamp-1 text-xs text-muted-foreground/80">
													{v.album}
												</p>
											{/if}
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>

				{:else}
					<!-- Channel Directory Grid -->
					<div class="flex flex-col gap-4">
						<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
							<SectionHeading
								title={`Channel Langganan (${channels.length})`}
								icon={UserGroup02Icon}
							/>
							<!-- Search filter input -->
							<div class="relative max-w-xs w-full">
								<HugeiconsIcon
									icon={Search01Icon}
									class="pointer-events-none absolute left-3 top-1/2 z-10 h-4 w-4 -translate-y-1/2 text-muted-foreground"
								/>
								<input
									type="text"
									bind:value={channelFilter}
									placeholder="Cari channel saya..."
									class="h-9 w-full rounded-lg border border-border/60 bg-card pl-9 pr-7 text-xs text-foreground placeholder:text-muted-foreground shadow-xs focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
								/>
								{#if channelFilter}
									<button
										type="button"
										class="absolute right-2 top-1/2 -translate-y-1/2 cursor-pointer text-muted-foreground hover:text-foreground"
										onclick={() => (channelFilter = '')}
									>
										<HugeiconsIcon icon={Cancel01Icon} size={13} />
									</button>
								{/if}
							</div>
						</div>

						{#if loading}
							<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
								{#each Array(10) as _, i (i)}
									<div class="flex flex-col items-center gap-2 p-4 rounded-xl border border-border/40 bg-card/50">
										<Skeleton class="h-16 w-16 rounded-full" />
										<Skeleton class="h-4 w-24 rounded" />
										<Skeleton class="h-3 w-16 rounded" />
									</div>
								{/each}
							</div>
						{:else if error}
							<ErrorState message={error} onRetry={() => loadChannels(true)} />
						{:else if filteredChannels.length === 0}
							<div class="flex flex-col items-center justify-center gap-2 py-16 text-center text-muted-foreground">
								<HugeiconsIcon icon={UserGroup02Icon} class="h-12 w-12 opacity-40" />
								<p class="text-sm font-medium">Tidak ada channel yang cocok.</p>
							</div>
						{:else}
							<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
								{#each filteredChannels as chan (chan.id)}
									<div
										role="button"
										tabindex="0"
										onclick={() => loadChannelVideos(chan)}
										onkeydown={(e) => e.key === 'Enter' && loadChannelVideos(chan)}
										class="group flex flex-col items-center text-center p-3.5 rounded-xl border border-border/40 bg-card hover:bg-accent/15 hover:border-primary/40 transition-all cursor-pointer shadow-xs hover:shadow-md"
									>
										<div class="relative mb-2">
											{#if chan.avatar_url}
												<img
													src={chan.avatar_url}
													alt={chan.title}
													class="h-16 w-16 rounded-full object-cover border border-border/60 transition-transform duration-300 group-hover:scale-105 shadow-xs"
													loading="lazy"
												/>
											{:else}
												<div class="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 text-primary font-bold text-lg">
													{chan.title.slice(0, 1)}
												</div>
											{/if}
										</div>
										<h3 class="line-clamp-1 text-xs font-semibold text-foreground group-hover:text-primary transition-colors">
											{chan.title}
										</h3>
										{#if chan.subscriber_count}
											<p class="mt-0.5 line-clamp-1 text-[11px] text-muted-foreground">
												{chan.subscriber_count}
											</p>
										{/if}
										<span class="mt-2.5 rounded-md bg-foreground/5 px-2 py-0.5 text-[10px] font-medium text-foreground/80 group-hover:bg-primary group-hover:text-primary-foreground transition-colors">
											Lihat Video
										</span>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

		{:else}
			<!-- Subscriptions, Trending, or Recommendations Tabs -->
			<div class="content-in flex flex-col gap-5">
				<SectionHeading
					title={activeTab === 'subscriptions'
						? 'Video Terbaru Langganan'
						: activeTab === 'trending'
							? `Trending: ${TRENDING_CATEGORIES.find((c) => c.id === activeTrendingCategory)?.label || 'Populer'}`
							: 'Rekomendasi YouTube Anda'}
					icon={activeTab === 'subscriptions'
						? Notification01Icon
						: activeTab === 'trending'
							? Fire02Icon
							: SparklesIcon}
				/>

				{#if loading && videos.length === 0}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each Array(8) as _, i (i)}
							<div class="flex flex-col gap-2">
								<Skeleton class="aspect-video w-full rounded-xl" />
								<Skeleton class="h-4 w-3/4 rounded" />
								<Skeleton class="h-3 w-1/2 rounded" />
							</div>
						{/each}
					</div>
				{:else if error && videos.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-14 px-4 text-center rounded-2xl border border-border/40 bg-card/60">
						<HugeiconsIcon icon={Video01Icon} class="h-10 w-10 text-muted-foreground opacity-60" />
						<div class="space-y-1 max-w-md">
							<h3 class="text-sm font-bold text-foreground">Gagal Memuat Konten</h3>
							<p class="text-xs text-muted-foreground leading-relaxed">{error}</p>
						</div>
						{#if !auth.account?.signedIn && (activeTab === 'subscriptions' || activeTab === 'recommendations')}
							<Button size="sm" onclick={() => api.loginWebview()} class="mt-2 text-xs cursor-pointer">
								Masuk dengan Akun Google
							</Button>
						{:else}
							<Button variant="outline" size="sm" onclick={() => switchTab(activeTab, true)} class="mt-2 text-xs cursor-pointer">
								Coba Lagi
							</Button>
						{/if}
					</div>
				{:else if videos.length === 0}
					<div class="flex flex-col items-center justify-center gap-3 py-16 text-center text-muted-foreground">
						<HugeiconsIcon icon={Video01Icon} class="h-12 w-12 opacity-40" />
						<p class="text-sm font-medium">Tidak ada video yang ditemukan.</p>
						{#if !auth.account?.signedIn && (activeTab === 'subscriptions' || activeTab === 'recommendations')}
							<p class="text-xs text-muted-foreground/80 max-w-sm">
								Hubungkan akun Google Anda untuk menyinkronkan video subscription dan rekomendasi otomatis.
							</p>
							<Button size="sm" onclick={() => api.loginWebview()} class="mt-1 text-xs cursor-pointer">
								Masuk Akun Google
							</Button>
						{/if}
					</div>
				{:else}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
						{#each videos as v (v.video_id)}
							<div
								role="button"
								tabindex="0"
								onclick={() => playVideoTrack(v)}
								onkeydown={(e) => e.key === 'Enter' && playVideoTrack(v)}
								class="group flex cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
								style="content-visibility: auto; contain-intrinsic-size: auto 240px;"
							>
								<!-- 16:9 Thumbnail with duration & play button -->
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
										<span class="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1.5 py-0.5 text-[10px] font-semibold text-white/90">
											{v.duration}
										</span>
									{/if}

									<!-- Hover play overlay -->
									<div class="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 transition-opacity group-hover:opacity-100">
										<div class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
											<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
										</div>
									</div>
								</div>

								<!-- Title and channel line matching Limusic cards -->
								<div class="flex flex-col gap-0.5">
									<h3 class="line-clamp-2 text-sm font-medium leading-snug text-foreground group-hover:text-primary transition-colors">
										{v.title}
									</h3>
									<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
										<span class="line-clamp-1">{v.artists || 'YouTube Creator'}</span>
									</div>
									{#if v.album}
										<span class="line-clamp-1 text-[11px] text-muted-foreground/80">
											{v.album}
										</span>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
