declare module '$wasm/chordz.js' {
  export default function init(): Promise<void>;
  export function get_roots(): any;
  export function get_all_scales(): any;
  export function get_parent_scale_names(): any;
  export function get_pairs(): any;
  export function resolve_pair(root_pc: number, scale_index: number, pair_index: number): any;
  export function pair_display(root_pc: number, scale_index: number, pair_index: number): string;
  export function get_fretboard_notes(): any;
  export function get_interval_name(semitone: number): string;
  export function get_families(): any;
  export function generate_voicings(root_index: number, family_index: number, note_count: number, prefer_crunch: boolean): any;
  export function get_presets(): any;
  export function solve_chart(chart_text: string, title: string, config: any): any;
  export function solve_chart_with_locks(chart_text: string, title: string, config: any, locks: any): any;
  export function synth_chord(positions: any, duration: number): Float32Array;
  export function synth_arpeggio(positions: any, note_duration: number): Float32Array;
  export function synth_bass_note(root_pc: number, duration: number): Float32Array;
  export function synth_single_note(midi: number, duration: number): Float32Array;
  export function generate_gmc_line(chart_text: string, title: string, pair_index: number, scale_overrides_js: any, figure_index: number, position_fret: number, pattern_js: any): any;
  export function shell_etude_preset(chart_text: string, title: string): any;
  export function generate_walking_bass(segments_js: any): any;
  export function get_default_scale_index(quality_name: string): any;
}
