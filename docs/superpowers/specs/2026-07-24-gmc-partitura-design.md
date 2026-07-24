# GMC Tune Mode — Standard Notation Above the Tab — Design

**Date:** 2026-07-24
**Status:** Approved (design), pending implementation plan

## Summary

Add a standard-notation staff to GMC tune mode in the **web app**, drawn directly above the existing
tablature and sharing its horizontal grid. The rule is unconditional: **where there is tab, there is
notation** — no toggle, no setting.

The staff is sight-readable, not decorative: real rhythm (stems, beams, rests, ties across barlines),
functional pitch spelling anchored on the chord, treble clef *8vb* per guitar convention.

Two halves:

1. **Rust** learns to spell. Each `NoteEvent` gains `step`/`alter`/`octave`, derived from the note's
   function over the chord — so a `G7alt` line reads `G A♭ A♯ B C♯ E♭ F`, with the third on B and the
   ♯11 on C♯. Spelling is music theory, so it lives in `src/theory/`.
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
- **`ChordChange.quality` carries the chord's actual intervals** — `ChordQuality { intervals:
  &'static [Interval] }` (`src/theory/chords.rs:5`). Whether a chord has a minor third, a tritone, a
  raised fifth or a diminished seventh is a direct query, not a name match.
- **Every emitted note is a scale tone.** `gmc::resolve_pair` splits the six non-root scale tones into
  two triads; `note_at` only ever returns fretboard positions of those pitch classes. The engine's
  "chromatic glue" (`line_engine.rs:331`) chooses a *rung of the ladder*, not an out-of-scale note.

Consequence: spelling is deterministic, not inference.

### Why scale degree is the wrong anchor

The obvious algorithm — walk the scale's seven degrees, assign the seven letters in order from the
root letter — is wrong, and it is wrong on the most important chord in the repertoire.

G Altered is `semitones: [0, 1, 3, 4, 6, 8, 10]` (`src/theory/scales.rs:161`). Its real spelling is:

```text
G   Ab   A#   B   C#   Eb   F
1   b9   #9   3   #11  b13  b7
```

Letter **A is used twice** (A♭ and A♯) and letter **D is not used at all**. There is no
degree-to-letter bijection to find. Pairing degrees to letters in order instead yields
`G Ab Bb Cb Db Eb F` — which spells the third of G7 as C♭ and the ♯11 as D♭. Nobody reads that.

The anchor is the **chord**, not the mode. The altered scale is a chord-scale: the chord tones fix
their letters, and everything else is a tension named relative to those.

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

/// Spell every pitch class of `scale` over a chord, as notation reads it.
/// Returns, for each of the 12 pitch classes, the letter and alteration to use — or `None` for
/// pitch classes the scale does not contain.
pub fn spell_scale(root_written: &str, quality: &ChordQuality, scale: &Scale) -> [Option<Spelled>; 12];

/// Spell one sounding pitch using a table from `spell_scale`.
pub fn spell_midi(table: &[Option<Spelled>; 12], midi: i32) -> Spelled;
```

The table is built once per chord change, not once per note.

**Letter assignment is chord-anchored.** For each scale tone, its interval above the root (in
semitones) picks a letter offset from the root's letter. Where a semitone distance is ambiguous, the
chord's own `intervals` decide:

| semitones | function | letter offset | condition |
|---|---|---|---|
| 1 | ♭9 | +1 | |
| 2 | 9 | +1 | |
| 3 | ♭3 | +2 | chord contains a minor third |
| 3 | ♯9 | +1 | otherwise |
| 4 | 4 | +3 | chord contains a minor third — the ♭3 already speaks for the third |
| 4 | 3 | +2 | otherwise |
| 5 | 11 | +3 | |
| 6 | ♭5 | +4 | chord contains a tritone (m7♭5, dim7) |
| 6 | ♯11 | +3 | otherwise |
| 7 | 5 | +4 | |
| 8 | ♯5 | +4 | chord contains a minor sixth (dom7♯5) |
| 8 | ♭13 | +5 | otherwise |
| 9 | ♭♭7 | +6 | chord is dim7 |
| 9 | 13 | +5 | otherwise |
| 10 | ♭7 | +6 | |
| 11 | 7 | +6 | |

Then: `step = (root_letter + offset) % 7`; `alter = pc − natural_pc(step)` wrapped into `−6..=5` so
it lands on the small value (pc 9 against letter B → `9 − 11 = −2` = B♭♭, not `+10`);
`octave = (midi − alter).div_euclid(12) − 1` — subtracting the alteration first is what puts B♯3 and
C♭4 in the octave their letter implies.

**A letter may carry two pitches, and that is not an error.** G Altered puts A♭ and A♯ on letter A
and uses no D at all. Any attempt to force seven notes onto seven distinct letters pushes the ♯9 onto
B and spells it B♭ — the original bug. The table above therefore assigns letters independently, with
no uniqueness constraint.

The one case that looks like a collision is not one. Over C dim7, semitone 3 is the chord's E♭ and
semitone 4 also seems to want letter E. It does not: over a chord that already owns a minor third,
semitone 4 is the mode's natural fourth, not its third — hence the `+3` row above, giving F♭. The
rule is functional, not a tie-break.

**Alteration bound.** With chart roots limited to the seventeen forms `root_to_pc` accepts
(`src/theory/chords.rs:195`), no combination should exceed a double accidental. An exhaustive test
over `ChordQuality::ALL` × `Scale::ALL` asserts this rather than assuming it; if some exotic pairing
does exceed it, that is a finding to report, not something to clamp silently.

**Spelling is strictly theoretical, anchored on the chord.** Double accidentals are rendered, never
simplified away, because the spelling *is* the functional information. Worked examples:

```text
G7 + Altered [0,1,3,4,6,8,10]      →  G  Ab  A#  B   C#  Eb  F
                                      1  b9  #9  3   #11 b13 b7
                                         ^^^^^^  letter A twice; letter D never

Cdim7 + Locrian bb7 [0,1,3,4,6,8,9]
                                   →  C  Db  Eb  Fb  Gb  Ab  Bbb
                                      1  b9  b3  4   b5  b13 bb7
```

`Locrian ♭♭7` is what `scale_defaults::default_scale` actually pairs with `dim7`
(`src/theory/scale_defaults.rs`); `Superlocrian ♭♭7` has identical semitones and spells the same.

Consequence: the glyph set must include 𝄫 and 𝄪.

**Fallback.** A pitch class the scale does not contain (currently unreachable, but the in-flight
chromatic approach-note work would hit it) falls back to `PC_NAMES` (`src/theory/notes.rs:3`) — the
jazz default of flats for D♭/E♭/A♭/B♭ and sharps for C♯/F♯. Neither function may panic.

**`NoteEvent` gains three fields** (`src/theory/line_engine.rs:13`):

```rust
pub step: u8,
pub alter: i8,
pub octave: i8,
```

To fill them, `generate_line` must retain what it currently discards. Today it resolves each chord's
scale inside a closure and keeps only the ladders (`line_engine.rs:229`). It will additionally build a
per-chord `Vec<[Option<Spelled>; 12]>` by calling `spell_scale(&change.root, change.quality, scale)`.
`run_pattern` already tracks `active_chord` at the single `events.push` site (`line_engine.rs:355`),
so it indexes that vec and calls `spell_midi`. One call site, no restructuring.

**Transport.** `src/wasm_api.rs` serialises the three new fields in `generate_gmc_line`'s event
mapping; `GmcLineEvent` in `web/src/lib/wasm.ts` declares them. Pure pass-through.

### Part 2 — Layout in TypeScript

**New file `web/src/lib/tabLayout.ts`.** The geometry constants currently living in `+page.svelte`
(`TAB_STRING_GAP`, `TAB_MEASURE_WIDTH`, `TAB_MARGIN_LEFT`, …) plus `tabX` move here, so the staff and
the tab cannot drift apart. The page imports them back.

**New file `web/src/lib/notation.ts`** — pure, no DOM, fully unit-testable. It exports one entry point
that turns a measure's events into everything the renderer needs to place glyphs:

- **Vertical position.** `staffStep({step, octave})` → offset in staff steps above the bottom line
  (E4 written), then a y offset. The staff is treble clef *8vb*: the guitar sounds an octave below
  what is written, so written pitch = sounding pitch + 12. That convention is what keeps the range
  readable — the open low E (sounding MIDI 40) writes as E3, three ledger lines below the staff,
  where notating it at sounding pitch would need seven. The 12th-fret high E (sounding MIDI 76)
  writes as E6, three ledger lines above. Symmetric, and the whole guitar range fits in roughly 90px.
  Ledger lines are emitted at every even staff step beyond the staff's 0–8 range.
- **Duration → figures.** Slot counts map to `(value, dots)`. A span takes the largest printable
  figure that fits up to the next barline, repeatedly, and the pieces are tied. In an eighth grid,
  2.5 beats becomes a half tied to an eighth. A figure may straddle a beat boundary — a half note
  starting on beat 1 is one figure, not two tied quarters — but never a barline. Ties across barlines
  are explicit output, so the renderer never has to reason about them.
  *Deliberately not done:* splitting at beat divisions to expose syncopation (writing an off-beat
  half as eighth-quarter-eighth tied). The engine's spans are short and grid-aligned, so this costs
  little today; it is the first thing to add if the rhythm vocabulary widens.
- **Rests.** The complement of the event spans within each measure, decomposed through the same
  function as notes. Rests consider notes still sounding from an earlier measure, so a note held
  across a barline is never covered by a rest.
- **Beam groups.** Notes grouped by beat. Beamability is decided by the PRINTED FIGURE — only values
  of 8 or shorter carry a flag — not by sounding length, which is what lets an eighth beam with the
  sixteenths beside it while keeping a triplet quarter (two slots, but printed as a quarter) out of
  the beam. A note extended by `hold_last` past a flagged value breaks the group. Triplet groups carry a flag so
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
`<svg>`. It receives the whole line at once, not one measure at a time — a note held across a
barline has to be filed into two measures simultaneously, which per-measure layout cannot express. It is mounted *inside* the existing tab `<svg>` so that one scroll container, one
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
       per chord: spelling::spell_scale(root as written, quality, resolved Scale)
                  -> [Option<Spelled>; 12]
  -> run_pattern
       -> spelling::spell_midi(table_of[active_chord], midi)
  -> NoteEvent { beat, string, fret, triad, pitch_class, midi, duration, step, alter, octave }
  -> wasm_api::generate_gmc_line  (JSON)
  -> GmcLineEvent (wasm.ts)
  -> notation.ts   (pure: figures, ties, rests, beams, stems, accidentals, staff y)
  -> StaffNotation.svelte  (<g> inside the tab <svg>, x from shared tabX)
```

## Error Handling

- Neither spelling function panics: a pitch class absent from the scale falls back to `PC_NAMES`, and
  a `root_written` that fails to parse falls back to the pitch-class name of `root_pc`.
- Letter assignment has no failure mode: every semitone distance has a row in the table, so no scale
  tone can be left unassigned and no search can fail to terminate.
- A duration that cannot be decomposed into figures (should be unreachable given the grid) degrades to
  the largest representable figure plus a tied remainder, looping until consumed, with a hard iteration
  cap so a malformed duration cannot hang the render.
- Pitches outside the drawable range still render, with as many ledger lines as needed; the staff block
  reserves fixed vertical space sized for the guitar range and lets extremes overflow visually rather
  than reflowing the layout.

## Testing

**Rust** (`src/theory/spelling.rs`, `cargo test --lib`):

- `Cmaj7` + Ionian spells C D E F G A B, no accidentals.
- **`G7` + Altered spells G A♭ A♯ B C♯ E♭ F** — the case that killed the degree-ladder algorithm.
  Asserts specifically that the third is B (not C♭) and the ♯11 is C♯ (not D♭), and that letter A
  carries two different pitches while letter D carries none.
- `Cdim7` + Locrian ♭♭7 spells C D♭ E♭ F♭ G♭ A♭ B𝄫 — exercises both the minor-third rule for
  semitone 4 (F♭, not a second E) and the ♭♭7 rule.
- `Cm7b5` + Locrian spells semitone 6 as G♭ (chord tone ♭5, letter+4), not F♯.
- `G7#5` + Altered spells semitone 8 as D♯ (chord tone ♯5), not E♭.
- The compound-interval trap: `dom7#9` must spell semitone 3 as a ♯9 on the ninth's letter. Because
  `Interval::SHARP9.semitones == 15` and `15 % 12 == 3`, a modulo comparison would report a minor
  third and produce B♭. Same for `SHARP11` (18) and `m13` (20).
- `C#7` and `Db7` over the same MIDI pitch produce sharp and flat spellings respectively.
- Octave crossing: B♯3 and C♭4 land in the octave the letter implies, not the one MIDI division implies.
- A pitch class absent from the scale hits the `PC_NAMES` fallback without panicking.
- Every (quality, scale) pair in `ChordQuality::ALL` × `Scale::ALL` spells without panicking and
  produces `|alter| <= 2` for every scale tone — a cheap exhaustive guard over all 28 × 19 combinations.

**Vitest** (`web/src/lib/notation.test.ts`, `npm run test`):

- Duration decomposition: 2.5 beats on an eighth grid → half tied to eighth.
- A note crossing a barline emits two tied figures split at the bar.
- Beam grouping: 2 per beat for eighths, 4 for sixteenths, 3 for triplets; a `hold_last` note breaks
  the group.
- `lead_rest` produces the correct rest figures at the head of the measure.
- Staff position: sounding open low E (MIDI 40) sits 7 staff steps below the bottom line (three
  ledger lines) under the 8vb clef; sounding MIDI 76 sits 6 steps above the top line.
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
- **The staff must render under the figure the line was GENERATED with**, not the one currently
  selected in the controls. The rhythmic-figure buttons do not regenerate the line, so deriving the
  grid from the live control silently re-interprets existing events — measured, clicking Triplet
  turned 256 beamed eighths into 256 unbeamed quarters while the tab sat unchanged.

## Out of Scope

- The native egui GMC tune view.
- Notation in chords tune mode or the browse views — neither renders tablature.
- Key signatures, multi-voice staves, chord-symbol engraving beyond the chord names the tab already prints.
- Export to MusicXML or MIDI files. The spelling fields are exactly what a future MusicXML export would
  need, but no exporter is built here.

---

## Addendum 2026-07-24 — engraving proportions and stacked systems

Two follow-ups Pedro asked for after seeing the first render: *"poe clave na proporção. tem que
arrumar a diagramação no geral."*

### Proportions come from SMuFL, not from taste

Every dimension is expressed in **staff spaces** (sp), the distance between two adjacent staff
lines. Here `STAFF_LINE_GAP = 7px`, so 1 sp = 7px. The numbers come from Bravura's
`engravingDefaults` and `glyphBBoxes` — the same metadata MuseScore consumes, which is why we can
take the metrics without touching MuseScore's GPL-3.0 source (this repo is MIT/Apache-2.0; reading
its engraving *rules* is fine, transplanting its *code* is not).

Key values: `staffLineThickness` 0.13, `stemThickness` 0.12, `beamThickness` 0.50,
`beamSpacing` 0.25, `legerLineThickness` 0.16, `legerLineExtension` 0.40,
`tieMidpointThickness` 0.22, noteheadBlack 1.18 × 1.00, accidentalSharp 0.996 wide,
accidentalFlat 0.904, accidentalNatural 0.672, augmentationDot 0.40 diameter, stem length 3.5
(the last from Gould's *Behind Bars*, not the font metadata).

The treble clef was the visible offender: measured at 63px against a 28px staff, about 9 staff
spaces where a real G clef spans ~6.5.

### Stacked systems, not one long strip

**The measurement that forces this:** on the sixteenth grid `tabX` yields 7.25px between notes,
while a single notehead is 8.26px wide. The notation does not fit, and no amount of glyph
shrinking fixes it — the first attempt squeezed noteheads to 73% and the accidentals still
collided. A sixteenth-note measure needs roughly 280px, not 140.

Widening measures inside one horizontal strip would drop a 1240px viewport from ~9 visible
measures to ~4, and a 32-bar tune would run 9000px wide. So the line **wraps into systems**: each
system holds as many measures as fit the container, staff above tab, systems stacked vertically.
Scrolling becomes vertical, which is how a score actually reads.

**The known trap.** `layoutLine` must still be called on the WHOLE line — its docstring says so,
and a partial slice resurrects the cross-barline bug it exists to prevent. So the split into
systems happens *after* layout: compute all measures once, then slice the resulting
`MeasureLayout[]` into systems. A tie whose partner lands in the next system must not draw a curve
across the break; real engraving ends the tie at the system edge and resumes on the next line.
The current tie lookup reaches `layouts[mi + 1]` unconditionally and would otherwise stretch a
curve to a wildly wrong x.
