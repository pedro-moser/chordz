import { createEffectsChain, type EffectsChain } from './effects';
import { loadSampler, playSample, isSamplerReady, samplerFailed } from './guitarSampler';

let ctx: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;
let clickBuffer: AudioBuffer | null = null;

let effects: EffectsChain | null = null;
function getEffects(audioCtx: AudioContext): EffectsChain {
  if (!effects) effects = createEffectsChain(audioCtx);
  return effects;
}

/** Kick off sample loading + ambience control. Call once at app init. */
export function initGuitarAudio() {
  const audioCtx = getContext();
  getEffects(audioCtx);
  loadSampler(audioCtx).catch(() => { /* falls back to WASM synth */ });
}

/** 0 = dry, 1 = max reverb. */
export function setAmbience(amount: number) {
  effects?.setAmbience(amount);
}

/** Standard-tuning open-string MIDI (low E to high E). Fretted MIDI = open + fret. */
const OPEN_STRING_MIDI = [40, 45, 50, 55, 59, 64];

/** Convert fret positions per string to the sounding MIDI notes (skips muted strings). */
function positionsToMidi(positions: (number | null)[]): number[] {
  const midi: number[] = [];
  for (let s = 0; s < positions.length && s < OPEN_STRING_MIDI.length; s++) {
    const fret = positions[s];
    if (fret == null) continue;
    midi.push(OPEN_STRING_MIDI[s] + fret);
  }
  return midi;
}

// Sample rate the Rust synth (src/audio/synth.rs SAMPLE_RATE) renders at. The
// AudioContext usually runs at the hardware rate (e.g. 48000); Web Audio resamples
// our buffers on playback, preserving pitch and duration.
const RUST_SAMPLE_RATE = 44100;

function getContext(): AudioContext {
  if (!ctx) {
    ctx = new AudioContext();
    clickBuffer = createClickBuffer(ctx);
  }
  // Autoplay policy can leave the context suspended even when created in a gesture
  // (some mobile/WebView cases); resume so currentTime advances and audio plays.
  if (ctx.state === 'suspended') {
    ctx.resume();
  }
  return ctx;
}

/** Current AudioContext clock time (seconds). Use as the origin for scheduled playback. */
export function getAudioTime(): number {
  return getContext().currentTime;
}

/** Build a stereo AudioBuffer from an interleaved L/R Float32Array. */
function interleavedToBuffer(
  audioCtx: AudioContext,
  samples: Float32Array,
  sampleRate = RUST_SAMPLE_RATE,
): AudioBuffer {
  const numFrames = samples.length / 2;
  const buffer = audioCtx.createBuffer(2, numFrames, sampleRate);
  const left = buffer.getChannelData(0);
  const right = buffer.getChannelData(1);
  for (let i = 0; i < numFrames; i++) {
    left[i] = samples[i * 2];
    right[i] = samples[i * 2 + 1];
  }
  return buffer;
}

function createClickBuffer(audioCtx: AudioContext): AudioBuffer {
  const sr = audioCtx.sampleRate;
  const len = Math.floor(sr * 0.02);
  const buffer = audioCtx.createBuffer(1, len, sr);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < len; i++) {
    const t = i / sr;
    const envelope = Math.exp(-t * 300);
    data[i] = envelope * Math.sin(2 * Math.PI * 1000 * t) * 0.4;
  }
  return buffer;
}

export function playClick() {
  const audioCtx = getContext();
  if (!clickBuffer) return;
  const source = audioCtx.createBufferSource();
  source.buffer = clickBuffer;
  source.connect(audioCtx.destination);
  source.start();
}

export function stopAll() {
  if (currentSource) {
    try { currentSource.stop(); } catch {}
    currentSource = null;
  }
}

function playInterleaved(samples: Float32Array, sampleRate = RUST_SAMPLE_RATE) {
  stopAll();
  const audioCtx = getContext();
  const source = audioCtx.createBufferSource();
  source.buffer = interleavedToBuffer(audioCtx, samples, sampleRate);
  source.connect(audioCtx.destination);
  source.start();
  currentSource = source;
  source.onended = () => {
    if (currentSource === source) currentSource = null;
  };
}

function playBuffer(samples: Float32Array, gain = 1.0, sampleRate = RUST_SAMPLE_RATE) {
  const audioCtx = getContext();
  const source = audioCtx.createBufferSource();
  source.buffer = interleavedToBuffer(audioCtx, samples, sampleRate);
  if (gain !== 1.0) {
    const gainNode = audioCtx.createGain();
    gainNode.gain.value = gain;
    source.connect(gainNode);
    gainNode.connect(audioCtx.destination);
  } else {
    source.connect(audioCtx.destination);
  }
  source.start();
  return source;
}

let bassVolume = 0.4;

export function getBassVolume() { return bassVolume; }
export function setBassVolume(v: number) { bassVolume = v; }

function midiToFreq(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}

/** Bass MIDI for a pitch class: octave-2 root (C2 = 36). */
function bassMidi(rootPc: number): number {
  return 36 + (rootPc % 12);
}

/** Plucked sine bass note, dry to the destination (no reverb). Decays rather than
 *  sustaining, so it doesn't drone/ring into the next chord. */
function playSine(midi: number, when: number, duration: number, gain = bassVolume): OscillatorNode {
  const audioCtx = getContext();
  const osc = audioCtx.createOscillator();
  osc.type = 'sine';
  osc.frequency.value = midiToFreq(midi);
  const env = audioCtx.createGain();
  const attack = 0.01;
  const release = 0.06;
  const end = when + Math.max(duration, 0.2);
  const decayTo = Math.max(when + attack + 0.02, end - release);
  env.gain.setValueAtTime(0, when);
  env.gain.linearRampToValueAtTime(gain, when + attack);
  // Sustain across the whole note with a gentle decay, then a short fade to silence at
  // the end — fills its duration without droning past it (callers cut on the next chord).
  env.gain.linearRampToValueAtTime(gain * 0.4, decayTo);
  env.gain.linearRampToValueAtTime(0, end);
  osc.connect(env);
  env.connect(audioCtx.destination);
  osc.start(when);
  osc.stop(end + 0.03);
  return osc;
}

export function playBass(rootPc: number, duration = 2.0) {
  registerScheduled(playSine(bassMidi(rootPc), getAudioTime(), duration, bassVolume));
}

export function playStrum(positions: (number | null)[]) {
  const audioCtx = getContext();
  if (isSamplerReady() && !samplerFailed()) {
    const fx = getEffects(audioCtx);
    const when = getAudioTime();
    for (const midi of positionsToMidi(positions)) {
      const src = playSample(audioCtx, fx.input, midi, when, 2.0);
      if (src) registerScheduled(src);
    }
    return;
  }
  const { synth_chord } = getWasmSync();
  const samples = synth_chord(positions, 2.0);
  if (samples.length > 0) playInterleaved(new Float32Array(samples));
}

export function playArpeggio(positions: (number | null)[]) {
  const audioCtx = getContext();
  if (isSamplerReady() && !samplerFailed()) {
    const fx = getEffects(audioCtx);
    const when = getAudioTime();
    positionsToMidi(positions).forEach((midi, i) => {
      const src = playSample(audioCtx, fx.input, midi, when + i * 0.06, 0.4);
      if (src) registerScheduled(src);
    });
    return;
  }
  const { synth_arpeggio } = getWasmSync();
  const samples = synth_arpeggio(positions, 0.4);
  if (samples.length > 0) playInterleaved(new Float32Array(samples));
}

let scheduledSources: AudioScheduledSourceNode[] = [];

function registerScheduled(source: AudioScheduledSourceNode) {
  scheduledSources.push(source);
  // Prune on completion so finished buffers can be collected mid-playback.
  source.onended = () => {
    const i = scheduledSources.indexOf(source);
    if (i >= 0) scheduledSources.splice(i, 1);
  };
}

export function playNote(midi: number, duration = 0.3) {
  const audioCtx = getContext();
  if (isSamplerReady() && !samplerFailed()) {
    const src = playSample(audioCtx, getEffects(audioCtx).input, midi, getAudioTime(), duration);
    if (src) registerScheduled(src);
    return;
  }
  const samples: Float32Array = getWasmSync().synth_single_note(midi, duration);
  if (samples.length > 0) playBuffer(samples);
}

/**
 * Schedule melody notes on the AudioContext clock. `startTime` is the clock origin
 * (defaults to "now"); each note starts at `startTime + n.time`. Pass an explicit
 * origin shared with scheduleBassLine so melody and bass stay phase-locked.
 */
export function scheduleNotes(
  notes: { midi: number; time: number; duration: number }[],
  startTime = getAudioTime(),
) {
  stopScheduled();
  const audioCtx = getContext();
  const fx = getEffects(audioCtx);
  const useSampler = isSamplerReady() && !samplerFailed();
  const synth = useSampler ? null : getWasmSync().synth_single_note;
  for (const n of notes) {
    if (!Number.isFinite(n.time) || !Number.isFinite(n.duration) || n.duration <= 0) continue;
    if (useSampler) {
      const src = playSample(audioCtx, fx.input, n.midi, startTime + n.time, n.duration);
      if (src) registerScheduled(src);
    } else {
      const samples: Float32Array = synth!(n.midi, n.duration);
      if (samples.length === 0) continue;
      const source = audioCtx.createBufferSource();
      source.buffer = interleavedToBuffer(audioCtx, samples);
      source.connect(audioCtx.destination);
      source.start(startTime + n.time);
      registerScheduled(source);
    }
  }
}

/**
 * Schedule a bass line of explicit MIDI notes (e.g. a walking line) on the same
 * AudioContext clock as scheduleNotes, registered so stopScheduled() silences them.
 * The caller picks the exact pitch per note. Call AFTER scheduleNotes, which clears
 * the registry.
 */
export function scheduleBassLine(
  notes: { midi: number; time: number; duration: number }[],
  startTime = getAudioTime(),
) {
  for (const n of notes) {
    if (!Number.isFinite(n.time) || !Number.isFinite(n.duration) || n.duration <= 0) continue;
    registerScheduled(playSine(n.midi, startTime + n.time, n.duration, bassVolume));
  }
}

export function stopScheduled() {
  for (const s of scheduledSources) {
    try { s.stop(); } catch {}
  }
  scheduledSources = [];
}

let wasmRef: any = null;

export function setWasmRef(wasm: any) {
  wasmRef = wasm;
}

function getWasmSync() {
  if (!wasmRef) throw new Error('WASM not set for audio');
  return wasmRef;
}
