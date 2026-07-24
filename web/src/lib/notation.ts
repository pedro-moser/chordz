/**
 * Pure layout for the GMC notation staff. No DOM, no Svelte — everything here is a
 * function of the engine's events, so it can be unit tested on its own.
 *
 * The staff is a treble clef with an 8 below: the guitar sounds an octave lower than
 * it is written. Staff steps count upward from the bottom line (written E4 = 0), so
 * even steps are lines and odd steps are spaces, and the staff proper spans 0..8.
 */

/** A pitch spelled for notation. Mirrors the Rust `Spelled` on `GmcLineEvent`. */
export interface Spelled {
  /** 0=C, 1=D, 2=E, 3=F, 4=G, 5=A, 6=B. */
  step: number;
  /** -2=𝄫, -1=♭, 0=♮, +1=♯, +2=𝄪. */
  alter: number;
  /** Sounding octave; middle C is C4. */
  octave: number;
}

/** Pixels between adjacent staff lines. One staff step is half of this. */
export const STAFF_LINE_GAP = 7;

/** The staff proper: step 0 is the bottom line, step 8 the top line. */
export const STAFF_TOP_STEP = 8;
export const STAFF_BOTTOM_STEP = 0;

/** Diatonic index of written E4, the bottom line: octave 4 × 7 + step 2. */
const BOTTOM_LINE_DIATONIC = 30;

/**
 * Staff steps above the bottom line. Positive is up. The accidental is irrelevant —
 * Eb and E occupy the same line.
 */
export function staffStep(p: Spelled): number {
  const writtenOctave = p.octave + 1; // treble 8vb: written = sounding + one octave
  return writtenOctave * 7 + p.step - BOTTOM_LINE_DIATONIC;
}

/** The ledger lines a note at `step` needs, ordered outward from the staff. */
export function ledgerSteps(step: number): number[] {
  const lines: number[] = [];
  if (step < STAFF_BOTTOM_STEP) {
    for (let s = -2; s >= step; s -= 2) lines.push(s);
  } else if (step > STAFF_TOP_STEP) {
    for (let s = 10; s <= step; s += 2) lines.push(s);
  }
  return lines;
}
