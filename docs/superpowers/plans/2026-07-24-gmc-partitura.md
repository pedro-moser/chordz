# GMC Partitura (Standard Notation Above the Tab) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draw a sight-readable standard-notation staff directly above the existing tablature in GMC tune mode on the web, sharing the tab's horizontal grid.

**Architecture:** Rust gains functional pitch spelling (letter + accidental + octave) anchored on the chord, exposed on every `NoteEvent`. TypeScript gains a pure layout module that turns those events into glyph placements — figures, ties, rests, beams, stems, accidentals, staff positions — and a Svelte component paints them as an SVG `<g>` inside the tab's existing `<svg>`.

**Tech Stack:** Rust (`src/theory/`), `wasm-bindgen` + `serde_json` transport, SvelteKit 2 / Svelte 5 runes, hand-authored SVG (no notation library), `cargo test` + `vitest`.

**Spec:** `docs/superpowers/specs/2026-07-24-gmc-partitura-design.md`

## Global Constraints

- **No new runtime dependencies.** `web/package.json` has zero runtime deps and keeps it that way. No VexFlow, no abcjs, no music font package. All glyphs are hand-authored SVG paths or geometry.
- **Web only.** Do not touch `src/ui/gmc_tune.rs` (the native egui view). It keeps its tab-only display.
- **No Claude attribution anywhere.** Commit messages must NOT carry a `Co-Authored-By` trailer or any Claude signature. Repo history was rewritten on 2026-07-21 to strip them.
- **Commit message style:** lowercase `tipo(escopo): descricao` in pt-BR **without accents**, matching `git log`.
- **Rust gates before finishing:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` must all pass (`docs/AGENT_GUIDE.md`).
- **Web gates before finishing:** from `web/`, `npm run check` (svelte-check) and `npm run test` (vitest).
- **`cargo` may not be on PATH.** If `cargo` is not found, source the Rust env first: `source "$HOME/.cargo/env"` (fish: `source "$HOME/.cargo/env.fish"`).
- **Spelling is functional and strict.** Double accidentals are rendered, never simplified. `G7` + Altered must read `G Ab A# B C# Eb F` — third on B, ♯11 on C♯, ♯9 on A♯.

---

### Task 1: Spelling foundations — `Spelled`, root parsing, letter offsets

The lookup layer for functional spelling. `letter_offset` is the heart: it decides which letter above the root a scale tone claims, using the chord's own intervals to break ties.

**THE TRAP IN THIS TASK.** `ChordQuality::intervals` stores tensions as **compound** intervals — `Interval::SHARP9.semitones == 15`, `SHARP11 == 18`, `m13 == 20`, `M9 == 14`, `M13 == 21` (`src/theory/intervals.rs`). If you compare with `i.semitones % 12`, then `dom7#9` reports "has a minor third" (15 % 12 == 3) and its ♯9 gets spelled B♭ instead of A♯ — the exact bug this whole design exists to avoid. Compare with **plain equality**, never modulo. Only true chord tones (semitones < 12) may anchor a letter.

**Files:**
- Create: `src/theory/spelling.rs`
- Modify: `src/theory/mod.rs` (add `pub mod spelling;`, alphabetical — between `scales` and `triad_shape`)
- Test: inline `#[cfg(test)] mod tests` in `src/theory/spelling.rs` (matches every other file in `src/theory/`)

**Interfaces:**
- Consumes: `crate::theory::chords::ChordQuality` (field `intervals: &'static [Interval]`), `crate::theory::intervals::Interval` (field `semitones: u8`)
- Produces:
  - `pub struct Spelled { pub step: u8, pub alter: i8, pub octave: i8 }` — `Clone, Copy, PartialEq, Eq, Debug`
  - `pub(crate) const NATURAL_PC: [u8; 7]`
  - `pub(crate) fn parse_root(root: &str) -> (u8, i8)`
  - `pub(crate) fn letter_offset(semitones: u8, quality: &ChordQuality) -> u8`

- [ ] **Step 1: Write the failing test**

Create `src/theory/spelling.rs` with only the test module and the item declarations it needs:

```rust
use crate::theory::chords::ChordQuality;

/// A pitch spelled as notation needs it: letter, accidental, octave.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Spelled {
    /// 0=C, 1=D, 2=E, 3=F, 4=G, 5=A, 6=B.
    pub step: u8,
    /// -2 = double flat, -1 = flat, 0 = natural, 1 = sharp, 2 = double sharp.
    pub alter: i8,
    /// Scientific pitch notation of the SOUNDING pitch; middle C is C4. The
    /// treble-8vb transposition is the renderer's job, not this module's.
    pub octave: i8,
}

/// Pitch class of each unaltered letter, indexed by `step`.
pub(crate) const NATURAL_PC: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Split a chart root like "Bb" or "F#" into `(step, alter)`.
pub(crate) fn parse_root(_root: &str) -> (u8, i8) {
    unimplemented!()
}

/// Which letter above the root a scale tone claims.
pub(crate) fn letter_offset(_semitones: u8, _quality: &ChordQuality) -> u8 {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn parses_chart_roots() {
        assert_eq!(parse_root("C"), (0, 0));
        assert_eq!(parse_root("Bb"), (6, -1));
        assert_eq!(parse_root("F#"), (3, 1));
        assert_eq!(parse_root("A"), (5, 0));
    }

    #[test]
    fn unparseable_root_falls_back_to_c_natural() {
        assert_eq!(parse_root(""), (0, 0));
        assert_eq!(parse_root("H"), (0, 0));
    }

    #[test]
    fn chord_tones_anchor_their_own_letter() {
        // m7 has a real minor third (3 semitones) -> letter+2.
        assert_eq!(letter_offset(3, quality("m7")), 2);
        // m7b5 has a real tritone -> letter+4, the flat fifth.
        assert_eq!(letter_offset(6, quality("m7b5")), 4);
        // dom7#5 has a real minor sixth -> letter+4, the sharp fifth.
        assert_eq!(letter_offset(8, quality("dom7#5")), 4);
        // dim7 has a real diminished seventh -> letter+6, the bb7.
        assert_eq!(letter_offset(9, quality("dim7")), 6);
    }

    #[test]
    fn tensions_do_not_anchor_letters_despite_compound_semitones() {
        // Interval::SHARP9.semitones == 15, and 15 % 12 == 3. A modulo comparison
        // would make dom7#9 claim a minor third and spell its #9 as Bb. It must not.
        assert_eq!(letter_offset(3, quality("dom7#9")), 1, "#9 belongs on the 9th's letter");
        // SHARP11 == 18, 18 % 12 == 6.
        assert_eq!(letter_offset(6, quality("dom7#11")), 3, "#11 belongs on the 11th's letter");
        // m13 == 20, 20 % 12 == 8.
        assert_eq!(letter_offset(8, quality("dom7b13")), 5, "b13 belongs on the 13th's letter");
        // M13 == 21, 21 % 12 == 9.
        assert_eq!(letter_offset(9, quality("dom13")), 5, "13 belongs on the 13th's letter");
    }

    #[test]
    fn semitone_four_moves_off_the_third_when_the_chord_owns_a_minor_third() {
        // Over dim7 the chord's b3 already speaks for the third, so semitone 4 is the
        // mode's natural fourth: C dim7 + Locrian bb7 reads Eb then Fb, never Eb then E.
        assert_eq!(letter_offset(4, quality("dim7")), 3);
        assert_eq!(letter_offset(4, quality("m7")), 3);
        // Over a dominant or major chord it is the third, on the third's letter.
        assert_eq!(letter_offset(4, quality("dom7")), 2);
        assert_eq!(letter_offset(4, quality("maj7")), 2);
    }

    #[test]
    fn plain_dominant_takes_the_altered_tension_readings() {
        let dom7 = quality("dom7");
        assert_eq!(letter_offset(0, dom7), 0);
        assert_eq!(letter_offset(1, dom7), 1); // b9
        assert_eq!(letter_offset(3, dom7), 1); // #9, not b3
        assert_eq!(letter_offset(4, dom7), 2); // 3
        assert_eq!(letter_offset(6, dom7), 3); // #11, not b5
        assert_eq!(letter_offset(8, dom7), 5); // b13, not #5
        assert_eq!(letter_offset(10, dom7), 6); // b7
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Add `pub mod spelling;` to `src/theory/mod.rs` first, then run:

```bash
cargo test --lib theory::spelling
```

Expected: FAIL — every test panics with `not implemented`.

- [ ] **Step 3: Write the implementation**

Replace the two `unimplemented!()` bodies:

```rust
pub(crate) fn parse_root(root: &str) -> (u8, i8) {
    let mut chars = root.chars();
    let step = match chars.next() {
        Some('C') => 0,
        Some('D') => 1,
        Some('E') => 2,
        Some('F') => 3,
        Some('G') => 4,
        Some('A') => 5,
        Some('B') => 6,
        _ => return (0, 0),
    };
    let alter = match chars.next() {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    (step, alter)
}

/// Which letter above the root a scale tone claims, given its distance in semitones
/// and the chord it sits over.
///
/// Chord tones anchor their own letter; the ambiguous distances (3, 6, 8, 9) are
/// decided by what the chord actually contains. The comparison is plain equality on
/// `semitones` — NOT modulo 12 — because `ChordQuality::intervals` stores tensions as
/// compound intervals (`SHARP9` is 15, `SHARP11` is 18, `m13` is 20). Reducing them
/// would make a #9 masquerade as a b3 and spell G7#9's A# as Bb.
pub(crate) fn letter_offset(semitones: u8, quality: &ChordQuality) -> u8 {
    let is_chord_tone = |s: u8| quality.intervals.iter().any(|i| i.semitones == s);
    match semitones {
        0 => 0,
        1 | 2 => 1,                                    // b9 / 9
        3 => if is_chord_tone(3) { 2 } else { 1 },     // b3 (chord tone) else #9
        // Over a chord that already owns a minor third, semitone 4 is not "the third" —
        // it is the natural fourth of the mode. C dim7 + Locrian bb7 reads Eb then Fb.
        4 => if is_chord_tone(3) { 3 } else { 2 },     // 4 (over a minor third) else 3
        5 => 3,                                        // 11
        6 => if is_chord_tone(6) { 4 } else { 3 },     // b5 (chord tone) else #11
        7 => 4,                                        // 5
        8 => if is_chord_tone(8) { 4 } else { 5 },     // #5 (chord tone) else b13
        9 => if is_chord_tone(9) { 6 } else { 5 },     // bb7 (dim7 chord tone) else 13
        10 | 11 => 6,                                  // b7 / maj7
        _ => 0,
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test --lib theory::spelling
```

Expected: PASS, 6 tests.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/theory/spelling.rs src/theory/mod.rs
git commit -m "teoria(grafia): tabela de letras ancorada nos intervalos do acorde"
```

---

### Task 2: `spell_scale` — build the per-chord pitch-class table

Turns a (root, quality, scale) triple into a 12-slot lookup: for each pitch class the scale contains, the letter and accidental to print. Built once per chord change, not once per note.

**Files:**
- Modify: `src/theory/spelling.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/theory/spelling.rs`

**Interfaces:**
- Consumes: `Spelled`, `NATURAL_PC`, `parse_root`, `letter_offset` (Task 1); `crate::theory::scales::Scale` (field `semitones: [u8; 7]`)
- Produces: `pub fn spell_scale(root_written: &str, quality: &ChordQuality, scale: &Scale) -> [Option<Spelled>; 12]` — indexed by pitch class; `octave` is left at `0` and filled in later by `spell_midi`.

- [ ] **Step 1: Write the failing test**

Add to `src/theory/spelling.rs` above the test module:

```rust
use crate::theory::scales::Scale;

/// Spell every pitch class of `scale` over a chord, as notation reads it.
///
/// Indexed by pitch class; `None` for pitch classes the scale does not contain.
/// The `octave` field is a placeholder here — `spell_midi` fills it from a real pitch.
pub fn spell_scale(
    _root_written: &str,
    _quality: &ChordQuality,
    _scale: &Scale,
) -> [Option<Spelled>; 12] {
    unimplemented!()
}
```

Add these tests inside `mod tests`:

```rust
    fn scale(name: &str) -> &'static Scale {
        Scale::ALL.iter().find(|s| s.name == name).unwrap()
    }

    /// Render a spelled table as ascending note names from the root, for readable asserts.
    fn spell_names(root: &str, quality_name: &str, scale_name: &str) -> Vec<String> {
        const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
        let q = quality(quality_name);
        let s = scale(scale_name);
        let table = spell_scale(root, q, s);
        let (root_step, root_alter) = parse_root(root);
        let root_pc = (NATURAL_PC[root_step as usize] as i16 + root_alter as i16).rem_euclid(12) as u8;
        s.semitones
            .iter()
            .map(|&semi| {
                let pc = (root_pc + semi) % 12;
                let sp = table[pc as usize].expect("scale tone must be spelled");
                let acc = match sp.alter {
                    -2 => "bb",
                    -1 => "b",
                    0 => "",
                    1 => "#",
                    2 => "##",
                    _ => "?",
                };
                format!("{}{}", LETTERS[sp.step as usize], acc)
            })
            .collect()
    }

    #[test]
    fn ionian_over_maj7_has_no_accidentals() {
        assert_eq!(
            spell_names("C", "maj7", "Ionian"),
            ["C", "D", "E", "F", "G", "A", "B"]
        );
    }

    #[test]
    fn altered_over_g7_reads_functionally() {
        // The case that killed the scale-degree algorithm: letter A is used twice
        // (Ab and A#) and letter D is not used at all. Third on B, #11 on C#.
        assert_eq!(
            spell_names("G", "dom7", "Altered"),
            ["G", "Ab", "A#", "B", "C#", "Eb", "F"]
        );
    }

    #[test]
    fn altered_reuses_a_letter_rather_than_forcing_a_bijection() {
        // The regression guard for the bug this design exists to avoid: a letter may
        // carry two pitches, and forcing seven notes onto seven distinct letters would
        // push the #9 onto B and spell it Bb.
        let names = spell_names("G", "dom7", "Altered");
        assert_eq!(names.iter().filter(|n| n.starts_with('A')).count(), 2, "{:?}", names);
        assert!(!names.iter().any(|n| n.starts_with('D')), "{:?}", names);
        assert!(!names.contains(&"Bb".to_string()), "the #9 must be A#, got {:?}", names);
    }

    #[test]
    fn dim7_keeps_its_double_flat_seventh() {
        // The chord's b3 already speaks for the third, so semitone 4 is the mode's
        // natural fourth: Fb. And the bb7 survives as a genuine double flat.
        assert_eq!(
            spell_names("C", "dim7", "Locrian \u{266D}\u{266D}7"),
            ["C", "Db", "Eb", "Fb", "Gb", "Ab", "Bbb"]
        );
    }

    #[test]
    fn chart_root_spelling_decides_sharps_versus_flats() {
        assert_eq!(spell_names("C#", "dom7", "Mixolydian")[0], "C#");
        assert_eq!(spell_names("Db", "dom7", "Mixolydian")[0], "Db");
    }

    #[test]
    fn m7b5_spells_its_flat_five_on_the_fifth_letter() {
        // Cm7b5 + Locrian: semitone 6 is the chord's b5 -> Gb, never F#.
        let names = spell_names("C", "m7b5", "Locrian");
        assert!(names.contains(&"Gb".to_string()), "got {:?}", names);
        assert!(!names.contains(&"F#".to_string()), "got {:?}", names);
    }

    #[test]
    fn dom7_sharp5_spells_its_raised_fifth_on_the_fifth_letter() {
        // G7#5 + Altered: semitone 8 is the chord's #5 -> D#, not Eb.
        let names = spell_names("G", "dom7#5", "Altered");
        assert!(names.contains(&"D#".to_string()), "got {:?}", names);
    }

    #[test]
    fn every_quality_and_scale_pair_spells_without_panicking() {
        for quality in ChordQuality::ALL {
            for scale in Scale::ALL {
                for root in ["C", "F#", "Bb", "Eb", "A"] {
                    let table = spell_scale(root, quality, scale);
                    let (rs, ra) = parse_root(root);
                    let root_pc =
                        (NATURAL_PC[rs as usize] as i16 + ra as i16).rem_euclid(12) as u8;
                    for &semi in &scale.semitones {
                        let pc = (root_pc + semi) % 12;
                        let sp = table[pc as usize].unwrap_or_else(|| {
                            panic!("{} {} {} left pc {} unspelled", root, quality.name, scale.name, pc)
                        });
                        assert!(
                            sp.alter.abs() <= 2,
                            "{} {} {} spelled pc {} with alter {}",
                            root, quality.name, scale.name, pc, sp.alter
                        );
                    }
                }
            }
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --lib theory::spelling
```

Expected: FAIL — `not implemented` from `spell_scale`.

- [ ] **Step 3: Write the implementation**

Replace the `spell_scale` body:

```rust
pub fn spell_scale(
    root_written: &str,
    quality: &ChordQuality,
    scale: &Scale,
) -> [Option<Spelled>; 12] {
    let (root_step, root_alter) = parse_root(root_written);
    let root_pc = (NATURAL_PC[root_step as usize] as i16 + root_alter as i16).rem_euclid(12) as u8;

    let mut table: [Option<Spelled>; 12] = [None; 12];

    for &semi in &scale.semitones {
        let pc = (root_pc + semi) % 12;
        if table[pc as usize].is_some() {
            continue; // a scale listing the same pitch class twice needs only one spelling
        }
        let step = (root_step + letter_offset(semi, quality)) % 7;
        table[pc as usize] = Some(Spelled {
            step,
            alter: alter_for(step, pc),
            octave: 0,
        });
    }
    table
}

/// The alteration that turns `step`'s natural pitch class into `pc`, on the short side
/// of the octave: pc 9 against letter B is -2 (Bbb), not +10.
fn alter_for(step: u8, pc: u8) -> i8 {
    let mut alter = pc as i16 - NATURAL_PC[step as usize] as i16;
    if alter > 5 {
        alter -= 12;
    } else if alter < -6 {
        alter += 12;
    }
    alter as i8
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test --lib theory::spelling
```

Expected: PASS, 14 tests.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/theory/spelling.rs
git commit -m "teoria(grafia): tabela por acorde com resolucao de colisao de letra"
```

---

### Task 3: `spell_midi` — attach the octave

Takes a table from Task 2 plus a sounding MIDI number and produces a complete `Spelled`. The octave must follow the letter, not the MIDI division: B♯3 and C♭4 are a semitone apart but sit in different letter-octaves.

**Files:**
- Modify: `src/theory/spelling.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/theory/spelling.rs`

**Interfaces:**
- Consumes: `Spelled`, `NATURAL_PC`, `spell_scale` (Tasks 1–2); `crate::theory::notes::PC_NAMES`
- Produces: `pub fn spell_midi(table: &[Option<Spelled>; 12], midi: i32) -> Spelled`

- [ ] **Step 1: Write the failing test**

Add above the test module:

```rust
/// Spell one sounding pitch using a table from `spell_scale`.
///
/// A pitch class the scale does not contain falls back to the jazz default names in
/// `PC_NAMES` (flats for Db/Eb/Ab/Bb, sharps for C#/F#). Never panics.
pub fn spell_midi(_table: &[Option<Spelled>; 12], _midi: i32) -> Spelled {
    unimplemented!()
}
```

Add these tests inside `mod tests`:

```rust
    #[test]
    fn octave_comes_from_middle_c() {
        let table = spell_scale("C", quality("maj7"), scale("Ionian"));
        // MIDI 60 is middle C = C4.
        let c4 = spell_midi(&table, 60);
        assert_eq!((c4.step, c4.alter, c4.octave), (0, 0, 4));
        // MIDI 40 is the guitar's open low E = E2.
        let e2 = spell_midi(&table, 40);
        assert_eq!((e2.step, e2.alter, e2.octave), (2, 0, 2));
    }

    #[test]
    fn octave_follows_the_letter_not_the_midi_division() {
        // Cb4 sounds at MIDI 59, which MIDI-divides into octave 3. As a C it is
        // still octave 4. Build a table that spells pc 11 as Cb.
        let mut table: [Option<Spelled>; 12] = [None; 12];
        table[11] = Some(Spelled { step: 0, alter: -1, octave: 0 });
        let cb = spell_midi(&table, 59);
        assert_eq!((cb.step, cb.alter, cb.octave), (0, -1, 4));

        // B#3 sounds at MIDI 60, which MIDI-divides into octave 4. As a B it is octave 3.
        let mut table: [Option<Spelled>; 12] = [None; 12];
        table[0] = Some(Spelled { step: 6, alter: 1, octave: 0 });
        let bs = spell_midi(&table, 60);
        assert_eq!((bs.step, bs.alter, bs.octave), (6, 1, 3));
    }

    #[test]
    fn pitch_class_outside_the_scale_falls_back_without_panicking() {
        let table = spell_scale("C", quality("maj7"), scale("Ionian"));
        // Ionian on C has no pc 6; PC_NAMES calls it F#.
        let fs = spell_midi(&table, 66);
        assert_eq!((fs.step, fs.alter, fs.octave), (3, 1, 4));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --lib theory::spelling
```

Expected: FAIL — `not implemented` from `spell_midi`.

- [ ] **Step 3: Write the implementation**

Replace the `spell_midi` body, and add the `PC_NAMES` import at the top of the file
(`use crate::theory::notes::PC_NAMES;`):

```rust
pub fn spell_midi(table: &[Option<Spelled>; 12], midi: i32) -> Spelled {
    let pc = midi.rem_euclid(12) as usize;
    let (step, alter) = match table[pc] {
        Some(s) => (s.step, s.alter),
        None => fallback_spelling(pc),
    };
    // Subtract the alteration first so the octave follows the LETTER: Cb4 sounds at
    // MIDI 59 but is a C, and B#3 sounds at MIDI 60 but is a B.
    let octave = (midi - alter as i32).div_euclid(12) - 1;
    Spelled {
        step,
        alter,
        octave: octave as i8,
    }
}

/// Jazz default spelling for a pitch class the chord-scale does not contain.
fn fallback_spelling(pc: usize) -> (u8, i8) {
    let name = PC_NAMES[pc];
    let mut chars = name.chars();
    let step = match chars.next() {
        Some('C') => 0,
        Some('D') => 1,
        Some('E') => 2,
        Some('F') => 3,
        Some('G') => 4,
        Some('A') => 5,
        _ => 6,
    };
    let alter = match chars.next() {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    (step, alter)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test --lib theory::spelling
```

Expected: PASS, 17 tests.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add src/theory/spelling.rs
git commit -m "teoria(grafia): oitava segue a letra, nao a divisao do midi"
```

---

### Task 4: Carry the spelling on every `NoteEvent`

Wire the spelling module into the line engine so generated events arrive pre-spelled.

**Files:**
- Modify: `src/theory/line_engine.rs` — the `NoteEvent` struct (line ~13), `generate_line` (line ~215), `run_pattern` (signature at line ~247 and the `events.push` site at line ~355)
- Test: inline `#[cfg(test)] mod tests` in `src/theory/line_engine.rs`

**Interfaces:**
- Consumes: `spell_scale`, `spell_midi`, `Spelled` (Tasks 1–3)
- Produces: `NoteEvent` gains `pub step: u8`, `pub alter: i8`, `pub octave: i8`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` in `src/theory/line_engine.rs`:

```rust
    #[test]
    fn events_carry_functional_spelling() {
        use crate::theory::chart::Chart;
        use crate::theory::gmc::PAIRS;
        use crate::theory::line_pattern::{Direction, Pattern, PatternBlock, RhythmicFigure, TriadId};
        use crate::theory::position::PositionSet;
        use crate::theory::scales::Scale;
        use crate::voicings::fretboard::Fretboard;

        let chart = Chart::parse("t", "G7").unwrap();
        let altered = Scale::ALL.iter().position(|s| s.name == "Altered").unwrap();
        let config = LineConfig {
            pattern: Pattern {
                name: "t",
                blocks: vec![PatternBlock {
                    count: 8,
                    direction: Direction::Asc,
                    triad: TriadId::T1,
                    ..Default::default()
                }],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::default(),
        };
        let events = generate_line(
            &chart,
            &[Some(altered)],
            &Fretboard::standard_tuning(),
            &PAIRS[0],
            &config,
        );

        assert!(!events.is_empty(), "the fixture must produce notes");
        for e in &events {
            // Every spelling must round-trip to the pitch class it came from.
            let natural = [0u8, 2, 4, 5, 7, 9, 11][e.step as usize];
            let spelled_pc = (natural as i16 + e.alter as i16).rem_euclid(12) as u8;
            assert_eq!(
                spelled_pc, e.pitch_class,
                "step {} alter {} does not spell pc {}",
                e.step, e.alter, e.pitch_class
            );
            // G7 + Altered never needs a double accidental.
            assert!(e.alter.abs() <= 1, "unexpected double accidental: {:?}", e);
        }

        // The third of G7 must read as B (step 6, natural), never as Cb.
        if let Some(third) = events.iter().find(|e| e.pitch_class == 11) {
            assert_eq!((third.step, third.alter), (6, 0), "third of G7 must be B");
        }
        // The #11 must read as C# (step 0, sharp), never as Db.
        if let Some(sharp11) = events.iter().find(|e| e.pitch_class == 1) {
            assert_eq!((sharp11.step, sharp11.alter), (0, 1), "#11 of G7 must be C#");
        }
    }
```

If `PatternBlock` does not implement `Default`, build the block with all its fields
explicitly instead of `..Default::default()` — read the struct in
`src/theory/line_pattern.rs` and fill each field.

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test --lib theory::line_engine::tests::events_carry_functional_spelling
```

Expected: FAIL to compile — `no field 'step' on type 'NoteEvent'`.

- [ ] **Step 3: Write the implementation**

In `src/theory/line_engine.rs`:

Add the import near the other `crate::theory::` imports:

```rust
use crate::theory::spelling::{self, Spelled};
```

Extend the struct:

```rust
#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub beat: f32,
    pub string: u8,
    pub fret: u8,
    pub triad: TriadId,
    pub pitch_class: u8,
    pub midi: i32,
    /// Sounding length in beats. Uniform (= the figure's grid) unless `hold_last` extends it.
    pub duration: f32,
    /// Notation spelling: 0=C … 6=B.
    pub step: u8,
    /// Notation spelling: -2=bb … +2=##.
    pub alter: i8,
    /// Notation spelling: sounding octave, middle C is C4.
    pub octave: i8,
}
```

In `generate_line`, build the per-chord spelling tables alongside the ladders and pass
them down. The scale is already resolved inside the existing closure — reuse it:

```rust
pub fn generate_line(
    chart: &Chart,
    scale_overrides: &[Option<usize>],
    fretboard: &Fretboard,
    pair: &TriadPairSet,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    // Per chord, build each triad's inversion ladder (the rungs a connector steps through)
    // and the spelling table notation reads the line through, then walk the pattern.
    let mut ladders: Vec<[TriadLadder; 2]> = Vec::with_capacity(chart.changes.len());
    let mut spellings: Vec<[Option<Spelled>; 12]> = Vec::with_capacity(chart.changes.len());
    for (i, change) in chart.changes.iter().enumerate() {
        let scale = scale_overrides
            .get(i)
            .and_then(|opt| opt.and_then(|idx| Scale::ALL.get(idx)))
            .unwrap_or_else(|| scale_defaults::default_scale(change.quality));
        let (pcs_a, pcs_b) = gmc::resolve_pair(change.root_pc, scale, pair);
        ladders.push([
            triad_ladder(fretboard, &config.positions, pcs_a),
            triad_ladder(fretboard, &config.positions, pcs_b),
        ]);
        spellings.push(spelling::spell_scale(&change.root, change.quality, scale));
    }
    run_pattern(chart, config, &ladders, &spellings)
}
```

Widen `run_pattern`'s signature:

```rust
fn run_pattern(
    chart: &Chart,
    config: &LineConfig,
    ladders: &[[TriadLadder; 2]],
    spellings: &[[Option<Spelled>; 12]],
) -> Vec<NoteEvent> {
```

And at the single `events.push` site, spell the note:

```rust
            let spelled = spelling::spell_midi(&spellings[active_chord], note.midi);
            events.push(NoteEvent {
                beat: beat_now,
                string: note.string,
                fret: note.fret,
                triad: block.triad,
                pitch_class: note.pitch_class,
                midi: note.midi,
                duration: step_slots as f32 * beat_dur,
                step: spelled.step,
                alter: spelled.alter,
                octave: spelled.octave,
            });
```

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test --lib
```

Expected: PASS. If other tests construct `NoteEvent` literals, they will fail to compile — add `step: 0, alter: 0, octave: 4` to each.

- [ ] **Step 5: Commit**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
git add src/theory/line_engine.rs
git commit -m "motor(linha): cada evento carrega sua grafia de partitura"
```

---

### Task 5: Transport the spelling to the web

Pure pass-through: serialise the three new fields and declare them in the TypeScript interface.

**Files:**
- Modify: `src/wasm_api.rs:635-653` (`line_events_json`)
- Modify: `web/src/lib/wasm.ts:162-170` (`GmcLineEvent`)

**Interfaces:**
- Consumes: `NoteEvent { step, alter, octave }` (Task 4)
- Produces: `GmcLineEvent` gains `step: number`, `alter: number`, `octave: number` — the shape every later web task reads.

- [ ] **Step 1: Add the fields to the JSON payload**

In `src/wasm_api.rs`, inside `line_events_json`'s `json!` block, after `"duration": e.duration,`:

```rust
                "step": e.step,
                "alter": e.alter,
                "octave": e.octave,
```

- [ ] **Step 2: Declare them in TypeScript**

In `web/src/lib/wasm.ts`, extend the interface:

```ts
export interface GmcLineEvent {
  beat: number;
  string: number;
  fret: number;
  triad: 'T1' | 'T2';
  pitchClass: number;
  midi: number;
  duration: number;
  /** Notation spelling: 0=C … 6=B. */
  step: number;
  /** Notation spelling: -2=𝄫 … +2=𝄪. */
  alter: number;
  /** Notation spelling: sounding octave, middle C is C4. */
  octave: number;
}
```

- [ ] **Step 3: Verify both sides compile**

```bash
cargo test --lib
cd web && npm run check
```

Expected: both PASS.

- [ ] **Step 4: Commit**

```bash
cargo fmt
git add src/wasm_api.rs web/src/lib/wasm.ts
git commit -m "wasm(gmc): grafia da nota atravessa para o front"
```

---

### Task 6: Extract the tab's geometry into a shared module

The staff must sit on exactly the same horizontal grid as the tab. Sharing one module is what guarantees they cannot drift apart. This task is a pure refactor — the page must look identical afterwards.

**Files:**
- Create: `web/src/lib/tabLayout.ts`
- Create: `web/src/lib/tabLayout.test.ts`
- Modify: `web/src/routes/gmc/tune/+page.svelte:394-430` (delete the constants and `tabX`/`tabY`, import them instead)

**Interfaces:**
- Consumes: `GmcLineEvent` (Task 5)
- Produces:
  - `TAB_STRING_GAP`, `TAB_MEASURE_WIDTH`, `TAB_MARGIN_LEFT`, `TAB_MARGIN_TOP`, `TAB_CHORD_Y`, `TAB_SCALE_Y_OFFSET`, `STRING_LABELS` — all `export const`
  - `export interface MeasureLike { index: number; startBeat: number; chord: { beats: number } }`
  - `export function tabX(event: { beat: number }, measure: MeasureLike): number`
  - `export function tabY(engineString: number): number`
  - `export function measureX(index: number): number`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/tabLayout.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { tabX, tabY, measureX, TAB_MEASURE_WIDTH, TAB_MARGIN_LEFT } from './tabLayout';

describe('tabLayout', () => {
  const measure = { index: 2, startBeat: 8, chord: { beats: 4 } };

  it('places measure 0 at the left margin', () => {
    expect(measureX(0)).toBe(TAB_MARGIN_LEFT);
  });

  it('spaces measures by one measure width', () => {
    expect(measureX(3) - measureX(2)).toBe(TAB_MEASURE_WIDTH);
  });

  it('places a note at the start of its measure with the leading pad only', () => {
    expect(tabX({ beat: 8 }, measure)).toBe(measureX(2) + 12);
  });

  it('places a mid-measure note proportionally to its beat', () => {
    const half = tabX({ beat: 10 }, measure);
    const start = tabX({ beat: 8 }, measure);
    expect(half - start).toBeCloseTo((TAB_MEASURE_WIDTH - 24) / 2);
  });

  it('maps engine string 0 (low E) to the bottom tab line', () => {
    expect(tabY(0)).toBeGreaterThan(tabY(5));
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- tabLayout
```

Expected: FAIL — cannot resolve `./tabLayout`.

- [ ] **Step 3: Create the module**

Create `web/src/lib/tabLayout.ts`:

```ts
/**
 * Geometry shared by the GMC tab and the notation staff drawn above it.
 *
 * The staff reuses `tabX` verbatim, which is what keeps a notehead vertically
 * aligned with its fret number. Keep every horizontal constant here — a copy
 * living in a component is a drift waiting to happen.
 */

export const TAB_STRING_GAP = 18;
export const TAB_MEASURE_WIDTH = 140;
export const TAB_MARGIN_LEFT = 10;
export const TAB_MARGIN_TOP = 28;
export const TAB_CHORD_Y = 12;
export const TAB_SCALE_Y_OFFSET = 16;
export const STRING_LABELS = ['e', 'B', 'G', 'D', 'A', 'E'];

/** Horizontal pad inside a measure, so notes never touch the barline. */
const MEASURE_PAD = 12;

export interface MeasureLike {
  index: number;
  startBeat: number;
  chord: { beats: number };
}

/** Left edge of a measure. */
export function measureX(index: number): number {
  return TAB_MARGIN_LEFT + index * TAB_MEASURE_WIDTH;
}

/** Horizontal position of an event within its measure. */
export function tabX(event: { beat: number }, measure: MeasureLike): number {
  const fraction = (event.beat - measure.startBeat) / measure.chord.beats;
  return measureX(measure.index) + MEASURE_PAD + fraction * (TAB_MEASURE_WIDTH - 2 * MEASURE_PAD);
}

/** Vertical position of a tab line. Engine string 0 is the low E; the tab draws it at the bottom. */
export function tabY(engineString: number): number {
  return TAB_MARGIN_TOP + (5 - engineString) * TAB_STRING_GAP;
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- tabLayout
```

Expected: PASS, 5 tests.

- [ ] **Step 5: Point the page at the shared module**

In `web/src/routes/gmc/tune/+page.svelte`, delete the local declarations of `TAB_STRING_GAP`, `TAB_MEASURE_WIDTH`, `TAB_MARGIN_LEFT`, `TAB_MARGIN_TOP`, `TAB_CHORD_Y`, `TAB_SCALE_Y_OFFSET`, `STRING_LABELS`, `tabY` and `tabX` (lines ~394–430), and add to the imports at the top of `<script>`:

```ts
  import {
    TAB_STRING_GAP,
    TAB_MEASURE_WIDTH,
    TAB_MARGIN_LEFT,
    TAB_MARGIN_TOP,
    TAB_CHORD_Y,
    TAB_SCALE_Y_OFFSET,
    STRING_LABELS,
    tabX,
    tabY,
    measureX,
  } from '$lib/tabLayout';
```

Leave every usage site unchanged — the names and signatures are identical.

- [ ] **Step 6: Verify the page still type-checks and the app is unchanged**

```bash
cd web && npm run check && npm run test
```

Expected: both PASS with no new errors.

- [ ] **Step 7: Commit**

```bash
git add web/src/lib/tabLayout.ts web/src/lib/tabLayout.test.ts web/src/routes/gmc/tune/+page.svelte
git commit -m "web(gmc): geometria da tab em modulo compartilhado"
```

---

### Task 7: Staff geometry — vertical position and ledger lines

Where a spelled pitch sits on a treble-8vb staff, and which ledger lines it needs.

**The 8vb convention.** Guitar sounds an octave below what is written, so written pitch = sounding pitch + 12. Staff steps count upward from the bottom line (written E4 = step 0); every even step is a line, every odd step a space, and the staff proper spans steps 0–8. The open low E sounds MIDI 40 and writes as E3 → step −7, three ledger lines below. Notating at sounding pitch would put it at step −14, seven ledger lines down — which is why the convention exists.

**Files:**
- Create: `web/src/lib/notation.ts`
- Create: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: nothing from earlier web tasks
- Produces:
  - `export interface Spelled { step: number; alter: number; octave: number }`
  - `export const STAFF_LINE_GAP = 7` — pixels between adjacent staff lines
  - `export function staffStep(p: Spelled): number`
  - `export function ledgerSteps(step: number): number[]`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/notation.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { staffStep, ledgerSteps } from './notation';

describe('staffStep', () => {
  it('puts written E4 on the bottom line', () => {
    // Sounding E3 (octave 3, step 2) writes as E4 under the 8vb clef.
    expect(staffStep({ step: 2, alter: 0, octave: 3 })).toBe(0);
  });

  it('puts written F5 on the top line', () => {
    // Sounding F4 writes as F5.
    expect(staffStep({ step: 3, alter: 0, octave: 4 })).toBe(8);
  });

  it('puts the guitar open low E three ledger lines below', () => {
    // Sounding E2 = MIDI 40.
    expect(staffStep({ step: 2, alter: 0, octave: 2 })).toBe(-7);
  });

  it('ignores the accidental — Eb and E share a staff position', () => {
    expect(staffStep({ step: 2, alter: -1, octave: 3 })).toBe(
      staffStep({ step: 2, alter: 0, octave: 3 }),
    );
  });
});

describe('ledgerSteps', () => {
  it('returns nothing for a note inside the staff', () => {
    expect(ledgerSteps(4)).toEqual([]);
    expect(ledgerSteps(0)).toEqual([]);
    expect(ledgerSteps(8)).toEqual([]);
  });

  it('returns the even steps down to a low note', () => {
    expect(ledgerSteps(-7)).toEqual([-2, -4, -6]);
  });

  it('returns the even steps up to a high note', () => {
    expect(ledgerSteps(13)).toEqual([10, 12]);
  });

  it('includes the note own line when it sits exactly on a ledger', () => {
    expect(ledgerSteps(-4)).toEqual([-2, -4]);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — cannot resolve `./notation`.

- [ ] **Step 3: Write the implementation**

Create `web/src/lib/notation.ts`:

```ts
/**
 * Pure layout for the GMC notation staff. No DOM, no Svelte — everything here is a
 * function of the engine's events, so it can be unit tested on its own.
 *
 * The staff is a treble clef with an 8 below: the guitar sounds an octave lower than
 * it is written. Staff steps count upward from the bottom line (written E4 = 0), so
 * even steps are lines and odd steps are spaces, and the staff proper spans 0..8.
 */

/** A pitch spelled for notation. Mirrors the Rust `Spelled` on `GmcLineEvent`. */
export interface Spelled {
  /** 0=C, 1=D, 2=E, 3=F, 4=G, 5=A, 6=B. */
  step: number;
  /** -2=𝄫, -1=♭, 0=♮, +1=♯, +2=𝄪. */
  alter: number;
  /** Sounding octave; middle C is C4. */
  octave: number;
}

/** Pixels between adjacent staff lines. One staff step is half of this. */
export const STAFF_LINE_GAP = 7;

/** The staff proper: step 0 is the bottom line, step 8 the top line. */
export const STAFF_TOP_STEP = 8;
export const STAFF_BOTTOM_STEP = 0;

/** Diatonic index of written E4, the bottom line: octave 4 × 7 + step 2. */
const BOTTOM_LINE_DIATONIC = 30;

/**
 * Staff steps above the bottom line. Positive is up. The accidental is irrelevant —
 * Eb and E occupy the same line.
 */
export function staffStep(p: Spelled): number {
  const writtenOctave = p.octave + 1; // treble 8vb: written = sounding + one octave
  return writtenOctave * 7 + p.step - BOTTOM_LINE_DIATONIC;
}

/** The ledger lines a note at `step` needs, ordered outward from the staff. */
export function ledgerSteps(step: number): number[] {
  const lines: number[] = [];
  if (step < STAFF_BOTTOM_STEP) {
    for (let s = -2; s >= step; s -= 2) lines.push(s);
  } else if (step > STAFF_TOP_STEP) {
    for (let s = 10; s <= step; s += 2) lines.push(s);
  }
  return lines;
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation
```

Expected: PASS, 8 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): posicao vertical na pauta e linhas suplementares"
```

---

### Task 8: Durations to figures, split and tied

Turn a sounding span into printable note values. A span that does not match one figure is split — at barlines first, then at beat boundaries — and the pieces are tied.

**Files:**
- Modify: `web/src/lib/notation.ts`
- Modify: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: nothing from Task 7
- Produces:
  - `export type GridKind = 'eighth' | 'sixteenth' | 'triplet'`
  - `export interface Grid { kind: GridKind; step: number; slotsPerBeat: number }`
  - `export const GRIDS: Record<GridKind, Grid>`
  - `export interface Figure { beat: number; beats: number; value: number; dots: 0 | 1; tiedToNext: boolean }`
  - `export function splitSpan(startBeat: number, durationBeats: number, grid: Grid, measureStarts: number[]): Figure[]`

- [ ] **Step 1: Write the failing test**

Append to `web/src/lib/notation.test.ts`:

```ts
import { splitSpan, GRIDS } from './notation';

describe('splitSpan', () => {
  const eighth = GRIDS.eighth;
  // A 4-bar chart of 4/4: measures start at 0, 4, 8, 12.
  const bars = [0, 4, 8, 12];

  it('emits one figure for a plain eighth', () => {
    expect(splitSpan(0, 0.5, eighth, bars)).toEqual([
      { beat: 0, beats: 0.5, value: 8, dots: 0, tiedToNext: false },
    ]);
  });

  it('emits one figure for a dotted quarter', () => {
    expect(splitSpan(0, 1.5, eighth, bars)).toEqual([
      { beat: 0, beats: 1.5, value: 4, dots: 1, tiedToNext: false },
    ]);
  });

  it('ties a half to an eighth for two and a half beats', () => {
    const figures = splitSpan(0, 2.5, eighth, bars);
    expect(figures.map((f) => [f.value, f.dots])).toEqual([
      [2, 0],
      [8, 0],
    ]);
    expect(figures[0].tiedToNext).toBe(true);
    expect(figures[1].tiedToNext).toBe(false);
  });

  it('splits at the barline and ties across it', () => {
    // Starts on beat 3 of bar 1, lasts two beats -> one beat in each bar.
    const figures = splitSpan(3, 2, eighth, bars);
    expect(figures).toHaveLength(2);
    expect(figures[0].beat).toBe(3);
    expect(figures[1].beat).toBe(4);
    expect(figures[0].tiedToNext).toBe(true);
    expect(figures[1].tiedToNext).toBe(false);
  });

  it('never returns an empty list for a positive duration', () => {
    for (const beats of [0.25, 0.5, 1, 1.25, 2, 3, 3.5, 4]) {
      expect(splitSpan(0, beats, GRIDS.sixteenth, bars).length).toBeGreaterThan(0);
    }
  });

  it('conserves total duration', () => {
    const total = (fs: { beats: number }[]) => fs.reduce((a, f) => a + f.beats, 0);
    expect(total(splitSpan(0, 2.5, eighth, bars))).toBeCloseTo(2.5);
    expect(total(splitSpan(3, 2, eighth, bars))).toBeCloseTo(2);
    expect(total(splitSpan(0, 1 / 3, GRIDS.triplet, bars))).toBeCloseTo(1 / 3);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — `splitSpan is not a function`.

- [ ] **Step 3: Write the implementation**

Append to `web/src/lib/notation.ts`:

```ts
export type GridKind = 'eighth' | 'sixteenth' | 'triplet';

export interface Grid {
  kind: GridKind;
  /** Beats in one grid slot. */
  step: number;
  /** Slots in one beat — also the size of a beam group. */
  slotsPerBeat: number;
}

export const GRIDS: Record<GridKind, Grid> = {
  eighth: { kind: 'eighth', step: 0.5, slotsPerBeat: 2 },
  sixteenth: { kind: 'sixteenth', step: 0.25, slotsPerBeat: 4 },
  triplet: { kind: 'triplet', step: 1 / 3, slotsPerBeat: 3 },
};

/** One printable note value. `value` is the denominator: 1=whole, 2=half, 4=quarter, 8, 16. */
export interface Figure {
  /** Absolute onset in beats. */
  beat: number;
  /** Sounding length in beats. */
  beats: number;
  value: number;
  dots: 0 | 1;
  tiedToNext: boolean;
}

/**
 * Slot counts that print as a single figure, largest first. Anything not listed is
 * split greedily into the largest entry that fits, and the pieces are tied.
 */
const FIGURE_TABLE: Record<GridKind, Array<[slots: number, value: number, dots: 0 | 1]>> = {
  eighth: [
    [8, 1, 0],
    [6, 2, 1],
    [4, 2, 0],
    [3, 4, 1],
    [2, 4, 0],
    [1, 8, 0],
  ],
  sixteenth: [
    [16, 1, 0],
    [12, 2, 1],
    [8, 2, 0],
    [6, 4, 1],
    [4, 4, 0],
    [3, 8, 1],
    [2, 8, 0],
    [1, 16, 0],
  ],
  // Inside a triplet bracket 1 slot is an eighth and 2 slots a quarter; 3 slots fills a
  // whole beat and prints as a plain quarter, with the bracket omitted by the beamer.
  triplet: [
    [12, 1, 0],
    [9, 2, 1],
    [6, 2, 0],
    [3, 4, 0],
    [2, 4, 0],
    [1, 8, 0],
  ],
};

/** Round to whole slots — `beat`/`duration` are f32 out of Rust and drift slightly. */
function toSlots(beats: number, grid: Grid): number {
  return Math.max(0, Math.round(beats / grid.step));
}

/**
 * Split a span into printable figures, breaking at barlines and then at beat
 * boundaries. All figures but the last are tied to their successor.
 */
export function splitSpan(
  startBeat: number,
  durationBeats: number,
  grid: Grid,
  measureStarts: number[],
): Figure[] {
  const pieces: Array<{ beat: number; slots: number }> = [];
  let beat = startBeat;
  let remaining = toSlots(durationBeats, grid);

  while (remaining > 0) {
    // Slots left before the next barline, and before the next beat.
    const nextBar = measureStarts.find((m) => m > beat + 1e-6);
    const toBar = nextBar === undefined ? remaining : toSlots(nextBar - beat, grid);
    const toBeat = grid.slotsPerBeat - (Math.round(beat / grid.step) % grid.slotsPerBeat || 0);
    let take = Math.min(remaining, toBar > 0 ? toBar : remaining);

    // Inside the bar, prefer a figure that does not straddle a beat unless it is a
    // clean value in its own right.
    if (take > toBeat && !FIGURE_TABLE[grid.kind].some(([s]) => s === take)) {
      take = Math.max(toBeat, 1);
    }
    const entry = FIGURE_TABLE[grid.kind].find(([s]) => s <= take);
    const slots = entry ? entry[0] : 1;
    pieces.push({ beat, slots });
    beat += slots * grid.step;
    remaining -= slots;
  }

  return pieces.map((p, i) => {
    const entry = FIGURE_TABLE[grid.kind].find(([s]) => s === p.slots) ?? [p.slots, 8, 0 as const];
    return {
      beat: p.beat,
      beats: p.slots * grid.step,
      value: entry[1],
      dots: entry[2] as 0 | 1,
      tiedToNext: i < pieces.length - 1,
    };
  });
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation
```

Expected: PASS, 14 tests. If the barline or beat-boundary arithmetic is off, fix
`splitSpan` — the tests pin the contract, not the implementation.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): duracoes viram figuras, com ligadura sobre a barra"
```

---

### Task 9: Rests from the gaps

Everything the events do not cover is silence. `lead_rest` and silent blocks are the only sources, but the renderer should not care where a gap came from.

**Files:**
- Modify: `web/src/lib/notation.ts`
- Modify: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: `Figure`, `Grid`, `splitSpan` (Task 8)
- Produces: `export function restFigures(events: Array<{ beat: number; duration: number }>, measureStart: number, measureBeats: number, grid: Grid, measureStarts: number[]): Figure[]`

- [ ] **Step 1: Write the failing test**

Append to `web/src/lib/notation.test.ts`:

```ts
import { restFigures } from './notation';

describe('restFigures', () => {
  const grid = GRIDS.eighth;
  const bars = [0, 4];

  it('returns nothing when the measure is full', () => {
    const events = [
      { beat: 0, duration: 2 },
      { beat: 2, duration: 2 },
    ];
    expect(restFigures(events, 0, 4, grid, bars)).toEqual([]);
  });

  it('fills a leading gap from a pickup rest', () => {
    // lead_rest of 2 eighth slots: the measure starts with one beat of silence.
    const events = [{ beat: 1, duration: 3 }];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests).toHaveLength(1);
    expect(rests[0].beat).toBe(0);
    expect(rests[0].beats).toBeCloseTo(1);
  });

  it('fills a trailing gap', () => {
    const events = [{ beat: 0, duration: 2 }];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(2);
  });

  it('fills a gap in the middle', () => {
    const events = [
      { beat: 0, duration: 1 },
      { beat: 3, duration: 1 },
    ];
    const rests = restFigures(events, 0, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(2);
    expect(rests[0].beat).toBeCloseTo(1);
  });

  it('fills an entirely empty measure', () => {
    const rests = restFigures([], 4, 4, grid, bars);
    expect(rests.reduce((a, r) => a + r.beats, 0)).toBeCloseTo(4);
    expect(rests[0].beat).toBe(4);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — `restFigures is not a function`.

- [ ] **Step 3: Write the implementation**

Append to `web/src/lib/notation.ts`:

```ts
/**
 * The rests a measure needs: every stretch of the measure no event covers.
 *
 * Events are assumed sorted by beat and non-overlapping, which is what the line
 * engine emits — a held note advances the cursor past the slots it occupies.
 */
export function restFigures(
  events: Array<{ beat: number; duration: number }>,
  measureStart: number,
  measureBeats: number,
  grid: Grid,
  measureStarts: number[],
): Figure[] {
  const end = measureStart + measureBeats;
  const rests: Figure[] = [];
  let cursor = measureStart;

  for (const e of events) {
    if (e.beat - cursor > 1e-6) {
      rests.push(...splitSpan(cursor, e.beat - cursor, grid, measureStarts));
    }
    cursor = Math.max(cursor, e.beat + e.duration);
  }
  if (end - cursor > 1e-6) {
    rests.push(...splitSpan(cursor, end - cursor, grid, measureStarts));
  }
  // Rests are never tied.
  return rests.map((r) => ({ ...r, tiedToNext: false }));
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation
```

Expected: PASS, 19 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): pausas preenchem os vaos do compasso"
```

---

### Task 10: Beam groups and stem direction

Group notes by beat so their flags become beams, and decide which way the stems point.

**Files:**
- Modify: `web/src/lib/notation.ts`
- Modify: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: `Grid`, `staffStep`, `Spelled` (Tasks 7–8)
- Produces:
  - `export interface BeamGroup<T> { notes: T[]; stemUp: boolean; bracket: boolean }`
  - `export function beamGroups<T extends { beat: number; beats: number; staffStep: number }>(notes: T[], grid: Grid): BeamGroup<T>[]`

- [ ] **Step 1: Write the failing test**

Append to `web/src/lib/notation.test.ts`:

```ts
import { beamGroups } from './notation';

describe('beamGroups', () => {
  const note = (beat: number, beats: number, staffStep: number) => ({ beat, beats, staffStep });

  it('beams two eighths per beat', () => {
    const groups = beamGroups(
      [note(0, 0.5, 0), note(0.5, 0.5, 1), note(1, 0.5, 2), note(1.5, 0.5, 3)],
      GRIDS.eighth,
    );
    expect(groups).toHaveLength(2);
    expect(groups[0].notes).toHaveLength(2);
    expect(groups[1].notes).toHaveLength(2);
  });

  it('beams four sixteenths per beat', () => {
    const notes = [0, 0.25, 0.5, 0.75].map((b) => note(b, 0.25, 0));
    const groups = beamGroups(notes, GRIDS.sixteenth);
    expect(groups).toHaveLength(1);
    expect(groups[0].notes).toHaveLength(4);
  });

  it('brackets a triplet group of three', () => {
    const notes = [0, 1 / 3, 2 / 3].map((b) => note(b, 1 / 3, 0));
    const groups = beamGroups(notes, GRIDS.triplet);
    expect(groups).toHaveLength(1);
    expect(groups[0].bracket).toBe(true);
  });

  it('does not bracket a note that fills a whole beat', () => {
    const groups = beamGroups([note(0, 1, 0)], GRIDS.triplet);
    expect(groups[0].bracket).toBe(false);
  });

  it('breaks the group at a note longer than one slot', () => {
    // A hold_last note occupying a whole beat cannot be beamed to its neighbour.
    const groups = beamGroups([note(0, 0.5, 0), note(0.5, 1.5, 0)], GRIDS.eighth);
    expect(groups).toHaveLength(2);
  });

  it('stems down when the group sits above the middle line', () => {
    // Middle line is staff step 4.
    const groups = beamGroups([note(0, 0.5, 6), note(0.5, 0.5, 7)], GRIDS.eighth);
    expect(groups[0].stemUp).toBe(false);
  });

  it('stems up when the group sits below the middle line', () => {
    const groups = beamGroups([note(0, 0.5, 1), note(0.5, 0.5, 0)], GRIDS.eighth);
    expect(groups[0].stemUp).toBe(true);
  });

  it('lets the note furthest from the middle line decide a mixed group', () => {
    // Step 0 is 4 away from the middle; step 5 is only 1 away. The low note wins.
    const groups = beamGroups([note(0, 0.5, 0), note(0.5, 0.5, 5)], GRIDS.eighth);
    expect(groups[0].stemUp).toBe(true);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — `beamGroups is not a function`.

- [ ] **Step 3: Write the implementation**

Append to `web/src/lib/notation.ts`:

```ts
/** The middle staff line (B4 written) — the pivot for stem direction. */
const MIDDLE_STEP = 4;

export interface BeamGroup<T> {
  notes: T[];
  stemUp: boolean;
  /** True for a triplet group, which prints a bracket and a 3. */
  bracket: boolean;
}

/**
 * Group notes into beams, one group per beat.
 *
 * A note longer than one grid slot cannot be beamed — a `hold_last` landing note
 * carries a flagless value and ends its group. Stem direction is decided per group by
 * the note furthest from the middle line, so the whole beam points the same way.
 */
export function beamGroups<T extends { beat: number; beats: number; staffStep: number }>(
  notes: T[],
  grid: Grid,
): BeamGroup<T>[] {
  const groups: BeamGroup<T>[] = [];
  let current: T[] = [];
  let currentBeat = -1;

  const flush = () => {
    if (current.length === 0) return;
    const furthest = current.reduce((best, n) =>
      Math.abs(n.staffStep - MIDDLE_STEP) > Math.abs(best.staffStep - MIDDLE_STEP) ? n : best,
    );
    groups.push({
      notes: current,
      stemUp: furthest.staffStep < MIDDLE_STEP,
      bracket: grid.kind === 'triplet' && current.length > 1,
    });
    current = [];
  };

  for (const n of notes) {
    const beatIndex = Math.floor(n.beat + 1e-6);
    const beamable = n.beats < 1 - 1e-6 && n.beats <= grid.step + 1e-6;
    if (beatIndex !== currentBeat || !beamable) {
      flush();
      currentBeat = beatIndex;
    }
    current.push(n);
    if (!beamable) flush();
  }
  flush();
  return groups;
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation
```

Expected: PASS, 27 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): grupos de barra por tempo e direcao da haste"
```

---

### Task 11: Which accidentals actually print

There is no key signature — the active scale changes every chord, so an armature would misrepresent the line. Every accidental is inline, with the standard within-measure suppression.

**Files:**
- Modify: `web/src/lib/notation.ts`
- Modify: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: `Spelled` (Task 7)
- Produces: `export function accidentalsToPrint(notes: Spelled[]): Array<number | null>` — one entry per input note, the alteration to print or `null` for none

- [ ] **Step 1: Write the failing test**

Append to `web/src/lib/notation.test.ts`:

```ts
import { accidentalsToPrint } from './notation';

describe('accidentalsToPrint', () => {
  const n = (step: number, alter: number, octave = 4) => ({ step, alter, octave });

  it('prints nothing for naturals that were never altered', () => {
    expect(accidentalsToPrint([n(0, 0), n(1, 0), n(2, 0)])).toEqual([null, null, null]);
  });

  it('prints an accidental the first time an altered note appears', () => {
    expect(accidentalsToPrint([n(6, -1)])).toEqual([-1]);
  });

  it('suppresses a repeat of the same altered note in the same measure', () => {
    expect(accidentalsToPrint([n(6, -1), n(6, -1)])).toEqual([-1, null]);
  });

  it('does not suppress across octaves', () => {
    expect(accidentalsToPrint([n(6, -1, 3), n(6, -1, 4)])).toEqual([-1, -1]);
  });

  it('prints a natural when an altered step returns to natural', () => {
    expect(accidentalsToPrint([n(6, -1), n(6, 0)])).toEqual([-1, 0]);
  });

  it('prints again when the alteration changes', () => {
    expect(accidentalsToPrint([n(5, -1), n(5, 1)])).toEqual([-1, 1]);
  });

  it('handles double accidentals like any other alteration', () => {
    expect(accidentalsToPrint([n(6, -2), n(6, -2)])).toEqual([-2, null]);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — `accidentalsToPrint is not a function`.

- [ ] **Step 3: Write the implementation**

Append to `web/src/lib/notation.ts`:

```ts
/**
 * Which accidental each note prints, for one measure's worth of notes in order.
 *
 * There is no key signature: the active scale changes per chord, so an armature would
 * misrepresent the line. Everything is inline, with the usual rules — state an
 * alteration once per (letter, octave) per measure, and print a natural when a
 * previously altered letter comes back.
 *
 * Call this once per measure; the state must not leak across barlines.
 */
export function accidentalsToPrint(notes: Spelled[]): Array<number | null> {
  const stated = new Map<string, number>();
  return notes.map((n) => {
    const key = `${n.step}:${n.octave}`;
    const previous = stated.get(key);
    if (previous === n.alter) return null;
    if (previous === undefined && n.alter === 0) return null;
    stated.set(key, n.alter);
    return n.alter;
  });
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation
```

Expected: PASS, 34 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): acidentes inline, sem armadura, com supressao por compasso"
```

---

### Task 12: Assemble a measure's layout

The single entry point the component calls. Everything above composes here.

**Files:**
- Modify: `web/src/lib/notation.ts`
- Modify: `web/src/lib/notation.test.ts`

**Interfaces:**
- Consumes: everything from Tasks 7–11; `tabX`, `MeasureLike` from `$lib/tabLayout` (Task 6); `GmcLineEvent` from `$lib/wasm` (Task 5)
- Produces:
  - `export interface LaidOutNote { x: number; staffStep: number; accidental: number | null; value: number; dots: 0 | 1; tiedToNext: boolean; triad: 'T1' | 'T2'; ledger: number[] }`
  - `export interface LaidOutRest { x: number; value: number; dots: 0 | 1 }`
  - `export interface MeasureLayout { notes: LaidOutNote[]; rests: LaidOutRest[]; beams: BeamGroup<LaidOutNote>[] }`
  - `export function layoutMeasure(measure: MeasureLike & { events: GmcLineEvent[] }, grid: Grid, measureStarts: number[]): MeasureLayout`

- [ ] **Step 1: Write the failing test**

Append to `web/src/lib/notation.test.ts`:

```ts
import { layoutMeasure } from './notation';
import { tabX } from './tabLayout';

describe('layoutMeasure', () => {
  const event = (beat: number, step: number, alter: number, octave: number) => ({
    beat,
    string: 0,
    fret: 5,
    triad: 'T1' as const,
    pitchClass: 0,
    midi: 60,
    duration: 0.5,
    step,
    alter,
    octave,
  });

  const measure = {
    index: 0,
    startBeat: 0,
    chord: { beats: 4 },
    events: [event(0, 6, -1, 3), event(0.5, 6, -1, 3), event(1, 2, 0, 3), event(1.5, 4, 0, 3)],
  };

  it('places each note at the same x the tab uses', () => {
    const layout = layoutMeasure(measure, GRIDS.eighth, [0, 4]);
    expect(layout.notes[0].x).toBeCloseTo(tabX({ beat: 0 }, measure));
    expect(layout.notes[2].x).toBeCloseTo(tabX({ beat: 1 }, measure));
  });

  it('prints the accidental once and suppresses the repeat', () => {
    const layout = layoutMeasure(measure, GRIDS.eighth, [0, 4]);
    expect(layout.notes[0].accidental).toBe(-1);
    expect(layout.notes[1].accidental).toBeNull();
  });

  it('beams the eighths two per beat', () => {
    const layout = layoutMeasure(measure, GRIDS.eighth, [0, 4]);
    expect(layout.beams).toHaveLength(2);
  });

  it('rests out the two beats the events do not cover', () => {
    const layout = layoutMeasure(measure, GRIDS.eighth, [0, 4]);
    expect(layout.rests.length).toBeGreaterThan(0);
  });

  it('carries the triad so the renderer can colour the notehead', () => {
    const layout = layoutMeasure(measure, GRIDS.eighth, [0, 4]);
    expect(layout.notes[0].triad).toBe('T1');
  });

  it('handles an empty measure without throwing', () => {
    const empty = { index: 1, startBeat: 4, chord: { beats: 4 }, events: [] };
    const layout = layoutMeasure(empty, GRIDS.eighth, [0, 4]);
    expect(layout.notes).toEqual([]);
    expect(layout.beams).toEqual([]);
    expect(layout.rests.length).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd web && npm run test -- notation
```

Expected: FAIL — `layoutMeasure is not a function`.

- [ ] **Step 3: Write the implementation**

Add the imports at the top of `web/src/lib/notation.ts`:

```ts
import { tabX, type MeasureLike } from './tabLayout';
import type { GmcLineEvent } from './wasm';
```

Append:

```ts
export interface LaidOutNote {
  x: number;
  staffStep: number;
  /** The alteration to print, or null when it is already in force. */
  accidental: number | null;
  value: number;
  dots: 0 | 1;
  tiedToNext: boolean;
  triad: 'T1' | 'T2';
  /** Ledger lines this note needs, in staff steps. */
  ledger: number[];
  /** Sounding length in beats — the beamer needs it. */
  beats: number;
  /** Absolute onset in beats — the beamer needs it. */
  beat: number;
}

export interface LaidOutRest {
  x: number;
  value: number;
  dots: 0 | 1;
}

export interface MeasureLayout {
  notes: LaidOutNote[];
  rests: LaidOutRest[];
  beams: BeamGroup<LaidOutNote>[];
}

/**
 * Everything the staff component needs to draw one measure.
 *
 * Horizontal positions come from `tabX`, the same function the tablature uses, which
 * is what keeps a notehead directly above its fret number.
 */
export function layoutMeasure(
  measure: MeasureLike & { events: GmcLineEvent[] },
  grid: Grid,
  measureStarts: number[],
): MeasureLayout {
  const events = [...measure.events].sort((a, b) => a.beat - b.beat);
  const accidentals = accidentalsToPrint(events);

  const notes: LaidOutNote[] = events.flatMap((e, i) => {
    const figures = splitSpan(e.beat, e.duration, grid, measureStarts);
    const step = staffStep(e);
    return figures.map((f, fi) => ({
      x: tabX({ beat: f.beat }, measure),
      staffStep: step,
      // Only the first figure of a tied chain carries the accidental.
      accidental: fi === 0 ? accidentals[i] : null,
      value: f.value,
      dots: f.dots,
      tiedToNext: f.tiedToNext,
      triad: e.triad,
      ledger: ledgerSteps(step),
      beats: f.beats,
      beat: f.beat,
    }));
  });

  const rests: LaidOutRest[] = restFigures(
    events,
    measure.startBeat,
    measure.chord.beats,
    grid,
    measureStarts,
  ).map((r) => ({
    x: tabX({ beat: r.beat }, measure),
    value: r.value,
    dots: r.dots,
  }));

  return { notes, rests, beams: beamGroups(notes, grid) };
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd web && npm run test -- notation && npm run check
```

Expected: PASS, 40 tests, and svelte-check clean.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/notation.ts web/src/lib/notation.test.ts
git commit -m "web(partitura): layout completo de um compasso"
```

---

### Task 13: The glyphs

SVG path data for the shapes geometry cannot produce. Everything else — noteheads, stems, beams, dots, ledger lines, ties — is drawn from primitives by the component.

All paths are authored in a coordinate system where **one staff step is 1 unit** and the origin is on the glyph's staff anchor, so the component scales by `STAFF_LINE_GAP / 2` and positions by staff step alone.

**Files:**
- Create: `web/src/lib/notationGlyphs.ts`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `export const TREBLE_CLEF_PATH: string`
  - `export const CLEF_OCTAVE_TEXT = '8'`
  - `export const REST_PATHS: Record<number, string>` — keyed by note value 1, 2, 4, 8, 16
  - `export const ACCIDENTAL_PATHS: Record<number, string>` — keyed by alteration −2, −1, 0, 1, 2

- [ ] **Step 1: Create the glyph module**

Create `web/src/lib/notationGlyphs.ts`. Author each path in the unit system described
above. Start from these shapes and refine visually in Task 15:

```ts
/**
 * SVG path data for the notation glyphs that geometry cannot produce.
 *
 * Coordinates are in staff steps: 1 unit = half the distance between two staff lines,
 * y grows downward, and the origin sits on each glyph's staff anchor — the G line for
 * the clef, the middle line for rests, the notehead's own line for accidentals. The
 * component scales by `STAFF_LINE_GAP / 2`, so nothing here needs pixel values.
 *
 * Noteheads, stems, beams, augmentation dots, ledger lines and ties are drawn from
 * primitives by `StaffNotation.svelte`, not from this file.
 */

/** Treble clef, anchored so y=0 is the G line (staff step 2). */
export const TREBLE_CLEF_PATH =
  'M0.9,6.2 C0.9,7.3 1.8,8.0 2.8,8.0 C4.0,8.0 4.8,7.1 4.8,5.9 ' +
  'C4.8,4.6 3.9,3.6 2.6,2.4 C1.5,1.4 0.6,0.5 0.6,-0.9 ' +
  'C0.6,-2.4 1.6,-3.6 2.6,-4.6 C3.4,-5.4 3.9,-6.2 3.9,-7.2 ' +
  'C3.9,-8.4 3.3,-9.2 2.7,-9.2 C2.0,-9.2 1.6,-8.3 1.6,-7.2 ' +
  'C1.6,-5.9 2.2,-4.6 2.9,-3.2 C3.9,-1.2 4.9,0.9 4.9,2.9 ' +
  'C4.9,4.9 3.7,6.3 2.2,6.3 C1.3,6.3 0.9,5.8 0.9,5.2';

/** The 8 under the clef: guitar sounds an octave below the written pitch. */
export const CLEF_OCTAVE_TEXT = '8';

/** Rests, anchored on the middle staff line, keyed by note value. */
export const REST_PATHS: Record<number, string> = {
  // Whole rest: a block hanging under the fourth line.
  1: 'M-1.0,-1.0 h2.0 v1.0 h-2.0 z',
  // Half rest: a block sitting on the middle line.
  2: 'M-1.0,0.0 h2.0 v1.0 h-2.0 z',
  // Quarter rest.
  4: 'M-0.4,-2.0 L0.6,-0.7 L-0.2,0.2 L0.8,1.6 L0.2,1.9 C-0.6,1.0 -1.0,0.4 -0.5,-0.2 L-1.0,-0.9 z',
  // Eighth rest: one hook.
  8: 'M0.6,-1.6 C0.6,-1.2 0.2,-0.9 -0.2,-1.0 L0.2,1.8 L-0.2,1.9 L-0.8,-1.2 C-0.4,-0.9 0.1,-1.1 0.2,-1.5 z',
  // Sixteenth rest: two hooks.
  16: 'M0.6,-2.4 C0.6,-2.0 0.2,-1.7 -0.2,-1.8 L0.2,1.8 L-0.2,1.9 L-1.0,-2.0 C-0.6,-1.7 -0.1,-1.9 0.0,-2.3 z ' +
    'M0.3,-0.6 C0.3,-0.2 -0.1,0.1 -0.5,0.0 L-0.7,-0.8 C-0.3,-0.5 0.0,-0.6 0.1,-0.9 z',
};

/** Accidentals, anchored on the notehead's own staff position, keyed by alteration. */
export const ACCIDENTAL_PATHS: Record<number, string> = {
  // Double flat.
  [-2]:
    'M-1.4,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z ' +
    'M0.2,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z',
  // Flat.
  [-1]: 'M-0.5,-2.6 v4.0 c0.4,-0.7 1.0,-1.0 1.4,-0.7 c0.4,0.3 0.2,1.0 -0.5,1.6 l-0.9,0.8 v-5.7 z',
  // Natural.
  0: 'M-0.5,-2.2 v3.6 l1.2,-0.3 v-3.6 z M-0.5,0.6 l1.2,-0.3 v1.0 l-1.2,0.3 z ' +
    'M-0.5,-1.4 l1.2,-0.3 v1.0 l-1.2,0.3 z',
  // Sharp.
  1: 'M-0.7,-1.6 l0.35,-0.1 v3.4 l-0.35,0.1 z M0.35,-1.9 l0.35,-0.1 v3.4 l-0.35,0.1 z ' +
    'M-0.9,-0.5 l1.8,-0.45 v0.6 l-1.8,0.45 z M-0.9,1.0 l1.8,-0.45 v0.6 l-1.8,0.45 z',
  // Double sharp.
  2: 'M-0.6,-0.6 h1.2 v1.2 h-1.2 z',
};
```

- [ ] **Step 2: Verify it type-checks**

```bash
cd web && npm run check
```

Expected: PASS. There is no unit test here — path data is verified visually in Task 15.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/notationGlyphs.ts
git commit -m "web(partitura): glifos svg de clave, pausas e acidentes"
```

---

### Task 14: The staff component

Renders an SVG `<g>` — not a standalone `<svg>` — so it can be mounted inside the tab's existing SVG and share its scroll container, highlight and click target.

**Files:**
- Create: `web/src/lib/components/StaffNotation.svelte`

**Interfaces:**
- Consumes: `layoutMeasure`, `staffStep`, `STAFF_LINE_GAP`, `GRIDS`, types (Task 12); glyphs (Task 13); `measureX`, `TAB_MEASURE_WIDTH`, `TAB_MARGIN_LEFT` (Task 6)
- Produces: a Svelte component with props
  `{ measures: Array<MeasureLike & { events: GmcLineEvent[] }>, grid: Grid, measureStarts: number[], top: number, t1Color: string, t2Color: string }`,
  plus `export const STAFF_BLOCK_HEIGHT = 96` from the same file for the page to size its SVG.

- [ ] **Step 1: Create the component**

Create `web/src/lib/components/StaffNotation.svelte`:

```svelte
<script lang="ts" module>
  /** Vertical space the staff block occupies, including ledger-line headroom. */
  export const STAFF_BLOCK_HEIGHT = 96;
</script>

<script lang="ts">
  import {
    layoutMeasure,
    STAFF_LINE_GAP,
    type Grid,
    type MeasureLayout,
  } from '$lib/notation';
  import { measureX, TAB_MEASURE_WIDTH, TAB_MARGIN_LEFT, type MeasureLike } from '$lib/tabLayout';
  import {
    TREBLE_CLEF_PATH,
    CLEF_OCTAVE_TEXT,
    REST_PATHS,
    ACCIDENTAL_PATHS,
  } from '$lib/notationGlyphs';
  import type { GmcLineEvent } from '$lib/wasm';

  interface Props {
    measures: Array<MeasureLike & { events: GmcLineEvent[] }>;
    grid: Grid;
    measureStarts: number[];
    /** Y of the staff's top line within the parent SVG. */
    top: number;
    t1Color: string;
    t2Color: string;
  }

  let { measures, grid, measureStarts, top, t1Color, t2Color }: Props = $props();

  /** Half a staff step in pixels — the unit the glyph paths are authored in. */
  const UNIT = STAFF_LINE_GAP / 2;
  /** Y of the bottom line. Staff step 0 is the bottom line, 8 the top. */
  let bottomY = $derived(top + 4 * STAFF_LINE_GAP);

  function y(step: number): number {
    return bottomY - step * UNIT;
  }

  let layouts = $derived(
    measures.map((m) => layoutMeasure(m, grid, measureStarts)) as MeasureLayout[],
  );

  let staffWidth = $derived(TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH);

  /** Stem length in pixels: a full octave of staff steps is the engraving default. */
  const STEM_LENGTH = 7 * UNIT;
</script>

<g class="staff">
  <!-- Staff lines -->
  {#each [0, 2, 4, 6, 8] as step}
    <line x1={0} y1={y(step)} x2={staffWidth} y2={y(step)} stroke="var(--border)" stroke-width="1" />
  {/each}

  <!-- Treble clef, 8vb -->
  <path
    d={TREBLE_CLEF_PATH}
    transform="translate({TAB_MARGIN_LEFT - 8}, {y(2)}) scale({UNIT})"
    fill="none"
    stroke="var(--text)"
    stroke-width={1.1 / UNIT}
    stroke-linecap="round"
  />
  <text
    x={TAB_MARGIN_LEFT - 4}
    y={y(-4)}
    text-anchor="middle"
    fill="var(--text)"
    font-size="8"
    font-family="var(--font)">{CLEF_OCTAVE_TEXT}</text
  >

  {#each layouts as layout, mi}
    <!-- Barline -->
    <line
      x1={measureX(mi)}
      y1={y(8)}
      x2={measureX(mi)}
      y2={y(0)}
      stroke="var(--text-disabled)"
      stroke-width="1"
    />

    <!-- Rests -->
    {#each layout.rests as rest}
      <path
        d={REST_PATHS[rest.value] ?? REST_PATHS[4]}
        transform="translate({rest.x}, {y(4)}) scale({UNIT})"
        fill="var(--text-disabled)"
      />
      {#if rest.dots === 1}
        <circle cx={rest.x + 1.6 * UNIT} cy={y(4) - UNIT} r={0.9} fill="var(--text-disabled)" />
      {/if}
    {/each}

    <!-- Ledger lines -->
    {#each layout.notes as note}
      {#each note.ledger as step}
        <line
          x1={note.x - 5}
          y1={y(step)}
          x2={note.x + 5}
          y2={y(step)}
          stroke="var(--border)"
          stroke-width="1"
        />
      {/each}
    {/each}

    <!-- Beams and stems -->
    {#each layout.beams as group}
      {@const up = group.stemUp}
      {@const first = group.notes[0]}
      {@const last = group.notes[group.notes.length - 1]}
      {@const tipY = up
        ? Math.min(...group.notes.map((n) => y(n.staffStep))) - STEM_LENGTH
        : Math.max(...group.notes.map((n) => y(n.staffStep))) + STEM_LENGTH}
      {#each group.notes as note}
        {#if note.value >= 2}
          <line
            x1={note.x + (up ? 4 : -4)}
            y1={y(note.staffStep)}
            x2={note.x + (up ? 4 : -4)}
            y2={tipY}
            stroke="var(--text)"
            stroke-width="1.1"
          />
        {/if}
      {/each}
      {#if group.notes.length > 1 && first.value >= 8}
        <line
          x1={first.x + (up ? 4 : -4)}
          y1={tipY}
          x2={last.x + (up ? 4 : -4)}
          y2={tipY}
          stroke="var(--text)"
          stroke-width="2.4"
        />
        {#if first.value >= 16}
          <line
            x1={first.x + (up ? 4 : -4)}
            y1={tipY + (up ? 4 : -4)}
            x2={last.x + (up ? 4 : -4)}
            y2={tipY + (up ? 4 : -4)}
            stroke="var(--text)"
            stroke-width="2.4"
          />
        {/if}
      {:else if first.value >= 8}
        <!-- Lone eighth or shorter: a flag, drawn as a short hook off the stem. -->
        <path
          d="M{first.x + (up ? 4 : -4)},{tipY} q4,2 3,6"
          fill="none"
          stroke="var(--text)"
          stroke-width="1.4"
        />
      {/if}
      {#if group.bracket}
        <text
          x={(first.x + last.x) / 2}
          y={tipY + (up ? -3 : 9)}
          text-anchor="middle"
          fill="var(--text-disabled)"
          font-size="8"
          font-family="var(--font)">3</text
        >
      {/if}
    {/each}

    <!-- Noteheads, accidentals, dots, ties -->
    {#each layout.notes as note, ni}
      {@const color = note.triad === 'T1' ? t1Color : t2Color}
      {#if note.accidental !== null}
        <path
          d={ACCIDENTAL_PATHS[note.accidental] ?? ACCIDENTAL_PATHS[0]}
          transform="translate({note.x - 8}, {y(note.staffStep)}) scale({UNIT})"
          fill={color}
        />
      {/if}
      <ellipse
        cx={note.x}
        cy={y(note.staffStep)}
        rx="4"
        ry="2.8"
        transform="rotate(-20 {note.x} {y(note.staffStep)})"
        fill={note.value <= 2 ? 'none' : color}
        stroke={color}
        stroke-width={note.value <= 2 ? 1.2 : 0}
      />
      {#if note.dots === 1}
        <circle cx={note.x + 7} cy={y(note.staffStep) - UNIT} r="1.1" fill={color} />
      {/if}
      {#if note.tiedToNext && layout.notes[ni + 1]}
        <path
          d="M{note.x + 5},{y(note.staffStep) + 5} Q{(note.x + layout.notes[ni + 1].x) / 2},{y(
            note.staffStep,
          ) + 10} {layout.notes[ni + 1].x - 5},{y(layout.notes[ni + 1].staffStep) + 5}"
          fill="none"
          stroke={color}
          stroke-width="1"
        />
      {/if}
    {/each}
  {/each}

  <!-- Final barline -->
  {#if measures.length > 0}
    <line
      x1={measureX(measures.length)}
      y1={y(8)}
      x2={measureX(measures.length)}
      y2={y(0)}
      stroke="var(--text-disabled)"
      stroke-width="2"
    />
  {/if}
</g>
```

- [ ] **Step 2: Verify it type-checks**

```bash
cd web && npm run check
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/components/StaffNotation.svelte
git commit -m "web(partitura): componente da pauta como grupo svg"
```

---

### Task 15: Mount the staff, rebuild the WASM, verify end to end

Wire the component into the page, make room for it, rebuild the bundle, and run every gate.

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte` — imports, `tabSvgHeight`, the selected-measure highlight rect (line ~667), the click target rect (line ~680), and the `<svg>` body

**Interfaces:**
- Consumes: `StaffNotation`, `STAFF_BLOCK_HEIGHT` (Task 14); `GRIDS` (Task 8)
- Produces: nothing new

- [ ] **Step 1: Import the component and the grid**

In `web/src/routes/gmc/tune/+page.svelte`, add to the `<script>` imports:

```ts
  import StaffNotation, { STAFF_BLOCK_HEIGHT } from '$lib/components/StaffNotation.svelte';
  import { GRIDS, type GridKind } from '$lib/notation';
```

- [ ] **Step 2: Derive the grid and the measure starts**

Add near the other `$derived` declarations. The page already tracks the selected
rhythmic figure by index (0=Eighth, 1=Sixteenth, 2=Triplet) — read the existing state
variable that feeds `generateGmcLine`'s `figureIndex` argument and map it:

```ts
  const GRID_KINDS: GridKind[] = ['eighth', 'sixteenth', 'triplet'];
  let staffGrid = $derived(GRIDS[GRID_KINDS[figure] ?? 'eighth']);
  let measureStarts = $derived(measures.map((m) => m.startBeat));
```

If the state variable is not named `figure`, use whatever name the page passes as
`figureIndex` to `generateGmcLine`.

- [ ] **Step 3: Make room in the SVG**

Shift the tab down by the staff block and grow the SVG. Replace the `tabSvgHeight`
derivation:

```ts
  let tabSvgHeight = $derived(
    STAFF_BLOCK_HEIGHT + TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET + 14,
  );
```

Wrap the entire existing tab content in a `<g>` that translates it down, so no other
coordinate in the page has to change. In the template, immediately inside `<svg …
class="tab-svg">`, add the staff and open the translate group:

```svelte
          <StaffNotation
            measures={measures}
            grid={staffGrid}
            measureStarts={measureStarts}
            top={12}
            t1Color={T1_COLOR}
            t2Color={T2_COLOR}
          />

          <g transform="translate(0, {STAFF_BLOCK_HEIGHT})">
```

and close it with `</g>` immediately before `</svg>`.

- [ ] **Step 4: Grow the highlight and click rects to span both systems**

The selected-measure highlight and the click target live inside the translated group,
so they must reach upward over the staff. Change the highlight rect (was at line ~667):

```svelte
              <rect
                x={mx}
                y={TAB_CHORD_Y - 4 - STAFF_BLOCK_HEIGHT}
                width={TAB_MEASURE_WIDTH}
                height={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET + 8 - TAB_CHORD_Y + 4 + STAFF_BLOCK_HEIGHT}
                fill="var(--primary-muted)"
                opacity="0.25"
                rx="3"
              />
```

and the click target (was at line ~680):

```svelte
            <rect
              x={mx}
              y={-STAFF_BLOCK_HEIGHT}
              width={TAB_MEASURE_WIDTH}
              height={tabSvgHeight}
              fill="transparent"
              style="cursor: pointer"
              role="button"
              tabindex="-1"
              onclick={() => selectedMeasure = mi}
            />
```

- [ ] **Step 5: Rebuild the WASM bundle**

The new `NoteEvent` fields do not reach the browser until the bundle is rebuilt.

```bash
wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
```

Expected: builds clean. If `wasm-pack` or `cargo` is not found, `source "$HOME/.cargo/env"` first.

- [ ] **Step 6: Run every gate**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cd web && npm run check && npm run test
```

Expected: all PASS.

- [ ] **Step 7: Look at it**

```bash
cd web && npm run dev
```

Open the GMC tune page, load a preset, and check by eye:

- Noteheads sit directly above their fret numbers, at every measure and every zoom.
- The clef reads as a treble clef. **If it looks amateurish, stop and report** — the
  spec's fallback is to bundle a subsetted Bravura (SIL OFL 1.1) with only clef, rests
  and accidentals, which is a design decision, not an implementation one.
- Accidentals do not collide with noteheads or ledger lines.
- Clicking a measure still selects it, and the highlight covers staff and tab together.
- Arrow-key navigation and scroll-to-measure still work.
- The fretboard panel below is still reachable without the layout breaking.

- [ ] **Step 8: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "web(gmc): partitura acima da tab no tune mode"
```

---

## Self-Review Notes

**Spec coverage.** Every section of the spec maps to a task: chord-anchored spelling → Tasks 1–3; `NoteEvent` fields and the `generate_line` change → Task 4; transport → Task 5; shared geometry → Task 6; staff position and ledger lines → Task 7; duration decomposition and ties → Task 8; rests → Task 9; beams, stems and triplet brackets → Task 10; inline accidentals with no key signature → Task 11; assembly → Task 12; glyphs → Task 13; the `<g>`-inside-`<svg>` component with T1/T2 colouring → Task 14; mounting, vertical space, WASM rebuild and the preserved interactions → Task 15.

**Known open risk.** The hand-authored treble clef path in Task 13 is the one deliverable that cannot be validated by a unit test. Task 15 Step 7 makes the visual check an explicit gate with a defined fallback rather than leaving it to taste.

**Deliberately out of scope**, per the spec: the native egui view, key signatures, MusicXML export, and notation anywhere other than GMC tune mode.
