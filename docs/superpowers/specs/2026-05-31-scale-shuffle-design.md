# 🎲 Cores — Shuffle Valid Scales per Chord — Design

**Date:** 2026-05-31
**Status:** Approved (design), pending implementation plan
**Author:** Pedro + Claude

## Summary

A **"🎲 Cores"** button on the GMC tune page that, for each chord in the chart, sets its scale
override to a **random scale valid for that chord's quality**, then regenerates — a fast way to
explore color combinations. Built on the existing per-chord `scaleOverrides`: the shuffle sets the
visible controls (transparent, editable), and re-clicking gives a new combination. It is the
exploratory counterpart to the Shell Étude preset (which *fixes* the characteristic color).

## What counts as a valid scale for a chord ("do tipo certo")

Pedro approved **rule (a): the scale contains the chord's root + 3rd + 7th** (the guide tones that
define the quality). Refined here for one wart found while specifying:

> **The 5th is constrained only when it is altered.** A scale is valid for a chord when it contains
> the chord's **3rd and 7th** semitones, AND — only if the chord's 5th is *not* the perfect 5th
> (semitone 7) — also the chord's **5th**.

Why the refinement: pure (a) ignores the 5th, so `m7b5` (b5), `dim7` (°5), and `dom7#5` (#5) would
admit natural-5 scales (e.g. Dorian over a half-diminished chord) — a genuine clash, not a color.
Constraining the 5th *only when it is characteristic* keeps maximum variety for plain maj7/m7/dom7
(where altered colors are wanted) while keeping the b5/°5/#5 family honest. **Pedro to confirm at
spec review; revert to pure (a) if preferred.**

Computation (semitones from root, `Scale.semitones: [u8;7]` includes 0; `ChordQuality.intervals`
is `[root, 3rd, 5th, 7th, …]`):
- `third = intervals[1].semitones`, `fifth = intervals[2].semitones`, `seventh = intervals[3].semitones`
- a scale is valid iff `scale.semitones` contains `third` and `seventh`, and (`fifth == 7` or
  `scale.semitones` contains `fifth`).

The list is never empty: the chord's own `default_scale` always satisfies the rule.

### Worked examples (the 28 modes in `Scale::ALL`)
- **maj7** (3=4, 7=11; P5): Ionian, Lydian, Lydian Augmented, Ionian #5, Harmonic Major… (bright major modes). Excludes every minor mode.
- **m7** (3=3, 7=10; P5): Dorian, Aeolian, Phrygian, Dorian ♭2, Dorian #4… Excludes major modes.
- **dom7** (3=4, 7=10; P5): Mixolydian, Lydian Dominant, **Altered**, Mixolydian ♭6, Phrygian Dominant, Mixolydian ♭2… (the full dominant palette — max color).
- **m7b5** (3=3, 5=6, 7=10): Locrian, Aeolian ♭5, Dorian ♭5… (half-diminished family). **Excludes Dorian/Aeolian** (the wart the refinement fixes).
- **dim7** (3=3, 5=6, 7=9): Locrian ♭♭7 (and any other mode with °5 + °7) — naturally restrictive.

## Architecture

### Core — `src/theory/scale_defaults.rs`

```rust
/// Indices into `Scale::ALL` of every scale valid for `quality`: it contains the chord's 3rd and
/// 7th, plus the 5th when the 5th is altered (not P5). Always non-empty (the default scale fits).
/// Used by the "🎲 Cores" scale shuffle.
pub fn valid_scales(quality: &ChordQuality) -> Vec<usize>
```

Pure, testable, one responsibility. (The shuffle's randomness lives in the web front, not here.)

### WASM — `src/wasm_api.rs`

```rust
/// Per-chord lists of scale indices valid for each chord's quality, for the "🎲 Cores" shuffle.
/// Returns `{ validScales: number[][] }` (or `{ error }` on a parse failure).
#[wasm_bindgen]
pub fn valid_scales_for_chart(chart_text: &str, title: &str) -> JsValue
```

Mirrors `shell_etude_preset`'s shape: parse the chart, map each `change.quality` through
`valid_scales`, return the per-chord index lists.

### Web

- `web/src/wasm.d.ts`: `export function valid_scales_for_chart(chart_text: string, title: string): any;`
- `web/src/lib/wasm.ts`:
  ```typescript
  export interface ValidScalesResult { validScales?: number[][]; error?: string }
  export function validScalesForChart(chartText: string, title: string): ValidScalesResult
  ```
- `web/src/routes/gmc/tune/+page.svelte`: a **"🎲 Cores"** button next to the existing **Scales**
  button. On click: `validScalesForChart(chartInput, titleInput)`; if not an error, for each chord
  pick a random index from its list (`list[Math.floor(Math.random()*list.length)]`), set
  `scaleOverrides` to those, set `overridesFor = chartInput` (so `generate()`'s positional reset
  guard doesn't wipe them — same load-bearing detail as the shell preset), then `generate()`. The
  per-chord scale labels visibly update; the user can re-shuffle or hand-edit any chord.

## Scope

The shuffle touches **only the scales** — not the triad pair, pattern, figure, or position.
One-shot apply, fully editable afterward. Re-clicking = a fresh random combination.

## Testing

- `valid_scales` (core): maj7 includes Lydian/Ionian and excludes minor modes; m7 includes
  Dorian/Aeolian and excludes major modes; **m7b5 includes Locrian/Aeolian ♭5 and EXCLUDES Dorian**
  (the refinement); dom7 includes Altered; every `ChordQuality::ALL` entry returns a non-empty list
  that contains its own `default_scale` index.

## Scope cuts (YAGNI)

- No "shuffle pairs" or "shuffle pattern" (this is scales only; pair-presets are a separate track).
- No seed/reproducibility — `Math.random` in the front is fine; re-shuffle for a new combo.
- No weighting toward "more colorful" scales — uniform random over the valid set.

## Open items for the plan

- Confirm the rule refinement (conditional 5th) vs. pure (a) with Pedro at spec review.
- Confirm the UI button placement and that `overridesFor` is the right state name (it is, per the
  shell-preset work).
