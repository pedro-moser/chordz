let wasmModule: typeof import('../../pkg/chordz.js') | null = null;

export async function initWasm() {
  if (!wasmModule) {
    const mod = await import('../../pkg/chordz.js');
    await mod.default();
    wasmModule = mod;
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
