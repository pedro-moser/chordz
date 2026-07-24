import { describe, it, expect } from 'vitest';
import {
  tabX,
  tabY,
  measureX,
  splitSystems,
  systemOf,
  TAB_MEASURE_WIDTH,
  TAB_MARGIN_LEFT,
  TAB_MARGIN_TOP,
  TAB_STRING_GAP,
} from './tabLayout';

describe('tabLayout', () => {
  const measure = { index: 2, startBeat: 8, chord: { beats: 4 } };

  it('places measure 0 at the left margin', () => {
    expect(measureX(0)).toBe(TAB_MARGIN_LEFT);
  });

  it('spaces measures by one measure width', () => {
    expect(measureX(3) - measureX(2)).toBe(TAB_MEASURE_WIDTH);
  });

  it('places a note at the start of its measure with the leading pad only', () => {
    expect(tabX({ beat: 8 }, measure)).toBe(measureX(2) + 12);
  });

  it('places a mid-measure note proportionally to its beat', () => {
    const half = tabX({ beat: 10 }, measure);
    const start = tabX({ beat: 8 }, measure);
    expect(half - start).toBeCloseTo((TAB_MEASURE_WIDTH - 24) / 2);
  });

  it('maps engine string 0 (low E) to the bottom tab line', () => {
    // Pin the exact mapping, not just the ordering: an off-by-one that still ordered
    // correctly would shift the whole tab by a string and pass a comparison test.
    expect(tabY(5)).toBe(TAB_MARGIN_TOP);
    expect(tabY(0)).toBe(TAB_MARGIN_TOP + 5 * TAB_STRING_GAP);
  });
});

describe('splitSystems', () => {
  /** Width that fits exactly four measures plus the left gutter. */
  const fourWide = TAB_MARGIN_LEFT + 4 * TAB_MEASURE_WIDTH;

  it('returns nothing for an empty line', () => {
    expect(splitSystems(0, fourWide)).toEqual([]);
  });

  it('keeps one system when the line fits', () => {
    expect(splitSystems(3, fourWide)).toEqual([{ first: 0, count: 3 }]);
  });

  it('wraps into systems of the width that fits', () => {
    expect(splitSystems(9, fourWide)).toEqual([
      { first: 0, count: 4 },
      { first: 4, count: 4 },
      { first: 8, count: 1 },
    ]);
  });

  it('covers every measure exactly once', () => {
    const systems = splitSystems(32, fourWide);
    expect(systems.reduce((a, s) => a + s.count, 0)).toBe(32);
    systems.forEach((s, i) => {
      if (i > 0) expect(s.first).toBe(systems[i - 1].first + systems[i - 1].count);
    });
  });

  it('never emits an empty system, even in a container narrower than one measure', () => {
    const systems = splitSystems(3, 10);
    expect(systems).toEqual([
      { first: 0, count: 1 },
      { first: 1, count: 1 },
      { first: 2, count: 1 },
    ]);
  });
});

describe('systemOf', () => {
  const systems = [
    { first: 0, count: 4 },
    { first: 4, count: 4 },
    { first: 8, count: 1 },
  ];

  it('finds the system holding a measure', () => {
    expect(systemOf(systems, 0)).toBe(0);
    expect(systemOf(systems, 3)).toBe(0);
    expect(systemOf(systems, 4)).toBe(1);
    expect(systemOf(systems, 8)).toBe(2);
  });
});
