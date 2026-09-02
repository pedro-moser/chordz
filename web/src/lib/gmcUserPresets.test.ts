import { describe, expect, it } from 'vitest';
import type { GmcPatternBlock } from './wasm';
import {
  GMC_USER_PRESETS_STORAGE_KEY,
  deleteGmcUserPreset,
  loadGmcUserPresets,
  resolvePatternPresetPairIndex,
  resolveScalePresetOverrides,
  saveGmcUserPreset,
} from './gmcUserPresets';

class MemoryStorage {
  private values = new Map<string, string>();

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
}

const blocks: GmcPatternBlock[] = [
  {
    count: 3,
    direction: 'asc',
    triad: 'T1',
    shape: [0, 2, 1],
    connector: 'invertUp',
    contour: [1, 3, 2],
  },
  { count: 2, direction: 'desc', triad: 'T2', holdLast: 2, leadRest: 1 },
];

describe('GMC user presets', () => {
  it('starts empty and recovers from corrupt or unavailable storage', () => {
    const storage = new MemoryStorage();
    expect(loadGmcUserPresets(storage)).toEqual({
      version: 1,
      harmonies: [],
      scales: [],
      patterns: [],
    });

    storage.setItem(GMC_USER_PRESETS_STORAGE_KEY, '{not json');
    expect(loadGmcUserPresets(storage)).toEqual({
      version: 1,
      harmonies: [],
      scales: [],
      patterns: [],
    });

    const unavailableStorage = {
      getItem(): string | null {
        throw new DOMException('Storage is disabled', 'SecurityError');
      },
    };
    expect(loadGmcUserPresets(unavailableStorage)).toEqual({
      version: 1,
      harmonies: [],
      scales: [],
      patterns: [],
    });
  });

  it('persists harmony, scale-selection, and self-contained pattern presets', () => {
    const storage = new MemoryStorage();

    saveGmcUserPreset(
      storage,
      'harmony',
      { name: 'Minor turnaround', title: 'My tune', chart: 'Dm7 | G7 | Cmaj7 | A7' },
      { id: 'harmony-1', savedAt: '2026-09-02T10:00:00.000Z' },
    );
    saveGmcUserPreset(
      storage,
      'scales',
      {
        name: 'Altered route',
        title: 'My tune',
        chart: 'Dm7 | G7 | Cmaj7 | A7',
        chordSymbols: ['Dm7', 'G7', 'Cmaj7', 'A7'],
        scaleRefs: [null, { parent: 'Melodic Minor', degree: 7 }, null, { parent: 'Melodic Minor', degree: 7 }],
      },
      { id: 'scales-1', savedAt: '2026-09-02T10:01:00.000Z' },
    );
    saveGmcUserPreset(
      storage,
      'pattern',
      { name: 'Wide arch', pairLabel: 'T/7no5', figureIndex: 1, blocks },
      { id: 'pattern-1', savedAt: '2026-09-02T10:02:00.000Z' },
    );

    expect(loadGmcUserPresets(storage)).toEqual({
      version: 1,
      harmonies: [
        {
          id: 'harmony-1',
          name: 'Minor turnaround',
          savedAt: '2026-09-02T10:00:00.000Z',
          title: 'My tune',
          chart: 'Dm7 | G7 | Cmaj7 | A7',
        },
      ],
      scales: [
        {
          id: 'scales-1',
          name: 'Altered route',
          savedAt: '2026-09-02T10:01:00.000Z',
          title: 'My tune',
          chart: 'Dm7 | G7 | Cmaj7 | A7',
          chordSymbols: ['Dm7', 'G7', 'Cmaj7', 'A7'],
          scaleRefs: [null, { parent: 'Melodic Minor', degree: 7 }, null, { parent: 'Melodic Minor', degree: 7 }],
        },
      ],
      patterns: [
        {
          id: 'pattern-1',
          name: 'Wide arch',
          savedAt: '2026-09-02T10:02:00.000Z',
          pairLabel: 'T/7no5',
          figureIndex: 1,
          blocks,
        },
      ],
    });
  });

  it('resolves scale references after catalog reordering and rejects a different harmony', () => {
    const preset = {
      id: 'scales-1',
      name: 'Stable scales',
      savedAt: '2026-09-02T10:00:00.000Z',
      title: 'Tune',
      chart: 'Dm7 | G7 | Cmaj7',
      chordSymbols: ['Dm7', 'G7', 'Cmaj7'],
      scaleRefs: [
        { parent: 'Major', degree: 2 },
        { parent: 'Melodic Minor', degree: 7 },
        null,
      ],
    };
    const reorderedCatalog = [
      { parent: 'Melodic Minor', degree: 7 },
      { parent: 'Major', degree: 1 },
      { parent: 'Major', degree: 2 },
    ];

    expect(resolveScalePresetOverrides(preset, reorderedCatalog, ['Dm7', 'G7', 'Cmaj7'])).toEqual({
      ok: true,
      overrides: [2, 0, null],
    });
    expect(resolveScalePresetOverrides(preset, reorderedCatalog, ['Dm7', 'Db7', 'Cmaj7'])).toEqual({
      ok: false,
      reason: 'harmony-mismatch',
    });
    expect(resolveScalePresetOverrides(preset, reorderedCatalog.slice(1), ['Dm7', 'G7', 'Cmaj7'])).toEqual({
      ok: false,
      reason: 'missing-scale',
    });
  });

  it('resolves a saved pair by label after pair-catalog reordering', () => {
    const preset = {
      id: 'pattern-1',
      name: 'Stable pair',
      savedAt: '2026-09-02T10:00:00.000Z',
      pairLabel: 'T/7no5',
      figureIndex: 0,
      blocks,
    };

    expect(resolvePatternPresetPairIndex(preset, [{ label: '7no5/T' }, { label: 'T/7no5' }])).toBe(1);
    expect(resolvePatternPresetPairIndex(preset, [{ label: 'T/T' }])).toBeNull();
  });

  it('overwrites a same-named preset within its category without duplicating it', () => {
    const storage = new MemoryStorage();
    saveGmcUserPreset(
      storage,
      'harmony',
      { name: 'Rhythm changes', title: 'A', chart: 'Cmaj7' },
      { id: 'first-id', savedAt: '2026-09-02T10:00:00.000Z' },
    );
    saveGmcUserPreset(
      storage,
      'harmony',
      { name: '  rhythm CHANGES ', title: 'B', chart: 'Fmaj7' },
      { id: 'unused-id', savedAt: '2026-09-02T11:00:00.000Z' },
    );

    expect(loadGmcUserPresets(storage).harmonies).toEqual([
      {
        id: 'first-id',
        name: 'rhythm CHANGES',
        savedAt: '2026-09-02T11:00:00.000Z',
        title: 'B',
        chart: 'Fmaj7',
      },
    ]);
  });

  it('deletes only the selected preset and ignores unknown ids', () => {
    const storage = new MemoryStorage();
    saveGmcUserPreset(
      storage,
      'harmony',
      { name: 'One', title: 'One', chart: 'Cmaj7' },
      { id: 'h1', savedAt: '2026-09-02T10:00:00.000Z' },
    );
    saveGmcUserPreset(
      storage,
      'harmony',
      { name: 'Two', title: 'Two', chart: 'Dm7' },
      { id: 'h2', savedAt: '2026-09-02T10:01:00.000Z' },
    );

    deleteGmcUserPreset(storage, 'harmony', 'missing');
    deleteGmcUserPreset(storage, 'harmony', 'h1');

    expect(loadGmcUserPresets(storage).harmonies.map((preset) => preset.id)).toEqual(['h2']);
  });

  it('drops malformed entries instead of exposing unsafe values to the GMC controls', () => {
    const storage = new MemoryStorage();
    storage.setItem(
      GMC_USER_PRESETS_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        harmonies: [
          { id: 'good', name: 'Good', savedAt: 'now', title: 'Tune', chart: 'Cmaj7' },
          { id: 'bad', name: '', savedAt: 'now', title: 'Tune', chart: 'Cmaj7' },
        ],
        scales: [
          {
            id: 'scales-good',
            name: 'Good scales',
            savedAt: 'now',
            title: 'Tune',
            chart: 'Cmaj7',
            chordSymbols: ['Cmaj7'],
            scaleRefs: [null],
          },
          {
            id: 'scales-bad',
            name: 'Bad scales',
            savedAt: 'now',
            title: 'Tune',
            chart: 'Cmaj7',
            chordSymbols: ['Cmaj7'],
            scaleRefs: [{ parent: 'Major', degree: 0 }],
          },
        ],
        patterns: [
          {
            id: 'pattern-good',
            name: 'Good pattern',
            savedAt: 'now',
            pairLabel: 'T/T',
            figureIndex: 0,
            blocks: [{ count: 3, direction: 'asc', triad: 'T1' }],
          },
          {
            id: 'pattern-bad',
            name: 'Bad pattern',
            savedAt: 'now',
            pairLabel: '',
            figureIndex: 0,
            blocks: [{ count: 3, direction: 'asc', triad: 'T1' }],
          },
        ],
      }),
    );

    const loaded = loadGmcUserPresets(storage);
    expect(loaded.harmonies.map((preset) => preset.id)).toEqual(['good']);
    expect(loaded.scales.map((preset) => preset.id)).toEqual(['scales-good']);
    expect(loaded.patterns.map((preset) => preset.id)).toEqual(['pattern-good']);
  });
});
