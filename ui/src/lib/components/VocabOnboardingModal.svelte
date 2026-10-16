<script lang="ts">
	import { onMount } from 'svelte';
	import { scale, fade } from 'svelte/transition';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		Tick02Icon,
		Cancel01Icon,
		SparklesIcon,
		Search01Icon,
		ArrowRight01Icon,
		CheckmarkCircle03Icon
	} from '@hugeicons/core-free-icons';
	import { vocab } from '$lib/vocab.svelte';
	import { Button } from '$lib/components/ui/button';
	import vocabData from '$lib/data/vocab-levels.json';

	let {
		open = $bindable(false),
		onclose
	}: {
		open: boolean;
		onclose?: () => void;
	} = $props();

	type LevelId = 'a1_a2' | 'b1' | 'b2';

	let step = $state<1 | 2>(1);
	let selectedLevel = $state<LevelId>('b1');
	let searchQuery = $state('');
	let saving = $state(false);

	// Active words based on cumulative selected level
	const activeWords = $derived.by(() => {
		const a1 = vocabData.levels.a1_a2.words;
		const b1 = vocabData.levels.b1.words;
		const b2 = vocabData.levels.b2.words;

		if (selectedLevel === 'a1_a2') {
			return a1;
		}
		if (selectedLevel === 'b1') {
			return [...a1, ...b1];
		}
		return [...a1, ...b1, ...b2];
	});

	// Set of words that user has verified as "known" (default all selected)
	let selectedSet = $state<Set<string>>(new Set());

	// Update selectedSet whenever selectedLevel changes
	$effect(() => {
		if (step === 1) {
			selectedSet = new Set(activeWords);
		}
	});

	// Filtered words for search in Step 2
	const filteredWords = $derived.by(() => {
		const q = searchQuery.toLowerCase().trim();
		if (!q) return activeWords;
		return activeWords.filter((w) => w.toLowerCase().includes(q));
	});

	// Sub-pagination / batching for smooth rendering in Step 2
	let page = $state(0);
	const PAGE_SIZE = 120;
	const totalPages = $derived(Math.ceil(filteredWords.length / PAGE_SIZE));
	const paginatedWords = $derived.by(() => {
		const start = page * PAGE_SIZE;
		return filteredWords.slice(start, start + PAGE_SIZE);
	});

	function toggleWord(word: string) {
		const next = new Set(selectedSet);
		if (next.has(word)) {
			next.delete(word);
		} else {
			next.add(word);
		}
		selectedSet = next;
	}

	function selectAll() {
		selectedSet = new Set(activeWords);
	}

	function deselectAll() {
		selectedSet = new Set();
	}

	async function handleSave() {
		if (saving) return;
		saving = true;
		try {
			const wordsToSave = Array.from(selectedSet);
			await vocab.bulkKnown(wordsToSave);
			localStorage.setItem('limusic_vocab_calibrated', 'true');
			open = false;
			if (onclose) onclose();
		} catch (e) {
			console.error('Failed to save calibrated vocabulary:', e);
		} finally {
			saving = false;
		}
	}

	function handleSkip() {
		localStorage.setItem('limusic_vocab_calibrated', 'true');
		open = false;
		if (onclose) onclose();
	}

	function handleClose() {
		open = false;
		if (onclose) onclose();
	}
</script>

{#if open}
	<!-- Backdrop with smooth blur -->
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		transition:fade={{ duration: 150 }}
		class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 sm:p-6"
		onclick={handleClose}
	>
		<!-- Modal Content Box -->
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
		<div
			transition:scale={{ duration: 200, start: 0.96 }}
			class="relative flex flex-col w-full max-w-2xl max-h-[88vh] rounded-2xl border border-border/80 bg-card/95 text-card-foreground shadow-2xl backdrop-blur-2xl overflow-hidden focus:outline-none"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-labelledby="modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-border/60 px-6 py-4 shrink-0 bg-muted/20">
				<div class="flex items-center gap-2.5">
					<div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-primary">
						<HugeiconsIcon icon={SparklesIcon} size={18} />
					</div>
					<div>
						<h2 id="modal-title" class="text-base font-bold text-foreground">
							Kalibrasi Level Kosakata
						</h2>
						<p class="text-xs text-muted-foreground">
							{step === 1 ? 'Langkah 1: Pilih perkiraan tingkat kemampuan Anda' : 'Langkah 2: Tinjau kata yang sudah Anda ketahui'}
						</p>
					</div>
				</div>

				<button
					type="button"
					onclick={handleClose}
					class="inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground hover:bg-muted hover:text-foreground transition cursor-pointer"
					title="Tutup"
				>
					<HugeiconsIcon icon={Cancel01Icon} size={16} />
				</button>
			</div>

			<!-- Body: Step 1 or Step 2 -->
			<div class="flex-1 overflow-y-auto px-6 py-5 min-h-0 space-y-5">
				{#if step === 1}
					<!-- Step 1: Level Cards -->
					<div class="space-y-3">
						<p class="text-xs text-muted-foreground">
							Pilih level yang paling mendekati penguasaan bahasa Inggris Anda saat ini. Sistem akan menyiapkan kata-kata umum agar saat membaca artikel atau menonton video tidak semua kata bergaris bawah merah:
						</p>

						<div class="grid grid-cols-1 gap-3 pt-1">
							<!-- Level A1-A2 -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div
								role="button"
								tabindex="0"
								onclick={() => (selectedLevel = 'a1_a2')}
								class="group flex cursor-pointer items-start gap-4 rounded-xl border p-4 text-left transition-all {selectedLevel === 'a1_a2'
									? 'border-primary bg-primary/5 ring-1 ring-primary/30 shadow-xs'
									: 'border-border/70 hover:border-border hover:bg-muted/40'}"
							>
								<div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border {selectedLevel === 'a1_a2' ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40'}">
									{#if selectedLevel === 'a1_a2'}
										<HugeiconsIcon icon={Tick02Icon} size={12} />
									{/if}
								</div>
								<div class="space-y-1">
									<div class="flex items-center gap-2">
										<h3 class="text-sm font-semibold text-foreground">Pemula (A1–A2)</h3>
										<span class="rounded bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
											~1.000 Kata Dasar
										</span>
									</div>
									<p class="text-xs text-muted-foreground leading-relaxed">
										{vocabData.levels.a1_a2.description}
									</p>
									<p class="text-[11px] text-muted-foreground/80 italic pt-0.5">
										Contoh: time, world, school, family, beautiful, money, problem, think, good...
									</p>
								</div>
							</div>

							<!-- Level B1 (Recommended) -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div
								role="button"
								tabindex="0"
								onclick={() => (selectedLevel = 'b1')}
								class="group flex cursor-pointer items-start gap-4 rounded-xl border p-4 text-left transition-all {selectedLevel === 'b1'
									? 'border-primary bg-primary/5 ring-1 ring-primary/30 shadow-xs'
									: 'border-border/70 hover:border-border hover:bg-muted/40'}"
							>
								<div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border {selectedLevel === 'b1' ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40'}">
									{#if selectedLevel === 'b1'}
										<HugeiconsIcon icon={Tick02Icon} size={12} />
									{/if}
								</div>
								<div class="space-y-1">
									<div class="flex items-center gap-2">
										<h3 class="text-sm font-semibold text-foreground">Menengah (B1)</h3>
										<span class="rounded bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30 px-2 py-0.5 text-[10px] font-semibold">
											Rekomendasi • ~2.500 Kata
										</span>
									</div>
									<p class="text-xs text-muted-foreground leading-relaxed">
										{vocabData.levels.b1.description}
									</p>
									<p class="text-[11px] text-muted-foreground/80 italic pt-0.5">
										Contoh: achieve, behavior, determine, however, maintain, particular, require, tradition...
									</p>
								</div>
							</div>

							<!-- Level B2 -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div
								role="button"
								tabindex="0"
								onclick={() => (selectedLevel = 'b2')}
								class="group flex cursor-pointer items-start gap-4 rounded-xl border p-4 text-left transition-all {selectedLevel === 'b2'
									? 'border-primary bg-primary/5 ring-1 ring-primary/30 shadow-xs'
									: 'border-border/70 hover:border-border hover:bg-muted/40'}"
							>
								<div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border {selectedLevel === 'b2' ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40'}">
									{#if selectedLevel === 'b2'}
										<HugeiconsIcon icon={Tick02Icon} size={12} />
									{/if}
								</div>
								<div class="space-y-1">
									<div class="flex items-center gap-2">
										<h3 class="text-sm font-semibold text-foreground">Lanjutan (B2)</h3>
										<span class="rounded bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
											~5.000 Kata Lengkap
										</span>
									</div>
									<p class="text-xs text-muted-foreground leading-relaxed">
										{vocabData.levels.b2.description}
									</p>
									<p class="text-[11px] text-muted-foreground/80 italic pt-0.5">
										Contoh: consequence, elaborate, furthermore, inevitable, sophisticated, sustainable...
									</p>
								</div>
							</div>
						</div>
					</div>
				{:else}
					<!-- Step 2: Step-by-Step Word Review & Selection -->
					<div class="space-y-4">
						<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
							<div class="space-y-0.5">
								<h3 class="text-sm font-semibold text-foreground">
									Tinjau Kata ({selectedSet.size} dari {activeWords.length} dipilih)
								</h3>
								<p class="text-[11px] text-muted-foreground">
									Klik kata yang <strong>belum Anda ketahui atau Anda ragu</strong> untuk menghapus centangnya.
								</p>
							</div>

							<!-- Quick Search & Select/Deselect All -->
							<div class="flex items-center gap-2 shrink-0">
								<button
									type="button"
									onclick={selectAll}
									class="text-[11px] text-primary hover:underline font-medium cursor-pointer"
								>
									Pilih Semua
								</button>
								<span class="text-muted-foreground/40">•</span>
								<button
									type="button"
									onclick={deselectAll}
									class="text-[11px] text-muted-foreground hover:text-foreground cursor-pointer"
								>
									Batalkan Semua
								</button>
							</div>
						</div>

						<!-- Search Filter -->
						<div class="relative">
							<HugeiconsIcon
								icon={Search01Icon}
								size={14}
								class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
							/>
							<input
								type="text"
								bind:value={searchQuery}
								placeholder="Cari kata spesifik untuk ditinjau..."
								class="h-8 w-full rounded-lg border border-border/70 bg-card pl-9 pr-3 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
							/>
						</div>

						<!-- Word Chips Grid -->
						<div class="max-h-[38vh] overflow-y-auto rounded-xl border border-border/60 bg-muted/20 p-3 scrollbar-thin">
							{#if paginatedWords.length === 0}
								<p class="py-8 text-center text-xs text-muted-foreground">
									Tidak ada kata yang cocok dengan "{searchQuery}".
								</p>
							{:else}
								<div class="flex flex-wrap gap-1.5">
									{#each paginatedWords as w}
										{@const isKnown = selectedSet.has(w)}
										<button
											type="button"
											onclick={() => toggleWord(w)}
											class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs font-medium transition cursor-pointer select-none {isKnown
												? 'bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border border-emerald-500/40 shadow-2xs'
												: 'bg-card text-muted-foreground/70 border border-border/60 line-through opacity-55 hover:opacity-100 hover:text-foreground'}"
											title={isKnown ? 'Klik untuk tandai belum tahu' : 'Klik untuk tandai sudah tahu'}
										>
											<span>{w}</span>
											{#if isKnown}
												<HugeiconsIcon icon={Tick02Icon} size={11} class="text-emerald-500" />
											{/if}
										</button>
									{/each}
								</div>
							{/if}
						</div>

						<!-- Pagination Controls -->
						{#if totalPages > 1}
							<div class="flex items-center justify-between text-xs text-muted-foreground pt-1">
								<span>Halaman {page + 1} dari {totalPages} ({filteredWords.length} total kata)</span>
								<div class="flex items-center gap-1.5">
									<Button
										variant="outline"
										size="sm"
										disabled={page === 0}
										onclick={() => page--}
										class="h-7 text-xs px-2.5"
									>
										Sebelumnya
									</Button>
									<Button
										variant="outline"
										size="sm"
										disabled={page >= totalPages - 1}
										onclick={() => page++}
										class="h-7 text-xs px-2.5"
									>
										Berikutnya
									</Button>
								</div>
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Footer Actions -->
			<div class="flex items-center justify-between border-t border-border/60 px-6 py-3.5 bg-muted/20 shrink-0">
				{#if step === 1}
					<button
						type="button"
						onclick={handleSkip}
						class="text-xs text-muted-foreground hover:text-foreground cursor-pointer transition"
					>
						Lewati (Mulai dari Nol)
					</button>

					<div class="flex items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onclick={() => {
								selectedSet = new Set(activeWords);
								handleSave();
							}}
							disabled={saving}
							class="text-xs rounded-lg"
						>
							Terapkan Cepat (Semua Tahu)
						</Button>
						<Button
							size="sm"
							onclick={() => {
								selectedSet = new Set(activeWords);
								page = 0;
								step = 2;
							}}
							class="gap-1.5 text-xs rounded-lg"
						>
							<span>Tinjau Kata</span>
							<HugeiconsIcon icon={ArrowRight01Icon} size={13} />
						</Button>
					</div>
				{:else}
					<Button
						variant="ghost"
						size="sm"
						onclick={() => (step = 1)}
						class="text-xs"
					>
						Kembali ke Pilihan Level
					</Button>

					<Button
						size="sm"
						onclick={handleSave}
						disabled={saving || selectedSet.size === 0}
						class="gap-1.5 text-xs rounded-lg"
					>
						<HugeiconsIcon icon={CheckmarkCircle03Icon} size={14} />
						<span>{saving ? 'Menyimpan...' : `Terapkan ${selectedSet.size} Kata sebagai 'Known'`}</span>
					</Button>
				{/if}
			</div>
		</div>
	</div>
{/if}
