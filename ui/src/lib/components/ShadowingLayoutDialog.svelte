<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { Dialog as DialogPrimitive, RadioGroup } from 'bits-ui';
	import { HugeiconsIcon, type IconSvgElement } from '@hugeicons/svelte';
	import {
		CameraSmile02Icon,
		CameraVideoIcon,
		Cancel01Icon,
		ClapperboardIcon,
		DashboardSquareEditIcon,
		DragDropVerticalIcon,
		Film01Icon,
		Film02Icon,
		ListStartIcon,
		ModernTvIcon,
		RefreshIcon,
		SparklesIcon,
		StarAward01Icon,
		TheaterIcon,
		Tick02Icon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import * as Dialog from '$lib/components/ui/dialog';
	import { dragScroll, SECTION_ROW_MIME } from '$lib/dnd';
	import { personal, resetShadowing, saveShadowing } from '$lib/player.svelte';
	import {
		CARD_SIZES,
		hiddenShadowingSections,
		shadowingIsDefault,
		type CardSize
	} from '$lib/personal';
	import { t } from '$lib/i18n.svelte';
	import { onWindowChrome } from '$lib/utils';

	let {
		open = $bindable(false),
		sections
	}: { open: boolean; sections: { key: string; title: string }[] } = $props();

	type Row = { key: string; title: string; shown: boolean };
	let rows = $state<Row[]>([]);
	let dragging = $state<number | null>(null);
	let list = $state<HTMLElement | null>(null);

	const shownCount = $derived(rows.filter((r) => r.shown).length);
	const customized = $derived(!shadowingIsDefault(personal));

	const ICONS: Record<string, IconSvgElement> = {
		'@starter': SparklesIcon,
		'@movieclips': Film02Icon,
		'@binge': ClapperboardIcon,
		'@criticpicks': StarAward01Icon,
		'@omeleto': TheaterIcon,
		'@keyandpeele': CameraSmile02Icon,
		'@scenecity': ModernTvIcon,
		'@clipempire': CameraVideoIcon,
		'@dollartheater': Film01Icon,
		'@firstslasts': ListStartIcon
	};

	const PREVIEW: Record<CardSize, { n: number; px: number }> = {
		small: { n: 4, px: 11 },
		medium: { n: 3, px: 15 },
		large: { n: 2, px: 24 }
	};

	function build() {
		const hidden = hiddenShadowingSections(personal);
		rows = sections.map((s) => ({
			key: s.key,
			title: s.title,
			shown: !hidden.has(s.key)
		}));
		dragging = null;
	}

	$effect(() => {
		if (open) untrack(build);
	});

	const arrangement = () => ({
		order: rows.map((r) => r.key),
		hidden: rows.filter((r) => !r.shown).map((r) => r.key)
	});
	const commit = () => saveShadowing(arrangement());

	function moveTo(to: number) {
		if (dragging === null || dragging === to) return;
		const next = rows.slice();
		next.splice(to, 0, ...next.splice(dragging, 1));
		rows = next;
		dragging = to;
	}

	async function nudge(i: number, by: number) {
		const to = i + by;
		if (to < 0 || to >= rows.length) return;
		const next = rows.slice();
		next.splice(to, 0, ...next.splice(i, 1));
		rows = next;
		commit();
		const row = rows[to];
		await tick();
		list?.querySelector<HTMLElement>(`[data-grip="${CSS.escape(row.key)}"]`)?.focus();
	}

	function show(row: Row, on: boolean) {
		row.shown = on;
		commit();
	}

	function showAll(on: boolean) {
		for (const r of rows) r.shown = on;
		commit();
	}

	function reset() {
		resetShadowing();
		build();
	}
</script>

{#snippet toggleRow(title: string, desc: string, checked: boolean, set: (on: boolean) => void)}
	<div class="flex items-center justify-between gap-6 px-4 py-3.5">
		<div class="min-w-0">
			<div class="text-sm font-medium">{title}</div>
			<p class="mt-1 text-xs leading-relaxed text-muted-foreground">{desc}</p>
		</div>
		<Switch {checked} onCheckedChange={set} aria-label={title} />
	</div>
{/snippet}

<Dialog.Root bind:open>
	<Dialog.Portal>
		<Dialog.Overlay
			class="bg-transparent bg-gradient-to-l from-black/50 via-black/15 to-transparent"
		/>
		<DialogPrimitive.Content
			onInteractOutside={(e) => onWindowChrome(e) && e.preventDefault()}
			class="fixed bottom-3 right-3 top-12 z-50 flex w-[25rem] max-w-[calc(100%-1.5rem)] flex-col overflow-hidden rounded-2xl border bg-popover text-popover-foreground shadow-2xl outline-none duration-200 data-open:animate-in data-open:fade-in-0 data-open:slide-in-from-right-8 data-closed:animate-out data-closed:fade-out-0 data-closed:slide-out-to-right-8"
		>
			<!-- Header -->
			<div class="flex items-start gap-3 border-b px-5 py-4">
				<span
					class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"
				>
					<HugeiconsIcon icon={DashboardSquareEditIcon} class="h-5 w-5" />
				</span>
				<div class="min-w-0 flex-1">
					<Dialog.Title class="font-heading text-base font-semibold">Tata Letak Shadowing</Dialog.Title>
					<Dialog.Description class="text-xs leading-relaxed text-muted-foreground">
						Atur urutan rak adegan film dan sesuaikan tampilan latihan shadowing Anda.
					</Dialog.Description>
				</div>
				<Button
					variant="ghost"
					size="icon-sm"
					class="-mr-1.5 shrink-0 cursor-pointer"
					aria-label={t('common.close')}
					onclick={() => (open = false)}
				>
					<HugeiconsIcon icon={Cancel01Icon} class="h-4 w-4" />
				</Button>
			</div>

			<!-- Scrollable content -->
			<div
				class="min-h-0 flex-1 overflow-y-auto px-5 py-5"
				{@attach (node) => dragScroll(node, SECTION_ROW_MIME)}
			>
				<!-- Look / Appearance -->
				<h3 class="mb-2 px-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
					Tampilan
				</h3>
				<div class="mb-7 divide-y divide-border/60 overflow-hidden rounded-xl border bg-card">
					<div class="px-4 py-3.5">
						<div class="text-sm font-medium">Ukuran Kartu Adegan</div>
						<p class="mt-1 text-xs leading-relaxed text-muted-foreground">Lebar thumbnail video pada setiap rak adegan.</p>
						<RadioGroup.Root
							value={personal.shadowing.cards}
							onValueChange={(v) => saveShadowing({ cards: v as CardSize })}
							aria-label="Ukuran Kartu Adegan"
							class="mt-3 grid grid-cols-3 gap-2"
						>
							{#each CARD_SIZES as size (size)}
								<RadioGroup.Item
									value={size}
									class="group flex cursor-pointer flex-col items-center gap-2 rounded-xl border px-2 pb-2 pt-3 text-xs font-medium text-muted-foreground outline-none transition-colors hover:border-foreground/25 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-checked:border-primary data-checked:bg-primary/8 data-checked:text-foreground"
								>
									<span class="flex h-6 items-end gap-[3px]" aria-hidden="true">
										{#each Array(PREVIEW[size].n) as _, i (i)}
											<span
												class="rounded-[3px] bg-muted-foreground/30 transition-colors group-data-checked:bg-primary/70"
												style="width:{PREVIEW[size].px}px;height:{PREVIEW[size].px * 0.65}px"
											></span>
										{/each}
									</span>
									<span class="capitalize">{size === 'small' ? 'Kecil' : size === 'medium' ? 'Sedang' : 'Besar'}</span>
								</RadioGroup.Item>
							{/each}
						</RadioGroup.Root>
					</div>

					{@render toggleRow(
						'Filter Kategori',
						'Tampilkan baris tombol filter kategori di bagian atas feed adegan.',
						personal.shadowing.chips,
						(on) => saveShadowing({ chips: on })
					)}
				</div>

				<!-- Shelves / Sections list -->
				<div class="mb-2 flex items-baseline justify-between gap-3 px-1">
					<h3 class="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
						Daftar Rak Adegan
					</h3>
					<div class="flex items-baseline gap-3">
						<span class="text-xs tabular-nums text-muted-foreground">
							{shownCount} dari {rows.length} ditampilkan
						</span>
						<button
							onclick={() => showAll(shownCount === 0)}
							disabled={!rows.length}
							class="cursor-pointer text-xs font-medium text-primary transition-colors hover:text-primary/80 disabled:pointer-events-none disabled:opacity-50"
						>
							{shownCount === 0 ? 'Tampilkan Semua' : 'Sembunyikan Semua'}
						</button>
					</div>
				</div>

				<div
					bind:this={list}
					role="list"
					class="divide-y divide-border/60 overflow-hidden rounded-xl border bg-card"
				>
					{#each rows as row, i (row.key)}
						<div
							role="listitem"
							draggable="true"
							ondragstart={(e) => {
								dragging = i;
								if (!e.dataTransfer) return;
								e.dataTransfer.setData(SECTION_ROW_MIME, row.key);
								e.dataTransfer.effectAllowed = 'move';
							}}
							ondragover={(e) => {
								if (dragging === null || !e.dataTransfer?.types.includes(SECTION_ROW_MIME)) return;
								e.preventDefault();
								e.dataTransfer.dropEffect = 'move';
								moveTo(i);
							}}
							ondragend={() => {
								dragging = null;
								commit();
							}}
							class="group flex items-center gap-3 px-3 py-2.5 transition-colors {dragging === i
								? 'bg-accent/40 opacity-60'
								: 'hover:bg-accent/15'}"
						>
							<!-- Drag handle -->
							<button
								type="button"
								data-grip={row.key}
								class="cursor-grab text-muted-foreground/60 active:cursor-grabbing hover:text-foreground"
								aria-label={`Geser ${row.title}`}
								onkeydown={(e) => {
									if (e.altKey && e.key === 'ArrowUp') {
										e.preventDefault();
										nudge(i, -1);
									} else if (e.altKey && e.key === 'ArrowDown') {
										e.preventDefault();
										nudge(i, 1);
									}
								}}
							>
								<HugeiconsIcon icon={DragDropVerticalIcon} class="h-4 w-4" />
							</button>

							<!-- Icon -->
							{#if ICONS[row.key]}
								<span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
									<HugeiconsIcon icon={ICONS[row.key]} class="h-3.5 w-3.5" />
								</span>
							{/if}

							<!-- Title -->
							<span class="min-w-0 flex-1 truncate text-xs font-medium text-foreground">
								{row.title}
							</span>

							<!-- Toggle -->
							<Switch
								checked={row.shown}
								onCheckedChange={(on) => show(row, on)}
								aria-label={row.title}
							/>
						</div>
					{/each}
				</div>
			</div>

			<!-- Footer -->
			<div class="flex items-center justify-between border-t bg-muted/20 px-5 py-3.5">
				<Button
					variant="ghost"
					size="sm"
					disabled={!customized}
					onclick={reset}
					class="gap-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer disabled:opacity-40"
				>
					<HugeiconsIcon icon={RefreshIcon} class="h-3.5 w-3.5" />
					<span>Atur Ulang</span>
				</Button>

				<Button
					size="sm"
					onclick={() => (open = false)}
					class="gap-1.5 text-xs font-semibold cursor-pointer"
				>
					<HugeiconsIcon icon={Tick02Icon} class="h-3.5 w-3.5" />
					<span>Selesai</span>
				</Button>
			</div>
		</DialogPrimitive.Content>
	</Dialog.Portal>
</Dialog.Root>
