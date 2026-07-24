import { describe, it, expect } from 'vitest';
import { staffStep, ledgerSteps, splitSpan, GRIDS, restFigures } from './notation';

describe('staffStep', () => {
  it('puts written E4 on the bottom line', () => {
    // Sounding E3 (octave 3, step 2) writes as E4 under the 8vb clef.
    expect(staffStep({ step: 2, alter: 0, octave: 3 })).toBe(0);
  });

  it('puts written F5 on the top line', () => {
    // Sounding F4 writes as F5.
    expect(staffStep({ step: 3, alter: 0, octave: 4 })).toBe(8);
  });

  it('puts the guitar open low E three ledger lines below', () => {
    // Sounding E2 = MIDI 40.
    expect(staffStep({ step: 2, alter: 0, octave: 2 })).toBe(-7);
  });

  it('ignores the accidental — Eb and E share a staff position', () => {
    expect(staffStep({ step: 2, alter: -1, octave: 3 })).toBe(
      staffStep({ step: 2, alter: 0, octave: 3 }),
    );
  });
});

describe('ledgerSteps', () => {
  it('returns nothing for a note inside the staff', () => {
    expect(ledgerSteps(4)).toEqual([]);
    expect(ledgerSteps(0)).toEqual([]);
    expect(ledgerSteps(8)).toEqual([]);
  });

  it('returns the even steps down to a low note', () => {
    expect(ledgerSteps(-7)).toEqual([-2, -4, -6]);
  });

  it('returns the even steps up to a high note', () => {
    expect(ledgerSteps(13)).toEqual([10, 12]);
  });

  it('includes the note own line when it sits exactly on a ledger', () => {
    expect(ledgerSteps(-4)).toEqual([-2, -4]);
  });
});

describe('splitSpan', () => {
  const eighth = GRIDS.eighth;
  // A 4-bar chart of 4/4: measures start at 0, 4, 8, 12.
  const bars = [0, 4, 8, 12];

  it('emits one figure for a plain eighth', () => {
    expect(splitSpan(0, 0.5, eighth, bars)).toEqual([
      { beat: 0, beats: 0.5, value: 8, dots: 0, tiedToNext: false },
    ]);
  });

  it('emits one figure for a dotted quarter', () => {
    expect(splitSpan(0, 1.5, eighth, bars)).toEqual([
      { beat: 0, beats: 1.5, value: 4, dots: 1, tiedToNext: false },
    ]);
  });

  it('ties a half to an eighth for two and a half beats', () => {
    const figures = splitSpan(0, 2.5, eighth, bars);
    expect(figures.map((f) => [f.value, f.dots])).toEqual([
      [2, 0],
      [8, 0],
    ]);
    expect(figures[0].tiedToNext).toBe(true);
    expect(figures[1].tiedToNext).toBe(false);
  });

  it('splits at the barline and ties across it', () => {
    // Starts on beat 3 of bar 1, lasts two beats -> one beat in each bar.
    const figures = splitSpan(3, 2, eighth, bars);
    expect(figures).toHaveLength(2);
    expect(figures[0].beat).toBe(3);
    expect(figures[1].beat).toBe(4);
    expect(figures[0].tiedToNext).toBe(true);
    expect(figures[1].tiedToNext).toBe(false);
  });

  it('never returns an empty list for a positive duration', () => {
    for (const beats of [0.25, 0.5, 1, 1.25, 2, 3, 3.5, 4]) {
      expect(splitSpan(0, beats, GRIDS.sixteenth, bars).length).toBeGreaterThan(0);
    }
  });

  it('conserves total duration', () => {
    const total = (fs: { beats: number }[]) => fs.reduce((a, f) => a + f.beats, 0);
    expect(total(splitSpan(0, 2.5, eighth, bars))).toBeCloseTo(2.5);
    expect(total(splitSpan(3, 2, eighth, bars))).toBeCloseTo(2);
    expect(total(splitSpan(0, 1 / 3, GRIDS.triplet, bars))).toBeCloseTo(1 / 3);
  });
});

describe('restFigures', () => {
  const grid = GRIDS.eighth;
  const bars = [0, 4];

  it('returns nothing when the measure is full', () => {
    const events = [
      { beat: 0, duration: 2 },
      { beat: 2, duration: 2 },
    ];
    expect(restFigures(events, 0, 4, grid, bars)).toEqual([]);
  });

  it('fills a leading gap from a pickup rest', () => {
    // lead_rest of 2 eighth slots: the measure starts with one beat of silence.
    const events = [{ beat: 1, duration: 3 }];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests).toHaveLength(1);
    expect(rests[0].beat).toBe(0);
    expect(rests[0].beats).toBeCloseTo(1);
  });

  it('fills a trailing gap', () => {
    const events = [{ beat: 0, duration: 2 }];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(2);
  });

  it('fills a gap in the middle', () => {
    const events = [
      { beat: 0, duration: 1 },
      { beat: 3, duration: 1 },
    ];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(2);
    expect(rests[0].beat).toBeCloseTo(1);
  });

  it('fills an entirely empty measure', () => {
    const rests = restFigures([], 4, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(4);
    expect(rests[0].beat).toBe(4);
  });
});
