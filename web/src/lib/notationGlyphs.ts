/**
 * SVG path data for the notation glyphs that geometry cannot produce.
 *
 * Coordinates are in staff steps: 1 unit = half the distance between two staff lines,
 * y grows downward, and the origin sits on each glyph's staff anchor — the G line for
 * the clef, the middle line for rests, the notehead's own line for accidentals. The
 * component scales by `STAFF_LINE_GAP / 2`, so nothing here needs pixel values.
 *
 * Noteheads, stems, beams, augmentation dots, ledger lines and ties are drawn from
 * primitives by `StaffNotation.svelte`, not from this file.
 */

/** Treble clef, anchored so y=0 is the G line (staff step 2). */
export const TREBLE_CLEF_PATH =
  'M0.9,6.2 C0.9,7.3 1.8,8.0 2.8,8.0 C4.0,8.0 4.8,7.1 4.8,5.9 ' +
  'C4.8,4.6 3.9,3.6 2.6,2.4 C1.5,1.4 0.6,0.5 0.6,-0.9 ' +
  'C0.6,-2.4 1.6,-3.6 2.6,-4.6 C3.4,-5.4 3.9,-6.2 3.9,-7.2 ' +
  'C3.9,-8.4 3.3,-9.2 2.7,-9.2 C2.0,-9.2 1.6,-8.3 1.6,-7.2 ' +
  'C1.6,-5.9 2.2,-4.6 2.9,-3.2 C3.9,-1.2 4.9,0.9 4.9,2.9 ' +
  'C4.9,4.9 3.7,6.3 2.2,6.3 C1.3,6.3 0.9,5.8 0.9,5.2';

/** The 8 under the clef: guitar sounds an octave below the written pitch. */
export const CLEF_OCTAVE_TEXT = '8';

/** Rests, anchored on the middle staff line, keyed by note value. */
export const REST_PATHS: Record<number, string> = {
  // Whole rest: a block hanging UNDER the fourth line, i.e. filling staff steps 5-6.
  // The anchor is the middle line (step 4) and y grows downward, so that is y -2 to -1.
  1: 'M-1.0,-2.0 h2.0 v1.0 h-2.0 z',
  // Half rest: a block sitting ON the middle line, filling staff steps 4-5, i.e. y -1 to 0.
  2: 'M-1.0,-1.0 h2.0 v1.0 h-2.0 z',
  // Quarter rest.
  4: 'M-0.4,-2.0 L0.6,-0.7 L-0.2,0.2 L0.8,1.6 L0.2,1.9 C-0.6,1.0 -1.0,0.4 -0.5,-0.2 L-1.0,-0.9 z',
  // Eighth rest: one hook.
  8: 'M0.6,-1.6 C0.6,-1.2 0.2,-0.9 -0.2,-1.0 L0.2,1.8 L-0.2,1.9 L-0.8,-1.2 C-0.4,-0.9 0.1,-1.1 0.2,-1.5 z',
  // Sixteenth rest: two hooks.
  16: 'M0.6,-2.4 C0.6,-2.0 0.2,-1.7 -0.2,-1.8 L0.2,1.8 L-0.2,1.9 L-1.0,-2.0 C-0.6,-1.7 -0.1,-1.9 0.0,-2.3 z ' +
    'M0.3,-0.6 C0.3,-0.2 -0.1,0.1 -0.5,0.0 L-0.7,-0.8 C-0.3,-0.5 0.0,-0.6 0.1,-0.9 z',
};

/** Accidentals, anchored on the notehead's own staff position, keyed by alteration. */
export const ACCIDENTAL_PATHS: Record<number, string> = {
  // Double flat.
  [-2]:
    'M-1.4,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z ' +
    'M0.2,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z',
  // Flat.
  [-1]: 'M-0.5,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z',
  // Natural.
  0: 'M-0.5,-2.2 v3.6 l1.2,-0.3 v-3.6 z M-0.5,0.6 l1.2,-0.3 v1.0 l-1.2,0.3 z ' +
    'M-0.5,-1.4 l1.2,-0.3 v1.0 l-1.2,0.3 z',
  // Sharp.
  1: 'M-0.7,-1.6 l0.35,-0.1 v3.4 l-0.35,0.1 z M0.35,-1.9 l0.35,-0.1 v3.4 l-0.35,0.1 z ' +
    'M-0.9,-0.5 l1.8,-0.45 v0.6 l-1.8,0.45 z M-0.9,1.0 l1.8,-0.45 v0.6 l-1.8,0.45 z',
  // Double sharp.
  2: 'M-0.6,-0.6 h1.2 v1.2 h-1.2 z',
};
