import { describe, it, expect } from 'vitest';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { GUITAR_MANIFEST } from './guitarManifest';
import { nearestSampledMidi } from './guitarSampler';

const here = dirname(fileURLToPath(import.meta.url));
const staticDir = resolve(here, '../../static');

describe('guitar manifest', () => {
  it('is non-empty', () => {
    expect(Object.keys(GUITAR_MANIFEST).length).toBeGreaterThan(0);
  });
  it('every entry points to a shipped file', () => {
    for (const url of Object.values(GUITAR_MANIFEST)) {
      expect(existsSync(resolve(staticDir, '.' + url)), `missing ${url}`).toBe(true);
    }
  });
});

describe('nearestSampledMidi', () => {
  const sampled = [40, 48, 52, 55];
  it('returns the closest sampled note', () => {
    expect(nearestSampledMidi(53, sampled)).toBe(52);
    expect(nearestSampledMidi(41, sampled)).toBe(40);
  });
  it('on an exact tie keeps the lower note', () => {
    expect(nearestSampledMidi(50, sampled)).toBe(48); // 48 and 52 are both 2 away
  });
});
