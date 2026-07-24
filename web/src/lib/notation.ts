/**
 * Pure layout for the GMC notation staff. No DOM, no Svelte — everything here is a
 * function of the engine's events, so it can be unit tested on its own.
 *
 * The staff is a treble clef with an 8 below: the guitar sounds an octave lower than
 * it is written. Staff steps count upward from the bottom line (written E4 = 0), so
 * even steps are lines and odd steps are spaces, and the staff proper spans 0..8.
 */

import { tabX, type MeasureLike } from './tabLayout';
import type { GmcLineEvent } from './wasm';

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

export type GridKind = 'eighth' | 'sixteenth' | 'triplet';

export interface Grid {
  kind: GridKind;
  /** Beats in one grid slot. */
  step: number;
}

export const GRIDS: Record<GridKind, Grid> = {
  eighth: { kind: 'eighth', step: 0.5 },
  sixteenth: { kind: 'sixteenth', step: 0.25 },
  triplet: { kind: 'triplet', step: 1 / 3 },
};

/** One printable note value. `value` is the denominator: 1=whole, 2=half, 4=quarter, 8, 16. */
export interface Figure {
  /** Absolute onset in beats. */
  beat: number;
  /** Sounding length in beats. */
  beats: number;
  value: number;
  dots: 0 | 1;
  tiedToNext: boolean;
}

/**
 * Slot counts that print as a single figure, largest first. Anything not listed is
 * split greedily into the largest entry that fits, and the pieces are tied.
 */
const FIGURE_TABLE: Record<GridKind, Array<[slots: number, value: number, dots: 0 | 1]>> = {
  eighth: [
    [8, 1, 0],
    [6, 2, 1],
    [4, 2, 0],
    [3, 4, 1],
    [2, 4, 0],
    [1, 8, 0],
  ],
  sixteenth: [
    [16, 1, 0],
    [12, 2, 1],
    [8, 2, 0],
    [6, 4, 1],
    [4, 4, 0],
    [3, 8, 1],
    [2, 8, 0],
    [1, 16, 0],
  ],
  // Inside a triplet bracket 1 slot is an eighth and 2 slots a quarter; 3 slots fills a
  // whole beat and prints as a plain quarter, with the bracket omitted by the beamer.
  triplet: [
    [12, 1, 0],
    [9, 2, 1],
    [6, 2, 0],
    [3, 4, 0],
    [2, 4, 0],
    [1, 8, 0],
  ],
};

/** Round to whole slots — `beat`/`duration` are f32 out of Rust and drift slightly. */
function toSlots(beats: number, grid: Grid): number {
  return Math.max(0, Math.round(beats / grid.step));
}

/**
 * Split a span into printable figures, breaking only at barlines: each figure is the
 * largest printable value (from `FIGURE_TABLE`) that fits between its start and the next
 * barline, so a figure may straddle a beat boundary but never a barline. All figures but
 * the last are tied to their successor.
 *
 * Caller contract: `measureStarts` must contain every bar start in the line. Past the
 * last entry this function has no more barlines to stop at, so a truncated list lets a
 * figure run through a barline it should have been split at.
 */
export function splitSpan(
  startBeat: number,
  durationBeats: number,
  grid: Grid,
  measureStarts: number[],
): Figure[] {
  const table = FIGURE_TABLE[grid.kind]; // sorted largest slot count first
  const pieces: Array<{ beat: number; slots: number }> = [];
  let beat = startBeat;
  let remaining = toSlots(durationBeats, grid);
  // A span needs at most one figure per slot. The guard makes a malformed duration
  // impossible to hang the render on.
  let guard = remaining + 1;

  while (remaining > 0 && guard-- > 0) {
    // A figure may straddle beats — a half note starting on beat 1 is one figure —
    // but never a barline. That is what ties are for.
    const nextBar = measureStarts.find((m) => m > beat + 1e-6);
    const toBar = nextBar === undefined ? remaining : toSlots(nextBar - beat, grid);
    const room = Math.max(1, Math.min(remaining, toBar));
    const entry = table.find(([slots]) => slots <= room) ?? table[table.length - 1];
    pieces.push({ beat, slots: entry[0] });
    beat += entry[0] * grid.step;
    remaining -= entry[0];
  }

  return pieces.map((p, i) => {
    const entry = table.find(([slots]) => slots === p.slots) ?? table[table.length - 1];
    return {
      beat: p.beat,
      beats: p.slots * grid.step,
      value: entry[1],
      dots: entry[2],
      tiedToNext: i < pieces.length - 1,
    };
  });
}

/**
 * The rests a measure needs: every stretch of the measure no event covers.
 *
 * Events are assumed sorted by beat and non-overlapping, which is what the line
 * engine emits — a held note advances the cursor past the slots it occupies.
 */
export function restFigures(
  events: Array<{ beat: number; duration: number }>,
  measureStart: number,
  measureBeats: number,
  grid: Grid,
  measureStarts: number[],
): Figure[] {
  const end = measureStart + measureBeats;
  const rests: Figure[] = [];
  let cursor = measureStart;

  for (const e of events) {
    if (e.beat - cursor > 1e-6) {
      rests.push(...splitSpan(cursor, e.beat - cursor, grid, measureStarts));
    }
    cursor = Math.max(cursor, e.beat + e.duration);
  }
  if (end - cursor > 1e-6) {
    rests.push(...splitSpan(cursor, end - cursor, grid, measureStarts));
  }
  // Rests are never tied.
  return rests.map((r) => ({ ...r, tiedToNext: false }));
}

/** The middle staff line (B4 written) — the pivot for stem direction. */
const MIDDLE_STEP = 4;

export interface BeamGroup<T> {
  notes: T[];
  stemUp: boolean;
  /** True for a triplet group, which prints a bracket and a 3. */
  bracket: boolean;
}

/**
 * Group notes into beams, one group per beat.
 *
 * A note longer than one grid slot cannot be beamed — a `hold_last` landing note
 * carries a flagless value and ends its group. Stem direction is decided per group by
 * the note furthest from the middle line, so the whole beam points the same way.
 */
export function beamGroups<
  T extends { beat: number; beats: number; staffStep: number; value: number },
>(
  notes: T[],
  grid: Grid,
): BeamGroup<T>[] {
  const groups: BeamGroup<T>[] = [];
  let current: T[] = [];
  let currentBeat = -1;

  const flush = () => {
    if (current.length === 0) return;
    const furthest = current.reduce((best, n) =>
      Math.abs(n.staffStep - MIDDLE_STEP) > Math.abs(best.staffStep - MIDDLE_STEP) ? n : best,
    );
    groups.push({
      notes: current,
      stemUp: furthest.staffStep < MIDDLE_STEP,
      bracket: grid.kind === 'triplet' && current.length > 1,
    });
    current = [];
  };

  for (const n of notes) {
    const beatIndex = Math.floor(n.beat + 1e-6);
    // Only figures that carry flags can be beamed: eighths and shorter, i.e. value >= 8.
    // Judging by the printed value rather than by sounding length is what lets an eighth
    // beam with the sixteenths beside it on a sixteenth grid, while still keeping a
    // triplet quarter (two slots long, but value 4) out of the beam.
    const beamable = n.value >= 8;
    if (beatIndex !== currentBeat || !beamable) {
      flush();
      currentBeat = beatIndex;
    }
    current.push(n);
    if (!beamable) flush();
  }
  flush();
  return groups;
}

/**
 * Which accidental each note prints, for one measure's worth of notes in order.
 *
 * There is no key signature: the active scale changes per chord, so an armature would
 * misrepresent the line. Everything is inline, with the usual rules — state an
 * alteration once per (letter, octave) per measure, and print a natural when a
 * previously altered letter comes back.
 *
 * Call this once per measure; the state must not leak across barlines.
 */
export function accidentalsToPrint(notes: Spelled[]): Array<number | null> {
  const stated = new Map<string, number>();
  return notes.map((n) => {
    const key = `${n.step}:${n.octave}`;
    const previous = stated.get(key);
    if (previous === n.alter) return null;
    if (previous === undefined && n.alter === 0) return null;
    stated.set(key, n.alter);
    return n.alter;
  });
}

export interface LaidOutNote {
  x: number;
  staffStep: number;
  /** The alteration to print, or null when it is already in force. */
  accidental: number | null;
  value: number;
  dots: 0 | 1;
  tiedToNext: boolean;
  triad: 'T1' | 'T2';
  /** Ledger lines this note needs, in staff steps. */
  ledger: number[];
  /** Sounding length in beats — the beamer needs it. */
  beats: number;
  /** Absolute onset in beats — the beamer needs it. */
  beat: number;
}

export interface LaidOutRest {
  x: number;
  value: number;
  dots: 0 | 1;
}

export interface MeasureLayout {
  notes: LaidOutNote[];
  rests: LaidOutRest[];
  beams: BeamGroup<LaidOutNote>[];
}

/**
 * Lay out every measure of a line at once.
 *
 * This is deliberately NOT per-measure. A note extended by the engine's `holdLast` can
 * sustain past a barline — measured on a ii-V-I, five of twelve pattern configurations
 * produce at least one such note — and standard notation draws it as two tied figures,
 * one in each measure. A per-measure function cannot do that: the second figure belongs
 * to a measure that never saw the event, and that measure would draw a rest over sound.
 * So we walk the whole line, split every span at barlines, and file each resulting figure
 * under the measure that actually contains it.
 *
 * Callers must pass the WHOLE line, not a slice of it: `measureOf` resolves any beat past
 * the last measure supplied into that last measure, so a partial slice collapses a
 * continuation into the wrong measure and resurrects the cross-barline bug this function
 * exists to prevent.
 *
 * Horizontal positions come from `tabX` — the same function the tablature uses — which is
 * what keeps a notehead directly above its fret number.
 */
export function layoutLine(
  measures: Array<MeasureLike & { events: GmcLineEvent[] }>,
  grid: Grid,
): MeasureLayout[] {
  const measureStarts = measures.map((m) => m.startBeat);
  const out: MeasureLayout[] = measures.map(() => ({ notes: [], rests: [], beams: [] }));
  if (measures.length === 0) return out;

  /** The measure a beat falls in. Beats past the last barline belong to the last measure. */
  const measureOf = (beat: number): number => {
    for (let i = measures.length - 1; i >= 0; i--) {
      if (beat >= measures[i].startBeat - 1e-6) return i;
    }
    return 0;
  };

  // Notes. Accidentals are decided per measure over that measure's OWN events, so a note
  // tied in from the previous bar does not consume the accidental a later note needs.
  measures.forEach((m) => {
    const events = [...m.events].sort((a, b) => a.beat - b.beat);
    const accidentals = accidentalsToPrint(events);
    events.forEach((e, i) => {
      const step = staffStep(e);
      const ledger = ledgerSteps(step);
      splitSpan(e.beat, e.duration, grid, measureStarts).forEach((f, fi) => {
        const target = measureOf(f.beat);
        out[target].notes.push({
          x: tabX({ beat: f.beat }, measures[target]),
          staffStep: step,
          // Only the head of a tied chain states the accidental; the continuation after a
          // barline does not restate it.
          accidental: fi === 0 ? accidentals[i] : null,
          value: f.value,
          dots: f.dots,
          tiedToNext: f.tiedToNext,
          triad: e.triad,
          ledger,
          beats: f.beats,
          beat: f.beat,
        });
      });
    });
  });

  // Rests. A measure's silence is whatever no SOUNDING note covers, so this must consider
  // notes that started earlier and are still ringing, not just the measure's own events.
  const allEvents = measures.flatMap((m) => m.events);
  measures.forEach((m, mi) => {
    const end = m.startBeat + m.chord.beats;
    const sounding = allEvents
      .filter((e) => e.beat < end - 1e-6 && e.beat + e.duration > m.startBeat + 1e-6)
      .sort((a, b) => a.beat - b.beat);
    out[mi].rests = restFigures(sounding, m.startBeat, m.chord.beats, grid, measureStarts).map(
      (r) => ({ x: tabX({ beat: r.beat }, m), value: r.value, dots: r.dots }),
    );
  });

  // Beams, per measure, over the figures that measure actually prints.
  out.forEach((layout) => {
    layout.notes.sort((a, b) => a.beat - b.beat);
    layout.beams = beamGroups(layout.notes, grid);
  });

  return out;
}
