# Jazz Guitar Sound — Design

- **Date:** 2026-05-28
- **Status:** Approved (design); pending implementation plan
- **Topic:** Replace the placeholder sine-based synth with a warm, modern jazz-guitar
  tone (reference: Kurt Rosenwinkel, Mike Moreno, Gilad Hekselman) using real samples
  plus a Web Audio effects chain.

## 1. Context & problem

All app audio is currently synthesized in Rust (`src/audio/synth.rs::generate_pluck` =
sine fundamental + 3 harmonics + exponential decay), exposed via `#[wasm_bindgen]`
(`synth_single_note`, `synth_bass_note`, `synth_chord`, `synth_arpeggio`) and played
through the Web Audio API in `web/src/lib/audio.ts`. The result is a "beep with body."
The code comment in `synth.rs` already anticipates this: *"placeholder — swap with real
guitar .wav samples later."*

Goal: a sound that reads as **modern jazz guitar** — clean, warm/dark but clear, smooth
(non-percussive) attack, long singing sustain, bathed in ambience (reverb).

## 2. Decisions (locked during brainstorming)

| Question | Decision |
|---|---|
| Priority | **Balance** — clearly more natural than today, small/moderate asset cost. |
| Tone reference | Modern, ambient, sustained (Rosenwinkel / Moreno / Hekselman). The signature is **envelope + ambience**, not the dry string spectrum. |
| Dry source | **Real samples** (not pure synth, not a soundfont player). |
| Sample sourcing | A **free/CC set chosen for us**. |
| Chosen set | **[tonejs-instruments](https://github.com/nbrosowsky/tonejs-instruments) `guitar-electric`** — web-ready mp3 multisamples, code MIT, samples **CC-BY 3.0** (one attribution line required). |
| Integration approach | **A** — hand-rolled Web Audio sampler + algorithmic effects. No new audio framework (no Tone.js dependency). Reverb via a runtime-generated impulse response (zero asset). |

**Fallback set if licensing/attribution becomes undesirable:** [FreePats Clean Electric
Guitar](https://freepats.zenvoid.org/ElectricGuitar/clean-electric-guitar.html) (CC0, jazz
variant) — more prep (WAV→mp3, subset, bridge-pickup darkened via lowpass). Not chosen now.

## 3. Architecture

**Today:** Rust generates PCM → `audio.ts` wraps in `AudioBuffer` → Web Audio plays it.

**New:** a JS **sampler** loads guitar mp3 samples (decoded once into `AudioBuffer`s).
Any MIDI note plays by selecting the **nearest sampled note** and pitch-shifting via
`AudioBufferSourceNode.detune` (cents), routed through a **shared effects chain** to the
destination. Rust synthesis is demoted to a **graceful-degradation fallback** used only
when sample loading fails (keeps the app sonorous offline / on network error). All the
scheduling logic fixed previously (`scheduleNotes`, `scheduleBass`, `getAudioTime`,
`stopScheduled`, the `scheduledSources` registry) is **reused** — only the buffer source
changes from "WASM-synthesized PCM" to "pitch-shifted sample through effects."

Rust theory (fret→MIDI, line generation, voicings) is unaffected.

## 4. Components

### 4.1 Sample assets
- Location: `web/static/samples/guitar-electric/*.mp3` (served as static files; SvelteKit
  serves `static/` at the site root).
- Curated subset of the tonejs `guitar-electric` set covering the guitar range
  **~E2–E5**, spaced **~3 semitones** apart (≈12 files) so the maximum pitch-shift is
  ±~1.5 semitones (clean, minimal artifact). **Mono** to halve size where the source
  allows. Target total **< ~1 MB**.
- `manifest`: a small static map of sampled MIDI number → filename, committed alongside
  the audio (e.g. `manifest.ts` or a JSON). Exact source note list is taken from the
  repo's `samples/guitar-electric/` directory during implementation; the manifest records
  whatever subset is shipped.
- **Attribution:** one CC-BY 3.0 credit line in the app footer / an About/credits entry.

### 4.2 `web/src/lib/guitarSampler.ts` (new)
- `loadSampler(): Promise<void>` — fetch each manifest file and `decodeAudioData` into a
  `Map<midi, AudioBuffer>`. Idempotent (a single in-flight promise is cached). May be
  called eagerly at app init or lazily on first sound.
- `nearestSample(midi: number): number` — **pure function** over the manifest's sampled
  MIDI list; returns the closest sampled MIDI. Unit-testable.
- `playSample(midi, when, duration, gain?)` — create a `BufferSource` from the nearest
  buffer, set `detune = (midi − sampledMidi) * 100`, apply a **gain envelope** (soft
  attack; release ~80–150 ms) so short notes are cut cleanly (no click) and the tone keeps
  a smooth, sustained character; connect to the effects input; `start(when)` /
  `stop(when + duration + release)`. Register the source so `stopScheduled` can stop it.

### 4.3 `web/src/lib/effects.ts` (new; or a section of `audio.ts`)
- Builds the shared graph **once** per AudioContext:
  `input → lowpass (BiquadFilter, ~3–4 kHz) → [dry / wet split]`,
  wet → `ConvolverNode` → `wetGain`; dry → `dryGain`; both → `masterGain` → `destination`.
- **Reverb impulse response generated at runtime** (zero asset): a stereo `AudioBuffer`
  (~2 s) filled with exponentially-decaying white noise, slightly decorrelated L/R.
- `setAmbience(amount: 0..1)` — controls the wet/dry mix. Default = moderate.
- Chorus / delay are **out of scope for phase 1** (see §7).

### 4.4 `web/src/lib/audio.ts` (integration)
- `scheduleNotes`, `scheduleBass`, `playStrum`, `playArpeggio`, `playBass`, `playNote`
  become thin wrappers that compute the relevant MIDI set and call the sampler+effects.
- `getAudioTime`, `stopScheduled`, the `scheduledSources` registry, and the metronome
  `playClick` are **kept as-is** (`playClick` stays a synthesized click — not a guitar).
- Chords/strum = N sampled notes played simultaneously (MIDI per string already known from
  fret positions). Bass = the same guitar samples played low (root at octave ~2–3, inside
  guitar range), through the same effects chain in phase 1; per-context reverb tuning
  (e.g. less on bass) is deferred to phase 2.

### 4.5 Fallback
- If `loadSampler()` rejects, set a flag and route the wrappers back to the existing WASM
  `synth_*` functions, with a `console.warn`. The app stays sonorous.

## 5. Data flow

```
Play (tune mode) → playThrough builds note events
  → scheduleNotes(notes, startTime)
    → per note: guitarSampler.playSample(midi, startTime + time, duration)
      → BufferSource(detune) → effects input → lowpass → reverb mix → master → destination
stopPlay → stopScheduled() stops all sampler sources
```

## 6. Error handling
- Sample-set load failure → fallback to WASM synth + `console.warn`.
- Per-file `decodeAudioData` failure → skip that sample; `nearestSample` simply widens to
  the next available.
- Suspended `AudioContext` → `resume()` (already implemented).
- Non-finite timings/durations → already guarded in `audio.ts` and in Rust `generate_pluck`.

## 7. Scope & phasing
- **Applies to:** all current sounds — tune-mode lines, strum, arpeggio, bass, single note.
  Metronome click unchanged.
- **Ambience toggle:** a dry/ambient toggle in the tune-mode playback bar (beside BPM /
  Bass), because this is a practice tool and heavy reverb muddies fast lines. Default:
  moderate ambience.
- **Phase 1 (this spec):** sampler + lowpass + algorithmic reverb + ambience toggle +
  fallback. This delivers the target vibe.
- **Phase 2 (optional, later):** chorus/short delay, per-context reverb amounts (e.g. less
  on bass), multi-velocity samples.

## 8. Testing
- No JS test runner exists today (only `svelte-check`).
- Keep `nearestSample` and the detune math as **pure functions**; add a **manifest
  integrity** check (every manifest entry has a shipped file).
- Tone quality is verified **auditorily by running the app** (the `verify` skill at the end).
- Rust `synth.rs` tests remain valid (fallback path).
- *Optional, non-blocking:* add Vitest for the pure functions.

## 9. Non-goals (YAGNI)
- No second audio framework (Tone.js) — hand-rolled Web Audio only.
- No multi-velocity / round-robin sampling in phase 1.
- No amp-sim / distortion (the target tone is clean).
- No baked reverb in Rust — ambience is a live Web Audio graph.

## 10. File-level change summary
- **New:** `web/static/samples/guitar-electric/*.mp3` + manifest; `web/src/lib/guitarSampler.ts`;
  `web/src/lib/effects.ts`.
- **Modified:** `web/src/lib/audio.ts` (wrappers route through sampler+effects; keep
  scheduling, registry, click, and WASM-synth fallback); `web/src/routes/gmc/tune/+page.svelte`
  (ambience toggle in the playback bar); app footer/about (CC-BY credit line).
- **Unchanged:** Rust theory + `synth.rs` (kept as fallback).
