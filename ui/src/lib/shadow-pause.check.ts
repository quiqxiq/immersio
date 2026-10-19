// Self-check for the shadowing pause engine (`shadow-pause.ts`), run by `run-checks.mjs`:
//
//     node --experimental-strip-types ui/src/lib/shadow-pause.check.ts
//
// The bug it guards: with auto-pause on, Next/Prev (and Replay) re-paused the sentence the user
// just left, because the tick already in flight still reported the old end while `playing`
// landed before the seek's echo — and the nav had already cleared `lastPausedSentenceId`, so the
// old sentence looked new again. The scenarios below replay those event orders on a little model
// of the player view (position ticks, playback-state, nav clicks) and assert one pause per
// sentence boundary, in the order the sentences were actually played.
import { navReleased, pausePoint, seekStep } from './shadow-pause.ts';
import type { Sample } from './shadow-pause.ts';

function ok(cond: boolean, what: string): void {
	if (!cond) throw new Error(`FAIL: ${what}`);
}

type Card = { id: number; start_ms: number; end_ms: number };

/** The component's wiring, minus Svelte: the sentence-index effect and the pause effect run
 *  together on every position sample and every playback-state change, exactly as in
 *  `VideoPlayerView.svelte`. Times are ms; `now`/`at` are `performance.now()`. */
class Panel {
	activeIndex = -1;
	isWaiting = false;
	lastPausedSentenceId = -1;
	navAt = 0;
	navTargetMs: number | null = null;
	posMs = 0;
	atMs = 0;
	paused = false;
	/** Ids of the sentences the engine paused on, in order. */
	pauses: number[] = [];
	sentences: Card[];

	constructor(sentences: Card[]) {
		this.sentences = sentences;
	}

	/** Component effect #1: which card the reported position falls in. Locked while waiting. */
	updateIndex(): void {
		if (this.isWaiting) return;
		let found = -1;
		for (let s = 0; s < this.sentences.length; s++) {
			const st = this.sentences[s];
			if (this.posMs >= st.start_ms && this.posMs < st.end_ms + 100) {
				found = s;
				break;
			} else if (this.posMs >= st.start_ms) {
				found = s;
			}
		}
		this.activeIndex = found;
	}

	/** The auto-pause effect: the nav guard first, then the pause point. */
	pauseEffect(): void {
		if (this.navTargetMs !== null) {
			const sample: Sample = { posMs: this.posMs, atMs: this.atMs };
			if (!navReleased(this.navAt, this.navTargetMs, sample)) return;
			this.navTargetMs = null;
		}
		const point = pausePoint({
			posMs: this.posMs,
			sentences: this.sentences,
			activeIndex: this.activeIndex,
			paused: this.paused,
			isWaiting: this.isWaiting,
			lastPausedSentenceId: this.lastPausedSentenceId
		});
		if (!point) return;
		// The component: mark waiting, togglePause() — mpv answers with paused=true.
		this.lastPausedSentenceId = point.card.id;
		this.isWaiting = true;
		this.paused = true;
		this.pauses.push(point.card.id);
	}

	flush(): void {
		this.updateIndex();
		this.pauseEffect();
	}

	/** `api.onPosition`: mpv was at `posMs` at wall-clock `atMs`. */
	position(posMs: number, atMs: number): void {
		if (seekStep(this.posMs, posMs)) {
			// A seek from outside the buttons (the slider): the engine re-arms on its own.
			this.isWaiting = false;
			this.lastPausedSentenceId = -1;
			this.navTargetMs = null;
		}
		this.posMs = posMs;
		this.atMs = atMs;
		this.flush();
	}

	/** Normal playback up to `toPosMs`, in tick-sized steps (one huge step would read as a
	 *  slider seek, which real playback never produces). */
	walkTo(toPosMs: number, atMs: number): void {
		let pos = this.posMs;
		let at = atMs;
		while (pos < toPosMs) {
			pos = Math.min(pos + 1000, toPosMs);
			at += 100;
			this.position(pos, at);
		}
	}

	/** `playback-state`: mpv started or stopped. Effects re-run on the sample we already have. */
	state(paused: boolean): void {
		this.paused = paused;
		this.flush();
	}

	/** `jumpSentence(delta)`: seek to the neighbouring card and resume. */
	jump(delta: number, now: number): void {
		const cur = this.activeIndex >= 0 ? this.activeIndex : 0;
		const nextIdx = Math.max(0, Math.min(this.sentences.length - 1, cur + delta));
		this.navAt = now;
		this.navTargetMs = this.sentences[nextIdx].start_ms;
		this.activeIndex = nextIdx;
		// Back on the same card (the list edges): that is a replay, it may pause there again.
		if (nextIdx === cur) this.lastPausedSentenceId = -1;
		this.isWaiting = false;
	}

	/** `replayCurrentSentence`: seek to this card's start and resume; it may pause again. */
	replay(now: number): void {
		const idx = this.activeIndex >= 0 ? this.activeIndex : 0;
		this.navAt = now;
		this.navTargetMs = this.sentences[idx].start_ms;
		this.lastPausedSentenceId = -1;
		this.isWaiting = false;
	}

	/** `resumeNext` (button, timer, space): no seek — keep playing from where we stopped. */
	resume(): void {
		this.isWaiting = false;
		this.paused = false;
		this.flush();
	}
}

const CARDS: Card[] = [
	{ id: 0, start_ms: 1000, end_ms: 4000 },
	{ id: 1, start_ms: 5000, end_ms: 9000 },
	{ id: 2, start_ms: 9300, end_ms: 13000 },
	{ id: 3, start_ms: 13100, end_ms: 17000 }
];

/** Bring a panel to the state the bug reports start from: waiting, paused, on card 1. */
function waitingOnCard1(): Panel {
	const p = new Panel(CARDS);
	p.position(5000, 100);
	p.state(false);
	p.walkTo(8900, 300);
	p.position(9000, 900); // the tick that crosses card 1's pause point
	ok(p.isWaiting && p.paused, 'setup: the engine paused on card 1');
	ok(p.pauses.join() === '1', `setup: one pause on card 1, got [${p.pauses}]`);
	return p;
}

// --- the guard itself ---------------------------------------------------------------------------
ok(navReleased(1000, null, { posMs: 1, atMs: 0 }), 'no nav: every sample counts');
ok(!navReleased(1000, 9300, { posMs: 9000, atMs: 900 }), 'a sample from before the nav is the old position');
ok(!navReleased(1000, 9300, { posMs: 9050, atMs: 1010 }), 'an in-flight tick is not the echo');
ok(navReleased(1000, 9300, { posMs: 9310, atMs: 1010 }), 'the seek echo releases the guard');
ok(navReleased(1000, 9300, { posMs: 9050, atMs: 1800 }), 'past the timeout a late tick counts again');

// --- Next: `playing` lands before the seek echo -------------------------------------------------
// The reported order: click Next → mpv reports playing → the tick still in flight (old end) →
// the seek echo. Before the guard, `playing` + the tick re-paused card 1 on the spot.
{
	const p = waitingOnCard1();
	p.jump(1, 1000);
	p.state(false); // playing first, position still 9000 from before the click
	ok(p.pauses.join() === '1', `playing on the stale tick must not pause again, got [${p.pauses}]`);
	p.position(9050, 1010); // the in-flight tick
	ok(p.pauses.join() === '1', `the stale tick must not pause again, got [${p.pauses}]`);
	p.position(9310, 1050); // the seek echo
	ok(p.pauses.join() === '1', 'the echo sits at the card start, not its end');
	ok(p.activeIndex === 2, 'the card moved to the next sentence');
	p.walkTo(12960, 1200);
	ok(p.pauses.join() === '1,2', `the next pause is the next sentence, got [${p.pauses}]`);
}

// --- Next: the echo lands before `playing` ------------------------------------------------------
{
	const p = waitingOnCard1();
	p.jump(1, 1000);
	p.position(9300, 1005); // echo first, while the panel still believes it is paused
	ok(p.pauses.join() === '1', 'the echo must not pause anything on a paused player');
	p.state(false);
	p.walkTo(12960, 1200);
	ok(p.pauses.join() === '1,2', `one pause per boundary regardless of order, got [${p.pauses}]`);
}

// --- Replay: the tick that repeats the card end -------------------------------------------------
{
	const p = waitingOnCard1();
	p.replay(1000);
	p.state(false); // playing first, position still the old end
	ok(p.pauses.join() === '1', `replay: playing on the stale end must not re-pause, got [${p.pauses}]`);
	p.position(9010, 1010); // in-flight tick
	ok(p.pauses.join() === '1', `replay: the in-flight tick must not re-pause, got [${p.pauses}]`);
	p.position(5010, 1050); // the echo at the sentence start
	ok(p.pauses.join() === '1', 'replay: the echo starts the sentence, no pause');
	p.walkTo(8960, 1200);
	ok(p.pauses.join() === '1,1', `replay: the sentence pauses again when it ends, got [${p.pauses}]`);
}

// --- Prev: the tick keeps describing the end of the sentence we left ----------------------------
{
	const p = waitingOnCard1();
	p.jump(-1, 1000);
	p.state(false);
	p.position(1010, 1050); // echo at card 0's start
	p.walkTo(3960, 1200);
	ok(p.pauses.join() === '1,0', `Prev advances one sentence back, got [${p.pauses}]`);
}

// --- Next with a gap under the echo tolerance ---------------------------------------------------
// 9000 → 9050 puts the old end inside the echo window. The kept `lastPausedSentenceId` (card 1)
// is the second line of defense, because the stale tick still maps to card 1.
{
	const p = new Panel([
		{ id: 0, start_ms: 1000, end_ms: 4000 },
		{ id: 1, start_ms: 5000, end_ms: 9000 },
		{ id: 2, start_ms: 9050, end_ms: 13000 }
	]);
	p.position(5000, 100);
	p.state(false);
	p.walkTo(8900, 300);
	p.position(9000, 900);
	ok(p.pauses.join() === '1', 'tight-gap setup pauses on card 1');
	p.jump(1, 1000);
	p.state(false);
	p.position(9000, 1010); // stale tick that also looks like the echo
	ok(p.pauses.join() === '1', `tight gap: the stale tick must not re-pause, got [${p.pauses}]`);
	p.position(9060, 1020);
	p.walkTo(12960, 1100);
	ok(p.pauses.join() === '1,2', `tight gap: the next pause is the next sentence, got [${p.pauses}]`);
}

// --- resumeNext and the timer resume: no seek, no re-pause on the card we stopped on ------------
{
	const p = waitingOnCard1();
	p.resume();
	ok(p.pauses.join() === '1', `resume must not re-pause the paused card, got [${p.pauses}]`);
	p.walkTo(12960, 1200);
	ok(p.pauses.join() === '1,2', `after resume the next pause is the next sentence, got [${p.pauses}]`);
}

// --- a clamped Next (last card) behaves like a replay -------------------------------------------
{
	const p = new Panel(CARDS);
	p.position(13100, 100);
	p.state(false);
	p.walkTo(15900, 300);
	p.position(16960, 900); // card 3's pause point
	ok(p.pauses.join() === '3', `clamped setup: paused on the last card, got [${p.pauses}]`);
	p.jump(1, 1000); // no card after 3: stays on 3
	p.state(false);
	p.position(16970, 1010); // in-flight tick
	p.position(13110, 1050); // echo
	p.walkTo(16960, 1200);
	ok(p.pauses.join() === '3,3', `clamped Next replays the last card, got [${p.pauses}]`);
}

// --- an external seek (the slider) re-arms the engine -------------------------------------------
{
	const p = waitingOnCard1();
	p.position(12000, 1100); // dragged into card 2: a far step, no nav guard
	ok(p.isWaiting === false, 'a slider seek clears the waiting state');
	ok(p.pauses.join() === '1', `a slider seek must not pause where it lands, got [${p.pauses}]`);
	p.state(false); // the drag resumed playback (or the user presses play)
	p.walkTo(12960, 1200);
	ok(p.pauses.join() === '1,2', `after a slider seek the next pause is where it landed, got [${p.pauses}]`);
}

// --- pausePoint's window bounds -----------------------------------------------------------------
const window = [CARDS[1]];
const at = (posMs: number, paused: boolean, isWaiting: boolean, lastPausedSentenceId: number) =>
	pausePoint({ posMs, sentences: window, activeIndex: 0, paused, isWaiting, lastPausedSentenceId });
ok(at(8959, false, false, -1) === null, 'just before the window: no pause');
ok(at(8960, false, false, -1) !== null, 'the window opens 40 ms before the card end');
ok(at(9000, false, false, 1) === null, 'the card that just paused is skipped');
ok(at(9000, true, false, -1) === null, 'a paused player is not paused again');
ok(at(9000, false, true, -1) === null, 'a waiting panel is not paused again');
ok(at(10301, false, false, -1) === null, 'past the window (last card: end + 800 + 500): no pause');

console.log('ok');
