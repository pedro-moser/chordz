# GMC Tune Mode — Standard Notation Above the Tab — Design

**Date:** 2026-07-24
**Status:** Approved (design), pending implementation plan

## Summary

Add a standard-notation staff to GMC tune mode in the **web app**, drawn directly above the existing
tablature and sharing its horizontal grid. The rule is unconditional: **where there is tab, there is
notation** — no toggle, no setting.

The staff is sight-readable, not decorative: real rhythm (stems, beams, rests, ties across barlines),
real pitch spelling driven by the active scale, treble clef *8vb* per guitar convention.

Two halves:

1. **Rust** learns to spell. Each `NoteEvent` gains `step`/`alter`/`octave`, derived from the note's
   degree in the chord's active scale. Spelling is music theory, so it lives in `src/theory/`.
2. **Web** learns to engrave. A pure layout module turns events into glyph placements; a Svelte
   component paints them into the tab's existing `<svg>`.

Scope is web-only. The native egui GMC tune view (`src/ui/gmc_tune.rs`) keeps its tab-only display.

## Background

### What exists today

The GMC tune tab is hand-drawn SVG inside `web/src/routes/gmc/tune/+page.svelte` (lines ~635–761).
There is no notation library anywhere in the project — `web/package.json` has zero runtime
dependencies beyond Svelte and the WASM bundle. Note positions come from two helpers:

```ts
function tabX(event: GmcLineEvent, measure): number   // +page.svelte:424
function tabY(engineString: number): number           // +page.svelte:418
```

The page also owns behaviour the staff must not break: per-measure click-to-select, scroll-to-selected-measure,
the selected-measure highlight rect, T1/T2 note colouring, and the scale name printed under each measure.

### What the engine already knows

Three facts make this tractable, and each was verified in the source rather than assumed:

- **`ChordChange.root` is the root as written in the chart** (`src/theory/chart.rs:44`, doc comment:
  *"Root name as written in the chart, e.g. `Bb`, `F#`"*). A chart that says `C#7` spells with sharps;
  `Db7` spells with flats. No key-detection heuristic needed.
- **Every scale is exactly seven notes** — `Scale { semitones: [u8; 7] }` (`src/theory/scales.rs:28`),
  across 28 modes of four parents (major, harmonic minor, melodic minor, harmonic major). Degree → letter
  is therefore a bijection. No eight-note diminished or six-note whole-tone case to special-case.
- **Every emitted note is a scale tone.** `gmc::resolve_pair` splits the six non-root scale tones into
  two triads; `note_at` only ever returns fretboard positions of those pitch classes. The engine's
  "chromatic glue" (`line_engine.rs:331`) chooses a *rung of the ladder*, not an out-of-scale note.

Consequence: spelling is deterministic, not inference.

### Rhythm is a closed problem

`RhythmicFigure` is `Eighth | Sixteenth | Triplet` (`src/theory/line_pattern.rs:39`) on a uniform grid.
The only departures are `hold_last` (last note of a block sustains `1 + hold_last` grid slots) and
`lead_rest` (N slots of silence before a block). So **every duration is an integer multiple of the grid
step**, and every gap comes from `lead_rest` or a silent block. This bounds the notation problem to
something a few hundred lines of pure code can solve exactly.

## Architecture

### Part 1 — Spelling in Rust

**New file `src/theory/spelling.rs`.** One pure function plus a small struct:

```rust
/// A pitch spelled as notation needs it: letter, accidental, octave.
pub struct Spelled {
    pub step: u8,    // 0=C, 1=D, … 6=B
    pub alter: i8,   // −2=♭♭, −1=♭, 0=♮, +1=♯, +2=𝄪
    pub octave: i8,  // scientific pitch notation; C4 = middle C
}

/// Spell `midi` as a degree of `scale` rooted on `root_written` (the chart's own spelling).
pub fn spell(root_written: &str, scale: &Scale, midi: i32) -> Spelled;
```

Algorithm:

1. Parse `root_written` into `(root_letter, root_alter)` — the letter drives the whole ladder.
2. Find degree `d` in `0..7` where `(root_pc + scale.semitones[d]) % 12 == midi.rem_euclid(12)`.
3. `step = (root_letter + d) % 7`.
4. `alter = midi_pc − natural_pc(step)`, wrapped into `−6..=5` so it lands on the small value
   (e.g. pc 9 against letter B → `9 − 11 = −2` = B♭♭, not `+10`).
5. `octave = (midi − alter).div_euclid(12) − 1`. Subtracting the alteration first is what makes
   B♯3 and C♭4 land in the right octave.

**Spelling is strictly theoretical.** Degree always wins: C Altered spells its third as F♭, and
Superlocrian ♭♭7 spells its seventh as B♭♭. Double accidentals are rendered, not simplified away.
The rationale is that the degree spelling is the functional information — reading F♭ tells you it is
the ♭4 of the scale, which reading E does not. Consequence: the glyph set must include 𝄫 and 𝄪.

**Fallback.** If step 2 finds no matching degree (currently unreachable, but a future chromatic
approach-note feature would hit it), fall back to `PC_NAMES` (`src/theory/notes.rs:3`) — the jazz
default of flats for D♭/E♭/A♭/B♭ and sharps for C♯/F♯. The function must not panic.

**`NoteEvent` gains three fields** (`src/theory/line_engine.rs:13`):

```rust
pub step: u8,
pub alter: i8,
pub octave: i8,
```

To fill them, `generate_line` must retain what it currently discards. Today it resolves each chord's
scale inside a closure and keeps only the ladders (`line_engine.rs:229`). It will build a parallel
`Vec<(&str, &Scale)>` of (root-as-written, resolved scale) per chord change. `run_pattern` already
tracks `active_chord` at the single `events.push` site (`line_engine.rs:355`), so it indexes that vec
and calls `spell`. One call site, no restructuring.

**Transport.** `src/wasm_api.rs` serialises the three new fields in `generate_gmc_line`'s event
mapping; `GmcLineEvent` in `web/src/lib/wasm.ts` declares them. Pure pass-through.

### Part 2 — Layout in TypeScript

**New file `web/src/lib/tabLayout.ts`.** The geometry constants currently living in `+page.svelte`
(`TAB_STRING_GAP`, `TAB_MEASURE_WIDTH`, `TAB_MARGIN_LEFT`, …) plus `tabX` move here, so the staff and
the tab cannot drift apart. The page imports them back.

**New file `web/src/lib/notation.ts`** — pure, no DOM, fully unit-testable. It exports one entry point
that turns a measure's events into everything the renderer needs to place glyphs:

- **Vertical position.** `staffPosition({step, octave})` → a diatonic index, then a y offset.
  The staff is treble clef *8vb*: written pitch = sounding pitch + 12. With that clef the open low E
  (MIDI 40) lands exactly on the bottom line and the 12th-fret high E needs only three ledger lines,
  so the whole guitar range fits in roughly 90px of vertical space. Ledger lines are emitted for every
  line position beyond the staff.
- **Duration → figures.** Slot counts map to `(value, dots)`. A span that does not match a single
  figure is split — first at barlines, then at beat divisions — into tied figures. In an eighth grid,
  2.5 beats becomes a half tied to an eighth. Ties across barlines are explicit output, so the renderer
  never has to reason about them.
- **Rests.** The complement of the event spans within each measure. Same beat-splitting decomposition
  as notes.
- **Beam groups.** Events grouped by beat: 2 per beat for eighths, 4 for sixteenths, 3 for triplets.
  A note extended by `hold_last` is not beamable and breaks the group. Triplet groups carry a flag so
  the renderer draws the bracket and the `3`.
- **Stem direction.** Per beam group, decided by the note furthest from the middle staff line; above
  the middle line stems point down, below they point up. Isolated notes decide individually.
- **Accidental display.** There is **no key signature** — the active scale changes per chord, so an
  armature would be a lie. All accidentals are inline. Display rules: show whenever `alter ≠ 0`;
  suppress a repeat of the same `(step, octave)` within the same measure while it is unchanged; emit
  a ♮ when a step that was altered earlier in the measure returns to natural.

**New file `web/src/lib/notationGlyphs.ts`.** SVG path data for the glyphs that geometry cannot
produce: treble clef (with the `8` below), rests (whole, half, quarter, eighth, sixteenth), and
accidentals (♯, ♭, ♮, 𝄫, 𝄪). Noteheads (rotated ellipses), stems, beams, augmentation dots, ledger
lines and ties (quadratic béziers) are drawn from geometry, not glyphs.

**New file `web/src/lib/components/StaffNotation.svelte`.** Renders an SVG `<g>`, not a standalone
`<svg>`. It is mounted *inside* the existing tab `<svg>` so that one scroll container, one
selected-measure highlight and one click target span staff and tab together — the Guitar Pro layout.
Noteheads inherit the T1/T2 colours already used for the tab fret numbers, so the triad-pair reading
survives into the notation.

**Page changes** (`web/src/routes/gmc/tune/+page.svelte`): import the shared layout constants, grow
`tabSvgHeight` and the highlight/click rects by the staff block, and mount `<StaffNotation>`. The
page is already ~1500 lines; all new rendering logic lands in the new files rather than growing it
further.

## Data Flow

```text
chart + pattern
  -> line_engine::generate_line
       per chord: (root as written, resolved Scale)
  -> run_pattern
       -> spelling::spell(root, scale, midi)
  -> NoteEvent { beat, string, fret, triad, pitch_class, midi, duration, step, alter, octave }
  -> wasm_api::generate_gmc_line  (JSON)
  -> GmcLineEvent (wasm.ts)
  -> notation.ts   (pure: figures, ties, rests, beams, stems, accidentals, staff y)
  -> StaffNotation.svelte  (<g> inside the tab <svg>, x from shared tabX)
```

## Error Handling

- `spell` never panics: an unmatched pitch class falls back to `PC_NAMES`, and `root_written` that
  fails to parse falls back to the pitch-class name of `root_pc`.
- A duration that cannot be decomposed into figures (should be unreachable given the grid) degrades to
  the largest representable figure plus a tied remainder, looping until consumed, with a hard iteration
  cap so a malformed duration cannot hang the render.
- Pitches outside the drawable range still render, with as many ledger lines as needed; the staff block
  reserves fixed vertical space sized for the guitar range and lets extremes overflow visually rather
  than reflowing the layout.

## Testing

**Rust** (`src/theory/spelling.rs`, `cargo test --lib`):

- C Ionian spells C D E F G A B, no accidentals.
- C Altered spells its third as F♭ (strict degree spelling, not E).
- C Superlocrian ♭♭7 spells its seventh as B♭♭ — the double-accidental case.
- `C#7` and `Db7` over the same MIDI pitch produce sharp and flat spellings respectively.
- Octave crossing: B♯3 and C♭4 land in the octave the letter implies, not the one MIDI division implies.
- An out-of-scale pitch class hits the `PC_NAMES` fallback without panicking.

**Vitest** (`web/src/lib/notation.test.ts`, `npm run test`):

- Duration decomposition: 2.5 beats on an eighth grid → half tied to eighth.
- A note crossing a barline emits two tied figures split at the bar.
- Beam grouping: 2 per beat for eighths, 4 for sixteenths, 3 for triplets; a `hold_last` note breaks
  the group.
- `lead_rest` produces the correct rest figures at the head of the measure.
- Staff position: sounding open low E (MIDI 40) sits on the bottom line under the 8vb clef.
- Accidental suppression: a repeated altered note within a measure prints one accidental; the same step
  returning to natural prints a ♮.

**Also run before completion** (per `docs/AGENT_GUIDE.md`): `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, `cd web && npm run check`.

## Risks

- **Hand-drawn treble clef.** The clef is the one glyph where hand-authored path data is likely to look
  amateurish. Mitigation: if it does not read well, bundle a subsetted Bravura (SIL OFL 1.1) containing
  only clef, rests and accidentals — a small woff2 and a licence file, still no JS dependency.
- **Vertical space.** The staff adds roughly 90px above the tab, pushing the fretboard panel down in
  `.tune-center`. The layout is desktop-only today (`CONTRIBUTING.md`), so this is a fit check rather
  than a responsive-design problem.
- **WASM rebuild required.** The new `NoteEvent` fields mean the web bundle must be rebuilt before the
  staff shows anything; the rebuild toolchain has known path quirks.

## Out of Scope

- The native egui GMC tune view.
- Notation in chords tune mode or the browse views — neither renders tablature.
- Key signatures, multi-voice staves, chord-symbol engraving beyond the chord names the tab already prints.
- Export to MusicXML or MIDI files. The spelling fields are exactly what a future MusicXML export would
  need, but no exporter is built here.
