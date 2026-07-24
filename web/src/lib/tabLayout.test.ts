import { describe, it, expect } from 'vitest';
import { tabX, tabY, measureX, TAB_MEASURE_WIDTH, TAB_MARGIN_LEFT } from './tabLayout';

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
    expect(tabY(0)).toBeGreaterThan(tabY(5));
  });
});
