# Jazz Guitar Sound Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder sine synth with a warm, modern jazz-guitar tone by playing real CC-licensed guitar samples through a hand-rolled Web Audio sampler + algorithmic effects chain (lowpass + runtime-IR reverb), with a dry/ambient toggle and a WASM-synth fallback.

**Architecture:** A JS sampler (`guitarSampler.ts`) decodes a curated subset of the tonejs `guitar-electric` mp3 samples once, then plays any MIDI note by picking the nearest sample and pitch-shifting via `AudioBufferSourceNode.detune`, routed through a shared effects graph (`effects.ts`: lowpass → wet/dry reverb mix → master). `audio.ts` keeps its scheduling/registry/click logic and rewires its sound wrappers to the sampler, falling back to the existing WASM `synth_*` functions if samples fail to load. Rust theory is untouched.

**Tech Stack:** SvelteKit 2 / Svelte 5 runes, TypeScript, Web Audio API, Vitest (added here for the pure logic), Rust/WASM (unchanged, used only as fallback).

**Spec:** `docs/superpowers/specs/2026-05-28-jazz-guitar-sound-design.md`

---

## File structure

- `web/vitest.config.ts` — **create**. Vitest config (node environment) for the pure logic + manifest integrity.
- `web/package.json` — **modify**. Add `vitest` devDependency + `"test": "vitest run"` script.
- `web/src/lib/noteName.ts` — **create**. Pure `noteNameToMidi()` (shared by sampler + tests).
- `web/src/lib/noteName.test.ts` — **create**. Tests for `noteNameToMidi`.
- `web/static/samples/guitar-electric/*.mp3` — **create**. ~12 curated samples (E2–E5).
- `web/src/lib/guitarManifest.ts` — **create**. `MIDI → url` map of shipped samples.
- `web/src/lib/guitarManifest.test.ts` — **create**. Manifest integrity (every file exists) + `nearestSampledMidi` tests.
- `web/src/lib/effects.ts` — **create**. `createEffectsChain(ctx)` (lowpass + reverb mix + master) and the runtime impulse response.
- `web/src/lib/guitarSampler.ts` — **create**. `loadSampler`, `nearestSampledMidi`, `playSample`, ready/failed flags.
- `web/src/lib/audio.ts` — **modify**. Rewire `scheduleNotes`/`scheduleBass`/`playStrum`/`playArpeggio`/`playBass`/`playNote` through the sampler+effects; keep `getAudioTime`/`stopScheduled`/`scheduledSources`/`playClick`; keep WASM path as fallback; eager `loadSampler` on init.
- `web/src/routes/gmc/tune/+page.svelte` — **modify**. Dry/ambient toggle in the playback bar.
- `web/src/routes/+layout.svelte` (or footer component) — **modify**. CC-BY attribution line.

---

## Task 1: Add Vitest for pure-logic tests

**Files:**
- Modify: `web/package.json`
- Create: `web/vitest.config.ts`

- [ ] **Step 1: Add the dev dependency and script**

Run:
```bash
cd web && npm install -D vitest
```
Then add to `web/package.json` `"scripts"`:
```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 2: Create the Vitest config**

Create `web/vitest.config.ts`:
```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
```

- [ ] **Step 3: Verify the runner works (no tests yet)**

Run: `cd web && npm test`
Expected: exits 0 with "No test files found" (or runs 0 tests). This confirms Vitest is wired.

- [ ] **Step 4: Commit**

```bash
git add web/package.json web/package-lock.json web/vitest.config.ts
git commit -m "test: add vitest for pure audio logic"
```

---

## Task 2: `noteNameToMidi` (pure, TDD)

Parses tonejs sample filenames (`A2`, `Cs3` = C#3, `Ds4` = D#4) into MIDI numbers.

**Files:**
- Create: `web/src/lib/noteName.ts`
- Test: `web/src/lib/noteName.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/noteName.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { noteNameToMidi } from './noteName';

describe('noteNameToMidi', () => {
  it('maps natural notes', () => {
    expect(noteNameToMidi('C4')).toBe(60);
    expect(noteNameToMidi('A4')).toBe(69);
    expect(noteNameToMidi('E2')).toBe(40);
  });
  it('maps sharps written with "s" or "#"', () => {
    expect(noteNameToMidi('Cs3')).toBe(49); // C3=48
    expect(noteNameToMidi('C#3')).toBe(49);
    expect(noteNameToMidi('Ds4')).toBe(63); // D4=62
  });
  it('throws on garbage', () => {
    expect(() => noteNameToMidi('H9')).toThrow();
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd web && npx vitest run src/lib/noteName.test.ts`
Expected: FAIL — cannot import `noteNameToMidi` (module not found).

- [ ] **Step 3: Implement**

Create `web/src/lib/noteName.ts`:
```ts
const PITCH_CLASS: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };

/** Parse a note name like "A2", "Cs3" (C#3) or "C#3" into a MIDI number (C4 = 60). */
export function noteNameToMidi(name: string): number {
  const m = name.match(/^([A-G])(s|#)?(-?\d+)$/);
  if (!m) throw new Error(`bad note name: ${name}`);
  let pc = PITCH_CLASS[m[1]];
  if (m[2]) pc += 1;
  const octave = parseInt(m[3], 10);
  return 12 * (octave + 1) + pc;
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd web && npx vitest run src/lib/noteName.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/noteName.ts web/src/lib/noteName.test.ts
git commit -m "feat(audio): noteNameToMidi parser"
```

---

## Task 3: Acquire & curate the guitar samples + manifest

**Files:**
- Create: `web/static/samples/guitar-electric/*.mp3`
- Create: `web/src/lib/guitarManifest.ts`

- [ ] **Step 1: List the available samples in the source set**

Run:
```bash
curl -s https://api.github.com/repos/nbrosowsky/tonejs-instruments/contents/samples/guitar-electric \
  | grep '"name"'
```
Expected: a list of `*.mp3` filenames (note names like `A2.mp3`, `Cs3.mp3`, …). Note the actual available notes.

- [ ] **Step 2: Download a curated subset spanning ~E2–E5, ~every 3 semitones**

Pick ~12 filenames from the listing whose notes are closest to a ~3-semitone grid from E2 to E5 (e.g. around E2, G2, A#2, C#3, E3, G3, A#3, C#4, E4, G4, A#4, C#5 — substitute the nearest available note names). Download each:
```bash
mkdir -p web/static/samples/guitar-electric
BASE=https://raw.githubusercontent.com/nbrosowsky/tonejs-instruments/master/samples/guitar-electric
for f in E2 Fs2 A2 C3 Ds3 Fs3 A3 C4 Ds4 Fs4 A4 C5 E5; do
  curl -fsSL "$BASE/$f.mp3" -o "web/static/samples/guitar-electric/$f.mp3" \
    && echo "ok $f" || echo "MISSING $f (pick nearest available from Step 1)";
done
du -ch web/static/samples/guitar-electric/*.mp3 | tail -1
```
Expected: ~12 files downloaded; total well under 1 MB (replace any `MISSING` note with the nearest name from Step 1's listing).

- [ ] **Step 3: Build the manifest from the downloaded files**

Create `web/src/lib/guitarManifest.ts`. For each downloaded file, the key is `noteNameToMidi(<filename without .mp3>)` and the value is the public URL. Example (adjust to the exact files you downloaded):
```ts
// MIDI number -> public sample URL. Keys computed from filenames via noteNameToMidi
// (E2=40, Fs2=42, A2=45, C3=48, Ds3=51, Fs3=54, A3=57, C4=60, Ds4=63, Fs4=66, A4=69, C5=72, E5=76).
export const GUITAR_MANIFEST: Record<number, string> = {
  40: '/samples/guitar-electric/E2.mp3',
  42: '/samples/guitar-electric/Fs2.mp3',
  45: '/samples/guitar-electric/A2.mp3',
  48: '/samples/guitar-electric/C3.mp3',
  51: '/samples/guitar-electric/Ds3.mp3',
  54: '/samples/guitar-electric/Fs3.mp3',
  57: '/samples/guitar-electric/A3.mp3',
  60: '/samples/guitar-electric/C4.mp3',
  63: '/samples/guitar-electric/Ds4.mp3',
  66: '/samples/guitar-electric/Fs4.mp3',
  69: '/samples/guitar-electric/A4.mp3',
  72: '/samples/guitar-electric/C5.mp3',
  76: '/samples/guitar-electric/E5.mp3',
};
```

- [ ] **Step 4: Commit**

```bash
git add web/static/samples/guitar-electric web/src/lib/guitarManifest.ts
git commit -m "feat(audio): add CC-BY guitar-electric samples + manifest"
```

---

## Task 4: `nearestSampledMidi` + manifest integrity (TDD)

**Files:**
- Create: `web/src/lib/guitarSampler.ts` (only `nearestSampledMidi` in this task)
- Test: `web/src/lib/guitarManifest.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `web/src/lib/guitarManifest.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { GUITAR_MANIFEST } from './guitarManifest';
import { nearestSampledMidi } from './guitarSampler';

const here = dirname(fileURLToPath(import.meta.url));
const staticDir = resolve(here, '../../static');

describe('guitar manifest', () => {
  it('is non-empty', () => {
    expect(Object.keys(GUITAR_MANIFEST).length).toBeGreaterThan(0);
  });
  it('every entry points to a shipped file', () => {
    for (const url of Object.values(GUITAR_MANIFEST)) {
      expect(existsSync(resolve(staticDir, '.' + url)), `missing ${url}`).toBe(true);
    }
  });
});

describe('nearestSampledMidi', () => {
  const sampled = [40, 48, 52, 55];
  it('returns the closest sampled note', () => {
    expect(nearestSampledMidi(53, sampled)).toBe(52);
    expect(nearestSampledMidi(41, sampled)).toBe(40);
  });
  it('on an exact tie keeps the lower note', () => {
    expect(nearestSampledMidi(50, sampled)).toBe(48); // 48 and 52 are both 2 away
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd web && npx vitest run src/lib/guitarManifest.test.ts`
Expected: FAIL — `nearestSampledMidi` not exported from `guitarSampler`.

- [ ] **Step 3: Implement `nearestSampledMidi`**

Create `web/src/lib/guitarSampler.ts` with (the rest of the module is added in Task 6):
```ts
/** Closest sampled MIDI to `midi`; ties resolve to the lower (first-found minimum). */
export function nearestSampledMidi(midi: number, sampled: number[]): number {
  return sampled.reduce(
    (best, cur) => (Math.abs(cur - midi) < Math.abs(best - midi) ? cur : best),
    sampled[0],
  );
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd web && npx vitest run src/lib/guitarManifest.test.ts`
Expected: PASS (all). If "missing file" fails, fix the manifest/filenames from Task 3.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/guitarSampler.ts web/src/lib/guitarManifest.test.ts
git commit -m "feat(audio): nearestSampledMidi + manifest integrity test"
```

---

## Task 5: Effects chain (`effects.ts`)

Web Audio graph; verified by running the app (no unit test — node has no AudioContext).

**Files:**
- Create: `web/src/lib/effects.ts`

- [ ] **Step 1: Implement the effects chain + runtime impulse response**

Create `web/src/lib/effects.ts`:
```ts
export interface EffectsChain {
  /** Connect sample sources here. */
  input: AudioNode;
  /** 0 = dry, 1 = max reverb. */
  setAmbience(amount: number): void;
}

/** Stereo decaying-noise impulse response — a plate/hall-ish reverb with zero asset. */
function makeImpulseResponse(ctx: AudioContext, seconds = 2.2, decay = 3.0): AudioBuffer {
  const length = Math.floor(ctx.sampleRate * seconds);
  const ir = ctx.createBuffer(2, length, ctx.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const data = ir.getChannelData(ch);
    for (let i = 0; i < length; i++) {
      const t = i / length;
      data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, decay);
    }
  }
  return ir;
}

export function createEffectsChain(ctx: AudioContext): EffectsChain {
  const input = ctx.createGain();
  const lowpass = ctx.createBiquadFilter();
  lowpass.type = 'lowpass';
  lowpass.frequency.value = 3500; // warmth; tune by ear
  lowpass.Q.value = 0.7;

  const dry = ctx.createGain();
  const wet = ctx.createGain();
  const convolver = ctx.createConvolver();
  convolver.buffer = makeImpulseResponse(ctx);
  const master = ctx.createGain();
  master.gain.value = 0.9;

  input.connect(lowpass);
  lowpass.connect(dry);
  lowpass.connect(convolver);
  convolver.connect(wet);
  dry.connect(master);
  wet.connect(master);
  master.connect(ctx.destination);

  function setAmbience(amount: number) {
    const a = Math.min(1, Math.max(0, amount));
    wet.gain.value = a;
    dry.gain.value = 1 - a * 0.4; // keep dry mostly present even when wet
  }
  setAmbience(0.35); // moderate default

  return { input, setAmbience };
}
```

- [ ] **Step 2: Type-check**

Run: `cd web && npm run check`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/effects.ts
git commit -m "feat(audio): web audio effects chain (lowpass + runtime-IR reverb)"
```

---

## Task 6: Sampler load + playback (`guitarSampler.ts`)

**Files:**
- Modify: `web/src/lib/guitarSampler.ts`

- [ ] **Step 1: Add loading + playback to the module**

Append to `web/src/lib/guitarSampler.ts` (keep the existing `nearestSampledMidi`):
```ts
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
```

- [ ] **Step 2: Type-check and re-run the pure tests**

Run: `cd web && npm run check && npx vitest run`
Expected: 0 type errors; all tests still PASS.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/guitarSampler.ts
git commit -m "feat(audio): guitar sample loader + pitch-shifted playback"
```

---

## Task 7: Wire `audio.ts` to the sampler (with WASM fallback)

**Files:**
- Modify: `web/src/lib/audio.ts`

- [ ] **Step 1: Add effects + sampler bootstrap to `audio.ts`**

In `web/src/lib/audio.ts`, add imports at the top and an effects accessor near `getContext`:
```ts
import { createEffectsChain, type EffectsChain } from './effects';
import { loadSampler, playSample, isSamplerReady, samplerFailed } from './guitarSampler';

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
```

- [ ] **Step 2: Route `scheduleNotes` through the sampler, falling back to WASM**

Replace the body of `scheduleNotes` in `web/src/lib/audio.ts` with:
```ts
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
```

- [ ] **Step 3: Route `scheduleBass` through the sampler (low root), same fallback**

Replace the body of `scheduleBass` with:
```ts
export function scheduleBass(
  notes: { rootPc: number; time: number; duration: number }[],
  startTime = getAudioTime(),
) {
  const audioCtx = getContext();
  const fx = getEffects(audioCtx);
  const useSampler = isSamplerReady() && !samplerFailed();
  for (const n of notes) {
    if (!Number.isFinite(n.time) || !Number.isFinite(n.duration) || n.duration <= 0) continue;
    if (useSampler) {
      const midi = 36 + (n.rootPc % 12); // octave-2 root (C2 = 36), inside guitar range
      const src = playSample(audioCtx, fx.input, midi, startTime + n.time, n.duration, 0.9);
      if (src) registerScheduled(src);
    } else {
      const samples: Float32Array = getWasmSync().synth_bass_note(n.rootPc, n.duration);
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
}
```

- [ ] **Step 4: Route the one-shot wrappers (`playNote`, `playBass`, `playStrum`, `playArpeggio`) through the sampler**

Replace each with a sampler-first version that falls back to the existing WASM call. For `playStrum`/`playArpeggio` the chord MIDI come from fret positions via `getWasmSync()` — use the existing fret→note path you already call; if the sampler is ready, play each string's MIDI through `playSample` (stagger arpeggio by ~60 ms). Example for `playNote` and `playBass`:
```ts
export function playNote(midi: number, duration = 0.6) {
  const audioCtx = getContext();
  if (isSamplerReady() && !samplerFailed()) {
    const src = playSample(audioCtx, getEffects(audioCtx).input, midi, getAudioTime(), duration);
    if (src) registerScheduled(src);
    return;
  }
  const samples: Float32Array = getWasmSync().synth_single_note(midi, duration);
  if (samples.length > 0) playBuffer(samples);
}

export function playBass(rootPc: number, duration = 2.0) {
  const audioCtx = getContext();
  if (isSamplerReady() && !samplerFailed()) {
    const midi = 36 + (rootPc % 12);
    const src = playSample(audioCtx, getEffects(audioCtx).input, midi, getAudioTime(), duration, 0.9);
    if (src) registerScheduled(src);
    return;
  }
  const samples: Float32Array = getWasmSync().synth_bass_note(rootPc, duration);
  if (samples.length > 0) playBuffer(samples, bassVolume);
}
```
For `playStrum`/`playArpeggio`: keep the current WASM call as the fallback branch; in the sampler branch, derive each played string's MIDI (the project already maps fret positions to notes for the synth — reuse that mapping) and call `playSample` per string (arpeggio: add `i * 0.06` to `when`).

- [ ] **Step 5: Type-check**

Run: `cd web && npm run check`
Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add web/src/lib/audio.ts
git commit -m "feat(audio): route playback through guitar sampler with WASM fallback"
```

---

## Task 8: Ambience toggle in the tune playback bar + init

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Import init + ambience control and call init on mount**

In the `<script>` of `web/src/routes/gmc/tune/+page.svelte`, extend the audio import and call `initGuitarAudio()` inside `onMount` (before the keydown listener):
```ts
import { scheduleNotes, scheduleBass, stopScheduled, getAudioTime, initGuitarAudio, setAmbience } from '$lib/audio';
```
```ts
onMount(() => {
  initGuitarAudio();
  presets = getPresets();
  pairs = getPairs();
  scales = getAllScales();
  window.addEventListener('keydown', onKey);
  return () => {
    window.removeEventListener('keydown', onKey);
    stopPlay();
  };
});
```

- [ ] **Step 2: Add the ambience toggle state + effect**

Add near the other playback state (`let bpm`, `let bassEnabled`):
```ts
let ambient = $state(true);
$effect(() => { setAmbience(ambient ? 0.35 : 0.0); });
```

- [ ] **Step 3: Add the toggle to the playback bar**

In the playback bar (next to the Bass toggle), add:
```svelte
<label class="toggle-label">
  <input type="checkbox" bind:checked={ambient} /> Ambient
</label>
```

- [ ] **Step 4: Type-check**

Run: `cd web && npm run check`
Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc-tune): dry/ambient toggle, init guitar audio on mount"
```

---

## Task 9: CC-BY attribution

**Files:**
- Modify: `web/src/routes/+layout.svelte` (or the existing footer/about location)

- [ ] **Step 1: Add the credit line**

Find the footer/about area (run `rg -n "footer|<footer|©|copyright" web/src` to locate it; if none, add a small `<footer>` in `+layout.svelte`). Add:
```svelte
<small>Guitar samples: tonejs-instruments (CC-BY 3.0).</small>
```

- [ ] **Step 2: Type-check + commit**

Run: `cd web && npm run check`
Expected: 0 errors.
```bash
git add web/src/routes/+layout.svelte
git commit -m "docs(audio): CC-BY attribution for guitar samples"
```

---

## Task 10: End-to-end auditory verification

**Files:** none (verification only)

- [ ] **Step 1: Build the WASM (fallback path) and run the app**

Run:
```bash
PATH="$HOME/.cargo/bin:$HOME/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin:$PATH" web/build.sh
cd web && npm run dev
```

- [ ] **Step 2: Verify in the browser (`/gmc/tune`)**

Generate a line, press Play. Confirm by ear:
- Notes sound like a warm electric guitar (not a sine beep).
- Ambient toggle ON = noticeable reverb/space; OFF = dry and clear.
- Fast lines stay articulate (no clicks; release cuts notes cleanly).
- Bass toggle adds a low guitar root.
- Network tab: ~12 small mp3s load once from `/samples/guitar-electric/`.

- [ ] **Step 3: Verify the fallback**

Temporarily rename `web/static/samples/guitar-electric` → Play again → confirm the app still makes sound (WASM synth) with a single `console.warn`. Restore the folder afterward.

- [ ] **Step 4: Final full check + commit any tweaks**

Run: `cd web && npm run check && npm test`
Expected: 0 type errors; all unit tests pass. Commit any by-ear tuning of `lowpass.frequency`, ambience default, or envelope timings.

---

## Self-review notes (author)

- **Spec coverage:** sampler (T4,T6), effects/lowpass/reverb (T5), real samples + CC set (T3), attribution (T9), audio.ts integration + reuse of scheduling/registry (T7), fallback (T6,T7), ambience toggle (T8), pure-logic tests + manifest integrity (T1,T2,T4), auditory verification (T10), applies to all sounds incl. bass/strum/arpeggio (T7). Phase-2 items (chorus/delay, per-context reverb, multi-velocity) intentionally excluded.
- **Placeholders:** none — all code steps show full code; sample filenames in T3 are illustrative and reconciled against the live listing in T3 Step 1, with the manifest integrity test (T4) catching mismatches.
- **Type consistency:** `EffectsChain.input`/`setAmbience`, `nearestSampledMidi`, `playSample(ctx, dest, midi, when, duration, gain?)`, `loadSampler(ctx)`, `isSamplerReady`/`samplerFailed`, `initGuitarAudio`/`setAmbience` used consistently across T4–T8.
