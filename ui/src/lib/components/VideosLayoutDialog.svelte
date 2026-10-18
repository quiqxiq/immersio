<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { Dialog as DialogPrimitive, RadioGroup } from 'bits-ui';
	import { HugeiconsIcon, type IconSvgElement } from '@hugeicons/svelte';
	import {
		AiBrain01Icon,
		Atom02Icon,
		Cancel01Icon,
		CodeIcon,
		DashboardSquareEditIcon,
		DragDropVerticalIcon,
		LaptopIcon,
		Mic01Icon,
		Notification01Icon,
		RefreshIcon,
		SparklesIcon,
		Tick02Icon,
		UserGroup02Icon,
		Video01Icon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import * as Dialog from '$lib/components/ui/dialog';
	import { dragScroll, SECTION_ROW_MIME } from '$lib/dnd';
	import { personal, resetVideos, saveVideos } from '$lib/player.svelte';
	import { CARD_SIZES, hiddenVideoSections, videosIsDefault, type CardSize } from '$lib/personal';
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
	const customized = $derived(!videosIsDefault(personal));

	const ICONS: Record<string, IconSvgElement> = {
		'@recommendations': SparklesIcon,
		'@subscriptions': Notification01Icon,
		'@channels': UserGroup02Icon,
		'@ai': AiBrain01Icon,
		'@tech': LaptopIcon,
		'@science': Atom02Icon,
		'@podcasts': Mic01Icon,
		'@coding': CodeIcon
	};

	const PREVIEW: Record<CardSize, { n: number; px: number }> = {
		small: { n: 4, px: 11 },
		medium: { n: 3, px: 15 },
		large: { n: 2, px: 24 }
	};

	function build() {
		const hidden = hiddenVideoSections(personal);
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
	const commit = () => saveVideos(arrangement());

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
		resetVideos();
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
					<Dialog.Title class="font-heading text-base font-semibold">Tata Letak Videos</Dialog.Title>
					<Dialog.Description class="text-xs leading-relaxed text-muted-foreground">
						Atur urutan rak video dan sesuaikan tampilan feed video Anda.
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
						<div class="text-sm font-medium">Ukuran Kartu Video</div>
						<p class="mt-1 text-xs leading-relaxed text-muted-foreground">Lebar thumbnail video pada setiap rak.</p>
						<RadioGroup.Root
							value={personal.videos.cards}
							onValueChange={(v) => saveVideos({ cards: v as CardSize })}
							aria-label="Ukuran Kartu Video"
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
						'Tampilkan baris tombol filter kategori di bagian atas feed.',
						personal.videos.chips,
						(on) => saveVideos({ chips: on })
					)}
				</div>

				<!-- Shelves / Sections list -->
				<div class="mb-2 flex items-baseline justify-between gap-3 px-1">
					<h3 class="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
						Daftar Rak Video
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
								const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
								const past = e.clientY > r.top + r.height / 2;
								if (i > dragging ? past : !past) moveTo(i);
							}}
							ondragend={() => {
								dragging = null;
								commit();
							}}
							class="flex cursor-grab items-center gap-2.5 py-2 pl-1.5 pr-3 transition-colors {dragging ===
							i
								? 'bg-muted opacity-60'
								: 'hover:bg-muted/40'}"
						>
							<button
								data-grip={row.key}
								aria-label={`Pindahkan rak ${row.title}`}
								onkeydown={(e) => {
									if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
									e.preventDefault();
									nudge(i, e.key === 'ArrowUp' ? -1 : 1);
								}}
								class="flex h-8 w-6 shrink-0 cursor-grab items-center justify-center rounded-md text-muted-foreground/50 outline-none transition-colors hover:text-foreground focus-visible:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
							>
								<HugeiconsIcon icon={DragDropVerticalIcon} class="h-4 w-4" />
							</button>

							<HugeiconsIcon
								icon={ICONS[row.key] ?? Video01Icon}
								class="h-4 w-4 shrink-0 transition-colors {row.shown
									? 'text-primary/80'
									: 'text-muted-foreground/40'}"
							/>

							<span
								class="min-w-0 flex-1 truncate text-sm transition-colors {row.shown
									? ''
									: 'text-muted-foreground'}"
								title={row.title}
							>
								{row.title}
							</span>

							<Switch
								size="sm"
								checked={row.shown}
								onCheckedChange={(on) => show(row, on)}
								aria-label={`Tampilkan rak ${row.title}`}
							/>
						</div>
					{/each}
				</div>
				<p class="mt-2 px-1 text-xs leading-relaxed text-muted-foreground">
					Tarik ikon grip untuk mengubah urutan rak, atau matikan switch untuk menyembunyikannya.
				</p>
			</div>

			<!-- Footer -->
			<div class="flex items-center gap-2 border-t px-5 py-3">
				<Button
					variant="ghost"
					size="sm"
					class="mr-auto cursor-pointer text-muted-foreground"
					disabled={!customized}
					onclick={reset}
				>
					<HugeiconsIcon icon={RefreshIcon} class="h-4 w-4" />
					Atur Ulang
				</Button>
				<Button size="sm" class="cursor-pointer" onclick={() => (open = false)}>
					<HugeiconsIcon icon={Tick02Icon} class="h-4 w-4" />
					Selesai
				</Button>
			</div>
		</DialogPrimitive.Content>
	</Dialog.Portal>
</Dialog.Root>
