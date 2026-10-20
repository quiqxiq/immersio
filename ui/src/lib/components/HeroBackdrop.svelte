<script lang="ts">
	// The blurred artwork canvas behind a page hero. Shared by Beranda's greeting and Musik's
	// compact header so the WebKitGTK workarounds stay in one place: `art-wash` on the canvas, not
	// the image, so the wash and the gradients that fade it out rasterize into one layer. With the
	// image promoted alone, an animated wheel scroll put the two layers a pixel apart at some
	// offsets, and a row of the raw wash flickered under the fade's last line
	// (perf/navprobe.py --hold --wheel: 8 of 20 captures).
	//
	// `enabled` is the Musik header's "Artwork backdrop" setting. Beranda always passes true: its
	// greeting should not change because a feed-layout toggle was switched off.
	import { playback } from '$lib/player.svelte';
	import { thumb } from '$lib/thumb';

	let {
		height = '30rem',
		enabled = true
	}: {
		height?: string;
		enabled?: boolean;
	} = $props();

	// Google's CDN doesn't serve every rewritten size, so a 404'd backdrop must degrade to nothing
	// rendered, never a broken-image glyph. Re-arm whenever the track changes.
	let artFailed = $state(false);
	$effect(() => {
		playback.now?.thumbnail; // re-arm when the track changes
		artFailed = false;
	});
</script>

{#if enabled}
	<div
		class="art-wash pointer-events-none absolute inset-x-0 top-0 -z-10 overflow-hidden"
		style="height:{height}"
		aria-hidden="true"
	>
		{#if playback.now?.thumbnail && !artFailed}
			<!-- 96px, not display size: blur-2xl is a 40px blur, so every detail above a handful of
			     pixels is thrown away anyway. The old 1200px source decoded to 5.7 MiB for this, and
			     re-decoded on every track change. -->
			<img
				src={thumb(playback.now.thumbnail, 96)}
				alt=""
				class="absolute inset-0 h-full w-full scale-110 object-cover opacity-70 blur-2xl"
				onerror={() => (artFailed = true)}
			/>
		{:else}
			<!-- Nothing playing, or the backdrop switched off: an accent wash keeps it a header rather
			     than a bare box. Inline style so it can't be lost to a stale dev stylesheet, and it
			     rides --primary so every preset theme gets its own. -->
			<div
				class="absolute inset-0 opacity-[0.2]"
				style="background:radial-gradient(90% 75% at 10% 0%, var(--primary) 0%, transparent 70%)"
			></div>
		{/if}
		<!-- Ends on the page's own background exactly at the canvas's bottom edge, so there is no seam. -->
		<div
			class="absolute inset-0 bg-gradient-to-b from-background/25 via-background/70 to-background"
		></div>
		<div
			class="absolute inset-0 bg-gradient-to-r from-background/60 via-background/15 to-transparent"
		></div>
	</div>
{/if}
