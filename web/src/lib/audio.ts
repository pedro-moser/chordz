let ctx: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;
let clickBuffer: AudioBuffer | null = null;

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

let bassVolume = 0.7;

export function getBassVolume() { return bassVolume; }
export function setBassVolume(v: number) { bassVolume = v; }

export function playBass(rootPc: number, duration = 2.0) {
  const { synth_bass_note } = getWasmSync();
  const samples = synth_bass_note(rootPc, duration);
  if (samples.length > 0) playBuffer(new Float32Array(samples), bassVolume);
}

export function playStrum(positions: (number | null)[]) {
  const { synth_chord } = getWasmSync();
  const samples = synth_chord(positions, 2.0);
  if (samples.length > 0) playInterleaved(new Float32Array(samples));
}

export function playArpeggio(positions: (number | null)[]) {
  const { synth_arpeggio } = getWasmSync();
  const samples = synth_arpeggio(positions, 0.4);
  if (samples.length > 0) playInterleaved(new Float32Array(samples));
}

let scheduledSources: AudioBufferSourceNode[] = [];

function registerScheduled(source: AudioBufferSourceNode) {
  scheduledSources.push(source);
  // Prune on completion so finished buffers can be collected mid-playback.
  source.onended = () => {
    const i = scheduledSources.indexOf(source);
    if (i >= 0) scheduledSources.splice(i, 1);
  };
}

export function playNote(midi: number, duration = 0.3) {
  const { synth_single_note } = getWasmSync();
  const samples: Float32Array = synth_single_note(midi, duration);
  if (samples.length > 0) playBuffer(samples);
}

/**
 * Schedule melody notes on the AudioContext clock. `startTime` is the clock origin
 * (defaults to "now"); each note starts at `startTime + n.time`. Pass an explicit
 * origin shared with scheduleBass so melody and bass stay phase-locked.
 */
export function scheduleNotes(
  notes: { midi: number; time: number; duration: number }[],
  startTime = getAudioTime(),
) {
  stopScheduled();
  const { synth_single_note } = getWasmSync();
  const audioCtx = getContext();
  for (const n of notes) {
    if (!Number.isFinite(n.time) || !Number.isFinite(n.duration) || n.duration <= 0) continue;
    const samples: Float32Array = synth_single_note(n.midi, n.duration);
    if (samples.length === 0) continue;
    const source = audioCtx.createBufferSource();
    source.buffer = interleavedToBuffer(audioCtx, samples);
    source.connect(audioCtx.destination);
    source.start(startTime + n.time);
    registerScheduled(source);
  }
}

/**
 * Schedule bass notes on the same AudioContext clock as scheduleNotes. Routed through
 * the shared scheduledSources registry so stopScheduled() silences them too (unlike the
 * fire-and-forget playBass, an in-flight scheduled bass note is stoppable). Call AFTER
 * scheduleNotes, since scheduleNotes clears the registry on entry.
 */
export function scheduleBass(
  notes: { rootPc: number; time: number; duration: number }[],
  startTime = getAudioTime(),
) {
  const { synth_bass_note } = getWasmSync();
  const audioCtx = getContext();
  for (const n of notes) {
    if (!Number.isFinite(n.time) || !Number.isFinite(n.duration) || n.duration <= 0) continue;
    const samples: Float32Array = synth_bass_note(n.rootPc, n.duration);
    if (samples.length === 0) continue;
    const source = audioCtx.createBufferSource();
    source.buffer = interleavedToBuffer(audioCtx, samples);
    const gainNode = audioCtx.createGain();
    gainNode.gain.value = bassVolume;
    source.connect(gainNode);
    gainNode.connect(audioCtx.destination);
    source.start(startTime + n.time);
    registerScheduled(source);
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
