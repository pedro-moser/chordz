import { writable } from 'svelte/store';
import { getRoots, getAllScales, getPairs, getFretboardNotes } from './wasm';
import type { ScaleInfo, PairInfo, FretNote } from './wasm';

export const rootIndex = writable(0);
export const scaleIndex = writable(1);
export const pairIndex = writable(0);
export const showIntervals = writable(false);
export const drawerOpen = writable(true);

export const roots = writable<string[]>([]);
export const scales = writable<ScaleInfo[]>([]);
export const pairs = writable<PairInfo[]>([]);
export const fretboardNotes = writable<FretNote[][]>([]);

export function initStores() {
  roots.set(getRoots());
  scales.set(getAllScales());
  pairs.set(getPairs());
  fretboardNotes.set(getFretboardNotes());
}
