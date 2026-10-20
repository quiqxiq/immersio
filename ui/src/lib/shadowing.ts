import { np, playSong, shadowing } from './player.svelte';
import type { SongItem } from './api';

/**
 * Start a movie scene in shadowing mode from anywhere (the Shadowing page, or the starter shelf on
 * Beranda). `np.open` is forced rather than left to the "open player on play" preference: choosing a
 * shadowing scene is a request to rehearse it, not background listening.
 */
export function startShadowingScene(v: SongItem) {
	shadowing.active = true;
	shadowing.lastPausedIndex = -1;
	shadowing.isWaiting = false;

	const item: SongItem = { ...v, is_video: true };
	playSong(item);
	np.open = true;
}
