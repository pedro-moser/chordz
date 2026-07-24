import { describe, it, expect } from 'vitest';
import { CONTOURS, contourPoints } from './contour';

describe('contour glyphs', () => {
  it('offers exactly the six 3-note contours', () => {
    expect(CONTOURS).toHaveLength(6);
    const seen = CONTOURS.map((c) => c.ranks.join(''));
    expect(new Set(seen).size).toBe(6);
  });

  it('every entry is a permutation of 1..3', () => {
    for (const c of CONTOURS) {
      expect([...c.ranks].sort()).toEqual([1, 2, 3]);
    }
  });

  it('maps rank to height so 3 is the top of the box', () => {
    // A 2-level sketch collapses <1 3 2> onto <2 3 1>; three levels keeps them distinct.
    const a = contourPoints([1, 3, 2], 30, 20).map((p) => p.y);
    const b = contourPoints([2, 3, 1], 30, 20).map((p) => p.y);
    expect(a).not.toEqual(b);
    // Rank 3 is the highest note, so the smallest y in SVG coordinates.
    const pts = contourPoints([1, 2, 3], 30, 20);
    expect(pts[2].y).toBeLessThan(pts[0].y);
  });
});
