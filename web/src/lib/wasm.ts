let wasmModule: typeof import('$wasm/chordz.js') | null = null;

export async function initWasm() {
  if (!wasmModule) {
    const mod = await import('$wasm/chordz.js');
    await mod.default();
    wasmModule = mod;
    // Make raw wasm available for audio module
    const { setWasmRef } = await import('./audio');
    setWasmRef(mod);
  }
}

function getWasm() {
  if (!wasmModule) throw new Error('WASM not initialized');
  return wasmModule;
}

export interface ScaleInfo {
  name: string;
  parent: string;
  degree: number;
  semitones: number[];
}

export interface PairInfo {
  label: string;
  indicesA: number[];
  indicesB: number[];
}

export interface ResolvedPair {
  triadA: number[];
  triadB: number[];
}

export interface FretNote {
  pc: number;
  name: string;
}

export function getRoots(): string[] {
  return getWasm().get_roots();
}

export function getAllScales(): ScaleInfo[] {
  return getWasm().get_all_scales();
}

export function getParentScaleNames(): string[] {
  return getWasm().get_parent_scale_names();
}

export function getPairs(): PairInfo[] {
  return getWasm().get_pairs();
}

export function resolvePair(rootPc: number, scaleIndex: number, pairIndex: number): ResolvedPair {
  return getWasm().resolve_pair(rootPc, scaleIndex, pairIndex);
}

export function pairDisplay(rootPc: number, scaleIndex: number, pairIndex: number): string {
  return getWasm().pair_display(rootPc, scaleIndex, pairIndex);
}

export function getFretboardNotes(): FretNote[][] {
  return getWasm().get_fretboard_notes();
}

export function getIntervalName(semitone: number): string {
  return getWasm().get_interval_name(semitone);
}

export interface VoicingInfo {
  chord: string;
  recipe: string;
  positions: (number | null)[];
  notes: ({ pc: number; name: string } | null)[];
  intervals: string[];
}

export interface SolvedAlternative {
  positions: (number | null)[];
  notes: ({ pc: number; name: string } | null)[];
  intervals: string[];
  recipe: string;
  tension: number;
  relaxation: string;
}

export interface SolvedChange {
  chord: string;
  rootPc: number;
  bassPc: number;
  recipe: string;
  positions: (number | null)[];
  notes: ({ pc: number; name: string } | null)[];
  intervals: string[];
  beats: number;
  alternatives: SolvedAlternative[];
  relaxation: string;
  tension: number;
}

export interface SolveResult {
  changes?: SolvedChange[];
  error?: string;
}

export interface SolverConfig {
  minStrings?: number;
  maxStrings?: number;
  maxFretSpan?: number;
  maxFret?: number;
  minFret?: number;
  tensionTarget?: number;
  abstraction?: number;
  smoothnessWeight?: number;
  jitter?: number;
  allowOpenStrings?: boolean;
  recipes?: string[];
  allowedStrings?: boolean[];
}

export interface Preset {
  title: string;
  chart: string;
}

export interface FamilyInfo {
  index: number;
  name: string;
}

export function getFamilies(): FamilyInfo[] {
  return getWasm().get_families();
}

export function generateVoicings(rootIndex: number, familyIndex: number, noteCount: number, preferCrunch = false): VoicingInfo[] {
  return getWasm().generate_voicings(rootIndex, familyIndex, noteCount, preferCrunch);
}

export function getPresets(): Preset[] {
  return getWasm().get_presets();
}

export function solveChart(chartText: string, title: string, config?: SolverConfig): SolveResult {
  return getWasm().solve_chart(chartText, title, config ?? {});
}

export function solveChartWithLocks(
  chartText: string,
  title: string,
  config: SolverConfig,
  locks: (Pick<SolvedAlternative, 'positions' | 'recipe'> | null)[],
): SolveResult {
  return getWasm().solve_chart_with_locks(chartText, title, config ?? {}, locks);
}

// --- GMC Tune Mode ---

export interface GmcLineEvent {
  beat: number;
  string: number;
  fret: number;
  triad: 'T1' | 'T2';
  pitchClass: number;
  midi: number;
}

export interface GmcChordInfo {
  chord: string;
  rootPc: number;
  bassPc: number | null;
  beats: number;
  defaultScale: string;
  defaultScaleIndex: number;
  activeScale: string;
  isOverride: boolean;
}

export interface GmcLineResult {
  events?: GmcLineEvent[];
  changes?: GmcChordInfo[];
  totalBeats?: number;
  error?: string;
}

export interface GmcPatternBlock {
  count: number;
  direction: 'asc' | 'desc';
  triad: 'T1' | 'T2';
  /** Voice order by triad role (0,1,2 = scale-index order). Absent/empty = monotonic walk. */
  shape?: number[];
  /** Landing note for the block's first note. Absent = nearest (legacy connect). */
  anchor?: 'root' | 'third' | 'fifth';
}

export function generateGmcLine(
  chartText: string,
  title: string,
  pairIndex: number,
  scaleOverrides: (number | null)[],
  figureIndex: number,
  positionFret: number,
  pattern: GmcPatternBlock[],
): GmcLineResult {
  return getWasm().generate_gmc_line(chartText, title, pairIndex, scaleOverrides, figureIndex, positionFret, pattern);
}

// --- Walking bass (pure core, ported from the old TS generator) ---

export interface WalkingBassSegment {
  rootPc: number;
  /** Slash-chord bass pitch class; null/undefined walks off the root. */
  bassPc?: number | null;
  /** Chord symbol, e.g. "Dm7", "G7", "Fm7/Bb" — parsed for its quality. */
  symbol: string;
  beats: number;
}

export interface WalkingBassNote {
  midi: number;
  beat: number;
  beats: number;
  /** Index of the segment (chord) this note belongs to. */
  chord: number;
}

export function generateWalkingBass(segments: WalkingBassSegment[]): WalkingBassNote[] {
  return getWasm().generate_walking_bass(segments);
}

export function synthSingleNote(midi: number, duration: number): Float32Array {
  return new Float32Array(getWasm().synth_single_note(midi, duration));
}

export function getDefaultScaleIndex(qualityName: string): number | null {
  return getWasm().get_default_scale_index(qualityName);
}
