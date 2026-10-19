// Self-check for the client-side segmenter (`shadow-segment.ts`), run by `run-checks.mjs`:
//
//     node --experimental-strip-types ui/src/lib/shadow-segment.check.ts
//
// The same fixtures as the Rust tests in `lyrics.rs::tests` (via the Python reference in
// `docs/shadowing-lab/prototype/segment_v2.py`): the two implementations have to cut the same
// captions the same way, or the transcript panel and the shadowing view drift apart.
import { reconstructSentences } from './shadow-segment.ts';
import type { LyricLine, LyricWord } from './api.ts';

function ok(cond: boolean, what: string): void {
	if (!cond) throw new Error(`FAIL: ${what}`);
}

function texts(lines: LyricLine[]): string[] {
	return reconstructSentences(lines).map((s) => s.text);
}

function line(time_ms: number | undefined, text: string, opts: Partial<LyricLine> = {}): LyricLine {
	return { time_ms, text, ...opts };
}

function word(text: string, start_ms: number, end_ms: number): LyricWord {
	return { text, start_ms, end_ms };
}

// 1. Abbreviations ("Dr.", "Mr.") do not end a sentence.
{
	const cards = reconstructSentences([
		line(1000, 'Dr. Watson said that Mr. Holmes was ready. But you', { end_time_ms: 3000 }),
		line(3000, 'must be very careful.', { end_time_ms: 5000 })
	]);
	ok(cards.length === 2, `abbreviations: 2 cards, got ${cards.length}`);
	ok(cards[0].text === 'Dr. Watson said that Mr. Holmes was ready.', cards[0].text);
	ok(cards[1].text === 'But you must be very careful.', cards[1].text);
}

// 2. Speaker turns from split dialogue cues stay one card each (the tiny "Nothing." continues
//    into "Just waiting." because it is the same turn; turns are kept apart).
{
	const lines = [
		line(10000, 'What are you doing here?', { end_time_ms: 12500 }),
		line(12500, 'Nothing. Just waiting.', { end_time_ms: 14000 }),
		line(15000, 'Look at that!', { end_time_ms: 17250 }),
		line(17250, 'Where?', { end_time_ms: 18000 })
	];
	const cards = texts(lines);
	ok(cards.length === 4, `turns: 4 cards, got ${cards.length}`);
	ok(cards[0] === 'What are you doing here?', cards[0]);
	ok(cards[1] === 'Nothing. Just waiting.', cards[1]);
	ok(cards[2] === 'Look at that!', cards[2]);
	ok(cards[3] === 'Where?', cards[3]);
}

// 3. ">>" markers bound the turn but never become text in a card.
{
	const cards = texts([
		line(1000, '>> What are you doing here?'),
		line(3000, '>> Nothing.'),
		line(4500, 'Just waiting.')
	]);
	ok(cards.length === 3, `markers: 3 cards, got ${cards.length}`);
	ok(cards.every((t) => !t.includes('>')), cards.join(' | '));
	ok(cards[0] === 'What are you doing here?', cards[0]);
}

// 4. A marker glued to the first word starts the turn and is stripped.
{
	const cards = texts([
		line(1000, 'This is the first sentence.', { end_time_ms: 3000 }),
		line(3200, '-Hello there.')
	]);
	ok(cards.length === 2, `marker prefix: 2 cards, got ${cards.length}`);
	ok(cards[1] === 'Hello there.', cards[1]);
}

// 5. Multi-letter initials ("A.A.", "J.R.") read as one word.
{
	const cards = texts([line(1000, 'A.A. Ron is here. J.R. too.')]);
	ok(cards.length === 2, `initials: 2 cards, got ${cards.length}`);
	ok(cards[0] === 'A.A. Ron is here.', cards[0]);
	ok(cards[1] === 'J.R. too.', cards[1]);
}

// 6. Non-verbal tags are never cards (they stay in the transcript lines).
{
	const cards = reconstructSentences([
		line(1000, '[Music]'),
		line(2000, 'Hello there.', { end_time_ms: 5000 }),
		line(5000, '(applause)', { end_time_ms: 7000 }),
		line(7000, '♪', { end_time_ms: 9000 })
	]);
	ok(cards.length === 1, `non-verbal: 1 card, got ${cards.length}`);
	ok(cards[0].text === 'Hello there.', cards[0].text);
	ok(cards.every((s) => !s.skip_pause), 'v2 never skips pause');
}

// 7. Salvage net: a 900ms pause inside a 16-word punctuated run ends the card there.
{
	const words16: LyricWord[] = Array.from({ length: 16 }, (_, i) =>
		word(`w${i + 1}`, 1000 + i * 1000, 2000 + i * 1000)
	);
	const cards = reconstructSentences([
		{
			time_ms: 1000,
			end_time_ms: 17000,
			text: words16.map((w) => w.text).join(' '),
			words: words16
		},
		{
			time_ms: 17900,
			end_time_ms: 19900,
			text: 'seventeen eighteen nineteen twenty.',
			words: [
				word('seventeen', 17900, 18400),
				word('eighteen', 18400, 18900),
				word('nineteen', 18900, 19400),
				word('twenty.', 19400, 19900)
			]
		}
	]);
	ok(cards.length === 2, `salvage: 2 cards, got ${cards.length}`);
	ok(cards[0].word_count === 16, `salvage: first card 16 words, got ${cards[0].word_count}`);
	ok(cards[0].end_ms === 17000, `salvage: ends at 17000, got ${cards[0].end_ms}`);
}

// 8. Unpunctuated runs stay bounded instead of one giant card.
{
	const cards = reconstructSentences([
		line(1000, Array.from({ length: 60 }, (_, i) => `w${i}`).join(' '), { end_time_ms: 20000 })
	]);
	const counts = cards.map((s) => s.word_count);
	ok(counts.join(',') === '28,28,4', `unpunctuated caps: got ${counts.join(',')}`);
}

// 9. A 1-2 word tail after a silence joins the sentence it continues.
{
	const cards = texts([
		line(1000, 'these things will appear', { end_time_ms: 5000 }),
		line(7100, 'upgrade.', { end_time_ms: 8000 })
	]);
	ok(cards.length === 1, `tail merge: 1 card, got ${cards.length}`);
	ok(cards[0] === 'these things will appear upgrade.', cards[0]);
}

// 10. A fragment that starts a turn (marker before it) does not merge forward.
{
	const cards = texts([
		line(1000, 'Hello there', { end_time_ms: 3000 }),
		line(3200, '>> Yes.', { end_time_ms: 4200 }),
		line(4400, 'I agree completely.', { end_time_ms: 6400 })
	]);
	ok(cards.join('|') === 'Hello there|Yes.|I agree completely.', cards.join('|'));
}

// 11. Rolling YouTube word timings (overlapping captions) still come out as the same two
//     sentences the Rust segmenter makes.
{
	const cards = reconstructSentences([
		{
			time_ms: 60239,
			end_time_ms: 62630,
			text: 'protecting his hand. He was lifting his',
			words: [
				word('protecting', 60239, 60589),
				word('his', 60640, 60990),
				word('hand.', 60879, 61229),
				word('He', 61840, 62190),
				word('was', 62000, 62350),
				word('lifting', 62160, 62510),
				word('his', 62399, 62749)
			]
		},
		{
			time_ms: 62640,
			end_time_ms: 66600,
			text: 'whole card way too high.',
			words: [
				word('whole', 62640, 62990),
				word('card', 62879, 63229),
				word('way', 63120, 63470),
				word('too', 63359, 63709),
				word('high.', 63600, 63950)
			]
		}
	]);
	ok(cards.length === 2, `rolling captions: 2 cards, got ${cards.length}`);
	ok(cards[0].text === 'protecting his hand.' && cards[0].end_ms === 61229, cards[0].text);
	ok(
		cards[1].text === 'He was lifting his whole card way too high.' && cards[1].end_ms === 63950,
		cards[1].text
	);
}

// 12. Punctuation inside parentheses: "(Mr. Worldwide)" is not a sentence end.
{
	const cards = reconstructSentences([
		line(
			1000,
			"It's a new generation of party people (Mr. Worldwide) get on the floor.",
			{ end_time_ms: 6000 }
		)
	]);
	ok(cards.length === 1, `parenthesised title: 1 card, got ${cards.length}`);
	ok(
		cards[0].text === "It's a new generation of party people (Mr. Worldwide) get on the floor.",
		cards[0].text
	);
}

console.log('ok');
