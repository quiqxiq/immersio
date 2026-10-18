<script lang="ts">
	import { HugeiconsIcon, type IconSvgElement } from '@hugeicons/svelte';
	import {
		ArrowLeft01Icon,
		ArrowRight01Icon,
		PlayIcon,
		Video01Icon
	} from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import { thumb } from '$lib/thumb';
	import type { SongItem } from '$lib/api';
	import type { CardSize } from '$lib/personal';
	import { t } from '$lib/i18n.svelte';

	let {
		title,
		icon,
		items,
		onMore,
		onPlay,
		size = 'medium'
	}: {
		title?: string;
		icon?: IconSvgElement;
		items: SongItem[];
		onMore?: () => void;
		onPlay?: (item: SongItem) => void;
		size?: CardSize;
	} = $props();

	const WIDTH: Record<CardSize, number> = { small: 14, medium: 18, large: 22 };
	const cardStyle = $derived(`width: ${WIDTH[size]}rem;`);
	const heightStyle = $derived(`contain-intrinsic-size: auto ${WIDTH[size] * 0.5625 + 6.5}rem;`);

	let row = $state<HTMLDivElement | null>(null);
	let canLeft = $state(false);
	let canRight = $state(false);
	let hovered = false;

	function update() {
		if (!row) return;
		canLeft = row.scrollLeft > 4;
		canRight = row.scrollLeft + row.clientWidth < row.scrollWidth - 4;
	}

	const measureOnEnter = (el: HTMLElement) => {
		const enter = () => {
			hovered = true;
			update();
		};
		const leave = () => (hovered = false);
		el.addEventListener('pointerenter', enter);
		el.addEventListener('pointerleave', leave);
		return () => {
			el.removeEventListener('pointerenter', enter);
			el.removeEventListener('pointerleave', leave);
		};
	};

	function page(dir: 1 | -1) {
		row?.scrollBy({ left: dir * Math.round(row.clientWidth * 0.85), behavior: 'smooth' });
	}

	$effect(() => {
		items;
		if (hovered) update();
	});
</script>

<svelte:window onresize={update} />

{#if items.length > 0}
	<section class="[content-visibility:auto]" style={heightStyle}>
		{#if title || onMore}
			<SectionHeading title={title ?? ''} {icon} {onMore} />
		{/if}

		<div class="group/shelf relative" {@attach measureOnEnter}>
			<div
				class="rail flex snap-x gap-3 overflow-x-auto pb-2"
				bind:this={row}
				onscroll={update}
			>
				{#each items as v (v.video_id)}
					<div
						class="min-w-0 shrink-0 snap-start"
						style={cardStyle}
					>
						<div
							role="button"
							tabindex="0"
							onclick={() => onPlay?.(v)}
							onkeydown={(e) => e.key === 'Enter' && onPlay?.(v)}
							class="group flex w-full cursor-pointer flex-col gap-2 rounded-xl p-2 text-left transition-colors hover:bg-accent/10"
						>
							<!-- 16:9 Video Thumbnail with duration and play overlay -->
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

								<!-- Hover Play Overlay -->
								<div class="absolute inset-0 flex items-center justify-center bg-black/35 opacity-0 transition-opacity group-hover:opacity-100">
									<div class="flex h-11 w-11 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-transform group-hover:scale-110">
										<HugeiconsIcon icon={PlayIcon} class="h-5 w-5 fill-current" />
									</div>
								</div>
							</div>

							<!-- Title & Channel & Badge -->
							<div class="flex flex-col gap-1">
								<h3
									class="line-clamp-2 text-sm font-medium leading-snug text-foreground transition-colors group-hover:text-primary"
									title={v.title}
								>
									{v.title}
								</h3>
								<div class="flex items-center gap-1.5 text-xs text-muted-foreground">
									<span class="line-clamp-1">{v.artists || 'YouTube Creator'}</span>
								</div>
								{#if v.album}
									<span class="line-clamp-1 text-[11px] text-muted-foreground/75">
										{v.album}
									</span>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</div>

			<!-- Shelf Navigation Arrows with subtle gradient fades -->
			{#if canLeft}
				<div
					class="pointer-events-none absolute inset-y-0 left-0 w-16 bg-gradient-to-r from-background to-transparent"
				></div>
				<button
					aria-label={t('a11y.scroll_left')}
					onclick={() => page(-1)}
					class="absolute left-1 top-1/2 flex h-9 w-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full border bg-background text-foreground opacity-0 shadow-lg transition hover:scale-105 focus-visible:opacity-100 group-hover/shelf:opacity-100"
				>
					<HugeiconsIcon icon={ArrowLeft01Icon} class="h-4 w-4" />
				</button>
			{/if}
			{#if canRight}
				<div
					class="pointer-events-none absolute inset-y-0 right-0 w-16 bg-gradient-to-l from-background to-transparent"
				></div>
				<button
					aria-label={t('a11y.scroll_right')}
					onclick={() => page(1)}
					class="absolute right-1 top-1/2 flex h-9 w-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full border bg-background text-foreground opacity-0 shadow-lg transition hover:scale-105 focus-visible:opacity-100 group-hover/shelf:opacity-100"
				>
					<HugeiconsIcon icon={ArrowRight01Icon} class="h-4 w-4" />
				</button>
			{/if}
		</div>
	</section>
{/if}
