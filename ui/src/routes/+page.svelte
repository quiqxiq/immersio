<script lang="ts">
	// Beranda: the personal dashboard. It answers "what should I do now?" across the app's
	// sections — continue listening, learning progress, a shadowing scene, a fresh article — where
	// /music is the YouTube Music feed and /library is the collection. Everything here is built
	// from stores Beranda already has or one cached request; nothing blocks the greeting.
	import GreetingHero from '$lib/components/GreetingHero.svelte';
	import HomeQuickLinks from '$lib/components/HomeQuickLinks.svelte';
	import HomeLearning from '$lib/components/HomeLearning.svelte';
	import HomeShadowing from '$lib/components/HomeShadowing.svelte';
	import HomeArticles from '$lib/components/HomeArticles.svelte';
	import RecentRail from '$lib/components/RecentRail.svelte';
	import { library, personal } from '$lib/player.svelte';
	import { freshen, recentItems } from '$lib/personal';

	// Unlike Musik's recents, nothing is filtered against the Shortcuts grid: Beranda has no grid to
	// duplicate. Six fills three columns without turning the row into a list.
	const recent = $derived(recentItems(personal, 6).map((r) => freshen(r, library.items)));
</script>

<!-- isolate: the greeting's artwork canvas sits at -z-10 and must stay inside this page, above the
     window's background rather than behind it. -->
<div class="relative isolate">
	<GreetingHero />

	<div class="content-in flex flex-col gap-10 px-6 pb-12 pt-2">
		<HomeQuickLinks />
		{#if recent.length}
			<!-- Beranda shows recents as re-entry points, not as things to drag into Musik's
			     Shortcuts grid or manage from a menu. -->
			<RecentRail items={recent} draggable={false} menu={false} />
		{/if}
		<HomeLearning />
		<HomeShadowing />
		<HomeArticles />
	</div>
</div>