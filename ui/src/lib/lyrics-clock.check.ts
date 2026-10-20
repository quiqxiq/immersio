// Self-check for the line clock (`lyrics-clock.ts`), run by `run-checks.mjs`:
//
//     node --experimental-strip-types ui/src/lib/lyrics-clock.check.ts
//
// Guards the two ways a line-level panel reads as "the lyrics are behind the song": a cue that
// waits for the next mpv tick instead of lighting on time, and a line that lights on time and then
// goes back out, because the tick that follows still carries a position rounded down behind the
// cue. The ticks below are a real mpv 0.41 sequence, gaps and 50 ms steps included.
import { lineClock, nextCue, onCue, onTick } from './lyrics-clock.ts';

function ok(cond: boolean, what: string): void {
	if (!cond) throw new Error(`FAIL: ${what}`);
}

// --- a cue lights on the cue, not on the tick after it ------------------------------------------
const cues = [100, 500, 700, 1000];
ok(nextCue(cues, 0) === 100, 'the first cue is the first wake');
ok(nextCue(cues, 100.001) === 500, 'a cue that has just passed is behind us');
ok(nextCue(cues, 1000) === null, 'no timer once the last cue has passed');

// --- the whole loop, the way the panel runs it --------------------------------------------------
// Wall-clock arrival and reported position, in ms, from mpv 0.41 playing a 60 s file. Note the
// fourth-from-last: the tick that arrives 37 ms *after* the cue at 500 still reports 496, which is
// the round-down that would flicker a line lit exactly on its cue.
const TICKS: Array<[arrival: number, pos: number]> = [
	[39, 0],
	[50, 8],
	[141, 99],
	[240, 199],
	[290, 249],
	[438, 397],
	[537, 496],
	[588, 546],
	[638, 596],
	[785, 744]
];
const CUE = 500;
const c = lineClock();
let shown = -1;
let fired = false;
let prevArrival = 0;
for (const [arrival, pos] of TICKS) {
	// the panel armed a timer for its next cue, so it fires exactly on that cue
	if (!fired && prevArrival < CUE && CUE <= arrival) {
		onCue(c, CUE);
		fired = true;
	}
	prevArrival = arrival;
	const now = onTick(c, pos);
	ok(now >= shown, `position does not go backwards (${now} after ${shown})`);
	if (arrival < CUE) {
		ok(now === pos, `before the cue the panel is at the reported position (${pos})`);
	} else {
		ok(now >= CUE, `from the cue on the line stays lit (arrival ${arrival}, shown ${now})`);
	}
	shown = now;
}
ok(fired, 'the cue timer fired');

// --- the tick that follows a fired cue does not put the line back ------------------------------
const d = lineClock();
ok(onCue(d, 500) === 500, 'a fired cue lifts the floor');
ok(onTick(d, 496) === 500, 'a tick rounded down behind the cue still shows the line');
ok(onTick(d, 548) === 548, 'the clock follows the ticks once they pass the cue');

// --- a seek is not a tick ----------------------------------------------------------------------
// The floor is 500 and the user jumps back well before it: the line goes with them.
const e = lineClock();
onCue(e, 500);
ok(onTick(e, 50) === 50, 'a seek behind the floor drops it');
ok(onTick(e, 120) === 120, 'and the next tick is the position again');
// A small step back is a tick's rounding, not a seek: the floor stays so the line does not blink.
const f = lineClock();
onCue(f, 500);
ok(onTick(f, 120) === 500, 'a step inside the tick window keeps the line lit');

console.log('ok');
