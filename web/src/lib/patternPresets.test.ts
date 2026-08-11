import { describe, it, expect } from 'vitest';
import { PATTERN_PRESETS } from './patternPresets';

describe('PATTERN_PRESETS', () => {
  it('has the generic, tonal, and distilled presets', () => {
    const labels = PATTERN_PRESETS.map((p) => p.label);
    expect(labels).toContain('Alternating 3+3');
    expect(labels).toContain('Continuous up');
    expect(labels).toContain('Short-long');
    expect(labels).toContain('Tonal T/T — 3+3');
    expect(labels).toContain('Tonal T/T — turn');
    expect(labels).toContain('Triad-Pair Arch');
    expect(labels).toContain('2-1-2 Run');
    expect(labels).toContain('3+3 Zigzag');
    expect(labels).toContain('3+3+2 Eighth Spine');
    expect(labels).toContain('Triplet 3+3');
    expect(labels).toContain('Arch (3rd-anchored)');
    expect(PATTERN_PRESETS.length).toBe(11);
  });

  it('every preset is well-formed', () => {
    for (const preset of PATTERN_PRESETS) {
      expect(preset.label.length).toBeGreaterThan(0);
      expect(preset.blocks.length).toBeGreaterThan(0);
      if (preset.figureIndex !== undefined) {
        expect([0, 1, 2]).toContain(preset.figureIndex);
      }
      if (preset.pairIndex !== undefined) {
        expect(preset.pairIndex).toBeGreaterThanOrEqual(0);
        expect(preset.pairIndex).toBeLessThan(10);
      }
      for (const b of preset.blocks) {
        // count is the engine's per-block length, clamped to 1..6 in the wasm parser.
        expect(b.count).toBeGreaterThanOrEqual(1);
        expect(b.count).toBeLessThanOrEqual(6);
        expect(['asc', 'desc']).toContain(b.direction);
        expect(['T1', 'T2']).toContain(b.triad);
        if (b.shape !== undefined) {
          expect(b.shape.length).toBeGreaterThan(0);
          for (const role of b.shape) expect([0, 1, 2]).toContain(role);
        }
        if (b.anchor !== undefined) {
          expect(['root', 'third', 'fifth']).toContain(b.anchor);
        }
      }
    }
  });
});
