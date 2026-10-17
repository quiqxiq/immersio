import { KokoroTTS } from 'kokoro-js';

export interface TTSVoiceOption {
	id: string;
	name: string;
	gender: 'Female' | 'Male';
	accent: string;
	flag: string;
	desc: string;
}

export const KOKORO_VOICES: TTSVoiceOption[] = [
	{ id: 'af_heart', name: 'Heart', gender: 'Female', accent: 'American', flag: '🇺🇸', desc: 'Hangat, ekspresif, & sangat natural (Pilihan Utama)' },
	{ id: 'af_sarah', name: 'Sarah', gender: 'Female', accent: 'American', flag: '🇺🇸', desc: 'Jelas, bersahabat, & nada santai' },
	{ id: 'am_adam', name: 'Adam', gender: 'Male', accent: 'American', flag: '🇺🇸', desc: 'Narator buku audio, bariton dalam & tegas' },
	{ id: 'am_michael', name: 'Michael', gender: 'Male', accent: 'American', flag: '🇺🇸', desc: 'Percakapan kasual & intonasi hidup' },
	{ id: 'bf_emma', name: 'Emma', gender: 'Female', accent: 'British', flag: '🇬🇧', desc: 'Aksen British formal (RP), artikulasi tajam' },
	{ id: 'bm_george', name: 'George', gender: 'Male', accent: 'British', flag: '🇬🇧', desc: 'Aksen British klasik, suara tenang & berwibawa' },
	{ id: 'af_nicole', name: 'Nicole', gender: 'Female', accent: 'American', flag: '🇺🇸', desc: 'Lembut & ritme membaca stabil' },
	{ id: 'bf_isabella', name: 'Isabella', gender: 'Female', accent: 'British', flag: '🇬🇧', desc: 'Elegan & tempo teratur' }
];

class TTSStore {
	// Engine & Model State
	engine = $state<'kokoro' | 'system'>('kokoro');
	modelLoaded = $state(false);
	loading = $state(false);
	loadProgress = $state(0);
	loadStatusText = $state('');
	error = $state<string | null>(null);

	// Voice & Audio Parameters
	voice = $state('af_heart');
	speed = $state(1.0);

	// Playback State
	playing = $state(false);
	paused = $state(false);
	generating = $state(false);
	activeSentenceIndex = $state<number | null>(null);
	sentences = $state<string[]>([]);

	// Internal instances
	private kokoroInstance: KokoroTTS | null = null;
	private currentAudio: HTMLAudioElement | null = null;
	private currentUtterance: SpeechSynthesisUtterance | null = null;
	private audioCache = new Map<string, string>();
	private playId = 0;

	constructor() {
		if (typeof window !== 'undefined') {
			const savedEngine = localStorage.getItem('limusic_tts_engine');
			if (savedEngine === 'kokoro' || savedEngine === 'system') {
				this.engine = savedEngine;
			}
			const savedVoice = localStorage.getItem('limusic_tts_voice');
			if (savedVoice) {
				this.voice = savedVoice;
			}
			const savedSpeed = localStorage.getItem('limusic_tts_speed');
			if (savedSpeed) {
				const n = parseFloat(savedSpeed);
				if (!isNaN(n) && n >= 0.5 && n <= 2.0) {
					this.speed = n;
				}
			}
		}
	}

	setEngine(eng: 'kokoro' | 'system') {
		this.engine = eng;
		if (typeof window !== 'undefined') {
			localStorage.setItem('limusic_tts_engine', eng);
		}
	}

	setVoice(v: string) {
		this.voice = v;
		if (typeof window !== 'undefined') {
			localStorage.setItem('limusic_tts_voice', v);
		}
		// If currently playing, we can continue or keep cache
	}

	setSpeed(s: number) {
		this.speed = s;
		if (typeof window !== 'undefined') {
			localStorage.setItem('limusic_tts_speed', String(s));
		}
		if (this.currentAudio) {
			this.currentAudio.playbackRate = s;
		}
		if (this.currentUtterance && this.playing) {
			// Restart utterance with new speed
			if (this.activeSentenceIndex !== null) {
				this.jumpTo(this.activeSentenceIndex);
			}
		}
	}

	/**
	 * Initialize or download the Kokoro ONNX model on-demand
	 */
	async initKokoro(): Promise<boolean> {
		if (this.kokoroInstance) {
			this.modelLoaded = true;
			return true;
		}

		if (this.loading) return false;

		this.loading = true;
		this.error = null;
		this.loadProgress = 0;
		this.loadStatusText = 'Mempersiapkan model neural Kokoro-82M...';

		try {
			// Use quantized q8 ONNX weights (~86MB total) with WASM device for universal Linux GTK support
			const tts = await KokoroTTS.from_pretrained('onnx-community/Kokoro-82M-v1.0-ONNX', {
				dtype: 'q8',
				device: 'wasm',
				progress_callback: (progress: any) => {
					if (!progress) return;
					if (progress.status === 'progress' && progress.total > 0) {
						const pct = Math.round((progress.loaded / progress.total) * 100);
						this.loadProgress = pct;
						const file = progress.file || 'model.onnx';
						const loadedMb = (progress.loaded / (1024 * 1024)).toFixed(1);
						const totalMb = (progress.total / (1024 * 1024)).toFixed(1);
						this.loadStatusText = `Mengunduh ${file} (${loadedMb}/${totalMb} MB)... ${pct}%`;
					} else if (progress.status === 'initiate') {
						this.loadStatusText = `Memulai unduhan komponen ${progress.file || ''}...`;
					} else if (progress.status === 'done') {
						this.loadStatusText = 'Komponen selesai dimuat.';
					}
				}
			});

			this.kokoroInstance = tts;
			this.modelLoaded = true;
			this.loadStatusText = 'Model Kokoro-82M siap digunakan!';
			return true;
		} catch (err: any) {
			console.error('Failed to load Kokoro TTS:', err);
			this.error = err?.message || 'Gagal memuat model Kokoro TTS. Beralih ke suara sistem.';
			this.engine = 'system';
			return false;
		} finally {
			this.loading = false;
		}
	}

	/**
	 * Synthesize a single sentence text to a Blob URL
	 */
	async synthesizeSentence(text: string, voiceId: string): Promise<string> {
		const clean = text.trim();
		const cacheKey = `${voiceId}_${clean}`;
		if (this.audioCache.has(cacheKey)) {
			return this.audioCache.get(cacheKey)!;
		}

		if (!this.kokoroInstance) {
			const ok = await this.initKokoro();
			if (!ok || !this.kokoroInstance) {
				throw new Error('Kokoro TTS engine tidak tersedia');
			}
		}

		const rawAudio = await this.kokoroInstance.generate(clean, {
			voice: voiceId as any,
			speed: 1.0 // apply speed at audio element level for instantaneous speed changes
		});

		const blob = rawAudio.toBlob();
		const url = URL.createObjectURL(blob);
		this.audioCache.set(cacheKey, url);
		return url;
	}

	/**
	 * Prefetch the next sentence in background for gapless playback
	 */
	private prefetchNext(index: number) {
		const nextIndex = index + 1;
		if (nextIndex < this.sentences.length && this.engine === 'kokoro' && this.kokoroInstance) {
			const nextText = this.sentences[nextIndex];
			if (nextText && nextText.trim()) {
				this.synthesizeSentence(nextText, this.voice).catch(() => {});
			}
		}
	}

	/**
	 * Play article sentences sequentially with karaoke auto-highlight
	 */
	async playArticle(sentences: string[], startIndex: number = 0) {
		this.stop();
		this.sentences = sentences;
		this.playing = true;
		this.paused = false;

		if (this.engine === 'kokoro' && !this.modelLoaded) {
			const ok = await this.initKokoro();
			if (!ok) {
				// Fallback to system voice if Kokoro fails or is cancelled
				this.engine = 'system';
			}
		}

		await this.playSentence(startIndex);
	}

	/**
	 * Play a specific sentence by index
	 */
	async playSentence(index: number): Promise<void> {
		if (index < 0 || index >= this.sentences.length) {
			this.stop();
			return;
		}

		const thisPlayId = ++this.playId;
		this.activeSentenceIndex = index;
		this.playing = true;
		this.paused = false;

		const text = this.sentences[index];
		if (!text || !text.trim()) {
			// Skip empty sentence
			return this.playSentence(index + 1);
		}

		if (this.engine === 'kokoro' && this.kokoroInstance) {
			try {
				this.generating = true;
				// Start prefetching next sentence while synthesizing/playing
				this.prefetchNext(index);

				const audioUrl = await this.synthesizeSentence(text, this.voice);
				if (this.playId !== thisPlayId) return; // Stale play command

				this.generating = false;

				if (this.currentAudio) {
					this.currentAudio.pause();
					this.currentAudio = null;
				}

				const audio = new Audio(audioUrl);
				audio.playbackRate = this.speed;
				this.currentAudio = audio;

				audio.onended = () => {
					if (this.playId === thisPlayId && this.playing && !this.paused) {
						this.playSentence(index + 1);
					}
				};

				audio.onerror = (e) => {
					console.error('Audio playback error:', e);
					if (this.playId === thisPlayId) {
						this.playSentence(index + 1);
					}
				};

				await audio.play();
			} catch (err: any) {
				console.error('Kokoro synthesis error, falling back to system:', err);
				this.generating = false;
				if (this.playId === thisPlayId) {
					this.playSystemUtterance(text, index, thisPlayId);
				}
			}
		} else {
			// System speech synthesis (Web Speech API)
			this.playSystemUtterance(text, index, thisPlayId);
		}
	}

	private playSystemUtterance(text: string, index: number, thisPlayId: number) {
		if (typeof window === 'undefined' || !('speechSynthesis' in window)) {
			this.stop();
			return;
		}

		window.speechSynthesis.cancel();
		const utter = new SpeechSynthesisUtterance(text);
		utter.rate = this.speed;
		utter.lang = this.voice.startsWith('b') ? 'en-GB' : 'en-US';

		utter.onend = () => {
			if (this.playId === thisPlayId && this.playing && !this.paused) {
				this.playSentence(index + 1);
			}
		};

		utter.onerror = (e) => {
			if (e.error !== 'canceled' && this.playId === thisPlayId) {
				this.playSentence(index + 1);
			}
		};

		this.currentUtterance = utter;
		window.speechSynthesis.speak(utter);
	}

	pause() {
		this.paused = true;
		if (this.currentAudio) {
			this.currentAudio.pause();
		}
		if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
			window.speechSynthesis.pause();
		}
	}

	resume() {
		this.paused = false;
		if (this.currentAudio) {
			this.currentAudio.play().catch(() => {});
		} else if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
			window.speechSynthesis.resume();
		} else if (this.activeSentenceIndex !== null) {
			this.playSentence(this.activeSentenceIndex);
		}
	}

	stop() {
		this.playId++;
		this.playing = false;
		this.paused = false;
		this.generating = false;
		this.activeSentenceIndex = null;

		if (this.currentAudio) {
			this.currentAudio.pause();
			this.currentAudio = null;
		}

		if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
			window.speechSynthesis.cancel();
		}
		this.currentUtterance = null;
	}

	next() {
		if (this.activeSentenceIndex !== null && this.activeSentenceIndex + 1 < this.sentences.length) {
			this.playSentence(this.activeSentenceIndex + 1);
		}
	}

	prev() {
		if (this.activeSentenceIndex !== null && this.activeSentenceIndex > 0) {
			this.playSentence(this.activeSentenceIndex - 1);
		}
	}

	jumpTo(index: number) {
		this.playSentence(index);
	}

	/**
	 * Speak a single word or short phrase on demand (e.g. from dictionary popup)
	 */
	async speakSingle(text: string) {
		const clean = text.trim();
		if (!clean) return;

		if (this.engine === 'kokoro' && this.kokoroInstance) {
			try {
				const audioUrl = await this.synthesizeSentence(clean, this.voice);
				const audio = new Audio(audioUrl);
				audio.playbackRate = this.speed;
				await audio.play();
				return;
			} catch (e) {
				console.error('Failed to speak single with Kokoro:', e);
			}
		}

		// Fallback to system Web Speech API
		if (typeof window !== 'undefined' && 'speechSynthesis' in window) {
			window.speechSynthesis.cancel();
			const utter = new SpeechSynthesisUtterance(clean);
			utter.lang = this.voice.startsWith('b') ? 'en-GB' : 'en-US';
			utter.rate = this.speed * 0.95;
			window.speechSynthesis.speak(utter);
		}
	}
}

export const tts = new TTSStore();
