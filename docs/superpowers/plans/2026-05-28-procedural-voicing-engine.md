# Procedural Voicing Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace recipe-based voice set generation with procedural subset generation from per-quality stability tables, producing all valid N-note voicings with close/drop2/drop3/drop2&3 transforms.

**Architecture:** New `stability.rs` defines interval stability tables per chord family. New `procedural.rs` generates all N-note subsets above a stability threshold, applies 4 voicing transforms × N inversions, and classifies each result. The solver and browser call the procedural generator instead of recipe-specific generators.

**Tech Stack:** Rust, existing VoiceSet/Fingering/Fretboard infrastructure

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/voicings/stability.rs` | Create | Stability tables, family detection, lookup |
| `src/voicings/procedural.rs` | Create | Subset generation, transforms, classification |
| `src/voicings/voice_set.rs` | Modify | Add `new_unchecked` for procedurally-generated sets |
| `src/voicings/mod.rs` | Modify | Register new modules |
| `src/voicings/solver.rs` | Modify | Use procedural generator for candidates |
| `src/voicings/ranking.rs` | Modify | Integrate stability score into ranking |
| `src/wasm_api.rs` | Modify | Update generate_voicings to use procedural |
| `src/ui/browser.rs` | Modify | Update browser to use procedural |

---

### Task 1: Stability tables

**Files:**
- Create: `src/voicings/stability.rs`
- Modify: `src/voicings/mod.rs`

- [ ] **Step 1: Create stability module with tables and tests**

Create `src/voicings/stability.rs`:

```rust
use crate::theory::chords::ChordQuality;

pub type StabilityTable = [u8; 12];

// Major (maj7): R=4 b9=0 9=3 b3=1 3=4 11=1 #11=2 5=4 b13=1 13=3 b7=0 7=4
const MAJOR: StabilityTable = [4, 0, 3, 1, 4, 1, 2, 4, 1, 3, 0, 4];

// Minor (m7): R=4 b9=1 9=3 b3=4 3=0 11=4 #11=2 5=4 b13=2 13=2 b7=4 maj7=2
const MINOR: StabilityTable = [4, 1, 3, 4, 0, 4, 2, 4, 2, 2, 4, 2];

// Dominant natural (→major): R=4 b9=2 9=3 #9=2 3=4 4/sus=3 #11=2 5=4 b13=2 13=3 b7=4 7=0
const DOM_NATURAL: StabilityTable = [4, 2, 3, 2, 4, 3, 2, 4, 2, 3, 4, 0];

// Dominant altered (→minor/tritone sub): R=4 b9=3 9=2 #9=3 3=4 4/sus=2 #11=2 5=4 b13=3 13=1 b7=4 7=0
const DOM_ALTERED: StabilityTable = [4, 3, 2, 3, 4, 2, 2, 4, 3, 1, 4, 0];

// Half-diminished (m7b5): R=4 b9=1 9=2 b3=4 3=0 11=3 b5=4 5=0 b13=2 13=1 b7=4 7=1
const HALF_DIM: StabilityTable = [4, 1, 2, 4, 0, 3, 4, 0, 2, 1, 4, 1];

// Diminished (dim7): R=4 b9=2 9=2 b3=4 3=1 11=2 b5=4 5=1 b13=2 dim7=4 13=2 b7=1
const DIM: StabilityTable = [4, 2, 2, 4, 1, 2, 4, 1, 2, 4, 2, 1];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChordFamily {
    Major,
    Minor,
    Dominant,
    HalfDiminished,
    Diminished,
}

pub fn detect_family(quality: &ChordQuality) -> ChordFamily {
    let name = quality.name;
    if name.starts_with("maj") {
        ChordFamily::Major
    } else if name.starts_with("m7b5") || name.starts_with("m9b11") {
        ChordFamily::HalfDiminished
    } else if name.starts_with("dim") {
        ChordFamily::Diminished
    } else if name.starts_with('m') {
        ChordFamily::Minor
    } else {
        ChordFamily::Dominant
    }
}

pub fn resolves_to_minor(next_quality: Option<&ChordQuality>) -> bool {
    match next_quality {
        Some(q) => {
            let family = detect_family(q);
            matches!(family, ChordFamily::Minor | ChordFamily::HalfDiminished)
        }
        None => false,
    }
}

pub fn get_stability_table(
    quality: &ChordQuality,
    next_quality: Option<&ChordQuality>,
) -> StabilityTable {
    match detect_family(quality) {
        ChordFamily::Major => MAJOR,
        ChordFamily::Minor => MINOR,
        ChordFamily::Dominant => {
            if resolves_to_minor(next_quality) {
                DOM_ALTERED
            } else {
                DOM_NATURAL
            }
        }
        ChordFamily::HalfDiminished => HALF_DIM,
        ChordFamily::Diminished => DIM,
    }
}

pub fn subset_stability(table: &StabilityTable, semitones: &[u8]) -> u16 {
    semitones.iter().map(|&s| table[s as usize % 12] as u16).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_core_tones_max_stability() {
        // R(0)=4, 3(4)=4, 5(7)=4, 7(11)=4 = 16
        assert_eq!(subset_stability(&MAJOR, &[0, 4, 7, 11]), 16);
    }

    #[test]
    fn major_with_extensions_lower() {
        // R(0)=4, 9(2)=3, 3(4)=4, 7(11)=4 = 15
        assert_eq!(subset_stability(&MAJOR, &[0, 2, 4, 11]), 15);
    }

    #[test]
    fn major_avoid_note_low() {
        // 11(5)=1
        assert_eq!(MAJOR[5], 1);
    }

    #[test]
    fn major_b9_forbidden() {
        assert_eq!(MAJOR[1], 0);
    }

    #[test]
    fn minor_11_very_stable() {
        assert_eq!(MINOR[5], 4);
    }

    #[test]
    fn dom_natural_vs_altered() {
        // b9: natural=2, altered=3
        assert_eq!(DOM_NATURAL[1], 2);
        assert_eq!(DOM_ALTERED[1], 3);
    }

    #[test]
    fn detect_family_covers_all_qualities() {
        for q in ChordQuality::ALL {
            let _ = detect_family(q);
        }
    }

    #[test]
    fn half_dim_b5_is_stable() {
        // b5 = semitone 6
        assert_eq!(HALF_DIM[6], 4);
    }

    #[test]
    fn half_dim_natural_5_forbidden() {
        assert_eq!(HALF_DIM[7], 0);
    }

    #[test]
    fn dom_resolving_to_minor_uses_altered() {
        let dom = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let minor = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let table = get_stability_table(dom, Some(minor));
        assert_eq!(table[1], 3); // b9=3 in altered
    }

    #[test]
    fn dom_resolving_to_major_uses_natural() {
        let dom = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let maj = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let table = get_stability_table(dom, Some(maj));
        assert_eq!(table[1], 2); // b9=2 in natural
    }
}
```

- [ ] **Step 2: Register module**

Add to `src/voicings/mod.rs`:
```rust
pub mod procedural;
pub mod stability;
```

- [ ] **Step 3: Run tests**

Run: `cargo test voicings::stability`
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/voicings/stability.rs src/voicings/mod.rs
git commit -m "feat: add per-quality interval stability tables"
```

---

### Task 2: Procedural generator

**Files:**
- Create: `src/voicings/procedural.rs`
- Modify: `src/voicings/voice_set.rs`

- [ ] **Step 1: Add `new_procedural` to VoiceSet**

The existing `VoiceSet::new()` asserts all intervals belong to `source_quality.intervals`. Procedurally-generated voice sets use intervals from the stability pool that may include extensions not in the base quality. Add a constructor that skips this check:

In `src/voicings/voice_set.rs`, add after the existing `new()`:

```rust
    pub fn new_procedural(
        root_pc: u8,
        intervals: Vec<Interval>,
        octave_offsets: Vec<i32>,
        recipe: VoicingRecipe,
        source_quality: &'static ChordQuality,
    ) -> Self {
        assert!(!intervals.is_empty());
        assert_eq!(intervals.len(), octave_offsets.len());
        Self {
            root_pc,
            intervals,
            octave_offsets,
            recipe,
            source_quality,
        }
    }
```

- [ ] **Step 2: Create procedural generator with tests**

Create `src/voicings/procedural.rs`:

```rust
use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

use super::recipe::VoicingRecipe;
use super::stability::{get_stability_table, subset_stability, StabilityTable};
use super::voice_set::VoiceSet;

const SEMITONE_TO_INTERVAL: [(u8, Interval); 12] = [
    (0, Interval::UNISON),
    (1, Interval::m2),
    (2, Interval::M2),
    (3, Interval::m3),
    (4, Interval::M3),
    (5, Interval::P4),
    (6, Interval::tritone),
    (7, Interval::P5),
    (8, Interval::m6),
    (9, Interval::M6),
    (10, Interval::m7),
    (11, Interval::M7),
];

fn semitone_to_interval(semitone: u8) -> Interval {
    SEMITONE_TO_INTERVAL[(semitone % 12) as usize].1
}

pub fn generate_all_voice_sets(
    root_pc: u8,
    quality: &'static ChordQuality,
    note_count: usize,
    next_quality: Option<&'static ChordQuality>,
    min_total_stability: u8,
) -> Vec<(VoiceSet, u16, &'static str)> {
    let table = get_stability_table(quality, next_quality);
    let available: Vec<u8> = (0u8..12).filter(|&s| table[s as usize] > 0).collect();

    if available.len() < note_count {
        return Vec::new();
    }

    let subsets = generate_subsets(&available, note_count);
    let mut result = Vec::new();

    for subset in &subsets {
        let stability = subset_stability(&table, subset);
        if stability < min_total_stability as u16 {
            continue;
        }

        let mut sorted = subset.clone();
        sorted.sort();

        for transform in &[Transform::Close, Transform::Drop2, Transform::Drop3, Transform::Drop2And3] {
            if note_count < 4 && !matches!(transform, Transform::Close) {
                continue;
            }
            for inversion in 0..note_count {
                let (intervals, octave_offsets) = apply_transform(&sorted, inversion, *transform);
                let label = classify_transform(*transform);
                let recipe = label_to_recipe(label);

                let vs = VoiceSet::new_procedural(
                    root_pc,
                    intervals,
                    octave_offsets,
                    recipe,
                    quality,
                );
                result.push((vs, stability, label));
            }
        }
    }

    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

#[derive(Clone, Copy)]
enum Transform {
    Close,
    Drop2,
    Drop3,
    Drop2And3,
}

fn apply_transform(
    sorted_semitones: &[u8],
    inversion: usize,
    transform: Transform,
) -> (Vec<Interval>, Vec<i32>) {
    let n = sorted_semitones.len();
    let mut close_order: Vec<u8> = Vec::with_capacity(n);
    for i in 0..n {
        close_order.push(sorted_semitones[(inversion + i) % n]);
    }

    let mut octaves: Vec<i32> = vec![0; n];
    for i in 1..n {
        if close_order[i] <= close_order[i - 1]
            || (i > 0 && octaves[i - 1] > 0 && close_order[i] <= close_order[0])
        {
            octaves[i] = octaves[i - 1] + 1;
        } else if octaves[i - 1] > 0 {
            octaves[i] = octaves[i - 1];
        }
    }

    match transform {
        Transform::Close => {}
        Transform::Drop2 => {
            if n >= 2 {
                let drop_idx = n - 2;
                octaves[drop_idx] -= 1;
                rotate_to_bass(&mut close_order, &mut octaves, drop_idx);
            }
        }
        Transform::Drop3 => {
            if n >= 3 {
                let drop_idx = n - 3;
                octaves[drop_idx] -= 1;
                rotate_to_bass(&mut close_order, &mut octaves, drop_idx);
            }
        }
        Transform::Drop2And3 => {
            if n >= 3 {
                let idx3 = n - 3;
                let idx2 = n - 2;
                octaves[idx3] -= 1;
                octaves[idx2] -= 1;
                let a = close_order.remove(idx3);
                let ao = octaves.remove(idx3);
                let b = close_order.remove(idx2 - 1);
                let bo = octaves.remove(idx2 - 1);
                close_order.insert(0, b);
                octaves.insert(0, bo);
                close_order.insert(0, a);
                octaves.insert(0, ao);
            }
        }
    }

    let intervals = close_order.iter().map(|&s| semitone_to_interval(s)).collect();
    let min_oct = *octaves.iter().min().unwrap_or(&0);
    let normalized: Vec<i32> = octaves.iter().map(|o| o - min_oct).collect();

    (intervals, normalized)
}

fn rotate_to_bass(notes: &mut Vec<u8>, octaves: &mut Vec<i32>, idx: usize) {
    let note = notes.remove(idx);
    let oct = octaves.remove(idx);
    notes.insert(0, note);
    octaves.insert(0, oct);
}

fn generate_subsets(available: &[u8], k: usize) -> Vec<Vec<u8>> {
    let mut result = Vec::new();
    let mut combo = vec![0usize; k];
    fn recurse(available: &[u8], k: usize, start: usize, combo: &mut Vec<usize>, depth: usize, result: &mut Vec<Vec<u8>>) {
        if depth == k {
            result.push(combo[..k].iter().map(|&i| available[i]).collect());
            return;
        }
        for i in start..available.len() {
            combo[depth] = i;
            recurse(available, k, i + 1, combo, depth + 1, result);
        }
    }
    recurse(available, k, 0, &mut combo, 0, &mut result);
    result
}

fn classify_transform(transform: Transform) -> &'static str {
    match transform {
        Transform::Close => "closed",
        Transform::Drop2 => "drop2",
        Transform::Drop3 => "drop3",
        Transform::Drop2And3 => "drop2&3",
    }
}

fn label_to_recipe(label: &str) -> VoicingRecipe {
    match label {
        "closed" => VoicingRecipe::Closed,
        "drop2" => VoicingRecipe::Drop2,
        "drop3" => VoicingRecipe::Drop3,
        _ => VoicingRecipe::Closed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn generates_voice_sets_for_maj7() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        assert!(!sets.is_empty(), "should generate voice sets for Cmaj7");
    }

    #[test]
    fn core_tones_have_highest_stability() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        let top = &sets[0];
        assert_eq!(top.1, 16, "R+3+5+7 = 4+4+4+4 = 16");
    }

    #[test]
    fn four_inversions_per_transform() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 16);
        let close_count = sets.iter().filter(|s| s.2 == "closed").count();
        assert_eq!(close_count, 4, "4 inversions of close position for max stability subset");
    }

    #[test]
    fn drop2_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        let drop2 = sets.iter().any(|s| s.2 == "drop2");
        assert!(drop2, "should have drop2 voice sets");
    }

    #[test]
    fn drop3_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        let drop3 = sets.iter().any(|s| s.2 == "drop3");
        assert!(drop3, "should have drop3 voice sets");
    }

    #[test]
    fn three_note_voicings_only_close() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 3, None, 8);
        assert!(sets.iter().all(|s| s.2 == "closed"), "3-note only close position");
    }

    #[test]
    fn stability_filter_works() {
        let quality = find_quality("maj7");
        let permissive = generate_all_voice_sets(0, quality, 4, None, 8);
        let strict = generate_all_voice_sets(0, quality, 4, None, 14);
        assert!(strict.len() < permissive.len());
    }

    #[test]
    fn em7b5_produces_core_shape() {
        let quality = find_quality("m7b5");
        let sets = generate_all_voice_sets(4, quality, 4, None, 8);
        // Should include subset [R, b3, b5, b7] = semitones [0,3,6,10] relative to root
        let has_core = sets.iter().any(|s| {
            let mut pcs: Vec<u8> = s.0.intervals.iter()
                .map(|iv| (4 + iv.semitones) % 12)
                .collect();
            pcs.sort();
            pcs == vec![2, 4, 7, 10] // E=4, G=7, Bb=10, D=2
        });
        assert!(has_core, "Em7b5 core shape R-b3-b5-b7 must be present");
    }

    #[test]
    fn dominant_altered_profile_with_minor_next() {
        let dom = find_quality("dom7");
        let minor = find_quality("m7");
        let sets = generate_all_voice_sets(0, dom, 4, Some(minor), 8);
        // b9(1) has stability 3 in altered — subsets containing it should appear
        let has_b9 = sets.iter().any(|s| {
            s.0.intervals.iter().any(|iv| iv.semitones == 1)
        });
        assert!(has_b9, "altered dominant should include b9 subsets");
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test voicings::procedural`
Expected: all tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/voicings/procedural.rs src/voicings/voice_set.rs
git commit -m "feat: add procedural voicing generator with subset + transform engine"
```

---

### Task 3: Wire into solver

**Files:**
- Modify: `src/voicings/solver.rs`

- [ ] **Step 1: Replace recipe-based candidate generation**

In `src/voicings/solver.rs`, the function `generate_candidates_with_relaxation` currently iterates over recipes. Replace it to use the procedural generator:

Read the current function, then replace the voice set generation loop. Key changes:

1. Add imports:
```rust
use super::procedural::generate_all_voice_sets;
use super::stability::get_stability_table;
```

2. In `generate_candidates_with_relaxation`, replace the recipe loop with:
```rust
let next_quality: Option<&'static ChordQuality> = None; // caller passes this
let min_stability = match config.tension_target {
    t if t < 0.15 => 14u8,
    t if t < 0.45 => 12,
    t if t < 0.75 => 10,
    _ => 8,
};

let voice_sets_with_meta = generate_all_voice_sets(
    root_pc,
    quality,
    0, // note_count filled per rules below
    next_quality,
    min_stability,
);

for (voice_set, stability, label) in &voice_sets_with_meta {
    if voice_set.len() < rules.min_strings as usize
        || voice_set.len() > rules.max_strings as usize
    {
        continue;
    }
    // Apply recipe/label filter if configured
    if !config.recipes.is_empty() {
        let recipe_matches = config.recipes.iter().any(|r| r.short_label() == *label);
        if !recipe_matches {
            continue;
        }
    }
    // ... rest of fingering generation (map_voice_set, filtering, ranking) stays the same
}
```

3. Add `next_quality: Option<&'static ChordQuality>` parameter to `generate_candidates_with_relaxation` and pass it through from `solve()` (from the chart's next chord change).

4. Remove `extended_for_voicing()` and the `expand_basic_chords` config usage.

5. Remove `recipe_tension()` and `quality_tension()` (replaced by stability scores).

- [ ] **Step 2: Update solve() to pass next_quality**

In the `solve` function's DP loop, when generating candidates for chord at index `i`, pass the quality of chord `i+1` as `next_quality` (or `None` for the last chord).

- [ ] **Step 3: Run existing solver tests**

Run: `cargo test voicings::solver`
Expected: all tests PASS (the Stella/solve tests should still work)

- [ ] **Step 4: Commit**

```bash
git add src/voicings/solver.rs
git commit -m "feat: solver uses procedural generator with stability-based candidates"
```

---

### Task 4: Wire into browser and WASM API

**Files:**
- Modify: `src/ui/browser.rs`
- Modify: `src/wasm_api.rs`

- [ ] **Step 1: Update browser.rs generate_groups**

Replace the recipe loop in `generate_groups()` with:

```rust
use crate::voicings::procedural::generate_all_voice_sets;

let min_stability = 10u8; // balanced default
let voice_sets = generate_all_voice_sets(root_pc, quality, note_count, None, min_stability);

for (voice_set, stability, label) in &voice_sets {
    // map_voice_set, filter, rank as before
    // use `label` instead of `recipe.short_label()`
}
```

- [ ] **Step 2: Update wasm_api.rs generate_voicings**

Replace the recipe loop in `generate_voicings()` similarly. The `prefer_crunch` flag continues to affect `rank_fingerings`.

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 4: Rebuild WASM and verify in browser**

```bash
wasm-pack build --target web --features wasm
mv pkg web/pkg
```

Verify: Em7b5 shows x7878x, Cmaj7 shows Em7-over-C shapes, drop2 inversions all present.

- [ ] **Step 5: Commit**

```bash
git add src/ui/browser.rs src/wasm_api.rs
git commit -m "feat: browser and WASM API use procedural voicing generator"
```

---

### Task 5: Cleanup old recipe generators

**Files:**
- Modify: `src/voicings/recipe.rs`
- Modify: `src/voicings/solver.rs`

- [ ] **Step 1: Remove recipe generation methods**

In `src/voicings/recipe.rs`, remove:
- `generate_voice_sets()` dispatcher
- `generate_shell()`
- `generate_closed()` (the new one with 5th-omitted subsets)
- `generate_drop2()`
- `generate_drop3()`
- `generate_rootless()`
- `generate_rootless_b()`
- `generate_quartal()`
- `generate_upper_structure_triad()`
- `generate_triad_pair()`
- All helper functions only used by these: `push_prefixes()`, `push_voice_set()`, `rootless_color_tones()`, `guide_tones()`

Keep:
- `VoicingRecipe` enum (used as labels)
- `name()`, `short_label()`, `all()`
- `typically_rootless()`

Move `guide_tones()` to `stability.rs` or `ranking.rs` if still needed by ranking.

- [ ] **Step 2: Remove `expand_basic_chords` from SolverConfig**

Remove the field from `SolverConfig`, its default value, and all references in solver.rs, wasm_api.rs, and the Svelte frontend.

- [ ] **Step 3: Remove `recipe_tension()` and `quality_tension()` from solver.rs**

These are replaced by stability scores.

- [ ] **Step 4: Run full test suite and clippy**

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

- [ ] **Step 5: Rebuild WASM, test in browser**

Verify everything still works. The UI may show "drop2&3" as a new label.

- [ ] **Step 6: Commit**

```bash
git add src/voicings/recipe.rs src/voicings/solver.rs src/wasm_api.rs web/
git commit -m "refactor: remove recipe generators, replaced by procedural engine"
```

---

### Task 6: Verification tests

**Files:**
- Modify: `src/voicings/procedural.rs` (add tests)

- [ ] **Step 1: Add classic shape verification tests**

```rust
    #[test]
    fn em7b5_x7878x_reachable() {
        let quality = find_quality("m7b5");
        let fb = crate::voicings::fretboard::Fretboard::standard_tuning();
        let rules = crate::voicings::rules::VoicingRules {
            min_strings: 4, max_strings: 4, max_fret_span: 5, max_fret: 15, require_root: false,
        };
        let sets = generate_all_voice_sets(4, quality, 4, None, 10);
        let mut found = false;
        for (vs, _, _) in &sets {
            let fingerings = crate::voicings::generate::map_voice_set(vs, &fb, &rules);
            if fingerings.iter().any(|f| {
                f.positions == [None, Some(7), Some(8), Some(7), Some(8), None]
            }) {
                found = true;
                break;
            }
        }
        assert!(found, "x7878x must be reachable for Em7b5");
    }

    #[test]
    fn cmaj7_em7_substitution_reachable() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 10);
        // Em7 over C = intervals 3(4), 5(7), 7(11), 9(2)
        let has_em7 = sets.iter().any(|s| {
            let mut sems: Vec<u8> = s.0.intervals.iter()
                .map(|iv| iv.semitones % 12)
                .collect();
            sems.sort();
            sems == vec![2, 4, 7, 11]
        });
        assert!(has_em7, "Em7 substitution (3,5,7,9) over Cmaj7 must be present");
    }

    #[test]
    fn four_drop2_inversions_for_any_4note() {
        let quality = find_quality("dom7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 16);
        let drop2_count = sets.iter().filter(|s| s.2 == "drop2").count();
        assert_eq!(drop2_count, 4, "exactly 4 drop2 inversions for max-stability 4-note subset");
    }

    #[test]
    fn solver_stella_regression() {
        use crate::theory::chart::Chart;
        use crate::voicings::fretboard::Fretboard;
        use crate::voicings::solver::{solve, SolverConfig};

        let chart = Chart::parse(
            "Test",
            "Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11",
        ).unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig::default();
        let result = solve(&chart, &fb, &config);
        assert!(result.is_some(), "Stella first 8 bars must solve");
        let solved = result.unwrap();
        assert_eq!(solved.fingerings.len(), chart.changes.len());
    }
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: all pass including new verification tests

- [ ] **Step 3: Commit**

```bash
git add src/voicings/procedural.rs
git commit -m "test: verify classic shapes and solver regression with procedural engine"
```
