import type { GmcPatternBlock } from './wasm';

export const GMC_USER_PRESETS_STORAGE_KEY = 'chordz.gmc.user-presets.v1';

export interface HarmonyUserPreset {
  id: string;
  name: string;
  savedAt: string;
  title: string;
  chart: string;
}

export interface ScaleReference {
  parent: string;
  degree: number;
}

export interface ScaleUserPreset {
  id: string;
  name: string;
  savedAt: string;
  title: string;
  chart: string;
  chordSymbols: string[];
  scaleRefs: (ScaleReference | null)[];
}

export interface PatternUserPreset {
  id: string;
  name: string;
  savedAt: string;
  pairLabel: string;
  figureIndex: number;
  blocks: GmcPatternBlock[];
}

export interface GmcUserPresetLibrary {
  version: 1;
  harmonies: HarmonyUserPreset[];
  scales: ScaleUserPreset[];
  patterns: PatternUserPreset[];
}

export type GmcUserPresetKind = 'harmony' | 'scales' | 'pattern';

export type HarmonyUserPresetDraft = Omit<HarmonyUserPreset, 'id' | 'savedAt'>;
export type ScaleUserPresetDraft = Omit<ScaleUserPreset, 'id' | 'savedAt'>;
export type PatternUserPresetDraft = Omit<PatternUserPreset, 'id' | 'savedAt'>;

export interface SavePresetMetadata {
  id?: string;
  savedAt?: string;
}

export type PresetStorage = Pick<Storage, 'getItem' | 'setItem'>;

const FIGURE_COUNT = 3;
const MAX_PATTERN_BLOCKS = 64;
const CONNECTORS = new Set([
  'nearestUp',
  'nearestDown',
  'invertUp',
  'invertDown',
  'voiceLead',
  'random',
]);
const ANCHORS = new Set(['root', 'third', 'fifth']);

function emptyLibrary(): GmcUserPresetLibrary {
  return { version: 1, harmonies: [], scales: [], patterns: [] };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function isNonNegativeInteger(value: unknown): value is number {
  return Number.isInteger(value) && Number(value) >= 0;
}

function sanitizeBase(value: Record<string, unknown>) {
  if (
    !isNonEmptyString(value.id) ||
    !isNonEmptyString(value.name) ||
    !isNonEmptyString(value.savedAt)
  ) {
    return null;
  }
  return { id: value.id, name: value.name.trim(), savedAt: value.savedAt };
}

function sanitizeHarmony(value: unknown): HarmonyUserPreset | null {
  if (!isRecord(value)) return null;
  const base = sanitizeBase(value);
  if (!base || typeof value.title !== 'string' || !isNonEmptyString(value.chart)) return null;
  return { ...base, title: value.title, chart: value.chart };
}

function sanitizeScaleReference(value: unknown): ScaleReference | null | false {
  if (value === null) return null;
  if (
    !isRecord(value) ||
    !isNonEmptyString(value.parent) ||
    !Number.isInteger(value.degree) ||
    Number(value.degree) < 1 ||
    Number(value.degree) > 7
  ) {
    return false;
  }
  return { parent: value.parent, degree: Number(value.degree) };
}

function sanitizeScale(value: unknown): ScaleUserPreset | null {
  if (!isRecord(value)) return null;
  const base = sanitizeBase(value);
  if (
    !base ||
    typeof value.title !== 'string' ||
    !isNonEmptyString(value.chart) ||
    !Array.isArray(value.chordSymbols) ||
    !Array.isArray(value.scaleRefs) ||
    value.chordSymbols.length !== value.scaleRefs.length ||
    value.chordSymbols.some((symbol) => !isNonEmptyString(symbol))
  ) {
    return null;
  }
  const scaleRefs = value.scaleRefs.map(sanitizeScaleReference);
  if (scaleRefs.some((reference) => reference === false)) return null;
  return {
    ...base,
    title: value.title,
    chart: value.chart,
    chordSymbols: [...value.chordSymbols] as string[],
    scaleRefs: scaleRefs as (ScaleReference | null)[],
  };
}

function sanitizeRoleOrder(value: unknown): number[] | undefined | null {
  if (value === undefined) return undefined;
  if (
    !Array.isArray(value) ||
    value.length === 0 ||
    value.some((role) => !Number.isInteger(role) || role < 0 || role > 2)
  ) {
    return null;
  }
  return [...value] as number[];
}

function sanitizeContour(value: unknown): number[] | undefined | null {
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || value.length === 0 || value.length > 8) return null;
  const ranks = value as unknown[];
  if (ranks.some((rank) => !Number.isInteger(rank) || Number(rank) < 1 || Number(rank) > ranks.length)) {
    return null;
  }
  if (new Set(ranks).size !== ranks.length) return null;
  return [...ranks] as number[];
}

function sanitizePatternBlock(value: unknown): GmcPatternBlock | null {
  if (!isRecord(value)) return null;
  if (
    !Number.isInteger(value.count) ||
    Number(value.count) < 1 ||
    Number(value.count) > 6 ||
    (value.direction !== 'asc' && value.direction !== 'desc') ||
    (value.triad !== 'T1' && value.triad !== 'T2')
  ) {
    return null;
  }

  const shape = sanitizeRoleOrder(value.shape);
  const contour = sanitizeContour(value.contour);
  if (shape === null || contour === null) return null;
  if (value.anchor !== undefined && !ANCHORS.has(String(value.anchor))) return null;
  if (value.connector !== undefined && !CONNECTORS.has(String(value.connector))) return null;
  if (value.holdLast !== undefined && (!isNonNegativeInteger(value.holdLast) || value.holdLast > 16)) return null;
  if (value.leadRest !== undefined && (!isNonNegativeInteger(value.leadRest) || value.leadRest > 16)) return null;

  const block: GmcPatternBlock = {
    count: Number(value.count),
    direction: value.direction,
    triad: value.triad,
  };
  if (shape !== undefined) block.shape = shape;
  if (value.anchor !== undefined) block.anchor = value.anchor as GmcPatternBlock['anchor'];
  if (value.holdLast !== undefined) block.holdLast = value.holdLast;
  if (value.leadRest !== undefined) block.leadRest = value.leadRest;
  if (value.connector !== undefined) block.connector = value.connector as GmcPatternBlock['connector'];
  if (contour !== undefined) block.contour = contour;
  return block;
}

function sanitizePattern(value: unknown): PatternUserPreset | null {
  if (!isRecord(value)) return null;
  const base = sanitizeBase(value);
  if (
    !base ||
    !isNonEmptyString(value.pairLabel) ||
    !isNonNegativeInteger(value.figureIndex) ||
    value.figureIndex >= FIGURE_COUNT ||
    !Array.isArray(value.blocks) ||
    value.blocks.length === 0 ||
    value.blocks.length > MAX_PATTERN_BLOCKS
  ) {
    return null;
  }
  const blocks = value.blocks.map(sanitizePatternBlock);
  if (blocks.some((block) => block === null)) return null;
  return {
    ...base,
    pairLabel: value.pairLabel,
    figureIndex: value.figureIndex,
    blocks: blocks as GmcPatternBlock[],
  };
}

function compactMap<T>(values: unknown, sanitize: (value: unknown) => T | null): T[] {
  if (!Array.isArray(values)) return [];
  return values.map(sanitize).filter((value): value is T => value !== null);
}

export function loadGmcUserPresets(storage: Pick<PresetStorage, 'getItem'>): GmcUserPresetLibrary {
  try {
    const raw = storage.getItem(GMC_USER_PRESETS_STORAGE_KEY);
    if (!raw) return emptyLibrary();

    const value: unknown = JSON.parse(raw);
    if (!isRecord(value) || value.version !== 1) return emptyLibrary();
    return {
      version: 1,
      harmonies: compactMap(value.harmonies, sanitizeHarmony),
      scales: compactMap(value.scales, sanitizeScale),
      patterns: compactMap(value.patterns, sanitizePattern),
    };
  } catch {
    return emptyLibrary();
  }
}

export type ScalePresetResolution =
  | { ok: true; overrides: (number | null)[] }
  | { ok: false; reason: 'harmony-mismatch' | 'missing-scale' };

export function resolveScalePresetOverrides(
  preset: ScaleUserPreset,
  scaleCatalog: readonly Pick<ScaleReference, 'parent' | 'degree'>[],
  parsedChordSymbols: readonly string[],
): ScalePresetResolution {
  if (
    preset.chordSymbols.length !== parsedChordSymbols.length ||
    preset.chordSymbols.some((symbol, index) => symbol !== parsedChordSymbols[index])
  ) {
    return { ok: false, reason: 'harmony-mismatch' };
  }

  const overrides: (number | null)[] = [];
  for (const reference of preset.scaleRefs) {
    if (reference === null) {
      overrides.push(null);
      continue;
    }
    const index = scaleCatalog.findIndex(
      (scale) => scale.parent === reference.parent && scale.degree === reference.degree,
    );
    if (index < 0) return { ok: false, reason: 'missing-scale' };
    overrides.push(index);
  }
  return { ok: true, overrides };
}

export function resolvePatternPresetPairIndex(
  preset: PatternUserPreset,
  pairCatalog: readonly { label: string }[],
): number | null {
  const index = pairCatalog.findIndex((pair) => pair.label === preset.pairLabel);
  return index >= 0 ? index : null;
}

function generatePresetId(): string {
  if (typeof globalThis.crypto?.randomUUID === 'function') return globalThis.crypto.randomUUID();
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

function saveLibrary(storage: PresetStorage, library: GmcUserPresetLibrary): void {
  storage.setItem(GMC_USER_PRESETS_STORAGE_KEY, JSON.stringify(library));
}

function upsertByName<T extends { id: string; name: string }>(items: T[], item: T): T[] {
  const normalized = item.name.toLocaleLowerCase();
  const index = items.findIndex((candidate) => candidate.name.toLocaleLowerCase() === normalized);
  if (index < 0) return [...items, item];
  const next = [...items];
  next[index] = { ...item, id: items[index].id };
  return next;
}

export function saveGmcUserPreset(
  storage: PresetStorage,
  kind: 'harmony',
  draft: HarmonyUserPresetDraft,
  metadata?: SavePresetMetadata,
): GmcUserPresetLibrary;
export function saveGmcUserPreset(
  storage: PresetStorage,
  kind: 'scales',
  draft: ScaleUserPresetDraft,
  metadata?: SavePresetMetadata,
): GmcUserPresetLibrary;
export function saveGmcUserPreset(
  storage: PresetStorage,
  kind: 'pattern',
  draft: PatternUserPresetDraft,
  metadata?: SavePresetMetadata,
): GmcUserPresetLibrary;
export function saveGmcUserPreset(
  storage: PresetStorage,
  kind: GmcUserPresetKind,
  draft: HarmonyUserPresetDraft | ScaleUserPresetDraft | PatternUserPresetDraft,
  metadata: SavePresetMetadata = {},
): GmcUserPresetLibrary {
  const library = loadGmcUserPresets(storage);
  const base = {
    ...draft,
    id: metadata.id ?? generatePresetId(),
    name: draft.name.trim(),
    savedAt: metadata.savedAt ?? new Date().toISOString(),
  };

  if (!base.name) throw new Error('Preset name cannot be empty.');

  if (kind === 'harmony') {
    const preset = sanitizeHarmony(base);
    if (!preset) throw new Error('Harmony preset is invalid.');
    library.harmonies = upsertByName(library.harmonies, preset);
  } else if (kind === 'scales') {
    const preset = sanitizeScale(base);
    if (!preset) throw new Error('Scale preset is invalid.');
    library.scales = upsertByName(library.scales, preset);
  } else {
    const preset = sanitizePattern(base);
    if (!preset) throw new Error('Pattern preset is invalid.');
    library.patterns = upsertByName(library.patterns, preset);
  }

  saveLibrary(storage, library);
  return library;
}

export function deleteGmcUserPreset(
  storage: PresetStorage,
  kind: GmcUserPresetKind,
  id: string,
): GmcUserPresetLibrary {
  const library = loadGmcUserPresets(storage);
  if (kind === 'harmony') {
    library.harmonies = library.harmonies.filter((preset) => preset.id !== id);
  } else if (kind === 'scales') {
    library.scales = library.scales.filter((preset) => preset.id !== id);
  } else {
    library.patterns = library.patterns.filter((preset) => preset.id !== id);
  }
  saveLibrary(storage, library);
  return library;
}
