<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		ArrowLeft01Icon,
		ArrowRight01Icon,
		UserGroup02Icon,
		UserIcon
	} from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import type { SubscribedChannel } from '$lib/api';
	import { t } from '$lib/i18n.svelte';

	let {
		channels,
		onSelectChannel,
		onMore
	}: {
		channels: SubscribedChannel[];
		onSelectChannel: (c: SubscribedChannel) => void;
		onMore?: () => void;
	} = $props();

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
		row?.scrollBy({ left: dir * Math.round(row.clientWidth * 0.8), behavior: 'smooth' });
	}

	$effect(() => {
		channels;
		if (hovered) update();
	});
</script>

<svelte:window onresize={update} />

{#if channels.length > 0}
	<section class="[content-visibility:auto]">
		<SectionHeading
			title="Channel Langganan Saya"
			icon={UserGroup02Icon}
			{onMore}
		/>

		<div class="group/shelf relative" {@attach measureOnEnter}>
			<div
				class="rail flex snap-x gap-4 overflow-x-auto pb-3 pt-1"
				bind:this={row}
				onscroll={update}
			>
				{#each channels as channel (channel.id)}
					<button
						type="button"
						onclick={() => onSelectChannel(channel)}
						class="group flex w-36 shrink-0 cursor-pointer snap-start flex-col items-center gap-2.5 rounded-2xl p-2.5 text-center transition-all hover:bg-accent/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
					>
						<!-- Channel Avatar with ring -->
						<div class="relative h-20 w-20 overflow-hidden rounded-full border-2 border-border/60 bg-muted shadow-xs transition-transform duration-200 group-hover:scale-105 group-hover:border-primary/60">
							{#if channel.avatar_url}
								<img
									src={channel.avatar_url}
									alt={channel.title}
									class="h-full w-full object-cover"
									loading="lazy"
								/>
							{:else}
								<div class="flex h-full w-full items-center justify-center bg-muted text-muted-foreground">
									<HugeiconsIcon icon={UserIcon} class="h-8 w-8" />
								</div>
							{/if}
						</div>

						<!-- Channel Name and Subs -->
						<div class="flex w-full flex-col items-center gap-0.5">
							<span class="w-full truncate text-xs font-semibold text-foreground transition-colors group-hover:text-primary" title={channel.title}>
								{channel.title}
							</span>
							{#if channel.subscriber_count}
								<span class="w-full truncate text-[11px] text-muted-foreground/80">
									{channel.subscriber_count}
								</span>
							{/if}
						</div>
					</button>
				{/each}
			</div>

			<!-- Shelf Navigation Arrows -->
			{#if canLeft}
				<div class="pointer-events-none absolute inset-y-0 left-0 w-16 bg-gradient-to-r from-background to-transparent"></div>
				<button
					aria-label={t('a11y.scroll_left')}
					onclick={() => page(-1)}
					class="absolute left-1 top-1/2 flex h-9 w-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-full border bg-background text-foreground opacity-0 shadow-lg transition hover:scale-105 focus-visible:opacity-100 group-hover/shelf:opacity-100"
				>
					<HugeiconsIcon icon={ArrowLeft01Icon} class="h-4 w-4" />
				</button>
			{/if}
			{#if canRight}
				<div class="pointer-events-none absolute inset-y-0 right-0 w-16 bg-gradient-to-l from-background to-transparent"></div>
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
