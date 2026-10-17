<script lang="ts">
	import { slide, scale, fade } from 'svelte/transition';
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		PlayIcon,
		PauseIcon,
		PreviousIcon,
		NextIcon,
		Cancel01Icon,
		ArrowReloadHorizontalIcon,
		VolumeHighIcon,
		SparklesIcon,
		CpuIcon,
		Layers01Icon,
		Settings02Icon
	} from '@hugeicons/core-free-icons';
	import { tts, KOKORO_VOICES, type TTSVoiceOption } from '$lib/tts.svelte';
	import { Button } from '$lib/components/ui/button';

	let { onclose }: { onclose?: () => void } = $props();

	let showSettings = $state(false);
	let showVoicePicker = $state(false);

	const activeVoiceObj = $derived(
		KOKORO_VOICES.find((v) => v.id === tts.voice) || KOKORO_VOICES[0]
	);

	const activeSentenceText = $derived.by(() => {
		if (tts.activeSentenceIndex === null || !tts.sentences.length) return '';
		return tts.sentences[tts.activeSentenceIndex] || '';
	});

	const progressPct = $derived.by(() => {
		if (tts.activeSentenceIndex === null || !tts.sentences.length) return 0;
		return Math.round(((tts.activeSentenceIndex + 1) / tts.sentences.length) * 100);
	});

	function handleTogglePlay() {
		if (tts.paused) {
			tts.resume();
		} else if (tts.playing) {
			tts.pause();
		} else if (tts.sentences.length > 0) {
			tts.playSentence(tts.activeSentenceIndex ?? 0);
		}
	}

	function handleClose() {
		tts.stop();
		onclose?.();
	}
</script>

<!-- Download & Model Preparation Modal (when Kokoro model is being downloaded) -->
{#if tts.loading}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-4 backdrop-blur-sm"
		transition:fade={{ duration: 150 }}
	>
		<div
			class="w-full max-w-md space-y-4 rounded-2xl border border-border/80 bg-card p-6 shadow-2xl"
			transition:scale={{ duration: 200, start: 0.95 }}
		>
			<div class="flex items-center gap-3">
				<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
					<HugeiconsIcon icon={SparklesIcon} size={20} class="animate-pulse" />
				</div>
				<div class="min-w-0 flex-1">
					<h3 class="text-sm font-bold text-foreground">
						Mengunduh Model Neural Kokoro-82M
					</h3>
					<p class="text-xs text-muted-foreground">
						Format ONNX Quantized (~86MB) • Disimpan lokal secara permanen
					</p>
				</div>
			</div>

			<!-- Live Progress Bar -->
			<div class="space-y-2">
				<div class="flex items-center justify-between text-xs">
					<span class="truncate font-medium text-foreground">{tts.loadStatusText}</span>
					<span class="font-mono font-bold text-primary">{tts.loadProgress}%</span>
				</div>
				<div class="h-2 w-full overflow-hidden rounded-full bg-foreground/10">
					<div
						class="h-full rounded-full bg-primary transition-all duration-300"
						style="width: {Math.max(5, tts.loadProgress)}%"
					></div>
				</div>
			</div>

			<!-- Actions -->
			<div class="flex items-center justify-between pt-2">
				<button
					type="button"
					onclick={() => {
						tts.setEngine('system');
						tts.stop();
						if (tts.sentences.length > 0) {
							tts.playSentence(tts.activeSentenceIndex ?? 0);
						}
					}}
					class="text-xs text-muted-foreground transition hover:text-foreground hover:underline cursor-pointer"
				>
					Gunakan Suara Sistem Saja (Instan)
				</button>

				<Button
					size="sm"
					variant="outline"
					onclick={() => tts.stop()}
					class="rounded-full text-xs"
				>
					Batal
				</Button>
			</div>
		</div>
	</div>
{/if}

<!-- Floating Article Audio Player Bar -->
{#if tts.playing || tts.paused || tts.activeSentenceIndex !== null}
	<aside
		class="fixed bottom-4 left-1/2 z-40 w-[calc(100%-2rem)] max-w-3xl -translate-x-1/2 overflow-hidden rounded-2xl border border-border/80 bg-background/95 p-3 shadow-2xl backdrop-blur-2xl transition-all"
		transition:slide={{ duration: 200 }}
		aria-label="Pemutar Audio Artikel"
	>
		<!-- Progress Line -->
		<div class="absolute left-0 top-0 h-1 w-full bg-foreground/10">
			<div
				class="h-full bg-primary transition-all duration-300"
				style="width: {progressPct}%"
			></div>
		</div>

		<div class="flex flex-col gap-2 pt-1 sm:flex-row sm:items-center sm:justify-between">
			<!-- Left: Sentence Indicator & Active Sentence Preview -->
			<div class="flex items-center gap-2.5 min-w-0 flex-1">
				<!-- Pulsating Audio Wave / Spinner -->
				<div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
					{#if tts.generating}
						<HugeiconsIcon icon={ArrowReloadHorizontalIcon} size={18} class="animate-spin text-primary" />
					{:else if tts.playing && !tts.paused}
						<div class="flex items-end gap-0.5 h-4">
							<span class="w-1 bg-primary rounded-full animate-bounce [animation-delay:-0.3s] h-3"></span>
							<span class="w-1 bg-primary rounded-full animate-bounce [animation-delay:-0.15s] h-4"></span>
							<span class="w-1 bg-primary rounded-full animate-bounce h-2"></span>
						</div>
					{:else}
						<HugeiconsIcon icon={VolumeHighIcon} size={18} class="text-muted-foreground" />
					{/if}
				</div>

				<div class="min-w-0 flex-1">
					<div class="flex items-center gap-2">
						<span class="font-mono text-[11px] font-bold text-primary">
							Kalimat {(tts.activeSentenceIndex ?? 0) + 1} / {tts.sentences.length}
						</span>

						<!-- Engine Pill Badge -->
						<button
							type="button"
							onclick={() => tts.setEngine(tts.engine === 'kokoro' ? 'system' : 'kokoro')}
							class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-semibold transition cursor-pointer {tts.engine === 'kokoro'
								? 'bg-purple-500/15 text-purple-600 dark:text-purple-400 border border-purple-500/30'
								: 'bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground'}"
							title="Klik untuk beralih antara Kokoro-82M Studio vs Suara Sistem"
						>
							{#if tts.engine === 'kokoro'}
								<HugeiconsIcon icon={SparklesIcon} size={10} />
								<span>Kokoro Neural</span>
							{:else}
								<HugeiconsIcon icon={CpuIcon} size={10} />
								<span>Suara Sistem</span>
							{/if}
						</button>
					</div>

					<p class="truncate text-xs text-foreground/90 font-medium" title={activeSentenceText}>
						{activeSentenceText || 'Menyiapkan kalimat...'}
					</p>
				</div>
			</div>

			<!-- Center Controls: Prev, Play/Pause, Next, Replay -->
			<div class="flex items-center justify-center gap-1.5 shrink-0">
				<!-- Prev Sentence -->
				<button
					type="button"
					onclick={() => tts.prev()}
					disabled={tts.activeSentenceIndex === 0}
					class="inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground disabled:opacity-30 cursor-pointer"
					title="Kalimat Sebelumnya"
				>
					<HugeiconsIcon icon={PreviousIcon} size={16} />
				</button>

				<!-- Play/Pause Button -->
				<button
					type="button"
					onclick={handleTogglePlay}
					class="inline-flex h-9 w-9 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-md transition hover:scale-105 active:scale-95 cursor-pointer"
					title={tts.playing && !tts.paused ? 'Jeda (Pause)' : 'Lanjutkan (Play)'}
				>
					{#if tts.playing && !tts.paused}
						<HugeiconsIcon icon={PauseIcon} size={18} />
					{:else}
						<HugeiconsIcon icon={PlayIcon} size={18} class="ml-0.5" />
					{/if}
				</button>

				<!-- Next Sentence -->
				<button
					type="button"
					onclick={() => tts.next()}
					disabled={tts.activeSentenceIndex !== null && tts.activeSentenceIndex >= tts.sentences.length - 1}
					class="inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground disabled:opacity-30 cursor-pointer"
					title="Kalimat Berikutnya"
				>
					<HugeiconsIcon icon={NextIcon} size={16} />
				</button>

				<!-- Replay Current Sentence -->
				<button
					type="button"
					onclick={() => {
						if (tts.activeSentenceIndex !== null) {
							tts.playSentence(tts.activeSentenceIndex);
						}
					}}
					class="inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
					title="Putar Ulang Kalimat Ini"
				>
					<HugeiconsIcon icon={ArrowReloadHorizontalIcon} size={15} />
				</button>
			</div>

			<!-- Right: Voice & Speed Selector, Close -->
			<div class="flex items-center justify-end gap-1.5 shrink-0">
				<!-- Speed Selector Pills -->
				<div class="inline-flex items-center rounded-full bg-foreground/5 p-0.5 text-[11px]">
					{#each [0.85, 1.0, 1.15, 1.25] as sp}
						<button
							type="button"
							onclick={() => tts.setSpeed(sp)}
							class="rounded-full px-1.5 py-0.5 font-semibold transition cursor-pointer {tts.speed === sp
								? 'bg-primary text-primary-foreground shadow-xs'
								: 'text-muted-foreground hover:text-foreground'}"
							title="Kecepatan {sp}x"
						>
							{sp}x
						</button>
					{/each}
				</div>

				<!-- Voice Picker Trigger -->
				{#if tts.engine === 'kokoro'}
					<div class="relative">
						<button
							type="button"
							onclick={() => (showVoicePicker = !showVoicePicker)}
							class="inline-flex items-center gap-1 rounded-full bg-foreground/5 px-2 py-1 text-xs font-semibold text-foreground transition hover:bg-foreground/10 cursor-pointer"
							title="Pilih Suara Kokoro Studio"
						>
							<span>{activeVoiceObj.flag}</span>
							<span class="max-w-[70px] truncate">{activeVoiceObj.name}</span>
						</button>

						<!-- Voice Dropdown Popover -->
						{#if showVoicePicker}
							<div
								class="absolute bottom-full right-0 mb-2 w-64 overflow-hidden rounded-2xl border border-border bg-card/95 p-2 shadow-2xl backdrop-blur-xl"
								transition:scale={{ duration: 150, start: 0.95 }}
							>
								<div class="border-b border-border/50 px-2.5 py-1.5">
									<span class="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">
										Pilihan Suara Kokoro Neural
									</span>
								</div>

								<div class="max-h-56 overflow-y-auto space-y-1 py-1">
									{#each KOKORO_VOICES as vc}
										<button
											type="button"
											onclick={() => {
												tts.setVoice(vc.id);
												showVoicePicker = false;
												if (tts.activeSentenceIndex !== null) {
													tts.playSentence(tts.activeSentenceIndex);
												}
											}}
											class="flex w-full items-center justify-between rounded-xl px-2.5 py-1.5 text-left transition cursor-pointer {tts.voice === vc.id
												? 'bg-primary/15 text-primary font-semibold'
												: 'text-foreground/90 hover:bg-foreground/5'}"
										>
											<div class="flex items-center gap-2 min-w-0">
												<span class="text-base">{vc.flag}</span>
												<div class="min-w-0">
													<div class="flex items-center gap-1.5">
														<span class="text-xs font-semibold">{vc.name}</span>
														<span class="text-[10px] text-muted-foreground">({vc.gender})</span>
													</div>
													<p class="text-[10px] text-muted-foreground truncate max-w-[150px]">{vc.desc}</p>
												</div>
											</div>
											{#if tts.voice === vc.id}
												<span class="text-xs text-primary font-bold">✓</span>
											{/if}
										</button>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				{/if}

				<!-- Close Button -->
				<button
					type="button"
					onclick={handleClose}
					class="inline-flex h-8 w-8 items-center justify-center rounded-full text-muted-foreground transition hover:bg-foreground/10 hover:text-foreground cursor-pointer"
					title="Hentikan Pembacaan (Stop)"
				>
					<HugeiconsIcon icon={Cancel01Icon} size={16} />
				</button>
			</div>
		</div>
	</aside>
{/if}
