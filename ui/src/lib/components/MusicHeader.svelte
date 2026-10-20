<script lang="ts">
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { HistoryIcon, Search01Icon } from '@hugeicons/core-free-icons';
	import SearchSuggest from '$lib/components/SearchSuggest.svelte';
	import HeroBackdrop from '$lib/components/HeroBackdrop.svelte';
	import { personal } from '$lib/player.svelte';
	import { t } from '$lib/i18n.svelte';

	let searchQuery = $state('');

	function goSearch() {
		if (!searchQuery.trim()) return;
		goto(`/search?${new URLSearchParams({ q: searchQuery }).toString()}`);
	}
</script>

<header class="relative">
	<!-- The artwork wash is the one "look" setting that still belongs to Musik: Edit Musik's
	     "Artwork backdrop" toggles it. Beranda always draws its own. -->
	<HeroBackdrop height="18rem" enabled={personal.home.backdrop} />

	<div class="flex items-center justify-between gap-6 px-6 pb-3 pt-8">
		<div class="min-w-0">
			<h1 class="truncate font-heading text-3xl font-bold tracking-tight drop-shadow-sm">
				{t('music.title')}
			</h1>
			<p class="mt-1 truncate text-sm text-muted-foreground">{t('music.subtitle')}</p>
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
