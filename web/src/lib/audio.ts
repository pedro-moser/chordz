let ctx: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;

function getContext(): AudioContext {
  if (!ctx) ctx = new AudioContext();
  return ctx;
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
