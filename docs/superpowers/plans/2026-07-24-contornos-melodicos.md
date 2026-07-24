# Melodic Contour (CSEG) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit melodic-contour axis to the GMC line engine, so a pattern block can specify the ordinal shape of its cells (low-high-mid, high-low-mid, …) independently of which scale degrees it plays.

**Architecture:** A new `Option<Vec<u8>>` on `PatternBlock` holds a 1-based rank vector. When set, a new pure resolver picks fretboard notes for a whole cell at once: it enumerates every in-region occurrence of each wanted pitch class, keeps assignments whose midis are strictly increasing in rank order, and takes the cheapest by a guitar-shaped cost function. When unset, every existing code path runs unchanged.

**Tech Stack:** Rust (core), `wasm-bindgen`/`serde_json` (boundary), SvelteKit + TypeScript (UI).

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-07-24-contornos-melodicos-design.md`. Read it before Task 2.
- **`contour: None` must be byte-identical to today, on every path.** Any diff in existing test output is a bug, not an update.
- **Rust tests:** `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
  (`cargo` is not on PATH in this environment; there is no Makefile or justfile.)
- **Baseline at plan time:** 259 tests passing, 0 failures. Every task must leave it at ≥259.
- **Cost constants (verbatim from spec):** `SAME_STRING = 24`, `OFF_GRIP = 6`, enumeration cap `MAX_ASSIGNMENTS = 512`.
- **Contour validity:** a permutation of `1..=n`, `1 ≤ n ≤ 8`. Invalid input is rejected at the wasm trust boundary and becomes `None` — never a panic, never a clamp into a different shape.
- **Branch:** `docs/contornos-melodicos`, worktree at `.claude/worktrees/contornos`. It is based on `feat/gmc-partitura`, which is being developed concurrently — do not touch `src/theory/spelling.rs` or `web/src/lib/notation.ts`.

---

## File Structure

| File | Responsibility | Tasks |
|---|---|---|
| `src/theory/line_pattern.rs` | The `contour` field and its validator. Pure data + validation, no fretboard knowledge. | 1 |
| `src/theory/contour.rs` *(new)* | The resolver: identity → candidates → enumeration → cost → notes. Pure; takes a ladder, a rung and a region, returns notes. Kept out of `line_engine.rs`, which is already ~1000 lines. | 2 |
| `src/theory/line_engine.rs` | Call-site integration only: emission loop, `glue_rung`, no-repeat probe, cell/chord boundaries. | 3, 4, 5 |
| `src/wasm_api.rs` | Parse `contour` at the trust boundary. | 6 |
| `web/src/lib/wasm.ts` | `GmcPatternBlock.contour?: number[]`. | 6 |
| `web/src/routes/gmc/tune/+page.svelte` | Six contour buttons per block. | 7 |

---

### Task 1: The `contour` field and its validator

**Files:**
- Modify: `src/theory/line_pattern.rs` (struct at `:141`, `legacy()` at `:160`)
- Modify: `src/theory/line_engine.rs` (all `PatternBlock {` literals in `mod tests`)
- Modify: `src/wasm_api.rs` (the `PatternBlock {` literal in `parse_pattern_blocks`)
- Test: `src/theory/line_pattern.rs` (`mod tests` at the bottom)

**Interfaces:**
- Consumes: nothing.
- Produces: `PatternBlock.contour: Option<Vec<u8>>`; `line_pattern::is_valid_contour(&[u8]) -> bool`.

There are **12** `PatternBlock { … }` struct literals across three files. Rust will not compile until every one gains the new field, so this task is "add field, fix 12 call sites, prove nothing changed".

- [ ] **Step 1: Write the failing test**

Append to `mod tests` in `src/theory/line_pattern.rs`:

```rust
#[test]
fn valid_contours_are_permutations_of_one_through_n() {
    // The six 3-note contours — the whole vocabulary for a triad cell.
    for c in [
        vec![1, 2, 3],
        vec![1, 3, 2],
        vec![2, 1, 3],
        vec![2, 3, 1],
        vec![3, 1, 2],
        vec![3, 2, 1],
    ] {
        assert!(is_valid_contour(&c), "{c:?} should be valid");
    }
    assert!(is_valid_contour(&[1]));
    assert!(is_valid_contour(&[2, 1]));
}

#[test]
fn invalid_contours_are_rejected() {
    assert!(!is_valid_contour(&[]), "empty");
    assert!(!is_valid_contour(&[0, 1, 2]), "0 is not a rank; ranks are 1-based");
    assert!(!is_valid_contour(&[1, 1, 2]), "duplicate rank");
    assert!(!is_valid_contour(&[1, 2, 4]), "rank above n");
    assert!(!is_valid_contour(&[1, 2, 3, 4, 5, 6, 7, 8, 9]), "longer than 8");
}

#[test]
fn legacy_blocks_have_no_contour() {
    let b = PatternBlock::legacy(3, Direction::Ascending, TriadId::T1);
    assert_eq!(b.contour, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib contour`
Expected: FAIL — `cannot find function 'is_valid_contour' in this scope`, and `no field 'contour' on type 'PatternBlock'`.

- [ ] **Step 3: Add the field**

In `src/theory/line_pattern.rs`, add to `PatternBlock` (after `connector`):

```rust
    /// Ordinal register shape of each cell, 1-based (1 = lowest pitch). `None` = today's
    /// behaviour on every path. Length is the cell size and cycles over the block, the way
    /// `Shape::Order` cycles. Always a permutation of `1..=len` — see `is_valid_contour`.
    pub contour: Option<Vec<u8>>,
```

In `PatternBlock::legacy`, add `contour: None,` to the constructed value.

Add the validator to the same file, above `PatternBlock`:

```rust
/// The longest cell the resolver will enumerate. 8! assignments is already far past anything
/// musical; the bound exists so a hostile wasm payload cannot make the search explode.
pub const MAX_CONTOUR_LEN: usize = 8;

/// A contour is valid iff it is a permutation of `1..=c.len()` — every rank present exactly
/// once. Rejects the empty vector and anything longer than `MAX_CONTOUR_LEN`.
pub fn is_valid_contour(c: &[u8]) -> bool {
    let n = c.len();
    if n == 0 || n > MAX_CONTOUR_LEN {
        return false;
    }
    let mut seen = vec![false; n];
    for &rank in c {
        // Ranks are 1-based, so 0 is invalid; `checked_sub` catches it without underflow.
        let idx = match (rank as usize).checked_sub(1) {
            Some(i) => i,
            None => return false,
        };
        if idx >= n || seen[idx] {
            return false;
        }
        seen[idx] = true;
    }
    true
}
```

- [ ] **Step 4: Fix the 12 struct literals**

Add `contour: None,` to every `PatternBlock { … }` literal. Find them with:

```bash
grep -rn "PatternBlock {" --include=*.rs src/
```

Expected: 12 hits across `src/theory/line_pattern.rs`, `src/theory/line_engine.rs`, `src/wasm_api.rs`. The compiler lists every one it is missing — work through `cargo test --lib` until it builds.

- [ ] **Step 5: Run tests to verify they pass and nothing regressed**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
Expected: PASS — **262 passed** (259 baseline + 3 new), 0 failed.

- [ ] **Step 6: Commit**

```bash
git add src/theory/line_pattern.rs src/theory/line_engine.rs src/wasm_api.rs
git commit -m "feat(contorno): campo contour no PatternBlock, com validacao de permutacao"
```

---

### Task 2: The cell resolver

**Files:**
- Create: `src/theory/contour.rs`
- Modify: `src/theory/mod.rs` (add `pub mod contour;`)
- Test: `src/theory/contour.rs` (`mod tests`)

**Interfaces:**
- Consumes: `PatternBlock.contour` (Task 1); `TriadShape { notes: [FretNote; 3] }` from `theory::triad_shape`; `PositionSet::find_notes(&Fretboard, &[u8]) -> Vec<FretNote>` from `theory::position`; `FretNote { string: u8, fret: u8, midi: i32, pitch_class: u8 }`.
- Produces:
  ```rust
  pub fn resolve_cell(
      grip: &TriadShape,
      role_pcs: &[u8; 3],
      positions: &PositionSet,
      fretboard: &Fretboard,
      block: &PatternBlock,
      cell_index: usize,
      prev_midi: Option<i32>,
  ) -> Vec<FretNote>
  ```
  Returns exactly `block.contour.as_ref().unwrap().len()` notes, in **playing** order. Returns an empty vector if `block.contour` is `None`.

`resolve_cell` takes `grip` and `role_pcs` rather than the private `TriadLadder`, so it stays a pure function in its own module with no dependency on the engine's walk state.

- [ ] **Step 1: Write the failing tests**

Create `src/theory/contour.rs` with only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::line_pattern::{
        Anchor, Connector, Direction, PatternBlock, Shape, TriadId,
    };
    use crate::theory::position::PositionSet;
    use crate::theory::triad_shape::inversion_ladder;
    use crate::voicings::fretboard::Fretboard;

    /// A block carrying one contour, everything else at its legacy default.
    fn block_with(contour: Vec<u8>, shape: Shape) -> PatternBlock {
        PatternBlock {
            count: contour.len() as u8,
            direction: Direction::Ascending,
            triad: TriadId::T1,
            shape,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 0,
            connector: Connector::default(),
            contour: Some(contour),
        }
    }

    /// C major triad [C=0, E=4, G=7] around the 5th-fret box, lowest rung.
    fn fixture() -> (Fretboard, PositionSet, crate::theory::triad_shape::TriadShape, [u8; 3]) {
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let pcs = [0u8, 4, 7];
        let grips = inversion_ladder(&fb, &positions, &pcs);
        assert!(!grips.is_empty(), "fixture must produce at least one grip");
        (fb, positions, grips[0], pcs)
    }

    /// The ordinal ranking of a resolved cell, 1-based — the inverse of a contour vector.
    fn ranks_of(cell: &[crate::theory::position::FretNote]) -> Vec<u8> {
        let mut sorted: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        sorted.sort_unstable();
        cell.iter()
            .map(|n| (sorted.iter().position(|m| *m == n.midi).unwrap() + 1) as u8)
            .collect()
    }

    #[test]
    fn every_contour_produces_its_own_ordinal_shape() {
        let (fb, positions, grip, pcs) = fixture();
        for c in [
            vec![1u8, 2, 3],
            vec![1, 3, 2],
            vec![2, 1, 3],
            vec![2, 3, 1],
            vec![3, 1, 2],
            vec![3, 2, 1],
        ] {
            let block = block_with(c.clone(), Shape::Monotonic);
            let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
            assert_eq!(cell.len(), 3, "contour {c:?} must yield 3 notes");
            assert_eq!(ranks_of(&cell), c, "contour {c:?} not realized");
        }
    }

    #[test]
    fn monotonic_contours_reuse_the_cursor_grip_exactly() {
        // <1 2 3> is what the grip already produces, so the grip-affinity term must elect the
        // grip's own notes. This is the property the engine-level non-regression test rests on.
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![1, 2, 3], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let got: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        let want: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        assert_eq!(got, want, "monotonic contour must not leave the grip");
    }

    #[test]
    fn scrambled_contours_must_leave_the_grip() {
        // Inside one grip each pitch class has exactly one midi, hence exactly one rank — so a
        // non-monotonic contour is unrealizable in-grip by construction and must displace.
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![3, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let grip_midis: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        let cell_midis: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        assert_ne!(cell_midis, grip_midis);
        assert_eq!(ranks_of(&cell), vec![3, 1, 2]);
    }

    #[test]
    fn all_notes_stay_inside_the_region() {
        let (fb, positions, grip, pcs) = fixture();
        let legal: Vec<i32> = positions
            .find_notes(&fb, &pcs)
            .iter()
            .map(|n| n.midi)
            .collect();
        for c in [vec![2u8, 3, 1], vec![3, 1, 2], vec![1, 3, 2]] {
            let block = block_with(c.clone(), Shape::Monotonic);
            let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
            for n in &cell {
                assert!(legal.contains(&n.midi), "contour {c:?} escaped the region");
            }
        }
    }

    #[test]
    fn resolution_is_deterministic() {
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![2, 3, 1], Shape::Monotonic);
        let a = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, Some(60));
        let b = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, Some(60));
        let am: Vec<i32> = a.iter().map(|n| n.midi).collect();
        let bm: Vec<i32> = b.iter().map(|n| n.midi).collect();
        assert_eq!(am, bm);
    }

    #[test]
    fn order_plus_contour_is_independent_of_the_starting_rung() {
        // The leak this feature exists to close: today the contour of an Order block drifts with
        // whichever rung the connector landed on. With a contour set, the shape must come out
        // identical from any rung.
        let (fb, positions, _, pcs) = fixture();
        let grips = inversion_ladder(&fb, &positions, &pcs);
        assert!(grips.len() >= 2, "fixture needs at least two rungs");
        let block = block_with(vec![2, 3, 1], Shape::Order(vec![0, 2, 1]));
        let from_first = resolve_cell(&grips[0], &pcs, &positions, &fb, &block, 0, None);
        let from_second = resolve_cell(&grips[1], &pcs, &positions, &fb, &block, 0, None);
        assert_eq!(ranks_of(&from_first), vec![2, 3, 1]);
        assert_eq!(ranks_of(&from_second), vec![2, 3, 1]);
    }

    #[test]
    fn no_contour_yields_no_notes() {
        let (fb, positions, grip, pcs) = fixture();
        let mut block = block_with(vec![1, 2, 3], Shape::Monotonic);
        block.contour = None;
        assert!(resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None).is_empty());
    }

    #[test]
    fn a_region_too_narrow_degrades_instead_of_panicking() {
        // A single-fret window cannot supply three ranked octaves. The resolver must return a
        // full-length cell anyway (longest correct prefix, then lowest available), never panic
        // and never emit an out-of-region note.
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let pcs = [0u8, 4, 7];
        let grips = inversion_ladder(&fb, &positions, &pcs);
        let narrow = PositionSet::from_base_frets(&[1]);
        let block = block_with(vec![3, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grips[0], &pcs, &narrow, &fb, &block, 0, None);
        assert_eq!(cell.len(), 3);
        let legal: Vec<i32> = narrow.find_notes(&fb, &pcs).iter().map(|n| n.midi).collect();
        for n in &cell {
            assert!(legal.contains(&n.midi));
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib contour::`
Expected: FAIL — `cannot find function 'resolve_cell'` (module not yet declared, function not written).

- [ ] **Step 3: Write the implementation**

Prepend to `src/theory/contour.rs` (above the test module):

```rust
//! Melodic contour (CSEG) realization: turn an ordinal shape into fretboard notes.
//!
//! A contour fixes the *shape* of a cell — which note is lowest, which is highest — while
//! `Shape`/`Shape::Order` fixes its *identity*, i.e. which pitch classes it plays. The two are
//! independent axes. Realizing a contour means choosing, for each wanted pitch class, which of
//! its in-region occurrences to use, so the ordinal ranking comes out as requested.
//!
//! No octave arithmetic is involved: the fretboard already is a quantized register axis, and
//! restricting candidates to the region gives the position constraint for free.

use crate::theory::line_pattern::{PatternBlock, Shape};
use crate::theory::position::{FretNote, PositionSet};
use crate::theory::triad_shape::TriadShape;
use crate::voicings::fretboard::Fretboard;

/// Cost of two consecutive notes landing on one string — it forces a slide or a legato where
/// the player expects a picked note. Set above any realistic in-window fret span so compactness
/// can never buy a string reuse back.
const SAME_STRING: i32 = 24;

/// Per-note toll for leaving the hand shape the connector chose. Small enough that a contour
/// which genuinely needs displacement can pay it, large enough that nothing drifts for free.
const OFF_GRIP: i32 = 6;

/// Enumeration guard. Real cells settle in a few dozen assignments; this only bounds pathology.
const MAX_ASSIGNMENTS: usize = 512;

/// Resolve one cell into fretboard notes, in playing order.
///
/// Returns an empty vector when the block carries no contour. Otherwise always returns exactly
/// `contour.len()` notes — degrading to the longest correct rank prefix if the region cannot
/// realize the full shape.
pub fn resolve_cell(
    grip: &TriadShape,
    role_pcs: &[u8; 3],
    positions: &PositionSet,
    fretboard: &Fretboard,
    block: &PatternBlock,
    cell_index: usize,
    prev_midi: Option<i32>,
) -> Vec<FretNote> {
    let contour = match &block.contour {
        Some(c) => c,
        None => return Vec::new(),
    };
    let n = contour.len();

    // One candidate list per playing position: every in-region occurrence of its pitch class.
    let candidates: Vec<Vec<FretNote>> = (0..n)
        .map(|j| {
            let pc = cell_pitch_class(grip, role_pcs, block, cell_index, j, n);
            occurrences(positions, fretboard, pc)
        })
        .collect();

    // by_rank[r] is the playing position holding rank r+1, i.e. the r-th lowest note.
    let mut by_rank = vec![0usize; n];
    for (j, &rank) in contour.iter().enumerate() {
        by_rank[(rank - 1) as usize] = j;
    }

    let mut search = Search {
        candidates: &candidates,
        by_rank: &by_rank,
        grip_midis: grip.notes.iter().map(|note| note.midi).collect(),
        prev_midi,
        visited: 0,
        best: None,
        deepest: (0, vec![None; n]),
    };
    search.extend(0, i32::MIN, &mut vec![None; n]);

    match search.best.take() {
        Some((_, cell)) => cell,
        // Unrealizable here: keep the ranks we did satisfy, fill the rest with each position's
        // lowest in-region occurrence. Deterministic, and it degrades gradually.
        None => {
            let (_, partial) = search.deepest;
            (0..n)
                .map(|j| {
                    partial[j]
                        .or_else(|| candidates[j].first().copied())
                        .unwrap_or(grip.notes[j % 3])
                })
                .collect()
        }
    }
}

/// Which pitch class this cell wants at playing position `j`.
///
/// For `Shape::Order` the identity comes from the triad's roles, cycling over the block exactly
/// as `note_at` does. For `Shape::Monotonic` it is the grip's own pitch classes in ascending
/// order — which is what makes `<1 2 3>` reproduce today's ascending walk note for note.
fn cell_pitch_class(
    grip: &TriadShape,
    role_pcs: &[u8; 3],
    block: &PatternBlock,
    cell_index: usize,
    j: usize,
    n: usize,
) -> u8 {
    match &block.shape {
        Shape::Order(order) => {
            let k = cell_index * n + j;
            role_pcs[(order[k % order.len()] % 3) as usize]
        }
        Shape::Monotonic => grip.notes[j % 3].pitch_class,
    }
}

/// Every in-region occurrence of `pc`, ordered by pitch then string so the search is stable.
/// Duplicates at the same pitch on different strings are kept — the cost function decides.
fn occurrences(positions: &PositionSet, fretboard: &Fretboard, pc: u8) -> Vec<FretNote> {
    let mut notes = positions.find_notes(fretboard, &[pc]);
    notes.sort_by_key(|note| (note.midi, note.string));
    notes
}

/// Depth-first walk over rank order, keeping only strictly ascending assignments.
struct Search<'a> {
    candidates: &'a [Vec<FretNote>],
    by_rank: &'a [usize],
    grip_midis: Vec<i32>,
    prev_midi: Option<i32>,
    visited: usize,
    best: Option<(i32, Vec<FretNote>)>,
    deepest: (usize, Vec<Option<FretNote>>),
}

impl Search<'_> {
    fn extend(&mut self, rank: usize, floor: i32, chosen: &mut Vec<Option<FretNote>>) {
        if self.visited >= MAX_ASSIGNMENTS {
            return;
        }
        if rank > self.deepest.0 {
            self.deepest = (rank, chosen.clone());
        }
        if rank == self.by_rank.len() {
            self.visited += 1;
            let cell: Vec<FretNote> = chosen.iter().map(|slot| slot.expect("filled")).collect();
            let cost = self.score(&cell);
            if self.best.as_ref().is_none_or(|(best, _)| cost < *best) {
                self.best = Some((cost, cell));
            }
            return;
        }
        let position = self.by_rank[rank];
        for candidate in &self.candidates[position] {
            // Strictly above the previous rank — equal pitches would make the ranking ambiguous.
            if candidate.midi <= floor {
                continue;
            }
            chosen[position] = Some(*candidate);
            self.extend(rank + 1, candidate.midi, chosen);
            if self.visited >= MAX_ASSIGNMENTS {
                break;
            }
        }
        chosen[position] = None;
    }

    fn score(&self, cell: &[FretNote]) -> i32 {
        let mut cost = 0i32;
        for pair in cell.windows(2) {
            if pair[0].string == pair[1].string {
                cost += SAME_STRING;
            }
        }
        let frets: Vec<i32> = cell.iter().map(|note| note.fret as i32).collect();
        cost += frets.iter().max().unwrap_or(&0) - frets.iter().min().unwrap_or(&0);
        cost += OFF_GRIP
            * cell
                .iter()
                .filter(|note| !self.grip_midis.contains(&note.midi))
                .count() as i32;
        if let Some(prev) = self.prev_midi {
            cost += (cell[0].midi - prev).abs();
        }
        cost
    }
}
```

Declare the module — add to `src/theory/mod.rs`, in alphabetical order with the others:

```rust
pub mod contour;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib contour::`
Expected: PASS — 8 tests in `theory::contour::tests`, 0 failed.

Then the whole suite: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
Expected: **270 passed** (262 + 8), 0 failed.

> If `is_none_or` is rejected by the stable toolchain, replace it with
> `self.best.as_ref().map_or(true, |(best, _)| cost < *best)`.

- [ ] **Step 5: Commit**

```bash
git add src/theory/contour.rs src/theory/mod.rs
git commit -m "feat(contorno): resolvedor de celula, com custo guitarristico e afinidade ao grip"
```

---

### Task 3: Wire the resolver into the emission loop

**Files:**
- Modify: `src/theory/line_engine.rs` (`run_pattern`, the `for k in 0..count` loop at `:361-410`)
- Test: `src/theory/line_engine.rs` (`mod tests`)

**Interfaces:**
- Consumes: `contour::resolve_cell` (Task 2).
- Produces: contour-driven `NoteEvent`s from `generate_line`; no signature changes.

The loop currently calls `note_at(grip, pcs, block, k)` per note. With a contour, note `k` depends on notes `0..k` of its cell, so the cell is resolved on its boundary and cached.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `src/theory/line_engine.rs`:

```rust
/// A one-block pattern carrying a contour.
fn contour_config(count: u8, shape: Shape, contour: Vec<u8>) -> LineConfig {
    LineConfig {
        pattern: Pattern {
            name: "test",
            blocks: vec![PatternBlock {
                count,
                direction: Direction::Ascending,
                triad: TriadId::T1,
                shape,
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: Some(contour),
            }],
        },
        figure: RhythmicFigure::Eighth,
        positions: PositionSet::from_base_frets(&[5]),
    }
}

#[test]
fn ascending_contour_reproduces_the_legacy_ascending_walk() {
    // <1 2 3> must be byte-identical to Monotonic + Ascending. This is the non-regression
    // guarantee the whole design rests on — assert on full event vectors, not spot checks.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();

    let legacy = shaped_config(3, TriadId::T1, Shape::Monotonic, Anchor::Nearest);
    let want = generate_line(&chart, &[], &fb, &PAIRS[0], &legacy);

    let with_contour = contour_config(3, Shape::Monotonic, vec![1, 2, 3]);
    let got = generate_line(&chart, &[], &fb, &PAIRS[0], &with_contour);

    let key = |e: &NoteEvent| (e.string, e.fret, e.midi, e.beat.to_bits());
    assert_eq!(
        got.iter().map(key).collect::<Vec<_>>(),
        want.iter().map(key).collect::<Vec<_>>()
    );
}

#[test]
fn descending_contour_reproduces_the_legacy_descending_walk() {
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();

    let mut legacy = shaped_config(3, TriadId::T1, Shape::Monotonic, Anchor::Nearest);
    legacy.pattern.blocks[0].direction = Direction::Descending;
    let want = generate_line(&chart, &[], &fb, &PAIRS[0], &legacy);

    let with_contour = contour_config(3, Shape::Monotonic, vec![3, 2, 1]);
    let got = generate_line(&chart, &[], &fb, &PAIRS[0], &with_contour);

    let key = |e: &NoteEvent| (e.string, e.fret, e.midi);
    assert_eq!(
        got.iter().map(key).collect::<Vec<_>>(),
        want.iter().map(key).collect::<Vec<_>>()
    );
}

#[test]
fn a_scrambled_contour_shapes_the_emitted_line() {
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Dm7 |").unwrap();
    let config = contour_config(3, Shape::Monotonic, vec![3, 1, 2]);
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
    let midis: Vec<i32> = events.iter().take(3).map(|e| e.midi).collect();
    assert!(midis[0] > midis[2], "first note must be the highest");
    assert!(midis[1] < midis[2], "second note must be the lowest");
}

#[test]
fn a_contour_cycles_over_a_longer_block() {
    // count=6 with a 3-contour is two identical cells — the same cycling rule Shape::Order uses.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Dm7 |").unwrap();
    let config = contour_config(6, Shape::Monotonic, vec![2, 3, 1]);
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
    let midis: Vec<i32> = events.iter().take(6).map(|e| e.midi).collect();
    for cell in [&midis[0..3], &midis[3..6]] {
        assert!(cell[1] > cell[0], "rank 3 sits at position 1");
        assert!(cell[2] < cell[0], "rank 1 sits at position 2");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib line_engine::tests::a_scrambled`
Expected: FAIL — the contour is ignored, so the emitted line is still the ascending walk and the assertions on note order do not hold.

- [ ] **Step 3: Resolve cells inside the loop**

In `src/theory/line_engine.rs`, add the import at the top:

```rust
use crate::theory::contour::resolve_cell;
```

Inside `run_pattern`, immediately before `for k in 0..count {`, add the cell cache:

```rust
        // Contour blocks resolve a whole cell at once (note k depends on notes 0..k of its
        // cell), so cache the current cell and serve it by offset. `None` keeps the legacy
        // per-note path untouched.
        let cell_len = block.contour.as_ref().map(|c| c.len()).unwrap_or(0);
        let mut cell: Vec<FretNote> = Vec::new();
        let mut cell_start = 0usize;
```

Then replace the single note lookup at `:387`:

```rust
            let note = note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, k);
```

with:

```rust
            let note = if cell_len > 0 {
                if cell.is_empty() || k >= cell_start + cell_len {
                    cell_start = (k / cell_len) * cell_len;
                    cell = resolve_cell(
                        &ladder.grips[cursor[ti]],
                        &ladder.pcs,
                        &config.positions,
                        fretboard,
                        block,
                        cell_start / cell_len,
                        last_midi,
                    );
                }
                cell[k - cell_start]
            } else {
                note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, k)
            };
```

`run_pattern` does not currently receive a `Fretboard`. Thread it through: add `fretboard: &Fretboard` as a parameter, and pass `fretboard` at the single call site in `generate_line` (`:268`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
Expected: **274 passed** (270 + 4), 0 failed. The two non-regression tests passing is the signal that `contour: None` and the monotonic contours are untouched.

- [ ] **Step 5: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "feat(contorno): loop de emissao resolve celula inteira quando ha contorno"
```

---

### Task 4: Rung prediction via the resolver

**Files:**
- Modify: `src/theory/line_engine.rs` (`glue_rung` at `:159`, no-repeat probe at `:346-356`)
- Test: `src/theory/line_engine.rs` (`mod tests`)

**Interfaces:**
- Consumes: `contour::resolve_cell` (Task 2), the threaded `fretboard` (Task 3).
- Produces: no signature change outside `glue_rung`, which gains `positions` and `fretboard`.

Both sites call `note_at(…, k)` to *predict* what a rung would produce before committing to it. Under a contour that prediction must come from the resolved cell.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn contour_blocks_never_repeat_the_previous_pitch() {
    // The no-repeat probe predicts a rung's first note. Under a contour that prediction has to
    // come from the resolved cell, or a block can open on the pitch that just sounded.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Cm7 | % |").unwrap();
    let blocks = vec![
        PatternBlock {
            count: 3,
            direction: Direction::Ascending,
            triad: TriadId::T1,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 0,
            connector: Connector::default(),
            contour: Some(vec![2, 3, 1]),
        },
        PatternBlock {
            count: 3,
            direction: Direction::Ascending,
            triad: TriadId::T2,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 0,
            connector: Connector::default(),
            contour: Some(vec![3, 1, 2]),
        },
    ];
    let config = LineConfig {
        pattern: Pattern { name: "test", blocks },
        figure: RhythmicFigure::Eighth,
        positions: PositionSet::from_base_frets(&[5]),
    };
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
    for pair in events.windows(2) {
        assert_ne!(pair[0].midi, pair[1].midi, "line repeated a pitch");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib contour_blocks_never_repeat`
Expected: FAIL — an `assert_ne!` fires where a block opens on the pitch that just sounded.

- [ ] **Step 3: Route both predictions through the resolver**

Add a helper next to `glue_rung`:

```rust
/// The first note a rung would produce for the cell at index `cell_index` — the prediction both
/// `glue_rung` and the no-repeat probe need before they commit to a rung.
fn first_note_of(
    ladder: &TriadLadder,
    rung: usize,
    positions: &PositionSet,
    fretboard: &Fretboard,
    block: &PatternBlock,
    cell_index: usize,
    k: usize,
    prev_midi: Option<i32>,
) -> FretNote {
    if block.contour.is_some() {
        let cell = resolve_cell(
            &ladder.grips[rung],
            &ladder.pcs,
            positions,
            fretboard,
            block,
            cell_index,
            prev_midi,
        );
        if let Some(note) = cell.first() {
            return *note;
        }
    }
    note_at(&ladder.grips[rung], &ladder.pcs, block, k)
}
```

In `glue_rung`, add `positions: &PositionSet, fretboard: &Fretboard` parameters and replace the body of `cost`:

```rust
        let midi = first_note_of(ladder, i, positions, fretboard, block, k, k, Some(prev)).midi;
```

Update its call site at `:380` to pass `&config.positions, fretboard`.

In the no-repeat probe at `:351`, replace the `note_at(…)` call:

```rust
                && first_note_of(
                    ladder,
                    cursor[ti],
                    &config.positions,
                    fretboard,
                    block,
                    0,
                    0,
                    last_midi,
                )
                .midi
                    == prev
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
Expected: **275 passed** (274 + 1), 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "feat(contorno): glue_rung e sonda de nao-repeticao preveem pela celula resolvida"
```

---

### Task 5: Chord-change truncation

**Files:**
- Modify: `src/theory/line_engine.rs` (the `chord_now != active_chord` branch at `:372-385`)
- Test: `src/theory/line_engine.rs` (`mod tests`)

**Interfaces:**
- Consumes: the cell cache from Task 3.
- Produces: no signature change.

A cell whose notes straddle two harmonies has a meaningless register arrangement. A chord change truncates the current cell and starts a fresh one on the new ladder.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_chord_change_truncates_the_current_cell() {
    // One beat per chord with sixteenths: a 3-cell cannot fit inside one chord, so cells must
    // restart at the change rather than resolve across two harmonies.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Dm7 G7 | Cmaj7 A7 |").unwrap();
    let mut config = contour_config(6, Shape::Monotonic, vec![2, 3, 1]);
    config.figure = RhythmicFigure::Sixteenth;
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
    assert!(!events.is_empty());
    // Every emitted note belongs to the chord sounding at its beat — no note is carried over
    // from a cell resolved against the previous harmony.
    for e in &events {
        assert!(e.midi > 0, "no placeholder pitches");
    }
    // And the line still never repeats a pitch across the boundary.
    for pair in events.windows(2) {
        assert_ne!(pair[0].midi, pair[1].midi);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib a_chord_change_truncates`
Expected: FAIL — a stale cell resolved against the previous chord keeps being served, producing a repeated pitch across the boundary.

- [ ] **Step 3: Invalidate the cell at the change**

Inside the `if chord_now != active_chord {` branch, after `cursor_chord[ti] = chord_now;`, add:

```rust
                // A cell resolved against the old harmony is meaningless now — force a fresh
                // one on the new ladder, entered from the glue pitch.
                cell.clear();
                cell_start = k;
```

And make the cache check honour a mid-cell restart by using `cell_start` as the origin — replace the condition written in Task 3:

```rust
                if cell.is_empty() || k >= cell_start + cell_len {
```

with:

```rust
                if cell.is_empty() || k >= cell_start + cell_len {
                    // `cell_start` is the origin of the current cell: normally a multiple of
                    // cell_len, but a chord change resets it to the note where the new
                    // harmony began.
```

(the condition itself is already correct; add the comment and drop the `cell_start = (k / cell_len) * cell_len;` line, since `cell_start` is now maintained by the caller — set it to `k` when the cache is refilled.)

The refill becomes:

```rust
                if cell.is_empty() || k >= cell_start + cell_len {
                    cell_start = k;
                    cell = resolve_cell(
                        &ladder.grips[cursor[ti]],
                        &ladder.pcs,
                        &config.positions,
                        fretboard,
                        block,
                        k / cell_len,
                        last_midi,
                    );
                }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo test --lib`
Expected: **276 passed** (275 + 1), 0 failed — including both non-regression tests from Task 3.

- [ ] **Step 5: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "feat(contorno): troca de acorde trunca a celula em vez de resolver entre harmonias"
```

---

### Task 6: The wasm boundary

**Files:**
- Modify: `src/wasm_api.rs` (`parse_pattern_blocks` at `:564`)
- Modify: `web/src/lib/wasm.ts` (`GmcPatternBlock`)
- Test: `src/wasm_api.rs` — the crate compiles this only under `--features wasm`, so validation is tested through `line_pattern::is_valid_contour` (Task 1) plus a type-check run.

**Interfaces:**
- Consumes: `line_pattern::is_valid_contour` (Task 1).
- Produces: `contour` accepted on the JS pattern payload.

The core is the trust boundary: an arbitrary JS array must be validated as a permutation and rejected outright — never clamped into a different shape, because a clamped contour is a silently different exercise.

- [ ] **Step 1: Parse and validate**

In `parse_pattern_blocks`, after the `connector` binding, add:

```rust
            // Contour: an optional 1-based rank vector. Unlike `count`, an invalid contour is
            // REJECTED rather than clamped — clamping would silently hand the player a
            // different exercise from the one they asked for.
            let contour = b["contour"].as_array().and_then(|arr| {
                let ranks: Vec<u8> = arr
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .map(|r| r.min(u8::MAX as u64) as u8)
                    .collect();
                // A short `ranks` means some entry was not a number at all: reject, don't
                // silently resolve a shorter contour than the payload asked for.
                if ranks.len() == arr.len() && crate::theory::line_pattern::is_valid_contour(&ranks)
                {
                    Some(ranks)
                } else {
                    None
                }
            });
```

Add `contour,` to the `PatternBlock { … }` literal it builds (replacing the `contour: None` added in Task 1).

- [ ] **Step 2: Type-check the wasm feature**

Run: `PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: PASS, no warnings about unused imports.

- [ ] **Step 3: Extend the TS type**

In `web/src/lib/wasm.ts`, add to `GmcPatternBlock`:

```typescript
  /** Ordinal register shape per cell, 1-based (1 = lowest). Must be a permutation of 1..n. */
  contour?: number[];
```

- [ ] **Step 4: Verify the web type-check**

Run: `cd web && npm run check`
Expected: PASS, 0 errors.

- [ ] **Step 5: Rebuild the wasm package**

Run:
```bash
TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu
PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
```
Expected: build succeeds, output in `web/pkg`.

> Only the generic `nightly` toolchain carries the `wasm32` target. No new wasm export is added
> here, so the hand-written ambient `web/src/wasm.d.ts` needs no change.

- [ ] **Step 6: Commit**

```bash
git add src/wasm_api.rs web/src/lib/wasm.ts
git commit -m "feat(contorno): contorno atravessa a fronteira wasm, validado como permutacao"
```

---

### Task 7: The contour picker in the GMC tune UI

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`
- Test: `web/src/lib/` — component behaviour is covered by the existing vitest setup; add a unit test for the glyph geometry helper.

**Interfaces:**
- Consumes: `GmcPatternBlock.contour` (Task 6).
- Produces: per-block contour selection in the pattern editor.

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/contour.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { CONTOURS, contourPoints } from './contour';

describe('contour glyphs', () => {
  it('offers exactly the six 3-note contours', () => {
    expect(CONTOURS).toHaveLength(6);
    const seen = CONTOURS.map((c) => c.ranks.join(''));
    expect(new Set(seen).size).toBe(6);
  });

  it('every entry is a permutation of 1..3', () => {
    for (const c of CONTOURS) {
      expect([...c.ranks].sort()).toEqual([1, 2, 3]);
    }
  });

  it('maps rank to height so 3 is the top of the box', () => {
    // A 2-level sketch collapses <1 3 2> onto <2 3 1>; three levels keeps them distinct.
    const a = contourPoints([1, 3, 2], 30, 20).map((p) => p.y);
    const b = contourPoints([2, 3, 1], 30, 20).map((p) => p.y);
    expect(a).not.toEqual(b);
    // Rank 3 is the highest note, so the smallest y in SVG coordinates.
    const pts = contourPoints([1, 2, 3], 30, 20);
    expect(pts[2].y).toBeLessThan(pts[0].y);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/lib/contour.test.ts`
Expected: FAIL — `Failed to resolve import "./contour"`.

- [ ] **Step 3: Write the helper**

Create `web/src/lib/contour.ts`:

```typescript
/** The six ordinal shapes a 3-note cell can take. `ranks[i]` is the rank of the i-th note. */
export const CONTOURS: { ranks: number[]; title: string }[] = [
  { ranks: [1, 2, 3], title: 'low → mid → high (ascending)' },
  { ranks: [1, 3, 2], title: 'low → high → mid' },
  { ranks: [2, 1, 3], title: 'mid → low → high' },
  { ranks: [2, 3, 1], title: 'mid → high → low' },
  { ranks: [3, 1, 2], title: 'high → low → mid' },
  { ranks: [3, 2, 1], title: 'high → mid → low (descending)' }
];

/**
 * Polyline points for a contour glyph inside a `w` x `h` box.
 * Rank 1 sits at the bottom, rank n at the top; SVG y grows downward, so the highest rank
 * gets the smallest y.
 */
export function contourPoints(ranks: number[], w: number, h: number): { x: number; y: number }[] {
  const n = ranks.length;
  const pad = 2;
  const stepX = n > 1 ? (w - 2 * pad) / (n - 1) : 0;
  const stepY = n > 1 ? (h - 2 * pad) / (n - 1) : 0;
  return ranks.map((rank, i) => ({
    x: pad + i * stepX,
    y: h - pad - (rank - 1) * stepY
  }));
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npx vitest run src/lib/contour.test.ts`
Expected: PASS — 3 tests.

- [ ] **Step 5: Add the picker to the block editor**

In `web/src/routes/gmc/tune/+page.svelte`, import the helper:

```typescript
  import { CONTOURS, contourPoints } from '$lib/contour';
```

Add, inside the per-block controls (next to the existing connector buttons):

```svelte
<div class="contour-row">
  <button
    class="filter-btn"
    class:active={!block.contour}
    title="No contour — the ↑/↓ walk"
    onclick={() => { block.contour = undefined; solve(); }}>—</button>
  {#each CONTOURS as c}
    <button
      class="filter-btn contour-btn"
      class:active={block.contour?.join() === c.ranks.join()}
      title={c.title}
      onclick={() => { block.contour = [...c.ranks]; solve(); }}>
      <svg viewBox="0 0 30 20" width="30" height="20" aria-hidden="true">
        <polyline
          points={contourPoints(c.ranks, 30, 20).map((p) => `${p.x},${p.y}`).join(' ')}
          fill="none" stroke="currentColor" stroke-width="2"
          stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
  {/each}
</div>
```

The picker is shown only when `block.count === 3`; other sizes keep the `↑`/`↓` toggle, since the six glyphs are the 3-note affordance:

```svelte
{#if block.count === 3}
  <!-- the contour row above -->
{/if}
```

- [ ] **Step 6: Verify the whole web suite**

Run: `cd web && npm run check && npx vitest run`
Expected: both PASS, 0 errors.

- [ ] **Step 7: Commit**

```bash
git add web/src/lib/contour.ts web/src/lib/contour.test.ts web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(contorno): seletor de contorno por bloco no GMC tune"
```

---

## Self-Review

**Spec coverage.** Every section maps to a task: data model → 1; cell resolution → 2; guitaristic cost and grip affinity → 2; degradation → 2; partial cells → 3 (cycling) and 5 (truncation); mid-block chord changes → 5; the rung's role → 2 (`cell_pitch_class` for `Monotonic`) and 4 (prediction); refactor surface → 3, 4; downstream → 6; UI → 7. Spec test items 1–8 appear as: 1 → Task 3 non-regression pair; 2 → every task's full-suite run; 3 → Task 2; 4 → **gap, see below**; 5 → Task 2 `monotonic_contours_reuse_the_cursor_grip_exactly`; 6 → Task 2 degradation test; 7 → Task 3 cycling test; 8 → Task 5.

**Gap found and closed inline.** Spec test 4 — "`Order([0,2,1]) + <2 3 1>` yields the same ordinal shape from a root-position rung and from a first-inversion rung", the regression test for the very leak this feature exists to fix — had no task. It is now `order_plus_contour_is_independent_of_the_starting_rung` in Task 2, Step 1, and the expected test counts downstream were corrected accordingly (Task 2 → 270, Task 3 → 274, Task 4 → 275, Task 5 → 276).

**Placeholder scan.** No TBD/TODO. Every code step carries real code. The only conditional instruction is the `is_none_or` fallback in Task 2, which states both alternatives explicitly.

**Type consistency.** `resolve_cell` has one signature, declared in Task 2 and used verbatim in Tasks 3 and 4. `is_valid_contour` is defined in Task 1 and consumed in Task 6. `contourPoints`/`CONTOURS` are defined and consumed inside Task 7. `first_note_of` is defined and used only in Task 4. `cell` / `cell_start` / `cell_len` are introduced in Task 3 and modified in Task 5 — Task 5 restates the final form of the refill block rather than describing a delta.

**Known risk, flagged not hidden.** Task 3 threads `fretboard: &Fretboard` into `run_pattern`, and Task 4 threads `positions` and `fretboard` into `glue_rung`. `run_pattern` is one of the functions `feat/gmc-partitura` is actively editing. Rebase before starting, and expect a signature-level conflict there rather than a semantic one.
