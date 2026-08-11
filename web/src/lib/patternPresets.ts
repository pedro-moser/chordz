import type { GmcPatternBlock } from '$lib/wasm';

export interface PatternPreset {
  label: string;
  blocks: GmcPatternBlock[];
  /** 0=Eighth, 1=Sixteenth, 2=Triplet. Omitted → leave the current figure as-is. */
  figureIndex?: number;
  /** Optional pair to make the preset self-contained for a first exercise. */
  pairIndex?: number;
  /** Reset any per-chord scale edits back to the quality defaults. */
  resetScales?: boolean;
}

// roles 0/1/2 = root/3rd/5th of the stacked-thirds pair. The first three are generic
// presets, the next two are tonal on-ramps, and the rest are distilled from Pedro's .gp
// études (2026-05-31 pattern mining).
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
    label: 'Tonal T/T — 3+3',
    figureIndex: 0,
    pairIndex: 0,
    resetScales: true,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], connector: 'voiceLead' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2], connector: 'voiceLead' },
    ],
  },
  {
    label: 'Tonal T/T — turn',
    figureIndex: 0,
    pairIndex: 0,
    resetScales: true,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], connector: 'invertUp' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2], connector: 'invertUp' },
      { count: 3, direction: 'desc', triad: 'T1', shape: [2, 1, 0], connector: 'invertDown' },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0], connector: 'invertDown' },
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
