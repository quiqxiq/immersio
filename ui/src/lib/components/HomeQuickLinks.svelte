<script lang="ts">
	// The four doors out of Beranda. Not "shortcuts" in Musik's sense (that grid is curated music
	// the user pinned); these are the app's own sections, and they stay fixed.
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { Film02Icon, MusicNote01Icon, News01Icon, Video01Icon } from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import { t } from '$lib/i18n.svelte';

	const links = $derived([
		{ href: '/music', icon: MusicNote01Icon, title: t('home.quick_music'), desc: t('home.quick_music_desc') },
		{ href: '/videos', icon: Video01Icon, title: t('home.quick_videos'), desc: t('home.quick_videos_desc') },
		{ href: '/shadowing', icon: Film02Icon, title: t('home.quick_shadowing'), desc: t('home.quick_shadowing_desc') },
		{ href: '/articles', icon: News01Icon, title: t('home.quick_articles'), desc: t('home.quick_articles_desc') }
	]);
</script>

<section>
	<SectionHeading title={t('home.quick_start')} />
	<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{#each links as link (link.href)}
			<!-- A plain <a>: these are navigation, and the browser's own middle-click/back behavior
			     should keep working. -->
			<a
				href={link.href}
				class="group flex cursor-pointer items-center gap-3 rounded-xl border bg-card p-4 transition-colors hover:border-foreground/20 hover:bg-accent/5"
			>
				<span
					class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
				>
					<HugeiconsIcon icon={link.icon} class="h-5 w-5" />
				</span>
				<span class="min-w-0">
					<span class="block truncate text-sm font-semibold">{link.title}</span>
					<span class="mt-0.5 block truncate text-xs text-muted-foreground">{link.desc}</span>
				</span>
			</a>
		{/each}
	</div>
</section>