// The clock a line-level lyrics panel moves on. Separated from `LyricsView.svelte` so the
// self-check (`lyrics-clock.check.ts`) can drive it through a real sequence of mpv ticks.
//
// mpv reports `time-pos` on its own schedule: the value is rounded down to its 50 ms update step
// and the notification lands up to ~150 ms after the moment it describes (measured against mpv
// 0.41; see the gaps in the check). A line-level panel has no word sweep to justify a frame loop
// (docs/UI-PERFORMANCE.md), and simply waiting for the next tick is what gets reported as "the
// lyrics are behind the song": the cue lights up on the tick that follows it, up to a step late,
// and the fade adds to that. So the panel arms a timer for its next cue, and holds the line once
// that timer has fired, because the ticks that follow it are still behind the cue they passed.

/** One panel's line clock. Plain state, mirrored into `$state` by the component. */
export type LineClock = {
	/** The cue the timer has already lit, in the same ms as `posMs`, or 0. */
	floorMs: number;
};

export const lineClock = (): LineClock => ({ floorMs: 0 });

/** How far the reported position may fall behind the floor without it being a seek instead of the
 *  next 50 ms step. A tick never goes backwards, so anything past this is the user jumping. */
const SEEK_MS = 400;

/** A position tick, in `posMs` terms. Drops the floor when the position jumped backwards past it
 *  (a seek: neither the cue nor the line it lit means anything there) and answers where the panel
 *  is now. */
export function onTick(c: LineClock, posMs: number): number {
	if (c.floorMs - posMs > SEEK_MS) c.floorMs = 0;
	return Math.max(posMs, c.floorMs);
}

/** The cue timer fired exactly on its cue: hold it until the reported position catches up.
 *  Answers the floor the panel renders with. */
export function onCue(c: LineClock, cueMs: number): number {
	c.floorMs = Math.max(c.floorMs, cueMs);
	return c.floorMs;
}

/** The first cue strictly after `posMs`, or null when the track's cues are done. `cues` comes from
 *  the lyrics, in the same ms as `posMs` and already sorted by time. */
export function nextCue(cues: readonly number[], posMs: number): number | null {
	for (const t of cues) if (t > posMs) return t;
	return null;
}
