# Shell Étude Generator (Motor E) — Design

**Date:** 2026-05-30
**Status:** Superseded by `2026-05-30-shell-etude-preset-design.md` (the separate engine was
replaced by a transparent preset over the existing GMC controls). Shipped then retired.

## Summary

A new generative mode for the GMC line engine that produces a **guide-tone /
shell étude** ("7no5 / 7no3") over any chord chart, rendered as tablature with a
bass voice. The first target tune is **Moment's Notice**. The étude style is
distilled from `materiais/meus/Airegin 7no5:7no3 etude.gp`, which is also the
golden-test oracle.

This is **Motor E** of five generative engines distilled from Pedro's Guitar Pro
library (see "Background"). It is implemented first because its output shape —
two 3-note groups per chord — is identical to the existing GMC triad-pair
engine, so it reuses the entire voice-leading / Shape / Anchor / fretboard
pipeline with a single new note-source resolver.

## Background

Pedro's ~22 `.gp` études reduce to five parameterizable generative engines:

- **A — Triad-Pair Line** (chord-driven) — *already the GMC engine in code*.
- **B — Quartal cell, diatonically sequenced** (scale-driven).
- **C — Inversion / voicing cycler** (chord-driven).
- **D — Barry Harris 6th-diminished harmonizer** (scale-driven).
- **E — Guide-tone / shell voice-leading over changes ("7no5/7no3")** (chord-driven). ← *this spec*

Decision (2026-05-30): use the `.gp` files as **reference** — distill the rules
and implement them as generators; the files become spec + test corpus (not a
runtime importer). Start with **Motor E**, minimal-refactor architecture,
upper-structure shells as the default flavor.

## The distilled rule

For each chord in the chart:

1. **Two 3-note shells.** A `7no5` group (1-3-7 shape) and a `7no3` group
   (1-5-7 shape), drawn from an **upper structure** of the chord. The two groups
   together spell the chord's extended color (guide tones + 9/11/13).
2. **Eighth-note line** weaving the 6 notes, voice-led to the previous note by
   nearest motion, including across bar lines.
3. **Bass voice** — chord root + a guide tone (3rd or 5th), as half notes.

### Shell table (degrees relative to chord root)

Distilled from the Airegin labels; **transposition-invariant** (identical across
all roots in the corpus, confirming a systematic rule).

| Quality   | Shell A (7no5)  | Shell B (7no3)  | Resulting color     |
|-----------|-----------------|-----------------|---------------------|
| `m7`      | b7 · 11 · 13    | b3 · 5 · 9      | Dorian m9/11/13     |
| `7alt`    | b7 · 3 · b13    | #9 · #11 · b9   | Altered scale       |
| `maj7`    | 7 · #11 · 13    | 3 · 5 · 9       | Lydian maj9#11/13   |
| `maj7#5`  | 7 · #11 · 13    | 3 · #5 · 9      | Lydian augmented    |
| `m7b5`    | b7 · 11 · b13   | b3 · b5 · 9     | Locrian #2          |

As semitone offsets from the root (degree→semitone: 1=0, b9=1, 9=2, b3=3, 3=4,
11=5, #11=6, 5=7, #5/b13=8, 13=9, b7=10, 7=11):

```
m7      A=[10, 5, 9]   B=[3, 7, 2]
7alt    A=[10, 4, 8]   B=[3, 6, 1]
maj7    A=[11, 6, 9]   B=[4, 7, 2]
maj7#5  A=[11, 6, 9]   B=[4, 8, 2]
m7b5    A=[10, 5, 8]   B=[3, 6, 2]
```

### Quality mapping & flavors

- **Upper-structure (default):** the table above. Note `maj7` defaults to a
  Lydian (#11) treatment — a deliberate, hip choice matching the corpus.
- **Literal shell (alternative flavor / fallback):** for any quality, build the
  two shells directly from the chord's own intervals — `7no5 = [root, 3rd, 7th]`,
  `7no3 = [root, 5th, 7th]`. Always theoretically correct; used as the fallback
  for qualities absent from the table (e.g. `dim7`).
- **Altered-dominant qualities** (`dom7b9`, `dom7#9`, `dom7#5`, `dom7alt`) map to
  the `7alt` row.
- **Plain `dom7` (e.g. Moment's Notice secondary dominants):** default to the
  `7alt` shells — matches how the Airegin étude treats every dominant, giving a
  consistent modern sound. A "natural dominant (9, 13)" treatment is a future
  toggle, not in scope.
- **`maj7#11`** maps to the `maj7` row (already Lydian).

## Architecture (minimal refactor)

The existing GMC line engine resolves two pitch-class triads per chord at one
call site:

```
// src/theory/line_engine.rs:57  (inside resolve_triad_notes)
let (pcs_a, pcs_b) = gmc::resolve_pair(root_pc, scale, pair);
```

`resolve_pair` returns `([u8;3], [u8;3])`. The shell engine produces the **same
shape**, so Motor E is a swap of this one line behind a source selector.

### New module: `src/theory/shells.rs`

- `SHELL_TABLE`: const mapping from `ChordQuality` (by name) to `[[i8; 3]; 2]`
  semitone offsets.
- `pub fn resolve_shell_pair(root_pc: u8, quality: &ChordQuality) -> ([u8;3],[u8;3])`
  — looks up the table; adds offsets to `root_pc` mod 12. Falls back to literal
  shells built from `quality.intervals` when the quality is not in the table.
- Pure, unit-testable, no fretboard/position dependency.

### Note-source selector in the line engine

```rust
pub enum NoteSource<'a> {
    Gmc(&'a TriadPairSet),  // existing behavior
    Shells,                  // new: chord-quality shells
}
```

- `generate_line` gains a `source: NoteSource` parameter.
- `resolve_triad_notes` branches:
  - `NoteSource::Gmc(pair)` → `gmc::resolve_pair(root_pc, scale, pair)` (unchanged).
  - `NoteSource::Shells` → `shells::resolve_shell_pair(root_pc, change.quality)`
    (the `scale` argument is ignored in this branch).
- The existing GMC call sites pass `NoteSource::Gmc(pair)` → behavior-identical.
  Net change ≈ 10 lines in `line_engine.rs`.

The `triad`/`TriadId` (T1/T2), `Shape`, `Anchor`, pattern walker,
`find_nearest`/`find_closest`/`nearest_of_pc` voice-leading, `NeckPosition`, and
fretboard realization are **all reused unchanged**.

### Bass voice

- `pub fn generate_bass(chart, fretboard) -> Vec<NoteEvent>` — per chord: root +
  one guide tone (3rd or 5th), as half notes on beats 1 and 3.
- Reuses the existing explicit-MIDI bass scheduling path (the `walking_bass` /
  bass work already in the audio core). A `voice: u8` tag (0 = melody, 1 = bass)
  is added to `NoteEvent`, or bass is returned as a separate vec — to be settled
  in the plan against the current bass wiring.

### WASM + UI

- `wasm_api.rs`: `generate_shell_etude(...)` marshalling the chart + config +
  `NoteSource::Shells`, returning melody + bass as JSON.
- UI: a **"Shell Étude"** preset/mode in the GMC tune tab (`ui/gmc_tune.rs`),
  rendering the tab and playing the two voices. Neck position uses the existing
  control. Default rhythmic figure = `Eighth`.

## Line shape

Reuse the `Pattern` walker. Default pattern: alternate `T1`/`T2` as eighth notes
with nearest-note connection across bar lines (the engine already re-resolves the
groups at each chord boundary and voice-leads continuously — it does not restart
on chord changes). Direction/contour variety via the existing Shape/Anchor
grammar; a sensible default preset ships with the mode.

## Testing — the `.gp` as oracle

- **Golden test (Airegin):** parse the Airegin changes, run `generate_line` with
  `NoteSource::Shells`, and assert that the **pitch-class set produced per bar**
  equals the union of the two shells for that chord (the sets labelled in
  `Airegin 7no5:7no3 etude.gp`). This pins the table to Pedro's actual étude.
- **Unit tests (`shells.rs`):** `resolve_shell_pair` returns the documented pcs
  for each quality at several roots; literal-shell fallback for an absent quality.
- **Smoke test (Moment's Notice):** every generated event belongs to a valid
  shell of its chord; inter-bar voice-leading leap ≤ ~4 semitones; event count
  matches `total_beats / figure`.

## Scope cuts (YAGNI)

- No Motors B / C / D yet (quartal, inversions, Barry Harris). The `NoteSource`
  enum is the seam to add them later; a general `Vec<Vec<u8>>` / trait
  abstraction is deferred until a motor needs >2 groups or ≠3 voices.
- No per-phrase silence segmentation: the Airegin étude is one continuous rule.
  (Silence-delimited multi-idea files like `newpairs.gp` matter only when we
  build a multi-rule importer — out of scope.)
- Bass is simple root + guide tone, not a walking line.
- Upper-structure is the default; literal shell is a flag.
- Plain dom7 defaults to `7alt`; a natural-dominant toggle is future work.

## Open items for the plan

- Confirm exact `ChordQuality` names in `src/theory/chords.rs` to key the table.
- Decide `NoteEvent.voice` field vs. separate bass vec, against current bass
  wiring in the audio/render path.
- Pick the default Shell-Étude `Pattern` preset (weave order + anchors).
