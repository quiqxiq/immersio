<script lang="ts">
	import { HugeiconsIcon } from '@hugeicons/svelte';
	import {
		RefreshIcon,
		Tick02Icon,
		BookOpenTextIcon,
		ViewOffSlashIcon,
		ViewIcon,
		Key01Icon,
		SparklesIcon,
		Layers01Icon,
		InformationCircleIcon,
		ExternalLinkIcon
	} from '@hugeicons/core-free-icons';
	import { Button } from '$lib/components/ui/button';
	import { Switch } from '$lib/components/ui/switch';
	import { vocab } from '$lib/vocab.svelte';
	import { toast } from '$lib/player.svelte';
	import * as api from '$lib/api';
	import VocabOnboardingModal from '$lib/components/VocabOnboardingModal.svelte';
	import vocabData from '$lib/data/vocab-levels.json';

	const GROUP = 'mb-7 last:mb-1';
	const LABEL =
		'mb-2 px-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground';
	const CARD = 'divide-y divide-border/60 overflow-hidden rounded-xl border bg-card';

	let checkingAnki = $state(false);
	let syncingIntervals = $state(false);
	let seedingWords = $state(false);
	let showCalibrationModal = $state(false);
	let bulkSeedingPreset = $state<string | null>(null);
	let recentCards = $state<api.FlashcardRow[]>([]);
	let loadingCards = $state(false);

	async function refreshAnki() {
		checkingAnki = true;
		try {
			await vocab.checkAnki();
			if (vocab.anki.connected) {
				toast('Berhasil terhubung ke AnkiConnect!');
			} else {
				toast.error('Gagal terhubung. Pastikan Anki Desktop terbuka di port 8765.');
			}
		} finally {
			checkingAnki = false;
		}
	}

	async function handleSyncIntervals() {
		syncingIntervals = true;
		try {
			const res = await vocab.syncAnkiIntervals();
			toast(`Sinkronisasi selesai! ${res} kata dipromosikan menjadi 'Known'.`);
		} catch (e) {
			toast.error(`Gagal sinkronisasi: ${e}`);
		} finally {
			syncingIntervals = false;
		}
	}

	const COMMON_ENGLISH_WORDS = [
		'the', 'be', 'to', 'of', 'and', 'a', 'in', 'that', 'have', 'i',
		'it', 'for', 'not', 'on', 'with', 'he', 'as', 'you', 'do', 'at',
		'this', 'but', 'his', 'by', 'from', 'they', 'we', 'say', 'her', 'she',
		'or', 'an', 'will', 'my', 'one', 'all', 'would', 'there', 'their', 'what',
		'so', 'up', 'out', 'if', 'about', 'who', 'get', 'which', 'go', 'me',
		'when', 'make', 'can', 'like', 'time', 'no', 'just', 'him', 'know', 'take',
		'people', 'into', 'year', 'your', 'good', 'some', 'could', 'them', 'see', 'other',
		'than', 'then', 'now', 'look', 'only', 'come', 'its', 'over', 'think', 'also',
		'back', 'after', 'use', 'two', 'how', 'our', 'work', 'first', 'well', 'way',
		'even', 'new', 'want', 'because', 'any', 'these', 'give', 'day', 'most', 'us',
		'is', 'are', 'was', 'were', 'been', 'has', 'had', 'did', 'does', 'am',
		'mine', 'yours', 'ours', 'theirs', 'hers', 'whose', 'whom', 'where', 'why',
		'here', 'there', 'again', 'further', 'once', 'always', 'never', 'sometimes', 'often',
		'very', 'too', 'such', 'both', 'few', 'own', 'same', 'than', 'off', 'down'
	];

	async function handleSeedCommonWords() {
		seedingWords = true;
		try {
			const count = await vocab.bulkKnown(COMMON_ENGLISH_WORDS);
			toast(`${count} kata dasar bahasa Inggris ditandai sebagai 'Known'!`);
		} catch (e) {
			toast.error(`Gagal menandai kata dasar: ${e}`);
		} finally {
			seedingWords = false;
		}
	}

	async function handleQuickPreset(level: 'a1_a2' | 'b1' | 'b2') {
		bulkSeedingPreset = level;
		try {
			let words: string[] = [];
			let label = '';
			if (level === 'a1_a2') {
				words = vocabData.levels.a1_a2.words;
				label = 'Top 1.000 Kata (Pemula A1–A2)';
			} else if (level === 'b1') {
				words = [...vocabData.levels.a1_a2.words, ...vocabData.levels.b1.words];
				label = 'Top 2.500 Kata (Menengah B1)';
			} else if (level === 'b2') {
				words = [
					...vocabData.levels.a1_a2.words,
					...vocabData.levels.b1.words,
					...vocabData.levels.b2.words
				];
				label = 'Top 5.000 Kata (Lanjutan B2)';
			}
			const count = await vocab.bulkKnown(words);
			toast(`${count} kata berhasil ditandai sebagai 'Known' (${label})!`);
		} catch (e: any) {
			toast.error(`Gagal menandai preset: ${e?.message || e}`);
		} finally {
			bulkSeedingPreset = null;
		}
	}

	async function loadRecentCards() {
		loadingCards = true;
		try {
			recentCards = await api.getFlashcards(10);
		} catch (e) {
			console.error(e);
		} finally {
			loadingCards = false;
		}
	}

	let openRouterToken = $state('');
	let showToken = $state(false);
	let savingToken = $state(false);
	let testingToken = $state(false);
	let testStatus = $state<{ success: boolean; message: string; model?: string } | null>(null);
	let aiCacheCount = $state(0);
	let clearingAiCache = $state(false);

	async function loadAiSettings() {
		try {
			openRouterToken = (await api.getOpenRouterToken()) || '';
			aiCacheCount = await api.getAiCacheCount();
		} catch (e) {
			console.error('Failed to load AI settings:', e);
		}
	}

	async function handleSaveToken() {
		savingToken = true;
		try {
			await api.setOpenRouterToken(openRouterToken.trim());
			toast('Token OpenRouter berhasil disimpan!');
		} catch (e: any) {
			toast.error(`Gagal menyimpan token: ${e?.message || e}`);
		} finally {
			savingToken = false;
		}
	}

	async function handleTestToken() {
		const token = openRouterToken.trim();
		if (!token) {
			toast.error('Silakan masukkan token OpenRouter terlebih dahulu.');
			return;
		}
		testingToken = true;
		testStatus = null;
		try {
			const model = await api.testOpenRouterToken(token);
			testStatus = { success: true, message: 'Koneksi ke OpenRouter aktif & valid!', model };
			toast(`Koneksi AI aktif! (${model})`);
		} catch (e: any) {
			const errMsg = typeof e === 'string' ? e : (e?.message || String(e));
			testStatus = { success: false, message: errMsg };
			toast.error(`Koneksi gagal: ${errMsg}`);
		} finally {
			testingToken = false;
		}
	}

	async function handleClearAiCache() {
		clearingAiCache = true;
		try {
			await api.clearAiCache();
			aiCacheCount = 0;
			toast('Cache respons AI berhasil dibersihkan!');
		} catch (e: any) {
			toast.error(`Gagal membersihkan cache: ${e?.message || e}`);
		} finally {
			clearingAiCache = false;
		}
	}

	$effect(() => {
		loadRecentCards();
		loadAiSettings();
	});
</script>

<!-- Vocabulary Metrics Cards -->
<section class={GROUP}>
	<h3 class={LABEL}>Ringkasan Kosakata (Vocabulary)</h3>
	<div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
		<!-- Known Words -->
		<div class="flex flex-col rounded-xl border border-border/70 bg-card p-3.5 shadow-sm transition hover:border-emerald-500/50">
			<div class="flex items-center justify-between text-muted-foreground">
				<span class="text-xs font-medium">Sudah Dikuasai</span>
				<div class="flex h-6 w-6 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-500">
					<HugeiconsIcon icon={Tick02Icon} size={14} />
				</div>
			</div>
			<div class="mt-2 flex items-baseline gap-2">
				<span class="text-2xl font-bold tracking-tight text-foreground">{vocab.counts.known}</span>
				<span class="text-[11px] text-muted-foreground">kata</span>
			</div>
			<span class="mt-1 text-[10px] text-emerald-600 dark:text-emerald-400 font-medium">Known (Status 1)</span>
		</div>

		<!-- Learning Words -->
		<div class="flex flex-col rounded-xl border border-border/70 bg-card p-3.5 shadow-sm transition hover:border-blue-500/50">
			<div class="flex items-center justify-between text-muted-foreground">
				<span class="text-xs font-medium">Sedang Dipelajari</span>
				<div class="flex h-6 w-6 items-center justify-center rounded-full bg-blue-500/10 text-blue-500">
					<HugeiconsIcon icon={BookOpenTextIcon} size={14} />
				</div>
			</div>
			<div class="mt-2 flex items-baseline gap-2">
				<span class="text-2xl font-bold tracking-tight text-foreground">{vocab.counts.learning}</span>
				<span class="text-[11px] text-muted-foreground">kata</span>
			</div>
			<span class="mt-1 text-[10px] text-blue-600 dark:text-blue-400 font-medium">Learning (Status 2)</span>
		</div>

		<!-- Ignored Words -->
		<div class="flex flex-col rounded-xl border border-border/70 bg-card p-3.5 shadow-sm transition hover:border-foreground/30">
			<div class="flex items-center justify-between text-muted-foreground">
				<span class="text-xs font-medium">Diabaikan</span>
				<div class="flex h-6 w-6 items-center justify-center rounded-full bg-muted text-muted-foreground">
					<HugeiconsIcon icon={ViewOffSlashIcon} size={14} />
				</div>
			</div>
			<div class="mt-2 flex items-baseline gap-2">
				<span class="text-2xl font-bold tracking-tight text-foreground">{vocab.counts.ignored}</span>
				<span class="text-[11px] text-muted-foreground">kata</span>
			</div>
			<span class="mt-1 text-[10px] text-muted-foreground font-medium">Ignored (Status 3)</span>
		</div>

		<!-- Mined Flashcards -->
		<div class="flex flex-col rounded-xl border border-border/70 bg-card p-3.5 shadow-sm transition hover:border-purple-500/50">
			<div class="flex items-center justify-between text-muted-foreground">
				<span class="text-xs font-medium">Total Flashcard</span>
				<div class="flex h-6 w-6 items-center justify-center rounded-full bg-purple-500/10 text-purple-500">
					<HugeiconsIcon icon={SparklesIcon} size={14} />
				</div>
			</div>
			<div class="mt-2 flex items-baseline gap-2">
				<span class="text-2xl font-bold tracking-tight text-foreground">{vocab.counts.flashcards ?? 0}</span>
				<span class="text-[11px] text-muted-foreground">kartu</span>
			</div>
			<span class="mt-1 text-[10px] text-purple-600 dark:text-purple-400 font-medium">Tersimpan di Limusic</span>
		</div>
	</div>
</section>

<!-- Native Dictionary & Translation Engine Section -->
<section class={GROUP}>
	<h3 class={LABEL}>Penyedia Kamus & Terjemahan (Native Rust Engine)</h3>
	<div class={CARD}>
		<!-- Dictionary Provider -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Penyedia Kamus Utama</span>
				<p class="text-xs text-muted-foreground">
					Kamus definisi bahasa Inggris berkecepatan tinggi yang diproses langsung oleh engine Rust bawaan Limusic.
				</p>
			</div>

			<div class="w-56 shrink-0">
				<select
					value={vocab.dictProvider}
					onchange={(e) => vocab.setDictProvider(e.currentTarget.value)}
					class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
				>
					<option value="cambridge">Cambridge Dictionary (Rekomendasi)</option>
					<option value="oxford">Oxford Learner's Dictionary</option>
					<option value="wiktionary">Wiktionary (Wikimedia API)</option>
					<option value="wordnet">WordNet / Datamuse</option>
				</select>
			</div>
		</div>

		<!-- Auto-Fallback Switch -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Pencarian Otomatis Berjenjang (Auto-Fallback)</span>
				<p class="text-xs text-muted-foreground">
					Jika kata tidak ditemukan di kamus utama, otomatis cari ke kamus lain (Cambridge → Oxford → Wiktionary → WordNet).
				</p>
			</div>

			<Switch
				checked={vocab.autoFallback}
				onCheckedChange={(checked) => vocab.setAutoFallback(checked)}
			/>
		</div>

		<!-- Translation Provider -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Penyedia Mesin Terjemahan</span>
				<p class="text-xs text-muted-foreground">
					Mesin yang digunakan untuk menerjemahkan kata dan seluruh kalimat konteks.
				</p>
			</div>

			<div class="w-56 shrink-0">
				<select
					value={vocab.transProvider}
					onchange={(e) => vocab.setTransProvider(e.currentTarget.value)}
					class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
				>
					<option value="google">Google Translate (Cepat & Akurat)</option>
					<option value="lingva">Lingva Translate (Google Mirror)</option>
					<option value="mymemory">MyMemory Translated</option>
				</select>
			</div>
		</div>

		<!-- Target Language -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Bahasa Terjemahan Target</span>
				<p class="text-xs text-muted-foreground">
					Bahasa tujuan penerjemahan kata dan kalimat.
				</p>
			</div>

			<div class="w-56 shrink-0">
				<select
					value={vocab.targetLang}
					onchange={(e) => vocab.setTargetLang(e.currentTarget.value)}
					class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
				>
					<option value="id">Bahasa Indonesia (id)</option>
					<option value="es">Español (es)</option>
					<option value="fr">Français (fr)</option>
					<option value="de">Deutsch (de)</option>
					<option value="ja">日本語 (ja)</option>
					<option value="ko">한국어 (ko)</option>
					<option value="zh">中文 (zh)</option>
				</select>
			</div>
		</div>

		<!-- Offline SQLite Cache Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<div class="flex items-center gap-2">
					<span class="text-sm font-medium">Cache Kamus Offline (SQLite)</span>
					<span class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-semibold text-muted-foreground">
						{vocab.dictCacheCount} kata tersimpan
					</span>
				</div>
				<p class="text-xs text-muted-foreground">
					Hasil pencarian kata disimpan secara permanen di database lokal agar dapat dibuka instan (&lt; 1ms) tanpa koneksi internet.
				</p>
			</div>

			<Button
				variant="outline"
				size="sm"
				onclick={() => vocab.clearCache()}
				disabled={vocab.dictCacheCount === 0}
				class="gap-1.5 cursor-pointer shrink-0 text-destructive hover:text-destructive"
			>
				<span>Bersihkan Cache</span>
			</Button>
		</div>
	</div>
</section>

<!-- OpenRouter AI Immersion Section -->
<section class={GROUP}>
	<h3 class={LABEL}>Integrasi AI Immersion Tutor (OpenRouter)</h3>
	<div class={CARD}>
		<!-- Token Input & Auto-fallback Description -->
		<div class="px-4 py-4 space-y-3">
			<div class="flex items-start justify-between gap-4">
				<div class="min-w-0 flex-1 space-y-1">
					<div class="flex items-center gap-2">
						<span class="text-sm font-medium">OpenRouter API Key</span>
						<span class="inline-flex items-center gap-1 rounded bg-purple-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-purple-600 dark:text-purple-400">
							<HugeiconsIcon icon={SparklesIcon} size={11} />
							Multi-Model Fallback
						</span>
					</div>
					<p class="text-xs text-muted-foreground leading-relaxed">
						Dukung pemahaman imersif mendalam via <strong>Ctrl + Klik</strong> (atau <strong>Cmd + Klik</strong> di macOS) pada kata apa saja.
						Cukup tempelkan token OpenRouter Anda—Limusic otomatis mengelola rangkaian model gratis terbaik
						dengan fallback otomatis berkecepatan tinggi tanpa konfigurasi rumit.
					</p>
				</div>
				<button
					type="button"
					onclick={() => api.openExternal('https://openrouter.ai/keys')}
					class="inline-flex items-center gap-1 rounded-md border border-input bg-background/50 px-2 py-1 text-xs font-medium text-muted-foreground transition hover:bg-accent hover:text-foreground shrink-0 cursor-pointer"
					title="Dapatkan API key OpenRouter gratis"
				>
					<span>Ambil API Key Gratis</span>
					<HugeiconsIcon icon={ExternalLinkIcon} size={12} />
				</button>
			</div>

			<!-- Input Row with Show/Hide toggle and Action Buttons -->
			<div class="flex items-center gap-2 pt-1">
				<div class="relative flex-1">
					<div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-3 text-muted-foreground">
						<HugeiconsIcon icon={Key01Icon} size={14} />
					</div>
					<input
						type={showToken ? 'text' : 'password'}
						bind:value={openRouterToken}
						placeholder="sk-or-v1-..."
						class="w-full rounded-lg border border-input bg-background pl-9 pr-10 py-1.5 text-xs text-foreground font-mono placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-primary"
					/>
					<button
						type="button"
						onclick={() => (showToken = !showToken)}
						class="absolute inset-y-0 right-0 flex items-center pr-3 text-muted-foreground transition hover:text-foreground cursor-pointer"
						title={showToken ? 'Sembunyikan token' : 'Tampilkan token'}
					>
						<HugeiconsIcon icon={showToken ? ViewOffSlashIcon : ViewIcon} size={14} />
					</button>
				</div>

				<Button
					size="sm"
					variant="secondary"
					onclick={handleSaveToken}
					disabled={savingToken}
					class="shrink-0 text-xs h-8 cursor-pointer"
				>
					{#if savingToken}
						<HugeiconsIcon icon={RefreshIcon} size={13} class="animate-spin mr-1" />
					{:else}
						<HugeiconsIcon icon={Tick02Icon} size={13} class="mr-1" />
					{/if}
					Simpan
				</Button>

				<Button
					size="sm"
					variant="outline"
					onclick={handleTestToken}
					disabled={testingToken || !openRouterToken.trim()}
					class="shrink-0 text-xs h-8 cursor-pointer"
				>
					{#if testingToken}
						<HugeiconsIcon icon={RefreshIcon} size={13} class="animate-spin mr-1" />
						Menghubungkan...
					{:else}
						<HugeiconsIcon icon={SparklesIcon} size={13} class="mr-1" />
						Tes Koneksi
					{/if}
				</Button>
			</div>

			<!-- Test Status Banner -->
			{#if testStatus}
				<div
					class="rounded-lg border px-3 py-2 text-xs flex items-center justify-between gap-2 {testStatus.success
						? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
						: 'border-destructive/30 bg-destructive/10 text-destructive'}"
				>
					<span class="font-medium">{testStatus.message}</span>
					{#if testStatus.model}
						<span class="rounded bg-background/50 px-1.5 py-0.5 text-[10px] font-mono opacity-80">
							{testStatus.model}
						</span>
					{/if}
				</div>
			{/if}
		</div>

		<!-- AI SQLite Cache Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<div class="flex items-center gap-2">
					<span class="text-sm font-medium">Cache Respons AI (SQLite)</span>
					<span class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-semibold text-muted-foreground">
						{aiCacheCount} respons tersimpan
					</span>
				</div>
				<p class="text-xs text-muted-foreground">
					Penjelasan AI yang pernah di-generate disimpan permanen di database lokal agar tidak membuang token dan langsung muncul instan (&lt; 1ms).
				</p>
			</div>

			<Button
				variant="outline"
				size="sm"
				onclick={handleClearAiCache}
				disabled={aiCacheCount === 0 || clearingAiCache}
				class="gap-1.5 cursor-pointer shrink-0 text-destructive hover:text-destructive"
			>
				{#if clearingAiCache}
					<HugeiconsIcon icon={RefreshIcon} size={12} class="animate-spin" />
				{/if}
				<span>Bersihkan Cache AI</span>
			</Button>
		</div>
	</div>
</section>

<!-- AnkiConnect Settings Section -->
<section class={GROUP}>
	<h3 class={LABEL}>Integrasi AnkiConnect</h3>
	<div class={CARD}>
		<!-- Connection Status Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<div class="flex items-center gap-2">
					<span class="text-sm font-medium">Status Koneksi Anki</span>
					{#if vocab.anki.connected}
						<span class="inline-flex items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[11px] font-semibold text-emerald-600 dark:text-emerald-400">
							<span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
							Terhubung (v{vocab.anki.version})
						</span>
					{:else}
						<span class="inline-flex items-center gap-1 rounded-full bg-amber-500/15 px-2 py-0.5 text-[11px] font-semibold text-amber-600 dark:text-amber-400">
							<span class="h-1.5 w-1.5 rounded-full bg-amber-500"></span>
							Anki Desktop Belum Dibuka
						</span>
					{/if}
				</div>
				<p class="text-xs text-muted-foreground">
					{vocab.anki.connected
						? 'Limusic terhubung langsung ke Anki Desktop di http://127.0.0.1:8765. Flashcard baru otomatis masuk ke deck pilihan.'
						: 'Buka Anki Desktop dan pastikan Add-on AnkiConnect (ID: 2055492111) telah aktif. Kartu tetap aman disimpan di database lokal jika Anki tutup.'}
				</p>
			</div>

			<Button
				variant="outline"
				size="sm"
				onclick={refreshAnki}
				disabled={checkingAnki}
				class="gap-1.5 cursor-pointer shrink-0"
			>
				<HugeiconsIcon icon={RefreshIcon} size={14} class={checkingAnki ? 'animate-spin' : ''} />
				<span>Cek Ulang</span>
			</Button>
		</div>

		<!-- Target Deck Selection Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Target Deck Anki</span>
				<p class="text-xs text-muted-foreground">
					Pilih deck tempat kartu kata baru disimpan saat Anda menekan tombol Anki (shortcut A).
				</p>
			</div>

			<div class="w-48 shrink-0">
				{#if vocab.decks.length > 0}
					<select
						bind:value={vocab.selectedDeck}
						class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
					>
						{#each vocab.decks as d}
							<option value={d}>{d}</option>
						{/each}
					</select>
				{:else}
					<input
						type="text"
						bind:value={vocab.selectedDeck}
						class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
						placeholder="Limusic::Immersion"
					/>
				{/if}
			</div>
		</div>

		<!-- Note Type / Model Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Model / Tipe Catatan</span>
				<p class="text-xs text-muted-foreground">
					Format field kartu: Front/Word, Sentence, Back/Meaning, Audio.
				</p>
			</div>

			<div class="w-48 shrink-0">
				{#if vocab.models.length > 0}
					<select
						bind:value={vocab.selectedModel}
						class="w-full rounded-lg border border-input bg-background px-3 py-1.5 text-xs text-foreground focus:outline-none focus:ring-1 focus:ring-primary"
					>
						{#each vocab.models as m}
							<option value={m}>{m}</option>
						{/each}
					</select>
				{:else}
					<span class="text-xs text-muted-foreground">Default (Basic)</span>
				{/if}
			</div>
		</div>

		<!-- Sync Intervals Row -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Sinkronisasi Interval dari Anki (Aturan Migaku ≥ 21 Hari)</span>
				<p class="text-xs text-muted-foreground">
					Kartu di Anki yang memiliki interval review 21 hari atau lebih otomatis dinaikkan statusnya menjadi "Sudah Dikuasai" (Known).
				</p>
			</div>

			<Button
				variant="secondary"
				size="sm"
				onclick={handleSyncIntervals}
				disabled={!vocab.anki.connected || syncingIntervals}
				class="gap-1.5 cursor-pointer shrink-0"
			>
				<HugeiconsIcon icon={RefreshIcon} size={14} class={syncingIntervals ? 'animate-spin' : ''} />
				<span>Sinkronkan Interval</span>
			</Button>
		</div>
	</div>
</section>

<!-- Vocabulary & Display Settings -->
<section class={GROUP}>
	<h3 class={LABEL}>Pengaturan Tampilan & Kosakata</h3>
	<div class={CARD}>
		<!-- Word Status Highlighting Toggle -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Tandai Status Kata Saat Membaca</span>
				<p class="text-xs text-muted-foreground">
					Tampilkan penanda status kata word-by-word (garis bawah merah untuk kata baru/Unknown, kuning untuk Learning, bersih untuk Known, dan redup untuk Ignored) saat membaca artikel, menonton video dengan transkrip, atau mendengarkan lirik musik. Jika dinonaktifkan, semua teks tampil biasa tanpa penanda status.
				</p>
			</div>

			<Switch
				checked={vocab.highlightEnabled}
				onCheckedChange={() => vocab.toggleHighlight()}
			/>
		</div>

		<!-- Vocabulary Level Calibration Wizard -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<div class="flex items-center gap-2">
					<span class="text-sm font-medium">Kalibrasi Level Kosakata (Wizard)</span>
					<span class="rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-primary">
						Step-by-Step
					</span>
				</div>
				<p class="text-xs text-muted-foreground">
					Buka wizard kalibrasi untuk memilih level kemampuan bahasa Inggris Anda (Pemula A1–A2, Menengah B1, atau Lanjutan B2) lalu seleksi kata-kata yang sudah Anda kuasai secara bertahap.
				</p>
			</div>

			<Button
				size="sm"
				onclick={() => (showCalibrationModal = true)}
				class="gap-1.5 cursor-pointer shrink-0"
			>
				<HugeiconsIcon icon={SparklesIcon} size={14} />
				<span>Kalibrasi Level</span>
			</Button>
		</div>

		<!-- Quick Presets -->
		<div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Tandai Cepat Berdasarkan Level (Quick Presets)</span>
				<p class="text-xs text-muted-foreground">
					Tandai seluruh kata pada level tertentu langsung sebagai 'Known' tanpa perlu konfirmasi manual satu per satu.
				</p>
			</div>

			<div class="flex items-center gap-1.5 shrink-0">
				<Button
					variant="outline"
					size="sm"
					onclick={() => handleQuickPreset('a1_a2')}
					disabled={bulkSeedingPreset !== null}
					class="text-xs h-8 px-2.5 cursor-pointer"
				>
					{bulkSeedingPreset === 'a1_a2' ? 'Memproses...' : '+ A1–A2 (1.000)'}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onclick={() => handleQuickPreset('b1')}
					disabled={bulkSeedingPreset !== null}
					class="text-xs h-8 px-2.5 cursor-pointer"
				>
					{bulkSeedingPreset === 'b1' ? 'Memproses...' : '+ B1 (2.500)'}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onclick={() => handleQuickPreset('b2')}
					disabled={bulkSeedingPreset !== null}
					class="text-xs h-8 px-2.5 cursor-pointer"
				>
					{bulkSeedingPreset === 'b2' ? 'Memproses...' : '+ B2 (5.000)'}
				</Button>
			</div>
		</div>

		<!-- Seed Common Words -->
		<div class="flex items-center justify-between gap-4 px-4 py-3.5">
			<div class="min-w-0 flex-1 space-y-0.5">
				<span class="text-sm font-medium">Tandai Kata Dasar Otomatis</span>
				<p class="text-xs text-muted-foreground">
					Tandai ~120 kata dasar bahasa Inggris paling sering digunakan (the, be, to, of, and, dll.) sebagai 'Known' agar tidak perlu ditandai manual satu per satu.
				</p>
			</div>

			<Button
				variant="outline"
				size="sm"
				onclick={handleSeedCommonWords}
				disabled={seedingWords}
				class="gap-1.5 cursor-pointer shrink-0"
			>
				<HugeiconsIcon icon={Tick02Icon} size={14} />
				<span>Tandai Kata Dasar</span>
			</Button>
		</div>
	</div>
</section>

<VocabOnboardingModal bind:open={showCalibrationModal} />

<!-- Recent Flashcards Section -->
{#if recentCards.length > 0}
	<section class={GROUP}>
		<div class="flex items-center justify-between mb-2 px-1">
			<h3 class="text-[11px] font-semibold uppercase tracking-[0.08em] text-muted-foreground">
				Flashcard Terbaru yang Dimining
			</h3>
			<span class="text-[10px] text-muted-foreground font-medium">{recentCards.length} kartu terakhir</span>
		</div>

		<div class="space-y-2">
			{#each recentCards as card (card.id)}
				<div class="flex flex-col gap-1 rounded-xl border border-border/60 bg-card p-3 text-xs shadow-xs transition hover:border-border">
					<div class="flex items-center justify-between">
						<span class="text-sm font-bold text-primary">{card.word}</span>
						{#if card.anki_note_id}
							<span class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-600 dark:text-emerald-400">
								Anki Note #{card.anki_note_id}
							</span>
						{:else}
							<span class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
								Lokal Limusic
							</span>
						{/if}
					</div>

					<p class="text-foreground/90 font-medium">
						"{card.sentence}"
					</p>

					<p class="italic text-muted-foreground">
						→ {card.translation}
					</p>

					{#if card.source_title}
						<div class="mt-1 flex items-center gap-1 text-[10px] text-muted-foreground/80">
							<span>Sumber:</span>
							<span class="truncate max-w-xs">{card.source_title}</span>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	</section>
{/if}
