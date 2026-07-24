/**
 * The notation glyphs that come from the font rather than from geometry.
 *
 * Clefs, rests and accidentals are real typographic shapes — two rounds of hand-authored
 * bezier paths produced a treble clef that was recognisable but wrong, and then one that
 * was the right SIZE but had lost its spiral entirely. They are drawn instead from a
 * 3.7 KB subset of Bravura (SIL OFL 1.1, see `../fonts/Bravura-OFL.txt`), the reference
 * SMuFL font.
 *
 * Noteheads, stems, beams, ledger lines, augmentation dots and ties stay geometric:
 * they are ellipses and straight lines, `StaffNotation.svelte` already sizes them from
 * the same SMuFL metrics, and keeping them as SVG primitives keeps the per-note colouring
 * straightforward.
 *
 * **The sizing rule.** SMuFL fonts are drawn so that one em equals four staff spaces.
 * Set `font-size` to `4 × STAFF_LINE_GAP` and every glyph lands at its engraved size with
 * no per-glyph scale factor. Each glyph's origin is its baseline, placed at the staff
 * position it attaches to — the G line for the clef, the notehead's own line for an
 * accidental, and the line named in `REST_ANCHOR_STEP` for a rest.
 */

/** Matches the `@font-face` family declared in `app.css`. */
export const SMUFL_FONT = 'Bravura';

/** U+E052 gClef8vb — treble clef with the 8 already below it. Origin sits on the G line. */
export const CLEF_8VB = '\uE052';

/** Rests by note value: whole, half, quarter, eighth, sixteenth. */
export const REST_GLYPH: Record<number, string> = {
  1: '\uE4E3',
  2: '\uE4E4',
  4: '\uE4E5',
  8: '\uE4E6',
  16: '\uE4E7',
};

/**
 * Which staff step each rest's baseline sits on.
 *
 * A whole rest hangs UNDER the fourth line and a half rest sits ON the middle line — they
 * are a space apart and not interchangeable. Bravura encodes that in the glyphs themselves
 * (restWhole is drawn below its origin, restHalf above it), so the whole rest anchors to
 * the fourth line and everything else to the middle line.
 */
export const REST_ANCHOR_STEP: Record<number, number> = {
  1: 6,
  2: 4,
  4: 4,
  8: 4,
  16: 4,
};

/** Accidentals by alteration: 𝄫 ♭ ♮ ♯ 𝄪. */
export const ACCIDENTAL_GLYPH: Record<number, string> = {
  [-2]: '\uE264',
  [-1]: '\uE260',
  0: '\uE261',
  1: '\uE262',
  2: '\uE263',
};

/**
 * Advance width of each accidental in staff spaces, from Bravura's `glyphBBoxes`. The
 * renderer needs these to hang an accidental to the LEFT of its notehead: one shared
 * offset cannot work when a natural is two thirds the width of a sharp and a double flat
 * is nearly twice it.
 */
export const ACCIDENTAL_WIDTH_SP: Record<number, number> = {
  [-2]: 1.644,
  [-1]: 0.904,
  0: 0.672,
  1: 0.996,
  2: 0.988,
};
