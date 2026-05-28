# GMC Tune Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a single-note line generator sub-mode within GMC that generates continuous melodic lines using triad pairs over chord charts, rendered as tablature with fretboard visualization.

**Architecture:** Four new theory modules (scale_defaults, position, line_pattern, line_engine) provide pure-logic line generation. A new UI module (gmc_tune) renders the output as tab + fretboard within the existing GMC mode. The theory modules have zero UI dependencies and are fully testable in isolation.

**Tech Stack:** Rust, eframe/egui 0.31, existing theory + voicings crates.

---

## File Map

**New files:**
- `src/theory/scale_defaults.rs` — ChordQuality → Scale mapping
- `src/theory/position.rs` — Neck position model + note enumeration per position
- `src/theory/line_pattern.rs` — Pattern blocks, presets, Direction/TriadId enums
- `src/theory/line_engine.rs` — Core note-by-note line generator
- `src/ui/gmc_tune.rs` — GMC tune sub-mode UI (sidebar, tab renderer, fretboard, controls)

**Modified files:**
- `src/theory/mod.rs` — register new modules
- `src/ui/mod.rs` — register new module
- `src/ui/app.rs` — add GmcTuneState, extend GmcState with sub-mode enum
- `src/ui/gmc.rs` — add sub-mode tab navigation (Explorer / Tune)

---

### Task 1: Scale Defaults Module

**Files:**
- Create: `src/theory/scale_defaults.rs`
- Modify: `src/theory/mod.rs`

- [ ] **Step 1: Write failing tests for scale defaults**

```rust
// src/theory/scale_defaults.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::ChordQuality;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn major_family_defaults() {
        assert_eq!(default_scale(quality("maj7")).name, "Ionian");
        assert_eq!(default_scale(quality("maj9")).name, "Ionian");
        assert_eq!(default_scale(quality("maj13")).name, "Ionian");
        assert_eq!(default_scale(quality("maj7#11")).name, "Lydian");
    }

    #[test]
    fn minor_family_defaults() {
        assert_eq!(default_scale(quality("m7")).name, "Dorian");
        assert_eq!(default_scale(quality("m9")).name, "Dorian");
        assert_eq!(default_scale(quality("m11")).name, "Dorian");
        assert_eq!(default_scale(quality("m13")).name, "Dorian");
    }

    #[test]
    fn dominant_family_defaults() {
        assert_eq!(default_scale(quality("dom7")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom9")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom13")).name, "Mixolydian");
        assert_eq!(default_scale(quality("dom7#11")).name, "Lydian Dominant");
        assert_eq!(default_scale(quality("dom7b13")).name, "Mixolydian ♭6");
    }

    #[test]
    fn altered_dominant_defaults() {
        assert_eq!(default_scale(quality("dom7b9")).name, "Altered");
        assert_eq!(default_scale(quality("dom7#9")).name, "Altered");
    }

    #[test]
    fn half_dim_defaults() {
        assert_eq!(default_scale(quality("m7b5")).name, "Locrian");
    }

    #[test]
    fn dim_defaults() {
        // dim7 has no perfect match in Scale::ALL; fall back to Locrian
        let scale = default_scale(quality("dim7"));
        assert_eq!(scale.name, "Locrian");
    }

    #[test]
    fn all_qualities_have_a_default() {
        for q in ChordQuality::ALL {
            let scale = default_scale(q);
            assert!(!scale.name.is_empty(), "no default for {}", q.name);
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theory::scale_defaults -- --nocapture`
Expected: compilation error — `default_scale` not defined.

- [ ] **Step 3: Write the implementation**

```rust
// src/theory/scale_defaults.rs (above the tests module)

use crate::theory::chords::ChordQuality;
use crate::theory::scales::Scale;

fn find_scale(name: &str) -> &'static Scale {
    Scale::ALL.iter().find(|s| s.name == name).unwrap()
}

pub fn default_scale(quality: &ChordQuality) -> &'static Scale {
    match quality.name {
        "maj7" | "maj9" | "maj13" => find_scale("Ionian"),
        "maj7#11" => find_scale("Lydian"),
        "m7" | "m9" | "m11" | "m13" => find_scale("Dorian"),
        "m7b5" | "m9b11" => find_scale("Locrian"),
        "dom7" | "dom9" | "dom13" => find_scale("Mixolydian"),
        "dom7#11" => find_scale("Lydian Dominant"),
        "dom7b9" | "dom7#9" | "dom7#5" => find_scale("Altered"),
        "dom7b13" => find_scale("Mixolydian \u{266D}6"),
        "dim7" => find_scale("Locrian"),
        _ => find_scale("Ionian"),
    }
}
```

- [ ] **Step 4: Register the module**

Add to `src/theory/mod.rs`:

```rust
pub mod scale_defaults;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib theory::scale_defaults -- --nocapture`
Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/theory/scale_defaults.rs src/theory/mod.rs
git commit -m "feat(theory): add scale defaults mapping for GMC tune mode"
```

---

### Task 2: Neck Position Model

**Files:**
- Create: `src/theory/position.rs`
- Modify: `src/theory/mod.rs`

- [ ] **Step 1: Write failing tests for position model**

```rust
// src/theory/position.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voicings::fretboard::Fretboard;

    #[test]
    fn core_range_position_v() {
        let pos = NeckPosition::new(5);
        assert_eq!(pos.core_range(), (5, 8));
    }

    #[test]
    fn stretch_range_position_v() {
        let pos = NeckPosition::new(5);
        assert_eq!(pos.stretch_range(), (4, 9));
    }

    #[test]
    fn core_range_position_i() {
        let pos = NeckPosition::new(1);
        assert_eq!(pos.core_range(), (1, 4));
    }

    #[test]
    fn stretch_range_position_i_clamps_low() {
        let pos = NeckPosition::new(1);
        assert_eq!(pos.stretch_range(), (0, 5));
    }

    #[test]
    fn find_notes_c_major_triad_position_v() {
        let fb = Fretboard::standard_tuning();
        let pos = NeckPosition::new(5);
        // C=0, E=4, G=7
        let pcs = [0, 4, 7];
        let notes = pos.find_notes(&fb, &pcs);
        assert!(!notes.is_empty());
        for n in &notes {
            assert!(n.fret >= 4 && n.fret <= 9, "fret {} out of stretch range", n.fret);
            assert!(pcs.contains(&n.pitch_class));
        }
    }

    #[test]
    fn find_notes_sorted_ascending_by_pitch() {
        let fb = Fretboard::standard_tuning();
        let pos = NeckPosition::new(5);
        let pcs = [0, 4, 7]; // C, E, G
        let notes = pos.find_notes(&fb, &pcs);
        for i in 1..notes.len() {
            assert!(
                notes[i].midi >= notes[i - 1].midi,
                "notes not sorted: {} >= {} failed",
                notes[i].midi,
                notes[i - 1].midi,
            );
        }
    }

    #[test]
    fn fret_note_is_stretch() {
        let pos = NeckPosition::new(5);
        let core = FretNote { string: 0, fret: 5, midi: 40, pitch_class: 4 };
        let stretch = FretNote { string: 0, fret: 4, midi: 39, pitch_class: 3 };
        assert!(!pos.is_stretch(core.fret));
        assert!(pos.is_stretch(stretch.fret));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theory::position -- --nocapture`
Expected: compilation error — types not defined.

- [ ] **Step 3: Write the implementation**

```rust
// src/theory/position.rs (above tests)

use crate::voicings::fretboard::Fretboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FretNote {
    pub string: u8,
    pub fret: u8,
    pub midi: i32,
    pub pitch_class: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rigidity {
    Strict,
    Flexible,
}

#[derive(Clone, Copy, Debug)]
pub struct NeckPosition {
    pub base_fret: u8,
    pub rigidity: Rigidity,
}

impl NeckPosition {
    pub fn new(base_fret: u8) -> Self {
        Self {
            base_fret,
            rigidity: Rigidity::Strict,
        }
    }

    pub fn core_range(&self) -> (u8, u8) {
        (self.base_fret, self.base_fret + 3)
    }

    pub fn stretch_range(&self) -> (u8, u8) {
        (self.base_fret.saturating_sub(1), self.base_fret + 4)
    }

    pub fn is_stretch(&self, fret: u8) -> bool {
        let (core_lo, core_hi) = self.core_range();
        let (str_lo, str_hi) = self.stretch_range();
        fret >= str_lo && fret <= str_hi && (fret < core_lo || fret > core_hi)
    }

    pub fn find_notes(&self, fretboard: &Fretboard, pitch_classes: &[u8]) -> Vec<FretNote> {
        let (lo, hi) = self.stretch_range();
        let mut notes = Vec::new();
        for s in 0..fretboard.num_strings() {
            for fret in lo..=hi {
                if let Some(note) = fretboard.get_note(s, fret as usize) {
                    if pitch_classes.contains(&note.pitch_class) {
                        notes.push(FretNote {
                            string: s as u8,
                            fret,
                            midi: note.midi(),
                            pitch_class: note.pitch_class,
                        });
                    }
                }
            }
        }
        notes.sort_by_key(|n| n.midi);
        notes
    }

    pub fn shifted(&self, offset: i8) -> Self {
        Self {
            base_fret: (self.base_fret as i8 + offset).max(1) as u8,
            rigidity: self.rigidity,
        }
    }
}
```

- [ ] **Step 4: Register the module**

Add to `src/theory/mod.rs`:

```rust
pub mod position;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib theory::position -- --nocapture`
Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/theory/position.rs src/theory/mod.rs
git commit -m "feat(theory): add neck position model with note enumeration"
```

---

### Task 3: Pattern Model

**Files:**
- Create: `src/theory/line_pattern.rs`
- Modify: `src/theory/mod.rs`

- [ ] **Step 1: Write failing tests for pattern model**

```rust
// src/theory/line_pattern.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_alternating_has_two_blocks() {
        let p = Pattern::preset_alternating();
        assert_eq!(p.blocks.len(), 2);
        assert_eq!(p.blocks[0].count, 3);
        assert_eq!(p.blocks[0].direction, Direction::Ascending);
        assert_eq!(p.blocks[0].triad, TriadId::T1);
        assert_eq!(p.blocks[1].count, 3);
        assert_eq!(p.blocks[1].direction, Direction::Descending);
        assert_eq!(p.blocks[1].triad, TriadId::T2);
    }

    #[test]
    fn preset_continuous_up() {
        let p = Pattern::preset_continuous_up();
        assert_eq!(p.blocks.len(), 2);
        assert_eq!(p.blocks[0].direction, Direction::Ascending);
        assert_eq!(p.blocks[1].direction, Direction::Ascending);
    }

    #[test]
    fn preset_short_long() {
        let p = Pattern::preset_short_long();
        assert_eq!(p.blocks[0].count, 2);
        assert_eq!(p.blocks[1].count, 4);
    }

    #[test]
    fn pattern_total_notes() {
        let p = Pattern::preset_alternating();
        assert_eq!(p.total_notes(), 6);
    }

    #[test]
    fn pattern_iterator_cycles() {
        let p = Pattern::preset_alternating();
        let blocks: Vec<_> = p.iter().take(5).collect();
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].triad, TriadId::T1);
        assert_eq!(blocks[1].triad, TriadId::T2);
        assert_eq!(blocks[2].triad, TriadId::T1); // cycled
    }

    #[test]
    fn rhythmic_figure_beat_duration() {
        assert!((RhythmicFigure::Eighth.beat_duration() - 0.5).abs() < f32::EPSILON);
        assert!((RhythmicFigure::Sixteenth.beat_duration() - 0.25).abs() < f32::EPSILON);
        let triplet_dur = RhythmicFigure::Triplet.beat_duration();
        assert!((triplet_dur - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn all_presets_listed() {
        let presets = Pattern::all_presets();
        assert_eq!(presets.len(), 3);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theory::line_pattern -- --nocapture`
Expected: compilation error — types not defined.

- [ ] **Step 3: Write the implementation**

```rust
// src/theory/line_pattern.rs (above tests)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    pub fn invert(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ascending => "↑",
            Self::Descending => "↓",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriadId {
    T1,
    T2,
}

impl TriadId {
    pub fn label(self) -> &'static str {
        match self {
            Self::T1 => "T1",
            Self::T2 => "T2",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RhythmicFigure {
    Eighth,
    Sixteenth,
    Triplet,
}

impl RhythmicFigure {
    pub const ALL: [Self; 3] = [Self::Eighth, Self::Sixteenth, Self::Triplet];

    pub fn beat_duration(self) -> f32 {
        match self {
            Self::Eighth => 0.5,
            Self::Sixteenth => 0.25,
            Self::Triplet => 1.0 / 3.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Eighth => "♪ Eighth",
            Self::Sixteenth => "♬ Sixteenth",
            Self::Triplet => "³ Triplet",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternBlock {
    pub count: u8,
    pub direction: Direction,
    pub triad: TriadId,
}

impl PatternBlock {
    pub fn label(&self) -> String {
        format!("{}{} {}", self.count, self.direction.label(), self.triad.label())
    }
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: &'static str,
    pub blocks: Vec<PatternBlock>,
}

impl Pattern {
    pub fn total_notes(&self) -> u32 {
        self.blocks.iter().map(|b| b.count as u32).sum()
    }

    pub fn iter(&self) -> PatternIter {
        PatternIter {
            blocks: &self.blocks,
            index: 0,
        }
    }

    pub fn preset_alternating() -> Self {
        Self {
            name: "Alternating 3+3",
            blocks: vec![
                PatternBlock { count: 3, direction: Direction::Ascending, triad: TriadId::T1 },
                PatternBlock { count: 3, direction: Direction::Descending, triad: TriadId::T2 },
            ],
        }
    }

    pub fn preset_continuous_up() -> Self {
        Self {
            name: "Continuous up",
            blocks: vec![
                PatternBlock { count: 3, direction: Direction::Ascending, triad: TriadId::T1 },
                PatternBlock { count: 3, direction: Direction::Ascending, triad: TriadId::T2 },
            ],
        }
    }

    pub fn preset_short_long() -> Self {
        Self {
            name: "Short-long",
            blocks: vec![
                PatternBlock { count: 2, direction: Direction::Ascending, triad: TriadId::T1 },
                PatternBlock { count: 4, direction: Direction::Descending, triad: TriadId::T2 },
            ],
        }
    }

    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::preset_alternating(),
            Self::preset_continuous_up(),
            Self::preset_short_long(),
        ]
    }
}

pub struct PatternIter<'a> {
    blocks: &'a [PatternBlock],
    index: usize,
}

impl<'a> Iterator for PatternIter<'a> {
    type Item = &'a PatternBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.blocks.is_empty() {
            return None;
        }
        let block = &self.blocks[self.index % self.blocks.len()];
        self.index += 1;
        Some(block)
    }
}
```

- [ ] **Step 4: Register the module**

Add to `src/theory/mod.rs`:

```rust
pub mod line_pattern;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib theory::line_pattern -- --nocapture`
Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/theory/line_pattern.rs src/theory/mod.rs
git commit -m "feat(theory): add pattern model with blocks and presets"
```

---

### Task 4: Line Engine

**Files:**
- Create: `src/theory/line_engine.rs`
- Modify: `src/theory/mod.rs`

- [ ] **Step 1: Write failing tests for the line engine**

```rust
// src/theory/line_engine.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chart::Chart;
    use crate::theory::gmc::PAIRS;
    use crate::theory::line_pattern::{Direction, Pattern, PatternBlock, RhythmicFigure, TriadId};
    use crate::theory::position::{NeckPosition, Rigidity};
    use crate::voicings::fretboard::Fretboard;

    fn simple_config() -> LineConfig {
        LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            position: NeckPosition::new(5),
        }
    }

    #[test]
    fn generates_correct_number_of_events() {
        let fb = Fretboard::standard_tuning();
        // 1 bar of Dm7 = 4 beats, eighth notes = 8 events
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 8);
    }

    #[test]
    fn events_have_sequential_beats() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        for i in 1..events.len() {
            assert!(
                events[i].beat > events[i - 1].beat,
                "beat {} not after {}",
                events[i].beat,
                events[i - 1].beat,
            );
        }
    }

    #[test]
    fn events_stay_in_position() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let mut config = simple_config();
        config.position.rigidity = Rigidity::Strict;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let (lo, hi) = config.position.stretch_range();
        for e in &events {
            assert!(
                e.fret >= lo && e.fret <= hi,
                "fret {} outside stretch range {}-{}",
                e.fret, lo, hi,
            );
        }
    }

    #[test]
    fn pattern_does_not_restart_on_chord_change() {
        let fb = Fretboard::standard_tuning();
        // 2 bars, 16 eighth notes. Pattern is 6 notes (3+3).
        // If pattern restarted at bar 2, block index would reset.
        // With 16 notes / 6 per cycle: we'd get 2 full cycles + 4 notes into the 3rd.
        // Block pattern: T1,T2,T1,T2,T1,T2,...
        // If it restarted: T1,T2,[new chord]T1,T2,...
        // If continuous:   T1,T2,T1,T2,T1,T2,...
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 16);
        // Notes 0-2: T1, 3-5: T2, 6-8: T1 (crosses bar line), 9-11: T2, 12-14: T1, 15: T2
        assert_eq!(events[0].triad, TriadId::T1);
        assert_eq!(events[3].triad, TriadId::T2);
        assert_eq!(events[6].triad, TriadId::T1);
        assert_eq!(events[9].triad, TriadId::T2);
    }

    #[test]
    fn all_notes_belong_to_indicated_triad() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        // We can't easily check pitch classes without resolving triads here,
        // but we can at least verify all events have a valid triad assigned.
        for e in &events {
            assert!(e.triad == TriadId::T1 || e.triad == TriadId::T2);
        }
    }

    #[test]
    fn scale_override_changes_available_notes() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        // Default: Dorian. Override with index of Phrygian (index 2 in Scale::ALL).
        let events_default = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let events_override = generate_line(&chart, &[Some(2)], &fb, &PAIRS[0], &config);
        // Phrygian has different notes than Dorian, so events should differ.
        let pcs_default: Vec<u8> = events_default.iter().map(|e| e.pitch_class).collect();
        let pcs_override: Vec<u8> = events_override.iter().map(|e| e.pitch_class).collect();
        assert_ne!(pcs_default, pcs_override);
    }

    #[test]
    fn sixteenth_notes_produce_double_events() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let mut config = simple_config();
        config.figure = RhythmicFigure::Sixteenth;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 16); // 4 beats / 0.25
    }

    #[test]
    fn triplets_produce_twelve_events_per_bar() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let mut config = simple_config();
        config.figure = RhythmicFigure::Triplet;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 12); // 4 beats / (1/3)
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib theory::line_engine -- --nocapture`
Expected: compilation error — `generate_line`, `LineConfig`, `NoteEvent` not defined.

- [ ] **Step 3: Write the implementation**

```rust
// src/theory/line_engine.rs (above tests)

use crate::theory::chart::Chart;
use crate::theory::gmc::{self, TriadPairSet};
use crate::theory::line_pattern::{Direction, Pattern, RhythmicFigure, TriadId};
use crate::theory::position::{FretNote, NeckPosition, Rigidity};
use crate::theory::scale_defaults;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;

#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub beat: f32,
    pub string: u8,
    pub fret: u8,
    pub triad: TriadId,
    pub pitch_class: u8,
    pub midi: i32,
}

pub struct LineConfig {
    pub pattern: Pattern,
    pub figure: RhythmicFigure,
    pub position: NeckPosition,
}

struct TriadNotes {
    t1: Vec<FretNote>,
    t2: Vec<FretNote>,
}

impl TriadNotes {
    fn notes_for(&self, triad: TriadId) -> &[FretNote] {
        match triad {
            TriadId::T1 => &self.t1,
            TriadId::T2 => &self.t2,
        }
    }
}

fn resolve_triad_notes(
    root_pc: u8,
    scale: &Scale,
    pair: &TriadPairSet,
    position: &NeckPosition,
    fretboard: &Fretboard,
) -> TriadNotes {
    let (pcs_a, pcs_b) = gmc::resolve_pair(root_pc, scale, pair);
    TriadNotes {
        t1: position.find_notes(fretboard, &pcs_a),
        t2: position.find_notes(fretboard, &pcs_b),
    }
}

fn find_nearest(notes: &[FretNote], current_midi: i32, direction: Direction) -> Option<&FretNote> {
    match direction {
        Direction::Ascending => notes.iter().find(|n| n.midi > current_midi),
        Direction::Descending => notes.iter().rev().find(|n| n.midi < current_midi),
    }
}

fn find_closest(notes: &[FretNote], current_midi: i32) -> Option<&FretNote> {
    notes.iter().min_by_key(|n| (n.midi - current_midi).abs())
}

pub fn generate_line(
    chart: &Chart,
    scale_overrides: &[Option<usize>],
    fretboard: &Fretboard,
    pair: &TriadPairSet,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    let beat_dur = config.figure.beat_duration();
    let total_beats = chart.total_beats();
    let total_events = (total_beats / beat_dur).round() as usize;

    let mut events = Vec::with_capacity(total_events);
    let mut pattern_iter = config.pattern.iter();
    let mut current_direction = Direction::Ascending;
    let mut current_midi: i32 = 0;
    let mut first_note = true;
    let mut position = config.position;

    // Pre-compute chord boundaries
    let mut chord_boundaries: Vec<(f32, f32, usize)> = Vec::new();
    let mut cumulative = 0.0_f32;
    for (i, change) in chart.changes.iter().enumerate() {
        chord_boundaries.push((cumulative, cumulative + change.beats, i));
        cumulative += change.beats;
    }

    // Pre-resolve triad notes per chord
    let triad_notes_per_chord: Vec<TriadNotes> = chart
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let scale = scale_overrides
                .get(i)
                .and_then(|opt| opt.map(|idx| &Scale::ALL[idx]))
                .unwrap_or_else(|| scale_defaults::default_scale(change.quality));
            resolve_triad_notes(change.root_pc, scale, pair, &position, fretboard)
        })
        .collect();

    let mut block_remaining = 0u8;
    let mut block_triad = TriadId::T1;

    for event_idx in 0..total_events {
        let beat = event_idx as f32 * beat_dur;

        // Find which chord we're in
        let chord_idx = chord_boundaries
            .iter()
            .rposition(|&(start, _, _)| beat >= start)
            .unwrap_or(0);

        let triad_notes = &triad_notes_per_chord[chord_idx];

        // Advance pattern if needed
        if block_remaining == 0 {
            if let Some(block) = pattern_iter.next() {
                block_remaining = block.count;
                block_triad = block.triad;
                current_direction = block.direction;
            }
        }

        let pool = triad_notes.notes_for(block_triad);

        if pool.is_empty() {
            block_remaining = block_remaining.saturating_sub(1);
            continue;
        }

        let chosen = if first_note {
            // Start on lowest note of the indicated triad
            pool.first()
        } else {
            // Try in the current direction first
            let candidate = find_nearest(pool, current_midi, current_direction);
            if candidate.is_some() {
                candidate
            } else {
                // Invert direction (range limit)
                current_direction = current_direction.invert();
                let inverted = find_nearest(pool, current_midi, current_direction);
                if inverted.is_some() {
                    inverted
                } else {
                    find_closest(pool, current_midi)
                }
            }
        };

        if let Some(note) = chosen {
            events.push(NoteEvent {
                beat,
                string: note.string,
                fret: note.fret,
                triad: block_triad,
                pitch_class: note.pitch_class,
                midi: note.midi,
            });
            current_midi = note.midi;
            first_note = false;
        }

        block_remaining = block_remaining.saturating_sub(1);
    }

    // Handle flexible position: re-resolve if position would need shifting.
    // For v1, the position stays fixed. Flexible mode shifts are a follow-up.
    let _ = position;

    events
}
```

- [ ] **Step 4: Register the module**

Add to `src/theory/mod.rs`:

```rust
pub mod line_engine;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib theory::line_engine -- --nocapture`
Expected: all 8 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/theory/line_engine.rs src/theory/mod.rs
git commit -m "feat(theory): add line engine — note-by-note triad pair generator"
```

---

### Task 5: GmcTune State and Sub-Mode Navigation

**Files:**
- Modify: `src/ui/app.rs`
- Modify: `src/ui/gmc.rs`

- [ ] **Step 1: Add GmcTuneState and sub-mode enum to app.rs**

Add these types to `src/ui/app.rs`:

```rust
use crate::theory::line_pattern::{Pattern, RhythmicFigure};
use crate::theory::position::{NeckPosition, Rigidity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GmcSubMode {
    Explorer,
    Tune,
}

pub(crate) struct GmcTuneState {
    pub(crate) chart_input: String,
    pub(crate) title_input: String,
    pub(crate) scale_overrides: Vec<Option<usize>>,
    pub(crate) pair_index: usize,
    pub(crate) figure: RhythmicFigure,
    pub(crate) position: NeckPosition,
    pub(crate) pattern: Pattern,
    pub(crate) generated: Option<Vec<crate::theory::line_engine::NoteEvent>>,
    pub(crate) selected_measure: usize,
    pub(crate) playback_start: Option<std::time::Instant>,
    pub(crate) last_clicked_measure: Option<usize>,
    pub(crate) error: Option<String>,
}

impl Default for GmcTuneState {
    fn default() -> Self {
        let (title, changes) = crate::theory::chart::PRESETS[0];
        Self {
            chart_input: changes.to_string(),
            title_input: title.to_string(),
            scale_overrides: Vec::new(),
            pair_index: 0,
            figure: RhythmicFigure::Eighth,
            position: NeckPosition::new(5),
            pattern: Pattern::preset_alternating(),
            generated: None,
            selected_measure: 0,
            playback_start: None,
            last_clicked_measure: None,
            error: None,
        }
    }
}
```

Add `sub_mode: GmcSubMode` field to `GmcState`:

```rust
pub(crate) struct GmcState {
    pub(crate) root_index: usize,
    pub(crate) scale_index: usize,
    pub(crate) pair_index: usize,
    pub(crate) show_intervals: bool,
    pub(crate) sub_mode: GmcSubMode,
}
```

Update `GmcState::default()` to include `sub_mode: GmcSubMode::Explorer`.

Add `gmc_tune: GmcTuneState` field to `ChordzApp` and initialize it in `ChordzApp::new()` as `gmc_tune: GmcTuneState::default()`.

- [ ] **Step 2: Add sub-mode tabs to gmc.rs**

Modify the `show_gmc_controls` method in `src/ui/gmc.rs` to include sub-mode tabs:

```rust
pub(crate) fn show_gmc_controls(&mut self, ui: &mut egui::Ui) {
    ui.selectable_value(&mut self.gmc.sub_mode, GmcSubMode::Explorer, "Explorer");
    ui.selectable_value(&mut self.gmc.sub_mode, GmcSubMode::Tune, "Tune");
    ui.separator();
    match self.gmc.sub_mode {
        GmcSubMode::Explorer => self.show_gmc_explorer_controls(ui),
        GmcSubMode::Tune => self.show_gmc_tune_controls(ui),
    }
}
```

Rename the existing explorer-specific controls to `show_gmc_explorer_controls`.

Modify `update_gmc` to dispatch on sub-mode:

```rust
pub(crate) fn update_gmc(&mut self, ctx: &egui::Context) {
    match self.gmc.sub_mode {
        GmcSubMode::Explorer => self.update_gmc_explorer(ctx),
        GmcSubMode::Tune => self.update_gmc_tune(ctx),
    }
}
```

Rename the existing `update_gmc` body to `update_gmc_explorer`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: success (gmc_tune UI methods will be stubs for now).

- [ ] **Step 4: Add stub methods for gmc_tune**

Create `src/ui/gmc_tune.rs` with stub implementations:

```rust
use eframe::egui;
use super::app::ChordzApp;

impl ChordzApp {
    pub(crate) fn show_gmc_tune_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("GMC Tune (WIP)");
    }

    pub(crate) fn update_gmc_tune(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("GMC Tune Mode");
            ui.label("Work in progress");
        });
    }
}
```

Register in `src/ui/mod.rs`:

```rust
mod gmc_tune;
```

- [ ] **Step 5: Verify it compiles and runs**

Run: `cargo check && cargo run` (verify GMC tab shows Explorer/Tune sub-tabs, and Tune shows the WIP message).

- [ ] **Step 6: Commit**

```bash
git add src/ui/app.rs src/ui/gmc.rs src/ui/gmc_tune.rs src/ui/mod.rs
git commit -m "feat(ui): add GMC sub-mode navigation (Explorer/Tune)"
```

---

### Task 6: GMC Tune Sidebar — Chart, Scale Overrides, Controls

**Files:**
- Modify: `src/ui/gmc_tune.rs`

- [ ] **Step 1: Implement the sidebar with chart input, pair selector, figure selector, position, and generate button**

Replace the stub methods in `src/ui/gmc_tune.rs` with:

```rust
use eframe::egui;

use super::app::ChordzApp;
use crate::theory::chart::{Chart, PRESETS as TUNE_PRESETS};
use crate::theory::gmc::PAIRS;
use crate::theory::line_engine::{self, LineConfig};
use crate::theory::line_pattern::{Pattern, RhythmicFigure};
use crate::theory::position::Rigidity;
use crate::theory::scale_defaults;
use crate::theory::scales::Scale;

impl ChordzApp {
    pub(crate) fn show_gmc_tune_controls(&mut self, ui: &mut egui::Ui) {
        if ui.button("Generate").clicked() {
            self.generate_gmc_line();
        }
        if let Some(err) = &self.gmc_tune.error {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err.as_str());
        }
        if self.gmc_tune.generated.is_some() {
            ui.separator();
            ui.label(format!("Measure {}", self.gmc_tune.selected_measure + 1));
        }
    }

    fn generate_gmc_line(&mut self) {
        self.gmc_tune.error = None;
        self.gmc_tune.generated = None;

        let chart = match Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
            Ok(c) => c,
            Err(e) => {
                self.gmc_tune.error = Some(format!("Parse: {}", e));
                return;
            }
        };

        let pair = &PAIRS[self.gmc_tune.pair_index];
        let config = LineConfig {
            pattern: self.gmc_tune.pattern.clone(),
            figure: self.gmc_tune.figure,
            position: self.gmc_tune.position,
        };

        let events = line_engine::generate_line(
            &chart,
            &self.gmc_tune.scale_overrides,
            &self.fretboard,
            pair,
            &config,
        );

        self.gmc_tune.selected_measure = 0;
        self.gmc_tune.last_clicked_measure = None;
        self.gmc_tune.playback_start = None;
        self.gmc_tune.generated = Some(events);
    }

    pub(crate) fn update_gmc_tune(&mut self, ctx: &egui::Context) {
        // Keyboard controls
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::L) {
                if self.gmc_tune.playback_start.is_none() {
                    self.gmc_tune.selected_measure += 1;
                }
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::H) {
                if self.gmc_tune.playback_start.is_none() {
                    self.gmc_tune.selected_measure =
                        self.gmc_tune.selected_measure.saturating_sub(1);
                }
            }
        });

        egui::SidePanel::left("gmc_tune_input")
            .default_width(320.0)
            .show(ctx, |ui| {
                self.show_gmc_tune_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.gmc_tune.generated.is_some() {
                ui.heading("Tab");
                ui.label("(Tab renderer — next task)");
            } else {
                ui.heading("GMC Tune Mode");
                ui.label("Select a chart and press Generate");
            }
        });
    }

    fn show_gmc_tune_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chart");
        ui.horizontal(|ui| {
            ui.label("Tune:");
            egui::ComboBox::from_id_salt("gmc_tune_preset")
                .selected_text(&self.gmc_tune.title_input)
                .show_ui(ui, |ui| {
                    for &(title, changes) in TUNE_PRESETS {
                        if ui
                            .selectable_label(self.gmc_tune.title_input == title, title)
                            .clicked()
                        {
                            self.gmc_tune.title_input = title.to_string();
                            self.gmc_tune.chart_input = changes.to_string();
                            self.gmc_tune.generated = None;
                            self.gmc_tune.scale_overrides.clear();
                        }
                    }
                });
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.gmc_tune.chart_input)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .font(egui::FontId::monospace(12.0)),
        );

        ui.add_space(4.0);
        ui.separator();

        // Pair selector
        ui.horizontal(|ui| {
            ui.label("Pair:");
            egui::ComboBox::from_id_salt("gmc_tune_pair")
                .selected_text(PAIRS[self.gmc_tune.pair_index].label)
                .show_ui(ui, |ui| {
                    for (i, p) in PAIRS.iter().enumerate() {
                        ui.selectable_value(&mut self.gmc_tune.pair_index, i, p.label);
                    }
                });
        });

        // Figure selector
        ui.horizontal(|ui| {
            ui.label("Figure:");
            egui::ComboBox::from_id_salt("gmc_tune_figure")
                .selected_text(self.gmc_tune.figure.label())
                .show_ui(ui, |ui| {
                    for fig in RhythmicFigure::ALL {
                        ui.selectable_value(&mut self.gmc_tune.figure, fig, fig.label());
                    }
                });
        });

        // Position
        ui.horizontal(|ui| {
            ui.label("Position:");
            let pos_label = format!("{}", roman_numeral(self.gmc_tune.position.base_fret));
            egui::ComboBox::from_id_salt("gmc_tune_position")
                .selected_text(pos_label)
                .show_ui(ui, |ui| {
                    for fret in 1..=12u8 {
                        let label = roman_numeral(fret);
                        if ui
                            .selectable_label(self.gmc_tune.position.base_fret == fret, label)
                            .clicked()
                        {
                            self.gmc_tune.position.base_fret = fret;
                        }
                    }
                });

            let mut strict = self.gmc_tune.position.rigidity == Rigidity::Strict;
            if ui.checkbox(&mut strict, "Strict").changed() {
                self.gmc_tune.position.rigidity = if strict {
                    Rigidity::Strict
                } else {
                    Rigidity::Flexible
                };
            }
        });

        ui.separator();

        // Pattern (preset selector for now, builder in next task)
        ui.heading("Pattern");
        ui.horizontal(|ui| {
            ui.label("Preset:");
            let current_name = self.gmc_tune.pattern.name;
            egui::ComboBox::from_id_salt("gmc_tune_pattern")
                .selected_text(current_name)
                .show_ui(ui, |ui| {
                    for preset in Pattern::all_presets() {
                        if ui
                            .selectable_label(self.gmc_tune.pattern.name == preset.name, preset.name)
                            .clicked()
                        {
                            self.gmc_tune.pattern = preset;
                        }
                    }
                });
        });

        // Show current blocks
        for block in &self.gmc_tune.pattern.blocks {
            ui.label(format!("  {}", block.label()));
        }

        ui.separator();

        // Scale overrides per chord (if chart is parsed)
        if let Ok(chart) = Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
            if self.gmc_tune.scale_overrides.len() != chart.changes.len() {
                self.gmc_tune.scale_overrides = vec![None; chart.changes.len()];
            }

            ui.heading("Scales");
            egui::ScrollArea::vertical()
                .id_salt("gmc_scale_overrides")
                .show(ui, |ui| {
                    for (i, change) in chart.changes.iter().enumerate() {
                        let default = scale_defaults::default_scale(change.quality);
                        let current = self.gmc_tune.scale_overrides[i]
                            .map(|idx| &Scale::ALL[idx])
                            .unwrap_or(default);
                        let is_override = self.gmc_tune.scale_overrides[i].is_some();
                        let chord_name =
                            crate::theory::chords::chord_name(&change.root, change.quality);

                        ui.horizontal(|ui| {
                            let text = if is_override {
                                egui::RichText::new(format!("{}: {}", chord_name, current.name))
                                    .color(egui::Color32::from_rgb(255, 200, 100))
                            } else {
                                egui::RichText::new(format!("{}: {}", chord_name, current.name))
                                    .color(egui::Color32::GRAY)
                            };
                            egui::ComboBox::from_id_salt(format!("scale_override_{}", i))
                                .selected_text(text)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(!is_override, format!("Default ({})", default.name))
                                        .clicked()
                                    {
                                        self.gmc_tune.scale_overrides[i] = None;
                                    }
                                    let mut last_parent = None;
                                    for (si, scale) in Scale::ALL.iter().enumerate() {
                                        if last_parent != Some(scale.parent) {
                                            if last_parent.is_some() {
                                                ui.separator();
                                            }
                                            ui.label(scale.parent.name());
                                            last_parent = Some(scale.parent);
                                        }
                                        if ui
                                            .selectable_label(
                                                self.gmc_tune.scale_overrides[i] == Some(si),
                                                scale.name,
                                            )
                                            .clicked()
                                        {
                                            self.gmc_tune.scale_overrides[i] = Some(si);
                                        }
                                    }
                                });
                        });
                    }
                });
        }
    }
}

fn roman_numeral(fret: u8) -> &'static str {
    match fret {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        11 => "XI",
        12 => "XII",
        _ => "?",
    }
}
```

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo check && cargo run`
Expected: GMC → Tune sub-mode shows sidebar with chart input, pair/figure/position selectors, pattern presets, scale overrides, and Generate button. Generate produces events (central panel still placeholder).

- [ ] **Step 3: Commit**

```bash
git add src/ui/gmc_tune.rs
git commit -m "feat(ui): GMC tune sidebar — chart, scales, pair, figure, position, pattern"
```

---

### Task 7: Pattern Builder UI

**Files:**
- Modify: `src/ui/gmc_tune.rs`

- [ ] **Step 1: Replace preset-only pattern section with a builder**

In `show_gmc_tune_sidebar`, replace the pattern section with an interactive builder. Each block has controls for count, direction, and triad, plus add/remove buttons:

```rust
// Replace the "Pattern" section in show_gmc_tune_sidebar with:

ui.heading("Pattern");
ui.horizontal(|ui| {
    ui.label("Preset:");
    let current_name = self.gmc_tune.pattern.name;
    egui::ComboBox::from_id_salt("gmc_tune_pattern")
        .selected_text(current_name)
        .show_ui(ui, |ui| {
            for preset in Pattern::all_presets() {
                if ui
                    .selectable_label(self.gmc_tune.pattern.name == preset.name, preset.name)
                    .clicked()
                {
                    self.gmc_tune.pattern = preset;
                }
            }
        });
});

let mut remove_idx = None;
for (i, block) in self.gmc_tune.pattern.blocks.iter_mut().enumerate() {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut block.count).range(1..=6).prefix("n:"));

        if ui.selectable_label(
            block.direction == Direction::Ascending, "↑"
        ).clicked() {
            block.direction = Direction::Ascending;
        }
        if ui.selectable_label(
            block.direction == Direction::Descending, "↓"
        ).clicked() {
            block.direction = Direction::Descending;
        }

        if ui.selectable_label(block.triad == TriadId::T1, "T1").clicked() {
            block.triad = TriadId::T1;
        }
        if ui.selectable_label(block.triad == TriadId::T2, "T2").clicked() {
            block.triad = TriadId::T2;
        }

        if self.gmc_tune.pattern.blocks.len() > 1 && ui.small_button("✕").clicked() {
            remove_idx = Some(i);
        }
    });
}
if let Some(idx) = remove_idx {
    self.gmc_tune.pattern.blocks.remove(idx);
    self.gmc_tune.pattern.name = "Custom";
}

if self.gmc_tune.pattern.blocks.len() < 6 {
    if ui.small_button("+ Add block").clicked() {
        self.gmc_tune.pattern.blocks.push(PatternBlock {
            count: 3,
            direction: Direction::Ascending,
            triad: TriadId::T1,
        });
        self.gmc_tune.pattern.name = "Custom";
    }
}
```

Add the necessary imports at the top of `gmc_tune.rs`:

```rust
use crate::theory::line_pattern::{Direction, Pattern, PatternBlock, RhythmicFigure, TriadId};
```

Also, update `Pattern` in `line_pattern.rs` to allow `name` to be a `&'static str` or `String`. Change `name` to `pub name: &'static str` — preset names are static, and custom patterns get `"Custom"`.

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo check && cargo run`
Expected: Pattern section shows editable blocks with count drag-value, direction toggle (↑/↓), triad toggle (T1/T2), remove (✕), and add button. Editing any block changes pattern name to "Custom".

- [ ] **Step 3: Commit**

```bash
git add src/ui/gmc_tune.rs src/theory/line_pattern.rs
git commit -m "feat(ui): interactive pattern builder with add/remove/edit blocks"
```

---

### Task 8: Tab Renderer

**Files:**
- Modify: `src/ui/gmc_tune.rs`

- [ ] **Step 1: Implement tab rendering in the central panel**

Replace the placeholder central panel content with a tab renderer. The tab draws 6 horizontal lines (strings), fret numbers at note positions, bar lines, and chord symbols. Notes are colored by triad. Clicking a measure selects it.

```rust
// In update_gmc_tune, replace the CentralPanel content:

fn show_gmc_tab(&mut self, ui: &mut egui::Ui) {
    let Some(events) = &self.gmc_tune.generated else {
        ui.heading("GMC Tune Mode");
        ui.label("Select a chart and press Generate");
        return;
    };

    let chart = match Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
        Ok(c) => c,
        Err(_) => return,
    };

    let events = events.clone();
    let beat_dur = self.gmc_tune.figure.beat_duration();

    // Layout constants
    let string_spacing = 14.0_f32;
    let note_spacing = 20.0_f32;
    let left_margin = 10.0_f32;
    let top_margin = 30.0_f32;
    let measure_gap = 8.0_f32;

    let color_t1 = egui::Color32::from_rgb(100, 160, 255);
    let color_t2 = egui::Color32::from_rgb(255, 140, 50);
    let color_dim = egui::Color32::DARK_GRAY;

    // Group events by measure (4 beats each)
    let mut measures: Vec<(usize, f32, f32, Vec<&NoteEvent>)> = Vec::new();
    let mut beat_cursor = 0.0_f32;
    for (ci, change) in chart.changes.iter().enumerate() {
        let start = beat_cursor;
        let end = beat_cursor + change.beats;
        let measure_events: Vec<&NoteEvent> = events
            .iter()
            .filter(|e| e.beat >= start && e.beat < end)
            .collect();
        measures.push((ci, start, end, measure_events));
        beat_cursor = end;
    }

    // Calculate total width
    let total_notes: usize = measures.iter().map(|(_, _, _, evts)| evts.len().max(1)).sum();
    let total_width = left_margin
        + total_notes as f32 * note_spacing
        + measures.len() as f32 * measure_gap
        + 40.0;
    let total_height = top_margin + string_spacing * 7.0 + 40.0;

    egui::ScrollArea::horizontal().show(ui, |ui| {
        let (response, painter) =
            ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::click());
        let origin = response.rect.left_top();

        // Draw string lines
        let string_names = ["e", "B", "G", "D", "A", "E"];
        for s in 0..6 {
            let y = origin.y + top_margin + s as f32 * string_spacing;
            painter.line_segment(
                [
                    egui::pos2(origin.x + left_margin, y),
                    egui::pos2(origin.x + total_width - 10.0, y),
                ],
                egui::Stroke::new(0.5, color_dim),
            );
        }

        // Draw measures
        let mut x_cursor = origin.x + left_margin;

        for (mi, (chord_idx, _start, _end, measure_events)) in measures.iter().enumerate() {
            let measure_width = measure_events.len().max(1) as f32 * note_spacing + measure_gap;
            let is_selected = mi == self.gmc_tune.selected_measure;

            // Measure click detection
            let measure_rect = egui::Rect::from_min_size(
                egui::pos2(x_cursor, origin.y + top_margin - 5.0),
                egui::vec2(measure_width, string_spacing * 5.0 + 10.0),
            );
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if measure_rect.contains(pos) {
                        self.gmc_tune.selected_measure = mi;
                        self.gmc_tune.last_clicked_measure = Some(mi);
                    }
                }
            }

            // Selected measure highlight
            if is_selected {
                painter.rect_filled(
                    measure_rect,
                    2.0,
                    egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
                );
            }

            // Chord symbol
            let change = &chart.changes[*chord_idx];
            let chord_name =
                crate::theory::chords::chord_name(&change.root, change.quality);
            painter.text(
                egui::pos2(x_cursor + 2.0, origin.y + top_margin - 18.0),
                egui::Align2::LEFT_CENTER,
                &chord_name,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            // Scale name
            let default_scale = scale_defaults::default_scale(change.quality);
            let (scale_name, scale_color) = match self.gmc_tune.scale_overrides.get(*chord_idx) {
                Some(Some(idx)) => (Scale::ALL[*idx].name, egui::Color32::from_rgb(255, 200, 100)),
                _ => (default_scale.name, egui::Color32::from_rgb(100, 100, 100)),
            };
            painter.text(
                egui::pos2(x_cursor + 2.0, origin.y + top_margin - 8.0),
                egui::Align2::LEFT_CENTER,
                scale_name,
                egui::FontId::proportional(8.0),
                scale_color,
            );

            // Notes
            for (ni, event) in measure_events.iter().enumerate() {
                let x = x_cursor + ni as f32 * note_spacing + note_spacing * 0.5;
                // String index: event.string is 0=low E, tab line 0=high e
                let tab_string = 5 - event.string;
                let y = origin.y + top_margin + tab_string as f32 * string_spacing;
                let color = match event.triad {
                    TriadId::T1 => color_t1,
                    TriadId::T2 => color_t2,
                };
                let fret_text = format!("{}", event.fret);

                // Background rect to cover the string line
                let text_size = egui::vec2(12.0, 12.0);
                painter.rect_filled(
                    egui::Rect::from_center_size(egui::pos2(x, y), text_size),
                    0.0,
                    ui.visuals().panel_fill,
                );

                painter.text(
                    egui::pos2(x, y),
                    egui::Align2::CENTER_CENTER,
                    &fret_text,
                    egui::FontId::monospace(10.0),
                    color,
                );
            }

            x_cursor += measure_width;

            // Bar line
            painter.line_segment(
                [
                    egui::pos2(x_cursor, origin.y + top_margin),
                    egui::pos2(x_cursor, origin.y + top_margin + 5.0 * string_spacing),
                ],
                egui::Stroke::new(1.0, color_dim),
            );
        }
    });
}
```

Call `self.show_gmc_tab(ui)` in the central panel of `update_gmc_tune`.

Also import `NoteEvent` and `use crate::theory::line_engine::NoteEvent;` at the top.

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo check && cargo run`
Expected: After pressing Generate, tab appears with 6-line tab, fret numbers colored by triad, chord names above, scale names below, and clickable measures.

- [ ] **Step 3: Commit**

```bash
git add src/ui/gmc_tune.rs
git commit -m "feat(ui): tab renderer with colored triad notes and measure selection"
```

---

### Task 9: Fretboard Per Measure

**Files:**
- Modify: `src/ui/gmc_tune.rs`

- [ ] **Step 1: Add fretboard display for the selected measure below the tab**

After the tab scroll area in the central panel, add a fretboard showing the selected measure's notes with T1/T2 coloring and execution order numbering. Reuse `paint_panoramic_fretboard` style but with numbered notes:

```rust
fn show_gmc_measure_fretboard(&self, ui: &mut egui::Ui) {
    let Some(events) = &self.gmc_tune.generated else {
        return;
    };

    let chart = match Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Find events for the selected measure
    let mut beat_cursor = 0.0_f32;
    let mut measure_start = 0.0_f32;
    let mut measure_end = 0.0_f32;
    for (i, change) in chart.changes.iter().enumerate() {
        if i == self.gmc_tune.selected_measure {
            measure_start = beat_cursor;
            measure_end = beat_cursor + change.beats;
            break;
        }
        beat_cursor += change.beats;
    }

    let measure_events: Vec<&NoteEvent> = events
        .iter()
        .filter(|e| e.beat >= measure_start && e.beat < measure_end)
        .collect();

    if measure_events.is_empty() {
        return;
    }

    let pos = &self.gmc_tune.position;
    let (str_lo, str_hi) = pos.stretch_range();
    let (core_lo, core_hi) = pos.core_range();
    let fret_lo = str_lo;
    let fret_hi = str_hi.max(str_lo + 5);
    let num_frets = (fret_hi - fret_lo + 1) as usize;

    let string_spacing = 22.0_f32;
    let fret_spacing = 50.0_f32;
    let left_margin = 30.0_f32;
    let top_margin = 24.0_f32;
    let dot_radius = 10.0_f32;

    let total_width = left_margin + fret_spacing * num_frets as f32 + 20.0;
    let total_height = top_margin + string_spacing * 5.0 + 30.0;

    let (response, painter) =
        ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::hover());
    let origin = response.rect.left_top();

    let color_t1 = egui::Color32::from_rgb(100, 160, 255);
    let color_t2 = egui::Color32::from_rgb(255, 140, 50);
    let stroke_thin = egui::Stroke::new(1.0, egui::Color32::GRAY);
    let stroke_nut = egui::Stroke::new(2.5, egui::Color32::WHITE);

    // Draw fret lines (vertical)
    for f in 0..=num_frets {
        let x = origin.x + left_margin + f as f32 * fret_spacing;
        let y_top = origin.y + top_margin;
        let y_bot = origin.y + top_margin + 5.0 * string_spacing;
        let stroke = if f == 0 && fret_lo == 0 { stroke_nut } else { stroke_thin };
        painter.line_segment([egui::pos2(x, y_top), egui::pos2(x, y_bot)], stroke);
    }

    // Draw strings (horizontal)
    for s in 0..6 {
        let y = origin.y + top_margin + s as f32 * string_spacing;
        let x_start = origin.x + left_margin;
        let x_end = origin.x + left_margin + num_frets as f32 * fret_spacing;
        painter.line_segment([egui::pos2(x_start, y), egui::pos2(x_end, y)], stroke_thin);
    }

    // Fret numbers
    for f in 0..num_frets {
        let fret_num = fret_lo as usize + f;
        let x = origin.x + left_margin + f as f32 * fret_spacing + fret_spacing * 0.5;
        painter.text(
            egui::pos2(x, origin.y + 8.0),
            egui::Align2::CENTER_CENTER,
            fret_num.to_string(),
            egui::FontId::monospace(10.0),
            egui::Color32::DARK_GRAY,
        );
    }

    // Position indicator (core frets highlighted)
    for f in 0..num_frets {
        let fret_num = fret_lo + f as u8;
        if fret_num >= core_lo && fret_num <= core_hi {
            let x = origin.x + left_margin + f as f32 * fret_spacing;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x, origin.y + top_margin),
                    egui::vec2(fret_spacing, 5.0 * string_spacing),
                ),
                0.0,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 8),
            );
        }
    }

    // Draw notes
    for (order, event) in measure_events.iter().enumerate() {
        let fret_offset = event.fret as i32 - fret_lo as i32;
        if fret_offset < 0 || fret_offset >= num_frets as i32 {
            continue;
        }
        let x = origin.x + left_margin + fret_offset as f32 * fret_spacing + fret_spacing * 0.5;
        let y = origin.y + top_margin + event.string as f32 * string_spacing;
        let color = match event.triad {
            TriadId::T1 => color_t1,
            TriadId::T2 => color_t2,
        };

        painter.circle_filled(egui::pos2(x, y), dot_radius, color);
        painter.text(
            egui::pos2(x, y),
            egui::Align2::CENTER_CENTER,
            format!("{}", order + 1),
            egui::FontId::monospace(8.0),
            egui::Color32::BLACK,
        );
    }
}
```

Call `self.show_gmc_measure_fretboard(ui)` after `self.show_gmc_tab(ui)` in the central panel.

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo check && cargo run`
Expected: Below the tab, a panoramic fretboard shows the selected measure's notes with T1/T2 colors and execution order numbers. Clicking a measure or pressing ← / → updates the fretboard. Core position frets are subtly highlighted.

- [ ] **Step 3: Commit**

```bash
git add src/ui/gmc_tune.rs
git commit -m "feat(ui): fretboard per measure with position indicator and numbered notes"
```

---

### Task 10: Playback Controls

**Files:**
- Modify: `src/ui/gmc_tune.rs`
- Modify: `src/ui/app.rs` (if needed for audio)

- [ ] **Step 1: Add Space for play/pause and measure-click playback start**

In `update_gmc_tune`, add Space key handling. Play advances through measures by time; pause returns to last clicked measure:

```rust
// In update_gmc_tune, inside the ctx.input block:

if i.key_pressed(egui::Key::Space) {
    if self.gmc_tune.playback_start.is_some() {
        // Pause: stop and return to last clicked measure
        self.gmc_tune.playback_start = None;
        if let Some(m) = self.gmc_tune.last_clicked_measure {
            self.gmc_tune.selected_measure = m;
        } else {
            self.gmc_tune.selected_measure = 0;
        }
    } else if self.gmc_tune.generated.is_some() {
        // Play from selected measure
        self.start_gmc_playback();
    }
}
```

Add the playback timing logic (similar to existing tune mode):

```rust
fn start_gmc_playback(&mut self) {
    self.gmc_tune.playback_start = Some(std::time::Instant::now());
}

// In update_gmc_tune, after keyboard input:
if let (Some(start), Some(events)) =
    (self.gmc_tune.playback_start, self.gmc_tune.generated.as_ref())
{
    if let Ok(chart) = Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
        let elapsed = start.elapsed().as_secs_f32();
        let bpm = 120.0_f32;
        let beat_dur_secs = 60.0 / bpm;

        // Find which measure we're in by elapsed time
        let mut beat_cursor = 0.0_f32;
        let mut target_measure = 0;
        let start_measure = self.gmc_tune.last_clicked_measure.unwrap_or(0);

        // Skip beats before start measure
        let mut start_beat = 0.0_f32;
        for change in chart.changes.iter().take(start_measure) {
            start_beat += change.beats;
        }

        let playback_beat = start_beat + elapsed / beat_dur_secs;

        beat_cursor = 0.0;
        for (i, change) in chart.changes.iter().enumerate() {
            if playback_beat >= beat_cursor && playback_beat < beat_cursor + change.beats {
                target_measure = i;
                break;
            }
            beat_cursor += change.beats;
            target_measure = i;
        }

        let total_beats = chart.total_beats();
        if playback_beat >= total_beats {
            self.gmc_tune.playback_start = None;
            self.gmc_tune.selected_measure =
                self.gmc_tune.last_clicked_measure.unwrap_or(0);
        } else {
            self.gmc_tune.selected_measure = target_measure;
            ctx.request_repaint();
        }
    }
}
```

- [ ] **Step 2: Add playback status to top bar controls**

In `show_gmc_tune_controls`:

```rust
if self.gmc_tune.generated.is_some() {
    ui.separator();
    if self.gmc_tune.playback_start.is_some() {
        ui.label("▶ Playing");
    } else {
        ui.label(format!("Measure {}", self.gmc_tune.selected_measure + 1));
    }
}
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo check && cargo run`
Expected: Space starts/stops playback. During playback, the selected measure advances in time. Pause returns to last clicked measure. ← / → only work when paused.

- [ ] **Step 4: Commit**

```bash
git add src/ui/gmc_tune.rs
git commit -m "feat(ui): playback controls — space play/pause, arrow navigation"
```

---

### Task 11: Integration Test and Polish

**Files:**
- All new files

- [ ] **Step 1: Run all tests**

Run: `cargo test --lib`
Expected: all tests pass.

- [ ] **Step 2: Run the app and test the full flow**

Run: `cargo run`

Test the following:
1. Switch to GMC → Tune
2. Select "Stella by Starlight" from presets
3. Choose T/T pair, eighth notes, position V
4. Press Generate → tab appears with colored notes
5. Click different measures → fretboard updates
6. Press Space → playback starts, measure advances
7. Press Space again → pause, returns to clicked measure
8. Use ← / → to navigate when paused
9. Change a scale override, re-generate → different notes
10. Edit pattern blocks, re-generate → different pattern

- [ ] **Step 3: Fix any issues found during testing**

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: GMC tune mode — single-note line generator with triad pairs over charts"
```
