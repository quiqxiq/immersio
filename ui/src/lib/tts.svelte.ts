import * as api from '$lib/api';

export interface TTSVoiceOption {
	id: string;
	name: string;
	gender: 'Female' | 'Male';
	accent: string;
	flag: string;
	desc: string;
	badge?: string;
}

export const TTS_VOICES: TTSVoiceOption[] = [
	{
		id: 'en-US-JennyNeural',
		name: 'Jenny (US)',
		gender: 'Female',
		accent: 'American',
		flag: '🇺🇸',
		desc: 'Suara studio natural, artikulasi jernih & intonasi hangat (Pilihan Utama)',
		badge: 'Edge Neural'
	},
	{
		id: 'en-US-GuyNeural',
		name: 'Guy (US)',
		gender: 'Male',
		accent: 'American',
		flag: '🇺🇸',
		desc: 'Suara pria Amerika bersahabat, ritme santai dan jelas',
		badge: 'Edge Neural'
	},
	{
		id: 'en-GB-SoniaNeural',
		name: 'Sonia (UK)',
		gender: 'Female',
		accent: 'British',
		flag: '🇬🇧',
		desc: 'Aksen British elegan, vokal tajam & pelafalan standar RP',
		badge: 'Edge Neural'
	},
	{
		id: 'en-GB-RyanNeural',
		name: 'Ryan (UK)',
		gender: 'Male',
		accent: 'British',
		flag: '🇬🇧',
		desc: 'Aksen pria British naratif, artikulasi teratur dan lugas',
		badge: 'Edge Neural'
	},
	{
		id: 'en-AU-NatashaNeural',
		name: 'Natasha (AU)',
		gender: 'Female',
		accent: 'Australian',
		flag: '🇦🇺',
		desc: 'Aksen Australian natural & tempo santai',
		badge: 'Edge Neural'
	},
	{
		id: 'espeak',
		name: 'Offline System (Linux)',
		gender: 'Male',
		accent: 'Offline',
		flag: '⚡',
		desc: 'Sintesis lokal 100% offline via espeak-ng, tanpa kuota internet',
		badge: 'Offline'
	}
];

class TTSStore {
	// Voice & Audio Parameters
	voice = $state('en-US-JennyNeural');
	speed = $state(1.0);

	// Playback State
	playing = $state(false);
	paused = $state(false);
	generating = $state(false);
	activeSentenceIndex = $state<number | null>(null);
	sentences = $state<string[]>([]);
	error = $state<string | null>(null);

	// Audio element & cache
	private currentAudio: HTMLAudioElement | null = null;
	private audioCache = new Map<string, string>();
	private playId = 0;

	constructor() {
		if (typeof window !== 'undefined') {
			const savedVoice = localStorage.getItem('limusic_tts_voice');
			if (savedVoice) {
				if (savedVoice === 'en') this.voice = 'en-US-JennyNeural';
				else if (savedVoice === 'en-GB') this.voice = 'en-GB-SoniaNeural';
				else if (savedVoice === 'en-AU') this.voice = 'en-AU-NatashaNeural';
				else this.voice = savedVoice;
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

	setVoice(v: string) {
		this.voice = v;
		if (typeof window !== 'undefined') {
			localStorage.setItem('limusic_tts_voice', v);
		}
		// If currently playing, replay active sentence with new voice
		if (this.playing && this.activeSentenceIndex !== null) {
			this.playSentence(this.activeSentenceIndex);
		}
	}

	setSpeed(s: number) {
		this.speed = s;
		if (typeof window !== 'undefined') {
			localStorage.setItem('limusic_tts_speed', String(s));
		}
		if (this.currentAudio) {
			this.currentAudio.playbackRate = s;
		}
	}

	/**
	 * Synthesize a single sentence text to a base64 audio data URL via Rust backend
	 */
	async synthesizeSentence(text: string, voiceId: string): Promise<string> {
		const clean = text.trim();
		const cacheKey = `${voiceId}:${clean}`;
		if (this.audioCache.has(cacheKey)) {
			return this.audioCache.get(cacheKey)!;
		}

		// Calls Tauri Rust backend which fetches natural MP3 or falls back to espeak-ng WAV
		const dataUrl = await api.synthesizeSpeech(clean, voiceId, this.speed);
		this.audioCache.set(cacheKey, dataUrl);
		return dataUrl;
	}

	/**
	 * Prefetch the next sentence in background for gapless playback
	 */
	private prefetchNext(index: number) {
		const nextIndex = index + 1;
		if (nextIndex < this.sentences.length) {
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
		this.error = null;
		this.playing = true;
		this.paused = false;

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
		this.error = null;

		const text = this.sentences[index];
		if (!text || !text.trim()) {
			// Skip empty sentence
			return this.playSentence(index + 1);
		}

		try {
			this.generating = true;
			// Start prefetching next sentence in background
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
			console.error('TTS synthesis error:', err);
			this.generating = false;
			this.error = err?.message || 'Gagal menyuarakan kalimat';
			if (this.playId === thisPlayId && this.playing && !this.paused) {
				// Advance to next after short delay
				setTimeout(() => {
					if (this.playId === thisPlayId) {
						this.playSentence(index + 1);
					}
				}, 1000);
			}
		}
	}

	pause() {
		this.paused = true;
		if (this.currentAudio) {
			this.currentAudio.pause();
		}
	}

	resume() {
		this.paused = false;
		if (this.currentAudio) {
			this.currentAudio.play().catch(() => {});
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

		try {
			const audioUrl = await this.synthesizeSentence(clean, this.voice);
			const audio = new Audio(audioUrl);
			audio.playbackRate = this.speed;
			await audio.play();
		} catch (e) {
			console.error('Failed to speak single word with backend TTS:', e);
		}
	}
}

export const tts = new TTSStore();
