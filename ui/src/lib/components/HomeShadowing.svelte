<script lang="ts">
	// A taste of Shadowing on Beranda. `starter` is the Rust preset collection, so this costs no
	// YouTube round-trip; the cache key is the same one the Shadowing page uses, which makes a
	// previous visit instant.
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { Film02Icon } from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import VideoShelf from './VideoShelf.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as api from '$lib/api';
	import type { SongItem } from '$lib/api';
	import { getCached, putCached } from '$lib/pagecache';
	import { startShadowingScene } from '$lib/shadowing';
	import { t } from '$lib/i18n.svelte';

	const KEY = 'shadowing:scenes:starter';
	let items = $state<SongItem[]>([]);
	let loading = $state(true);

	onMount(() => {
		const cached = getCached<SongItem[]>(KEY);
		if (cached) {
			items = cached;
			loading = false;
			return;
		}
		api
			.getShadowingScenes('starter')
			.then((res) => {
				items = res || [];
				if (items.length) putCached(KEY, items);
			})
			.catch(() => {})
			.finally(() => (loading = false));
	});
</script>

{#if loading}
	<section aria-hidden="true">
		<div class="mb-4 h-6 w-48 animate-pulse rounded bg-muted"></div>
		<div class="flex gap-3 overflow-hidden pb-2">
			{#each Array(4) as _, i (i)}
				<div class="h-40 w-64 shrink-0 animate-pulse rounded-xl bg-muted"></div>
			{/each}
		</div>
	</section>
{:else if items.length}
	<VideoShelf
		title={t('home.shadowing_title')}
		icon={Film02Icon}
		items={items.slice(0, 4)}
		onMore={() => goto('/shadowing')}
		onPlay={startShadowingScene}
	/>
{:else}
	<!-- The preset should always load; if it does not, hand over the door rather than an empty row. -->
	<section>
		<SectionHeading title={t('home.shadowing_title')} icon={Film02Icon} />
		<div
			class="flex flex-col items-start gap-3 rounded-2xl border bg-card p-5 sm:flex-row sm:items-center sm:justify-between"
		>
			<p class="max-w-xl text-sm text-muted-foreground">{t('home.shadowing_empty')}</p>
			<Button variant="outline" size="sm" onclick={() => goto('/shadowing')}>
				{t('common.see_all')}
			</Button>
		</div>
	</section>
{/if}