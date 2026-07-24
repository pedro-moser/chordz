/**
 * Geometry shared by the GMC tab and the notation staff drawn above it.
 *
 * The staff reuses `tabX` verbatim, which is what keeps a notehead vertically
 * aligned with its fret number. Keep every horizontal constant here — a copy
 * living in a component is a drift waiting to happen.
 */

export const TAB_STRING_GAP = 18;
export const TAB_MEASURE_WIDTH = 140;
/**
 * Left gutter before the first measure of every system. It holds the string labels and
 * the clef, which is about 19px wide — with the old 10px margin the clef sat on top of
 * the opening barline.
 */
export const TAB_MARGIN_LEFT = 34;
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

// ---------------------------------------------------------------------------
// Systems
//
// A line of music does not run off to the right forever — it wraps, the way a
// score does, into stacked systems. That is not cosmetic here: a sixteenth-note
// measure needs roughly 280px to engrave, so a 32-bar tune on one strip would
// run 9000px wide and show four bars at a time.
// ---------------------------------------------------------------------------

/** A contiguous run of measures drawn on one line, staff above tab. */
export interface System {
  /** Index of this system's first measure in the whole line. */
  first: number;
  /** How many measures it holds. */
  count: number;
}

/** Gap between one system's tab and the next system's staff. */
export const SYSTEM_GAP = 26;

/**
 * Break a line into systems that fit `availableWidth`.
 *
 * Always at least one measure per system, so a container narrower than a single
 * measure still renders (clipped) instead of dividing by zero or looping forever.
 */
export function splitSystems(measureCount: number, availableWidth: number): System[] {
  if (measureCount <= 0) return [];
  const usable = availableWidth - TAB_MARGIN_LEFT;
  const perSystem = Math.max(1, Math.floor(usable / TAB_MEASURE_WIDTH));
  const systems: System[] = [];
  for (let first = 0; first < measureCount; first += perSystem) {
    systems.push({ first, count: Math.min(perSystem, measureCount - first) });
  }
  return systems;
}

/** Which system a measure falls in. Returns 0 when the index is out of range. */
export function systemOf(systems: System[], measureIndex: number): number {
  for (let i = systems.length - 1; i >= 0; i--) {
    if (measureIndex >= systems[i].first) return i;
  }
  return 0;
}
