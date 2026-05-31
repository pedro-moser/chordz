import type { GmcPatternBlock } from '$lib/wasm';

export interface PatternPreset {
  label: string;
  blocks: GmcPatternBlock[];
  /** 0=Eighth, 1=Sixteenth, 2=Triplet. Omitted → leave the current figure as-is. */
  figureIndex?: number;
}

// roles 0/1/2 = root/3rd/5th of the stacked-thirds pair. The first three are the original
// generic presets; the rest are distilled from Pedro's .gp études (2026-05-31 pattern mining).
export const PATTERN_PRESETS: PatternPreset[] = [
  {
    label: 'Alternating 3+3',
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1' },
      { count: 3, direction: 'desc', triad: 'T2' },
    ],
  },
  {
    label: 'Continuous up',
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1' },
      { count: 3, direction: 'asc', triad: 'T2' },
    ],
  },
  {
    label: 'Short-long',
    blocks: [
      { count: 2, direction: 'asc', triad: 'T1' },
      { count: 4, direction: 'desc', triad: 'T2' },
    ],
  },
  {
    label: 'Triad-Pair Arch',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [2, 0] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
      { count: 3, direction: 'desc', triad: 'T1', shape: [2, 1, 0] },
    ],
  },
  {
    label: '2-1-2 Run',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [2, 0] },
    ],
  },
  {
    label: '3+3 Zigzag',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
    ],
  },
  {
    label: '3+3+2 Eighth Spine',
    figureIndex: 0,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'desc', triad: 'T1', shape: [2, 1] },
    ],
  },
  {
    label: 'Triplet 3+3',
    figureIndex: 2,
    blocks: [
      { count: 3, direction: 'desc', triad: 'T1', shape: [1, 0, 2] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [0, 1, 2] },
    ],
  },
  {
    label: 'Arch (3rd-anchored)',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [1, 2, 0], anchor: 'third' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [0, 1] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
      { count: 3, direction: 'desc', triad: 'T1', shape: [2, 1, 0] },
    ],
  },
];
