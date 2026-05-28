let ctx: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;
let clickBuffer: AudioBuffer | null = null;

function getContext(): AudioContext {
  if (!ctx) {
    ctx = new AudioContext();
    clickBuffer = createClickBuffer(ctx);
  }
  return ctx;
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

function playInterleaved(samples: Float32Array, sampleRate = 44100) {
  stopAll();
  const audioCtx = getContext();
  const numFrames = samples.length / 2;
  const buffer = audioCtx.createBuffer(2, numFrames, sampleRate);
  const left = buffer.getChannelData(0);
  const right = buffer.getChannelData(1);
  for (let i = 0; i < numFrames; i++) {
    left[i] = samples[i * 2];
    right[i] = samples[i * 2 + 1];
  }
  const source = audioCtx.createBufferSource();
  source.buffer = buffer;
  source.connect(audioCtx.destination);
  source.start();
  currentSource = source;
  source.onended = () => {
    if (currentSource === source) currentSource = null;
  };
}

function playBuffer(samples: Float32Array, gain = 1.0, sampleRate = 44100) {
  const audioCtx = getContext();
  const numFrames = samples.length / 2;
  const buffer = audioCtx.createBuffer(2, numFrames, sampleRate);
  const left = buffer.getChannelData(0);
  const right = buffer.getChannelData(1);
  for (let i = 0; i < numFrames; i++) {
    left[i] = samples[i * 2];
    right[i] = samples[i * 2 + 1];
  }
  const source = audioCtx.createBufferSource();
  source.buffer = buffer;
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

export let bassVolume = 0.7;

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

let wasmRef: any = null;

export function setWasmRef(wasm: any) {
  wasmRef = wasm;
}

function getWasmSync() {
  if (!wasmRef) throw new Error('WASM not set for audio');
  return wasmRef;
}
