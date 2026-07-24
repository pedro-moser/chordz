/** The six ordinal shapes a 3-note cell can take. `ranks[i]` is the rank of the i-th note. */
export const CONTOURS: { ranks: number[]; title: string }[] = [
  { ranks: [1, 2, 3], title: 'low → mid → high (ascending)' },
  { ranks: [1, 3, 2], title: 'low → high → mid' },
  { ranks: [2, 1, 3], title: 'mid → low → high' },
  { ranks: [2, 3, 1], title: 'mid → high → low' },
  { ranks: [3, 1, 2], title: 'high → low → mid' },
  { ranks: [3, 2, 1], title: 'high → mid → low (descending)' }
];

/**
 * Polyline points for a contour glyph inside a `w` x `h` box.
 * Rank 1 sits at the bottom, rank n at the top; SVG y grows downward, so the highest rank
 * gets the smallest y.
 */
export function contourPoints(ranks: number[], w: number, h: number): { x: number; y: number }[] {
  const n = ranks.length;
  const pad = 2;
  const stepX = n > 1 ? (w - 2 * pad) / (n - 1) : 0;
  const stepY = n > 1 ? (h - 2 * pad) / (n - 1) : 0;
  return ranks.map((rank, i) => ({
    x: pad + i * stepX,
    y: h - pad - (rank - 1) * stepY
  }));
}
