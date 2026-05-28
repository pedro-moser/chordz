/** Closest sampled MIDI to `midi`; ties resolve to the lower (first-found minimum). */
export function nearestSampledMidi(midi: number, sampled: number[]): number {
  return sampled.reduce(
    (best, cur) => (Math.abs(cur - midi) < Math.abs(best - midi) ? cur : best),
    sampled[0],
  );
}

import { GUITAR_MANIFEST } from './guitarManifest';

let buffers: Map<number, AudioBuffer> | null = null;
let loadPromise: Promise<void> | null = null;
let failed = false;

export function isSamplerReady(): boolean { return buffers !== null; }
export function samplerFailed(): boolean { return failed; }

/** Decode all manifest samples once. Idempotent. Throws (and sets failed) if none load. */
export function loadSampler(ctx: AudioContext): Promise<void> {
  if (loadPromise) return loadPromise;
  loadPromise = (async () => {
    const map = new Map<number, AudioBuffer>();
    await Promise.all(
      Object.entries(GUITAR_MANIFEST).map(async ([midiStr, url]) => {
        try {
          const res = await fetch(url);
          const arr = await res.arrayBuffer();
          map.set(Number(midiStr), await ctx.decodeAudioData(arr));
        } catch (e) {
          console.warn(`guitarSampler: failed to load ${url}`, e);
        }
      }),
    );
    if (map.size === 0) { failed = true; throw new Error('no guitar samples loaded'); }
    buffers = map;
  })();
  return loadPromise;
}

/**
 * Play one note via the nearest sample, pitch-shifted with detune, through a soft
 * gain envelope (attack + release) into `destination`. Returns the source (for stop
 * tracking) or null if the sampler isn't ready.
 */
export function playSample(
  ctx: AudioContext,
  destination: AudioNode,
  midi: number,
  when: number,
  duration: number,
  gain = 1.0,
): AudioBufferSourceNode | null {
  if (!buffers) return null;
  const sampledMidi = nearestSampledMidi(midi, [...buffers.keys()]);
  const buf = buffers.get(sampledMidi);
  if (!buf) return null;

  const src = ctx.createBufferSource();
  src.buffer = buf;
  src.detune.value = (midi - sampledMidi) * 100;

  const env = ctx.createGain();
  const attack = 0.005;
  const release = 0.12;
  const hold = when + Math.max(duration, attack + 0.02);
  env.gain.setValueAtTime(0, when);
  env.gain.linearRampToValueAtTime(gain, when + attack);
  env.gain.setValueAtTime(gain, hold);
  env.gain.linearRampToValueAtTime(0, hold + release);

  src.connect(env);
  env.connect(destination);
  src.start(when);
  src.stop(hold + release + 0.02);
  return src;
}
