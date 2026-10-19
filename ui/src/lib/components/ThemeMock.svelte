<script lang="ts">
	// A miniature of the app (sidebar, a shelf of cards, the player bar) in one palette. It carries
	// `theme-<id>` and `dark` itself, so every token re-resolves for this subtree alone, and the
	// overrides land as inline vars here the way `apply` puts them on <html>. That is what lets the
	// Appearance tab follow a slider live: dragging restyles a few dozen elements instead of the
	// whole document (docs/UI-PERFORMANCE.md, "Per-track and per-tick writes land on a leaf").
	//
	// Sized in em off the root's font-size. Radii are scaled by --k: a real corner on a card this
	// small would read as a pill.
	import { onAccent, type Custom, type ThemeId } from '$lib/theme.svelte';

	type Overrides = Pick<Custom, 'accent' | 'hue' | 'radius' | 'fontSans' | 'fontHeading'>;

	let {
		id,
		dark,
		overrides,
		detail
	}: {
		id: ThemeId;
		dark: boolean;
		overrides: Overrides;
		detail: { heading: string; title: string; artist: string; play: string; cover?: string };
	} = $props();

	const fg = $derived(overrides.accent ? onAccent(overrides.accent) : null);
	let coverFailed = $state(false);
	$effect(() => {
		void detail.cover;
		coverFailed = false;
	});
</script>

<div
	class="mock theme-{id} {dark ? 'dark' : ''}"
	style:--primary={overrides.accent}
	style:--primary-foreground={fg}
	style:--accent={overrides.accent}
	style:--accent-foreground={fg}
	style:--hue={overrides.hue === null ? null : String(overrides.hue)}
	style:--radius={overrides.radius === null ? null : `${overrides.radius}rem`}
	style:--font-sans={overrides.fontSans}
	style:--font-heading={overrides.fontHeading}
	aria-hidden="true"
>
	<div class="body">
		<div class="side">
			<i class="logo"></i>
			<i class="nav on"></i>
			<i class="nav"></i>
			<i class="nav short"></i>
			<i class="nav"></i>
		</div>
		<div class="main">
			<p class="heading">{detail.heading}</p>
			<span class="pill">{detail.play}</span>
			<!-- Cut off at the bottom when it doesn't fit, like a page that scrolls on. -->
			<div class="shelf">
				{#each [0, 1, 2] as n (n)}
					<div class="card">
						<i class="art"></i>
						<i class="line"></i>
						<i class="line faint"></i>
					</div>
				{/each}
			</div>
		</div>
	</div>
	<div class="bar">
		{#if detail.cover && !coverFailed}
			<img class="cover" src={detail.cover} alt="" onerror={() => (coverFailed = true)} />
		{:else}
			<i class="cover"></i>
		{/if}
		<div class="meta">
			<span class="title">{detail.title}</span>
			<span class="artist">{detail.artist}</span>
		</div>
		<div class="track"><i class="fill"></i></div>
		<i class="play"></i>
	</div>
</div>
