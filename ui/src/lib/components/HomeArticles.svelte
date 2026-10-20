<script lang="ts">
	// Three headlines from the same feed the Articles page reads, and the same client cache key, so
	// opening Articles after Beranda is instant. A dashboard row, not a reader: no images, no
	// highlighting, just enough to decide whether to open one.
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { ArrowRight01Icon, News01Icon } from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import * as api from '$lib/api';
	import type { ArticleCard } from '$lib/api';
	import { getCached, putCached } from '$lib/pagecache';
	import { t } from '$lib/i18n.svelte';

	const KEY = 'articles:feed:all';
	let articles = $state<ArticleCard[]>([]);
	let loading = $state(true);

	onMount(() => {
		const cached = getCached<ArticleCard[]>(KEY);
		if (cached) {
			articles = cached;
			loading = false;
			return;
		}
		api
			.getArticlesFeed()
			.then((res) => {
				articles = res || [];
				if (articles.length) putCached(KEY, articles);
			})
			.catch(() => {
				// Offline, or the feeds are down: Beranda loses one section, not the whole page.
			})
			.finally(() => (loading = false));
	});

	/** The same parameters the Articles page builds in `openArticle`. */
	const href = (a: ArticleCard) =>
		`/articles/read?${new URLSearchParams({ url: a.link, title: a.title, source: a.source }).toString()}`;
</script>

{#if loading}
	<section aria-hidden="true">
		<div class="mb-4 h-6 w-40 animate-pulse rounded bg-muted"></div>
		<div class="divide-y overflow-hidden rounded-2xl border bg-card">
			{#each Array(3) as _, i (i)}
				<div class="flex items-center gap-3 px-4 py-3.5">
					<div class="h-4 flex-1 animate-pulse rounded bg-muted"></div>
					<div class="h-4 w-8 animate-pulse rounded bg-muted"></div>
				</div>
			{/each}
		</div>
	</section>
{:else if articles.length}
	<section>
		<SectionHeading
			title={t('home.latest_articles')}
			icon={News01Icon}
			onMore={() => goto('/articles')}
		/>
		<div class="divide-y overflow-hidden rounded-2xl border bg-card">
			{#each articles.slice(0, 3) as a (a.link)}
				<a href={href(a)} class="group flex items-center gap-4 px-4 py-3.5 transition-colors hover:bg-accent/5">
					<div class="min-w-0 flex-1">
						<div class="truncate text-sm font-medium group-hover:text-primary">{a.title}</div>
						<div class="mt-1 truncate text-xs text-muted-foreground">
							{a.source}{#if a.reading_time} · {t('home.min_read', { count: a.reading_time })}{/if}
						</div>
					</div>
					<HugeiconsIcon
						icon={ArrowRight01Icon}
						class="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5"
					/>
				</a>
			{/each}
		</div>
	</section>
{/if}