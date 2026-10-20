<script lang="ts">
	// Vocabulary progress, straight from the store the layout already hydrates: no request, no
	// skeleton longer than `vocab.loaded`. Words are marked while reading or watching, so the card
	// points back there rather than at a vocabulary page that does not exist.
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import { SparklesIcon } from '@hugeicons/core-free-icons';
	import SectionHeading from './SectionHeading.svelte';
	import { Button } from '$lib/components/ui/button';
	import { vocab } from '$lib/vocab.svelte';
	import { t } from '$lib/i18n.svelte';

	// `vocab.loaded` never flips if `init()` throws before writing counts; Beranda is the landing
	// page, so a permanent skeleton is worse than showing the honest empty state.
	let timedOut = $state(false);
	onMount(() => {
		const id = setTimeout(() => (timedOut = true), 1200);
		return () => clearTimeout(id);
	});
	const showSkeleton = $derived(!vocab.loaded && !timedOut);

	const total = $derived(
		(vocab.counts.known ?? 0) + (vocab.counts.learning ?? 0) + (vocab.counts.flashcards ?? 0)
	);
	const stats = $derived([
		{ label: t('home.words_known'), value: vocab.counts.known ?? 0 },
		{ label: t('home.words_learning'), value: vocab.counts.learning ?? 0 },
		{ label: t('home.flashcards'), value: vocab.counts.flashcards ?? 0 }
	]);
</script>

<section>
	<SectionHeading title={t('home.learning')} icon={SparklesIcon} />
	<div class="rounded-2xl border bg-card p-5">
		{#if showSkeleton}
			<!-- A real zero and a not-loaded-yet zero look the same; hold the shape rather than
			     flash "0 words" on every launch. -->
			<div class="grid grid-cols-3 gap-6" aria-hidden="true">
				{#each Array(3) as _, i (i)}
					<div>
						<div class="h-7 w-14 animate-pulse rounded bg-muted"></div>
						<div class="mt-2 h-3 w-20 animate-pulse rounded bg-muted"></div>
					</div>
				{/each}
			</div>
		{:else if total === 0}
			<div class="flex flex-col items-start gap-3 sm:flex-row sm:items-center sm:justify-between">
				<p class="max-w-xl text-sm text-muted-foreground">{t('home.learning_empty')}</p>
				<Button size="sm" onclick={() => goto('/articles')}>{t('home.learning_cta')}</Button>
			</div>
		{:else}
			<div class="flex flex-col gap-5 sm:flex-row sm:items-center sm:justify-between">
				<div class="grid grid-cols-3 gap-6">
					{#each stats as stat (stat.label)}
						<div>
							<div class="text-2xl font-semibold tabular-nums">{stat.value.toLocaleString()}</div>
							<div class="mt-0.5 text-xs text-muted-foreground">{stat.label}</div>
						</div>
					{/each}
				</div>
				<Button variant="outline" size="sm" onclick={() => goto('/articles')}>
					{t('home.learning_cta')}
				</Button>
			</div>
		{/if}
	</div>
</section>