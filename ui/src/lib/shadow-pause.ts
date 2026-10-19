// The shadowing auto-pause engine's decisions, separated from `VideoPlayerView.svelte` so the
// self-check (`shadow-pause.check.ts`) can drive them through the event orders that used to
// re-pause the sentence the user just left (Next/Prev "stuck repeating the same sentence").
//
// The player emits `position` right when it seeks, but the tick already in flight still carries
// the pre-seek position, and `playback-state: playing` can land before either. Acting on that
// stale tick re-pauses the sentence the user just left, and because the panel locks the card
// while `isWaiting`, Next/Prev look like they repeat themselves. So a nav (Next, Prev, Replay,
// the slider) arms a guard that ignores samples until a sample that can only be post-seek
// arrives: the seek's own echo, or — when the player emits no echo — anything past a timeout.

/** One reported position: the value and when we heard it. */
export type Sample = {
	/** The reported position, in ms. */
	posMs: number;
	/** `performance.now()` when it arrived (`playback.positionAt`). */
	atMs: number;
};

/** How far the seek's echo may sit from its target and still count as the landing. */
export const NAV_ECHO_MS = 150;

/** If no echo shows up (a player that does not report seeks), whatever arrives this long after
 *  the nav is past the seek too: give the guard up so auto-pause cannot fall silent. */
export const NAV_TIMEOUT_MS = 700;

/** Whether `sample` may drive a pause decision. False while it still describes the position the
 *  nav left behind. */
export function navReleased(navAt: number, navTargetMs: number | null, sample: Sample): boolean {
	if (navTargetMs === null) return true;
	if (sample.atMs >= navAt && Math.abs(sample.posMs - navTargetMs) <= NAV_ECHO_MS) return true;
	return sample.atMs >= navAt + NAV_TIMEOUT_MS;
}

/** A step far beyond what one tick covers is a seek from outside the shadowing buttons (the
 *  player's slider), not playback. */
export const SEEK_JUMP_MS = 1500;

export function seekStep(prevPosMs: number, posMs: number): boolean {
	return Math.abs(posMs - prevPosMs) > SEEK_JUMP_MS;
}

export type PauseCard = { id: number; start_ms: number; end_ms: number };

export type PauseInput = {
	posMs: number;
	sentences: readonly PauseCard[];
	activeIndex: number;
	paused: boolean;
	isWaiting: boolean;
	lastPausedSentenceId: number;
};

/** The sentence to pause on for the reported position, or null. Stop 40 ms before the card ends
 *  (or 70 ms before the next cue) so the next sentence never leaks in, and pause at most once per
 *  card — a nav that may re-pause the same card (Replay, a clamped Next) clears the id first. */
export function pausePoint(input: PauseInput): { index: number; card: PauseCard } | null {
	if (input.paused || input.isWaiting) return null;
	const { sentences, activeIndex } = input;
	if (activeIndex < 0 || activeIndex >= sentences.length) return null;
	const card = sentences[activeIndex];
	if (input.lastPausedSentenceId === card.id) return null;
	const nextStartMs = activeIndex + 1 < sentences.length
		? sentences[activeIndex + 1].start_ms
		: card.end_ms + 800;
	const targetMs = Math.min(card.end_ms - 40, nextStartMs - 70);
	if (input.posMs < targetMs || input.posMs > nextStartMs + 500) return null;
	return { index: activeIndex, card };
}
