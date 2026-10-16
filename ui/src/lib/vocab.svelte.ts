// Reactive Language Immersion & Vocabulary State (Svelte 5 Runes)
import * as api from './api';
import type { VocabCounts, AnkiStatus, MineCardRequest, FlashcardRow } from './api';
import { toast } from './player.svelte';
import {
	cleanWordToken,
	getLemmaCandidates,
	getPrimaryLemma,
	CONTRACTIONS_MAP,
	IRREGULAR_VERBS_MAP
} from './lemma';

export { cleanWordToken, getLemmaCandidates, getPrimaryLemma };

export type WordStatus = 'known' | 'learning' | 'ignored' | 'unknown';

export interface DictSense {
	pos: string;
	definition: string;
	example?: string;
	examples?: string[];
	synonyms?: string[];
}

export interface DictLookupResult {
	word: string;
	lemma?: string;
	phonetic?: string;
	audio_url?: string;
	senses: DictSense[];
	translation_id?: string;
	sentence_translation_id?: string;
	provider?: string;
	from_cache?: boolean;
}

export interface TextStats {
	totalWords: number;
	knownWords: number;
	newWordsCount: number;
	comprehensionPct: number;
}

/**
 * Normalizes a word to its canonical base lemma using the rich lemmatization engine.
 */
export function normalizeLemma(word: string): string {
	return getPrimaryLemma(word) || cleanWordToken(word);
}

const DICT_CACHE = new Map<string, DictLookupResult>();

class VocabStore {
	statuses = $state<Record<string, string>>({});
	counts = $state<VocabCounts>({ known: 0, learning: 0, ignored: 0, flashcards: 0 });
	anki = $state<AnkiStatus>({ connected: false, port: 8765 });
	decks = $state<string[]>([]);
	selectedDeck = $state<string>(localStorage.getItem('limusic_anki_deck') || 'English');
	models = $state<string[]>([]);
	selectedModel = $state<string>(localStorage.getItem('limusic_anki_model') || 'Basic');
	highlightEnabled = $state<boolean>(localStorage.getItem('limusic_vocab_highlight') !== 'false');

	// Modular Rust Dictionary & Translation Engine settings
	dictProvider = $state<string>(localStorage.getItem('limusic_dict_provider') || 'cambridge');
	transProvider = $state<string>(localStorage.getItem('limusic_trans_provider') || 'google');
	targetLang = $state<string>(localStorage.getItem('limusic_dict_target_lang') || 'id');
	autoFallback = $state<boolean>(localStorage.getItem('limusic_dict_auto_fallback') !== 'false');
	dictCacheCount = $state<number>(0);

	loaded = $state<boolean>(false);

	get highlightLearning(): boolean {
		return this.highlightEnabled;
	}
	set highlightLearning(val: boolean) {
		this.highlightEnabled = val;
		localStorage.setItem('limusic_vocab_highlight', String(val));
	}

	async init() {
		if (this.loaded) return;
		try {
			const [s, c, cacheCount] = await Promise.all([
				api.getWordStatuses('en').catch(() => ({})),
				api.getVocabCounts('en').catch(() => ({ known: 0, learning: 0, ignored: 0, flashcards: 0 })),
				api.getDictCacheCount().catch(() => 0)
			]);
			this.statuses = s || {};
			this.counts = c || { known: 0, learning: 0, ignored: 0, flashcards: 0 };
			this.dictCacheCount = cacheCount;
			this.loaded = true;
			this.checkAnki();
		} catch (e) {
			console.error('Failed to init vocab store:', e);
		}
	}

	normalizeLemma(rawWord: string): string {
		return normalizeLemma(rawWord);
	}

	getStatus(rawWord: string): WordStatus {
		if (!rawWord) return 'unknown';
		const candidates = getLemmaCandidates(rawWord);
		if (candidates.length === 0) return 'unknown';

		// 1. Direct match on surface token
		const exact = candidates[0];
		if (this.statuses[exact]) {
			return this.statuses[exact] as WordStatus;
		}

		// 2. Check each candidate in priority order (primary lemma, constituent root, etc.)
		for (let i = 1; i < candidates.length; i++) {
			const cand = candidates[i];
			if (this.statuses[cand]) {
				return this.statuses[cand] as WordStatus;
			}
		}

		return 'unknown';
	}

	async setStatus(rawWord: string, status: WordStatus) {
		const clean = cleanWordToken(rawWord);
		if (!clean) return;
		const primaryLemma = getPrimaryLemma(clean);

		const old = this.getStatus(clean);

		if (status === 'unknown') {
			delete this.statuses[clean];
			if (primaryLemma && primaryLemma !== clean) {
				delete this.statuses[primaryLemma];
			}
			this.statuses = { ...this.statuses };
		} else {
			this.statuses[clean] = status;
			if (primaryLemma && primaryLemma !== clean) {
				this.statuses[primaryLemma] = status;
			}
			this.statuses = { ...this.statuses };
		}

		// Update counts optimistically
		if (old !== status) {
			if (old === 'known') this.counts.known = Math.max(0, this.counts.known - 1);
			if (old === 'learning') this.counts.learning = Math.max(0, this.counts.learning - 1);
			if (old === 'ignored') this.counts.ignored = Math.max(0, this.counts.ignored - 1);

			if (status === 'known') this.counts.known++;
			if (status === 'learning') this.counts.learning++;
			if (status === 'ignored') this.counts.ignored++;
		}

		try {
			await api.setWordStatus(clean, status, 'en');
			if (primaryLemma && primaryLemma !== clean && status !== 'unknown') {
				api.setWordStatus(primaryLemma, status, 'en').catch(() => {});
			}
		} catch (e) {
			console.error('Failed to persist word status:', e);
		}
	}

	async bulkKnown(words: string[]): Promise<number> {
		try {
			const count = await api.bulkSetKnown(words, 'en');
			// Refresh statuses and counts
			const [s, c] = await Promise.all([api.getWordStatuses('en'), api.getVocabCounts('en')]);
			this.statuses = s || {};
			this.counts = c || { known: 0, learning: 0, ignored: 0 };
			return count;
		} catch (e) {
			console.error('Failed bulkSetKnown:', e);
			return 0;
		}
	}

	setDeck(deck: string) {
		this.selectedDeck = deck;
		localStorage.setItem('limusic_anki_deck', deck);
	}

	setModel(model: string) {
		this.selectedModel = model;
		localStorage.setItem('limusic_anki_model', model);
	}

	toggleHighlight() {
		this.highlightEnabled = !this.highlightEnabled;
		localStorage.setItem('limusic_vocab_highlight', String(this.highlightEnabled));
	}

	setDictProvider(p: string) {
		this.dictProvider = p;
		localStorage.setItem('limusic_dict_provider', p);
	}

	setTransProvider(p: string) {
		this.transProvider = p;
		localStorage.setItem('limusic_trans_provider', p);
	}

	setTargetLang(l: string) {
		this.targetLang = l;
		localStorage.setItem('limusic_dict_target_lang', l);
	}

	setAutoFallback(v: boolean) {
		this.autoFallback = v;
		localStorage.setItem('limusic_dict_auto_fallback', String(v));
	}

	async refreshCacheCount() {
		try {
			this.dictCacheCount = await api.getDictCacheCount();
		} catch (e) {
			console.warn('Failed to get dict cache count:', e);
		}
	}

	async clearCache() {
		try {
			const count = await api.clearDictCache();
			this.dictCacheCount = 0;
			DICT_CACHE.clear();
			toast(`Cache kamus dibersihkan (${count} entri dihapus).`);
		} catch (e: any) {
			toast.error(`Gagal membersihkan cache: ${e?.message || e}`);
		}
	}

	computeTextStats(text: string): TextStats {
		if (!text || !text.trim()) {
			return { totalWords: 0, knownWords: 0, newWordsCount: 0, comprehensionPct: 100 };
		}
		// Match words including apostrophes (e.g. didn't, they're, it's)
		const tokens = text.match(/[a-zA-Z]+(?:['’][a-zA-Z]+)*/g) || [];
		if (tokens.length === 0) {
			return { totalWords: 0, knownWords: 0, newWordsCount: 0, comprehensionPct: 100 };
		}

		let total = 0;
		let known = 0;
		const newLemmas = new Set<string>();

		for (const rawToken of tokens) {
			const clean = cleanWordToken(rawToken);
			if (!clean || clean.length < 2) continue;

			const st = this.getStatus(clean);
			// Status 'ignored' is completely excluded from the learning burden
			if (st === 'ignored') continue;

			total++;
			if (st === 'known') {
				known++;
			} else if (st === 'unknown') {
				newLemmas.add(getPrimaryLemma(clean));
			}
		}

		const comprehensionPct = total > 0 ? Math.round((known / total) * 100) : 100;
		return {
			totalWords: total,
			knownWords: known,
			newWordsCount: newLemmas.size,
			comprehensionPct
		};
	}

	getWordClass(rawWord: string): string {
		const baseClass = 'hover:bg-primary/15 cursor-pointer select-text transition-colors duration-150';
		if (!this.highlightEnabled) return baseClass;

		const st = this.getStatus(rawWord);
		switch (st) {
			case 'unknown':
				return `border-b-2 border-rose-500 hover:bg-rose-500/15 ${baseClass}`;
			case 'learning':
				return `border-b-2 border-amber-500 hover:bg-amber-500/15 font-medium ${baseClass}`;
			case 'known':
				return `border-b-2 border-transparent ${baseClass}`;
			case 'ignored':
				return `opacity-40 hover:opacity-80 border-b-2 border-transparent ${baseClass}`;
			default:
				return baseClass;
		}
	}

	async checkAnki(): Promise<AnkiStatus> {
		try {
			const st = await api.ankiStatus();
			this.anki = st;
			if (st.connected) {
				const [d, m]: [string[], string[]] = await Promise.all([
					api.ankiGetDecks().catch(() => [] as string[]),
					api.ankiGetModels().catch(() => [] as string[])
				]);
				this.decks = d;
				this.models = m;
				if (d.length > 0 && (!this.selectedDeck || !d.includes(this.selectedDeck))) {
					const pref = d.find((name) => /english|mining|default/i.test(name)) || d[0];
					this.setDeck(pref);
				}
				if (m.length > 0 && (!this.selectedModel || !m.includes(this.selectedModel))) {
					const pref = m.find((name) => /basic|mining/i.test(name)) || m[0];
					this.setModel(pref);
				}
			}
			return st;
		} catch {
			this.anki = { connected: false, port: 8765, error: 'Tidak dapat terhubung' };
			return this.anki;
		}
	}

	async mineCard(req: Omit<MineCardRequest, 'deck_name'>): Promise<api.MineCardResponse> {
		const fullReq: MineCardRequest = {
			...req,
			deck_name: this.selectedDeck || 'English'
		};

		// 1. Optimistically set word status to 'learning'
		this.setStatus(req.word, 'learning');

		try {
			const res = await api.ankiMineCard(fullReq);
			if (res.success) {
				toast(`Kartu '${req.word}' ditambahkan ke Anki (${this.selectedDeck})`);
			} else {
				toast(res.message || 'Kartu disimpan di Limusic');
			}
			return res;
		} catch (e: any) {
			const msg = e?.message || String(e);
			toast(`Anki tidak aktif: Flashcard disimpan di database Limusic`);
			// Save locally
			await api.saveFlashcard({
				id: `fc-${Date.now()}`,
				word: req.word,
				lemma: req.lemma,
				sentence: req.sentence,
				translation: req.translation,
				source_title: req.source_title,
				source_url: req.source_url,
				status: 'mined',
				created_at: Math.floor(Date.now() / 1000)
			}).catch(() => {});
			return { success: false, message: msg };
		}
	}

	async syncAnkiIntervals(): Promise<number> {
		try {
			const count = await api.ankiSyncIntervals('en');
			if (count > 0) {
				toast(`${count} kata dipromosikan menjadi 'Known' dari Anki!`);
				const [s, c] = await Promise.all([api.getWordStatuses('en'), api.getVocabCounts('en')]);
				this.statuses = s || {};
				this.counts = c || { known: 0, learning: 0, ignored: 0 };
			} else {
				toast('Semua kata sudah tersinkronisasi dengan Anki.');
			}
			return count;
		} catch (e: any) {
			toast(`Gagal sinkronisasi Anki: ${e?.message || e}`);
			return 0;
		}
	}

	/**
	 * Instant native dictionary lookup powered by 100% Rust backend
	 * with Cambridge / Oxford / Wiktionary / WordNet, translation, and SQLite caching.
	 * Automatically falls back to base lemma when inflected/contraction entry has no definition.
	 */
	async lookup(rawWord: string, sentenceContext = ''): Promise<DictLookupResult> {
		const clean = cleanWordToken(rawWord);
		if (!clean) {
			return { word: rawWord, senses: [] };
		}

		const primaryLemma = getPrimaryLemma(clean);

		try {
			let res = await api.lookupWord(
				clean,
				sentenceContext || undefined,
				this.dictProvider,
				this.transProvider,
				this.targetLang,
				this.autoFallback
			);

			// If no dictionary senses found for an inflected form or contraction,
			// fallback lookup using the primary lemma (e.g. "didn't" -> "do", "ran" -> "run")
			if (res.senses.length === 0 && primaryLemma && primaryLemma !== clean) {
				const fallbackRes = await api.lookupWord(
					primaryLemma,
					sentenceContext || undefined,
					this.dictProvider,
					this.transProvider,
					this.targetLang,
					this.autoFallback
				).catch(() => null);

				if (fallbackRes && fallbackRes.senses.length > 0) {
					res = {
						...fallbackRes,
						word: clean, // Keep the original word clicked
						word_translation: res.word_translation || fallbackRes.word_translation,
						sentence_translation: res.sentence_translation || fallbackRes.sentence_translation
					};
				}
			}

			// Refresh cache count in background
			this.refreshCacheCount();

			return {
				word: res.word,
				lemma: primaryLemma !== clean ? primaryLemma : undefined,
				phonetic: res.phonetic || undefined,
				audio_url: res.audio_url || undefined,
				senses: res.senses.map((s) => ({
					pos: s.pos,
					definition: s.definition,
					example: s.examples && s.examples.length > 0 ? s.examples[0] : undefined,
					examples: s.examples
				})),
				translation_id: res.word_translation || undefined,
				sentence_translation_id: res.sentence_translation || undefined,
				provider: res.provider,
				from_cache: res.from_cache
			};
		} catch (e) {
			console.error('Native lookup error:', e);
			return {
				word: clean,
				lemma: primaryLemma !== clean ? primaryLemma : undefined,
				senses: [],
				provider: this.dictProvider
			};
		}
	}

	async translateText(text: string, targetLang = this.targetLang): Promise<string | undefined> {
		if (!text || !text.trim()) return undefined;
		try {
			const res = await api.lookupWord(
				'word',
				text.trim(),
				'cambridge',
				this.transProvider,
				targetLang,
				false
			);
			return res.sentence_translation || undefined;
		} catch {
			return undefined;
		}
	}
}

export const vocab = new VocabStore();
