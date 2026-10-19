// Sentence reconstruction for transcripts where the backend did not send its own sentences:
// pure functions over `LyricLine[]`, no Svelte runes, so `run-checks.mjs` can run them under plain
// Node (see `shadow-segment.check.ts`).
import type { LyricLine, LyricSentence } from './api';

function cleanDialogueSpeech(text: string): string {
	return text
		.replace(/\[[^\]]*\]/g, ' ')
		.replace(/\([^\)]*\)/g, ' ')
		.replace(/[♪♫>]+/g, ' ')
		.replace(/[^\p{L}\p{N}\s]/gu, '')
		.trim();
}

function splitDialogueLines(rawLines: LyricLine[]): LyricLine[] {
	const out: LyricLine[] = [];
	for (const line of rawLines) {
		const rawText = (line.text || '').replace(/\r/g, '');
		if (!rawText.trim()) continue;

		const startMs = line.time_ms ?? 0;
		const durMs = line.end_time_ms ? Math.max(1000, line.end_time_ms - startMs) : 2500;

		const splitParts: string[] = [];
		const nlParts = rawText.split('\n').map(l => l.trim()).filter(Boolean);
		if (nlParts.length > 1 && nlParts.slice(1).some(l => /^[-—>]/.test(l))) {
			splitParts.push(...nlParts);
		} else if (rawText.includes('>>')) {
			splitParts.push(...rawText.split('>>').map(s => s.trim()).filter(Boolean));
		} else {
			const delim = rawText.includes(' — ') ? ' — ' : (rawText.includes(' - ') ? ' - ' : null);
			if (delim) {
				const idx = rawText.indexOf(delim);
				const left = rawText.slice(0, idx).trim();
				const lastChar = left.slice(-1);
				if (/[.?!\"'”’]/.test(lastChar)) {
					const right = rawText.slice(idx + delim.length).trim();
					if (left && right) {
						splitParts.push(left, right);
					}
				}
			}
		}

		if (splitParts.length <= 1) {
			out.push(line);
			continue;
		}

		const cleanParts = splitParts.map(p => p.replace(/^[-—>]+\s*/, '').replace(/\s+/g, ' ').trim()).filter(Boolean);
		const wordCounts = cleanParts.map(p => Math.max(1, p.split(/\s+/).filter(Boolean).length));
		const totalWords = wordCounts.reduce((a, b) => a + b, 0);

		let curMs = startMs;
		for (let i = 0; i < cleanParts.length; i++) {
			const partDur = i + 1 === cleanParts.length
				? Math.max(500, startMs + durMs - curMs)
				: Math.max(400, Math.round(durMs * (wordCounts[i] / totalWords)));
			const endMs = curMs + partDur;
			out.push({
				time_ms: curMs,
				end_time_ms: endMs,
				text: cleanParts[i]
			});
			curMs = endMs;
		}
	}
	return out;
}

/**
 * Client-side sentence reconstruction for transcripts where backend sentences are absent.
 *
 * Mirrors the Rust v2 segmenter (`reconstruct_sentences` in `lyrics.rs`): speaker turns stay
 * apart, a sentence is never cut mid-phrase, and a 1-2 word tail joins the sentence it continues.
 * The numbers come from the corpus in `docs/shadowing-lab` (see RESULTS.md); change them there
 * first, then here and in Rust.
 */
export function reconstructSentences(rawLines: LyricLine[]): LyricSentence[] {
	if (!rawLines || rawLines.length === 0) return [];
	const lines = splitDialogueLines(rawLines);

	const ABBR_SET = new Set([
		'mr.', 'mrs.', 'ms.', 'dr.', 'prof.', 'sr.', 'jr.', 'st.',
		'u.s.', 'u.k.', 'e.g.', 'i.e.', 'etc.', 'vs.', 'inc.', 'ltd.', 'corp.',
		'jan.', 'feb.', 'mar.', 'apr.', 'jun.', 'jul.', 'aug.', 'sep.', 'sept.', 'oct.', 'nov.', 'dec.',
		'no.', 'vol.', 'dept.', 'est.', 'approx.', 'min.', 'sec.', 'hr.'
	]);

	const INCOMPLETE = new Set([
		'a', 'an', 'the',
		'in', 'on', 'at', 'to', 'for', 'with', 'of', 'from', 'by', 'about', 'into', 'through',
		'his', 'her', 'my', 'their', 'our', 'its', 'your', 'this', 'that', 'these', 'those',
		'he', 'she', 'it', 'we', 'they', 'i', 'you', 'who', 'whom', 'whose', 'which',
		'is', 'are', 'was', 'were', 'be', 'been', 'being', 'have', 'has', 'had',
		'will', 'would', 'shall', 'should', 'can', 'could', 'may', 'might', 'must',
		'too', 'very', 'so', 'more', 'most', 'quite', 'really',
		'and', 'or', 'but', 'nor', 'yet', 'because', 'if', 'when', 'than', 'as'
	]);

	/** Trim quote/bracket chars from both ends (Rust `trim_matches`; Python `strip`). */
	function trimPunct(w: string): string {
		return w
			.replace(/^["'()[\]“”‘’]+/, '')
			.replace(/["'()[\]“”‘’]+$/, '');
	}

	function isAbbreviation(w: string): boolean {
		const clean = trimPunct(w).toLowerCase();
		if (ABBR_SET.has(clean)) return true;
		if (clean.length === 2 && clean.endsWith('.') && /[a-z]/i.test(clean[0])) return true;
		if (/^[\d.,]+$/.test(clean) && clean.includes('.')) return true;
		return false;
	}

	function endsWithTerminator(w: string): boolean {
		const clean = w
			.replace(/^["'()[\]“”’]+/, '')
			.replace(/["'()[\]“”’]+$/, '');
		if (isAbbreviation(clean)) return false;
		return (
			clean.endsWith('.') ||
			clean.endsWith('?') ||
			clean.endsWith('!') ||
			clean.endsWith('...') ||
			clean.endsWith('…')
		);
	}

	function isClauseStarter(w: string): boolean {
		const clean = w.replace(/^[^\p{L}]+/u, '').toLowerCase();
		return ['and', 'but', 'so', 'because', 'if', 'when', 'then', 'while', 'although', 'now', 'well', 'or', 'however'].includes(clean);
	}

	function isIncompleteEnding(w: string): boolean {
		if (endsWithTerminator(w)) return false;
		const clean = w
			.replace(/^[^\p{L}]+/u, '')
			.replace(/[^\p{L}]+$/u, '')
			.toLowerCase();
		return INCOMPLETE.has(clean);
	}

	function isNonVerbal(text: string): boolean {
		return /^(\[[^\]]*\]|\([^\)]*\)|[♪♫>]+)$/.test(text.trim());
	}

	/** Speaker-turn marker ("-", "—", ">>") or non-verbal tag: bounds a card, never part of one. */
	function isSeparator(text: string): boolean {
		const t = text.trim();
		return isNonVerbal(text) || (t.length > 0 && /^[-—>]+$/.test(t));
	}

	/** "-Hello" / ">>She": strip the marker, but the caller flushes the turn first. */
	function stripMarkerPrefix(text: string): string {
		const stripped = text.replace(/^[-—>]+\s*/, '').trim();
		return stripped || text.trim();
	}

	/** A terminator, except multi-letter initials ("A.A.", "J.R."). */
	function isTerminatorV2(text: string): boolean {
		const trimmed = text
			.trim()
			.replace(/^["'()[\]“”’]+/, '')
			.replace(/["'()[\]“”’]+$/, '');
		return !/^([A-Za-z]\.){2,}$/.test(trimmed) && endsWithTerminator(text);
	}

	function calculatePause(durMs: number, count: number): number {
		const audioBased = Math.round(durMs * 1.25);
		const wordBased = count * 450;
		const base = Math.max(audioBased, wordBased) + 800;
		return Math.min(12000, Math.max(1800, base));
	}

	interface WordToken {
		text: string;
		start_ms: number;
		end_ms: number;
	}

	const words: WordToken[] = [];
	for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
		const line = lines[lineIdx];
		const lineText = (line.text || '').trim();
		if (!lineText) continue;

		const startMs = line.time_ms ?? 0;
		const endMs = line.end_time_ms ?? (
			lineIdx + 1 < lines.length && lines[lineIdx + 1].time_ms != null
				? lines[lineIdx + 1].time_ms!
				: startMs + Math.max(1500, lineText.split(/\s+/).filter(Boolean).length * 380)
		);

		if (line.words && line.words.length > 0) {
			for (const lw of line.words) {
				const subTokens = lw.text.trim().split(/\s+/).filter(Boolean);
				if (subTokens.length <= 1) {
					words.push({
						text: lw.text,
						start_ms: lw.start_ms,
						end_ms: lw.end_ms
					});
				} else {
					const totalSubDur = Math.max(subTokens.length * 150, lw.end_ms - lw.start_ms);
					const subDur = Math.floor(totalSubDur / subTokens.length);
					for (let sIdx = 0; sIdx < subTokens.length; sIdx++) {
						const sStart = lw.start_ms + (sIdx * subDur);
						const sEnd = (sIdx + 1 === subTokens.length) ? lw.end_ms : (sStart + subDur);
						words.push({
							text: subTokens[sIdx],
							start_ms: sStart,
							end_ms: sEnd
						});
					}
				}
			}
			continue;
		}

		const tokens = lineText.split(/\s+/).filter(Boolean);
		if (tokens.length === 0) continue;

		const totalDur = Math.max(tokens.length * 200, endMs - startMs);
		const durPerWord = Math.floor(totalDur / tokens.length);

		for (let wIdx = 0; wIdx < tokens.length; wIdx++) {
			const wStart = startMs + (wIdx * durPerWord);
			const wEnd = (wIdx + 1 === tokens.length) ? endMs : (wStart + durPerWord);
			words.push({
				text: tokens[wIdx],
				start_ms: wStart,
				end_ms: wEnd
			});
		}
	}

	if (words.length === 0) return [];

	const hasPunctuation = words
		.filter(w => !isSeparator(w.text))
		.some(w => endsWithTerminator(w.text));

	interface V2Card {
		text: string;
		start_ms: number;
		end_ms: number;
		word_count: number;
		reason: string;
	}

	const cards: V2Card[] = [];
	let current: { token: WordToken; clean: string }[] = [];
	let quotesDouble = 0;
	let quotesLeft = 0;
	let quotesRight = 0;
	const insideQuote = () => quotesDouble % 2 === 1 || quotesLeft > quotesRight;

	const flush = (reason: string) => {
		if (current.length > 0) {
			const text = current.map(w => w.clean).join(' ');
			// Non-verbal-only runs are never cards (they still show in the transcript lines).
			if (cleanDialogueSpeech(text)) {
				cards.push({
					text,
					start_ms: current[0].token.start_ms,
					end_ms: current[current.length - 1].token.end_ms,
					word_count: current.length,
					reason
				});
			}
		}
		current = [];
		quotesDouble = 0;
		quotesLeft = 0;
		quotesRight = 0;
	};

	for (let i = 0; i < words.length; i++) {
		const raw = words[i];
		if (isSeparator(raw.text)) {
			// Markers and non-verbal tags bound the turn; they are never part of a card.
			flush('marker');
			continue;
		}

		const cleanText = stripMarkerPrefix(raw.text);
		if (cleanText !== raw.text.trim()) {
			// A word carrying a marker prefix ("-Hello", ">>She") starts a speaker turn.
			flush('marker');
		}

		current.push({ token: raw, clean: cleanText });
		quotesDouble += (cleanText.match(/"/g) || []).length;
		quotesLeft += (cleanText.match(/“/g) || []).length;
		quotesRight += (cleanText.match(/”/g) || []).length;

		const next = i + 1 < words.length ? words[i + 1] : null;
		if (!next || isSeparator(next.text)) continue;
		const gap = Math.max(0, next.start_ms - raw.end_ms);
		const curLen = current.length;

		if (hasPunctuation) {
			if (isTerminatorV2(cleanText) && !insideQuote()) flush('terminator');
			else if (curLen >= 50) flush('salvage-cap');
			else if (curLen >= 42 && gap >= 250) flush('salvage-cap');
			else if (curLen >= 30 && gap >= 500) flush('salvage-cap');
			else if (curLen >= 14 && gap >= 800) flush('salvage-silence');
			else if (curLen >= 14 && cleanText.endsWith(',') && gap >= 500) flush('salvage-comma');
			else if (gap >= 3000) flush('silence');
			else if (gap >= 2000 && !isIncompleteEnding(cleanText)) flush('silence');
			continue;
		}

		// Unpunctuated transcript (raw ASR without periods).
		if (curLen >= 36) flush('cap');
		else if (isIncompleteEnding(cleanText)) {
			// Never end a card on "the", "of", "to": the sentence continues.
		} else if (curLen >= 28 && gap >= 150) flush('cap');
		else if (isTerminatorV2(cleanText) && !insideQuote()) flush('terminator');
		else if (gap >= 900) flush('silence');
		else if (gap >= 400 && isClauseStarter(next.text) && curLen >= 5) flush('clause');
		else if (curLen >= 20 && (gap >= 250 || curLen >= 28)) flush('cap');
	}
	flush('end');

	const joinText = (a: string, b: string) => `${a.trim()} ${b.trim()}`.trim();
	const sameWord = (a: string, b: string) => {
		const ca = cleanDialogueSpeech(a).toLowerCase();
		const cb = cleanDialogueSpeech(b).toLowerCase();
		return ca.length > 0 && ca === cb;
	};

	// Very short utterances attach to the previous card when they clearly continue it: it ended
	// at a silence, or the two repeat each other.
	const merged: V2Card[] = [];
	for (const card of cards) {
		if (merged.length > 0 && card.word_count <= 2) {
			const prev = merged[merged.length - 1];
			const gap = Math.max(0, card.start_ms - prev.end_ms);
			const afterSilence =
				(prev.reason === 'silence' || prev.reason === 'salvage-silence') && gap >= 900;
			const repeat = prev.word_count <= 2 && gap <= 800 && sameWord(prev.text, card.text);
			if (afterSilence || repeat) {
				prev.text = joinText(prev.text, card.text);
				prev.end_ms = card.end_ms;
				prev.word_count += card.word_count;
				continue;
			}
		}
		merged.push(card);
	}

	// A short utterance followed immediately by more speech becomes the next card's prefix —
	// unless it starts a speaker turn: turns must stay apart.
	for (let i = 0; i < merged.length; ) {
		const card = merged[i];
		const startsTurn =
			card.reason === 'marker' || (i > 0 && merged[i - 1].reason === 'marker');
		if (card.word_count <= 2 && !startsTurn && i + 1 < merged.length) {
			const next = merged[i + 1];
			const gapAfter = Math.max(0, next.start_ms - card.end_ms);
			const gapBefore =
				i > 0 ? Math.max(0, card.start_ms - merged[i - 1].end_ms) : Number.MAX_SAFE_INTEGER;
			if (gapAfter <= 250 && gapAfter <= gapBefore) {
				next.text = joinText(card.text, next.text);
				next.start_ms = card.start_ms;
				next.word_count += card.word_count;
				merged.splice(i, 1);
				continue;
			}
		}
		i++;
	}

	return merged.map((card, id) => {
		const durationMs = Math.max(0, card.end_ms - card.start_ms);
		return {
			id,
			text: card.text,
			start_ms: card.start_ms,
			end_ms: card.end_ms,
			duration_ms: durationMs,
			word_count: card.word_count,
			suggested_pause_ms: calculatePause(durationMs, card.word_count),
			skip_pause: false
		};
	});
}
