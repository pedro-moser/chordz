/**
 * Geometry shared by the GMC tab and the notation staff drawn above it.
 *
 * The staff reuses `tabX` verbatim, which is what keeps a notehead vertically
 * aligned with its fret number. Keep every horizontal constant here — a copy
 * living in a component is a drift waiting to happen.
 */

export const TAB_STRING_GAP = 18;
export const TAB_MEASURE_WIDTH = 140;
export const TAB_MARGIN_LEFT = 10;
export const TAB_MARGIN_TOP = 28;
export const TAB_CHORD_Y = 12;
export const TAB_SCALE_Y_OFFSET = 16;
export const STRING_LABELS = ['e', 'B', 'G', 'D', 'A', 'E'];

/** Horizontal pad inside a measure, so notes never touch the barline. */
const MEASURE_PAD = 12;

export interface MeasureLike {
  index: number;
  startBeat: number;
  chord: { beats: number };
}

/** Left edge of a measure. */
export function measureX(index: number): number {
  return TAB_MARGIN_LEFT + index * TAB_MEASURE_WIDTH;
}

/** Horizontal position of an event within its measure. */
export function tabX(event: { beat: number }, measure: MeasureLike): number {
  const fraction = (event.beat - measure.startBeat) / measure.chord.beats;
  return measureX(measure.index) + MEASURE_PAD + fraction * (TAB_MEASURE_WIDTH - 2 * MEASURE_PAD);
}

/** Vertical position of a tab line. Engine string 0 is the low E; the tab draws it at the bottom. */
export function tabY(engineString: number): number {
  return TAB_MARGIN_TOP + (5 - engineString) * TAB_STRING_GAP;
}
