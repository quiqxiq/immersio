<script lang="ts">
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { HistoryIcon, Search01Icon } from '@hugeicons/core-free-icons';
	import SearchSuggest from '$lib/components/SearchSuggest.svelte';
	import HeroBackdrop from '$lib/components/HeroBackdrop.svelte';
	import { auth } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';
	import { t, type TranslationKey } from '$lib/i18n.svelte';

	// Fixed at mount — a greeting that flips mid-session is uncanny.
	const hour = new Date().getHours();
	const daypartKey: TranslationKey =
		hour < 5
			? 'home.good_night'
			: hour < 12
				? 'home.good_morning'
				: hour < 18
					? 'home.good_afternoon'
					: 'home.good_evening';
	const daypart = $derived(t(daypartKey));

	let searchQuery = $state('');

	function goSearch() {
		if (!searchQuery.trim()) return;
		goto(`/search?${new URLSearchParams({ q: searchQuery }).toString()}`);
	}
</script>

<header class="relative">
	<!-- The canvas runs past the header, under the first sections, and fades into the page instead
	     of ending at a line. The wrapper page opens the stacking context (`isolate`), so -z-10 can
	     never slip behind the window's own background. -->
	<HeroBackdrop height="30rem" />

	<div class="flex items-center justify-between gap-6 px-6 pb-4 pt-10">
		<div class="flex min-w-0 items-center gap-4">
			{#if auth.account?.signedIn && auth.account.thumbnail}
				<!-- max-width:none defeats Tailwind Preflight's `img{max-width:100%}`, which in a tight box
				     clamps width to the content-box while height stays fixed → a vertical oval. Inline so
				     it's immune to Preflight and to stale dev CSS. -->
				<img
					src={thumb(auth.account.thumbnail, 128)}
					alt=""
					style="width:3rem;height:3rem;max-width:none"
					class="shrink-0 rounded-full object-cover shadow-md"
				/>
			{/if}
			<h1 class="truncate font-heading text-4xl font-bold tracking-tight drop-shadow-sm">
				{daypart}{auth.account?.name ? `, ${auth.account.name.split(' ')[0]}` : ''}
			</h1>
		</div>
		<div class="flex shrink-0 items-center gap-2">
			<button
				onclick={() => goto('/history')}
				title={t('nav.history')}
				aria-label={t('nav.history')}
				class="flex h-9 w-9 shrink-0 cursor-pointer items-center justify-center rounded-full border border-border text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
			>
				<HugeiconsIcon icon={HistoryIcon} class="h-5 w-5" />
			</button>
			<form class="relative w-full max-w-xs" onsubmit={(e) => { e.preventDefault(); goSearch(); }}>
				<HugeiconsIcon
					icon={Search01Icon}
					class="pointer-events-none absolute left-3 top-1/2 z-10 h-4 w-4 -translate-y-1/2 text-muted-foreground"
				/>
				<!-- The panel is wider than this field and hangs off its right edge: the rows carry
				     artwork and two lines of text, which 20rem can't hold. -->
				<SearchSuggest
					bind:value={searchQuery}
					placeholder={t('common.search')}
					inputClass="rounded-full pl-9"
					panelClass="right-0 w-[26rem]"
				/>
			</form>
		</div>
	</div>
</header>
