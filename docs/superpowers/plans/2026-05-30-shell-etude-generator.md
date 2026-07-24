# Shell Étude Generator (Motor E) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Shell Étude" generator (Motor E) that produces a guide-tone (7no5/7no3) single-note line over any chord chart, first targeting Moment's Notice, reusing the existing GMC line engine.

**Architecture:** A new pure `shells.rs` resolves two 3-note shells per chord quality (a transposition-invariant table distilled from `Airegin 7no5:7no3 etude.gp`, with a literal-shell fallback). The line engine's per-chord resolution is the only swap: we extract the existing note-walking loop into a shared `run_pattern`, leave `generate_line` (GMC) byte-for-byte equivalent, and add a sibling `generate_shell_line` that feeds the same loop shell-resolved notes. A WASM export and a source toggle in the web GMC-tune page expose it; rendering and the existing walking-bass toggle are reused unchanged.

**Tech Stack:** Rust core (cargo, native nightly), `wasm-bindgen`/`wasm-pack` (wasm32), SvelteKit web shell (TypeScript, Web Audio).

**Spec:** `docs/superpowers/specs/2026-05-30-shell-etude-generator-design.md`

**Toolchain note (from project memory — `cargo`/`rustc` are NOT on PATH):**
- Native tests: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
- WASM type-check: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
- WASM build: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
- Web checks (in `web/`): `npm run check`, `npx vitest run`, `npm run build`

> Confirm the dated toolchain name with `ls ~/.rustup/toolchains/` — the date drifts.

---

## File Structure

- **Create** `src/theory/shells.rs` — pure shell-pair resolver (table + literal fallback). One responsibility: chord quality → two pitch-class shells.
- **Modify** `src/theory/mod.rs` — register `pub mod shells;`.
- **Modify** `src/theory/line_engine.rs` — extract `run_pattern`; keep `generate_line` behavior-identical; add `resolve_shell_notes` + `generate_shell_line`.
- **Modify** `src/wasm_api.rs` — add `generate_shell_etude` export.
- **Modify** `web/src/wasm.d.ts` — declare the new export (hand-written ambient shadow; required or svelte-check fails).
- **Modify** `web/src/lib/wasm.ts` — typed `generateShellEtude` wrapper.
- **Modify** `web/src/routes/gmc/tune/+page.svelte` — source toggle (Triad Pairs / Shell Étude) branching the generate call; reuse rendering + bass.

---

## Task 1: Shell-pair resolver (`shells.rs`)

**Files:**
- Create: `src/theory/shells.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/theory/shells.rs`
- Modify: `src/theory/mod.rs` (add `pub mod shells;`)

- [ ] **Step 1: Register the module**

In `src/theory/mod.rs`, add the line (keep alphabetical order — between `scales` and `walking_bass`):

```rust
pub mod shells;
```

- [ ] **Step 2: Write the failing test**

Create `src/theory/shells.rs` with ONLY the tests first (the function is referenced but not yet defined, so it fails to compile = a failing test):

```rust
//! Guide-tone "7no5 / 7no3" shell pairs per chord quality (pure core).
//!
//! Distilled from `materiais/meus/Airegin 7no5:7no3 etude.gp`: over each chord the line
//! draws from two 3-note upper-structure shells — a `7no5` (1-3-7 shape) and a `7no3`
//! (1-5-7 shape) — that together spell the chord's extended color. The table is in
//! degrees relative to the chord root, so it is transposition-invariant. Qualities absent
//! from the table fall back to literal shells built from the chord's own intervals.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::ChordQuality;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    // Roots: C=0, Db=1, D=2, Eb=3, E=4, F=5, F#=6, G=7, Ab=8, A=9, Bb=10, B=11.

    #[test]
    fn m7_shells_match_airegin_labels() {
        // Fm7 (Eb, Bb, D + Ab, C, G) — offsets b7,11,13 | b3,5,9 from F=5.
        let (a, b) = resolve_shell_pair(5, quality("m7"));
        assert_eq!(a, [3, 10, 2]); // Eb, Bb, D
        assert_eq!(b, [8, 0, 7]); // Ab, C, G
    }

    #[test]
    fn maj7_shells_are_lydian_and_match_labels() {
        // C∆7 (B, F#, A + E, G, D) — offsets 7,#11,13 | 3,5,9 from C=0.
        let (a, b) = resolve_shell_pair(0, quality("maj7"));
        assert_eq!(a, [11, 6, 9]); // B, F#, A
        assert_eq!(b, [4, 7, 2]); // E, G, D
    }

    #[test]
    fn dominant_defaults_to_altered_shells() {
        // G7 → 7alt shells (G7alt label: F, B, Eb + Bb, Db, Ab) from G=7.
        let (a, b) = resolve_shell_pair(7, quality("dom7"));
        assert_eq!(a, [5, 11, 3]); // F, B, Eb
        assert_eq!(b, [10, 1, 8]); // Bb, Db, Ab
    }

    #[test]
    fn m7b5_shells_match_labels() {
        // Cm7b5 (Bb, F, Ab + Eb, Gb, D) — offsets b7,11,b13 | b3,b5,9 from C=0.
        let (a, b) = resolve_shell_pair(0, quality("m7b5"));
        assert_eq!(a, [10, 5, 8]); // Bb, F, Ab
        assert_eq!(b, [3, 6, 2]); // Eb, Gb, D
    }

    #[test]
    fn absent_quality_falls_back_to_literal_shells() {
        // dim7 has no table entry → literal: 7no5 = root-3-7, 7no3 = root-5-7.
        // C dim7 intervals = [1, b3, b5, bb7] = [0, 3, 6, 9].
        let (a, b) = resolve_shell_pair(0, quality("dim7"));
        assert_eq!(a, [0, 3, 9]); // root, b3, bb7
        assert_eq!(b, [0, 6, 9]); // root, b5, bb7
    }

    #[test]
    fn shells_are_transposition_invariant() {
        // Same m7 shape one whole step up (F=5 → G=7): every pc shifts by +2 mod 12.
        let (a5, b5) = resolve_shell_pair(5, quality("m7"));
        let (a7, b7) = resolve_shell_pair(7, quality("m7"));
        for i in 0..3 {
            assert_eq!(a7[i], (a5[i] + 2) % 12);
            assert_eq!(b7[i], (b5[i] + 2) % 12);
        }
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib shells`
Expected: FAIL — compile error `cannot find function resolve_shell_pair`.

- [ ] **Step 4: Write the implementation**

Add ABOVE the `#[cfg(test)]` block in `src/theory/shells.rs`:

```rust
use crate::theory::chords::ChordQuality;

/// `[7no5, 7no3]` shells as semitone offsets from the chord root.
/// Degree→semitone: 1=0 b9=1 9=2 b3=3 3=4 11=5 #11=6 5=7 #5/b13=8 13=9 b7=10 7=11.
type ShellPair = [[i8; 3]; 2];

const MAJ7: ShellPair = [[11, 6, 9], [4, 7, 2]]; //  7,#11,13 | 3,5,9   (Lydian)
const ALT: ShellPair = [[10, 4, 8], [3, 6, 1]]; //  b7,3,b13 | #9,#11,b9 (altered)
const M7: ShellPair = [[10, 5, 9], [3, 7, 2]]; //   b7,11,13 | b3,5,9  (Dorian)
const M7B5: ShellPair = [[10, 5, 8], [3, 6, 2]]; // b7,11,b13 | b3,b5,9 (Locrian #2)

/// Table lookup by quality family. Order matters: `maj*` and `m7b5`/`m9b11` are matched
/// before the generic minor branch (they also start with 'm'). Dominants all map to the
/// altered shells (the corpus treats every dominant as alt). `None` → literal fallback.
fn table_for(quality: &ChordQuality) -> Option<ShellPair> {
    let n = quality.name;
    if n.starts_with("maj") {
        Some(MAJ7)
    } else if n == "m7b5" || n == "m9b11" {
        Some(M7B5)
    } else if n.starts_with('m') {
        Some(M7)
    } else if n.starts_with("dom") {
        Some(ALT)
    } else {
        None
    }
}

/// Literal shells from the chord's own tones: `7no5 = root-3-7`, `7no3 = root-5-7`.
/// For every `ChordQuality`, `intervals[1..=3]` are the 3rd/5th/7th (all < 12 semitones).
fn literal(quality: &ChordQuality) -> ShellPair {
    let s = |i: usize| quality.intervals.get(i).map(|iv| iv.semitones as i8).unwrap_or(0);
    [[0, s(1), s(3)], [0, s(2), s(3)]]
}

/// Resolve a chord's two guide-tone shells to concrete pitch classes (0..11), in offset
/// order (not sorted) so a `Shape::Order`/`Anchor` can target a specific voice later.
pub fn resolve_shell_pair(root_pc: u8, quality: &ChordQuality) -> ([u8; 3], [u8; 3]) {
    let pair = table_for(quality).unwrap_or_else(|| literal(quality));
    let pc = |off: i8| (((root_pc as i16 + off as i16) % 12 + 12) % 12) as u8;
    (
        [pc(pair[0][0]), pc(pair[0][1]), pc(pair[0][2])],
        [pc(pair[1][0]), pc(pair[1][1]), pc(pair[1][2])],
    )
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib shells`
Expected: PASS — 6 tests.

- [ ] **Step 6: Commit**

```bash
git add src/theory/shells.rs src/theory/mod.rs
git commit -m "feat(gmc): shell-pair resolver distilled from Airegin étude"
```

---

## Task 2: Extract `run_pattern` (pure refactor, GMC untouched)

The existing note-walking loop is extracted so a second note-source can drive it. `generate_line`'s signature and behavior are unchanged — the existing line-engine tests are the safety net.

**Files:**
- Modify: `src/theory/line_engine.rs:91-232` (the `generate_line` body)

- [ ] **Step 1: Confirm the safety-net tests pass before refactoring**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib line_engine`
Expected: PASS (record the count, e.g. "N passed").

- [ ] **Step 2: Replace the `generate_line` body with a thin wrapper + extracted `run_pattern`**

In `src/theory/line_engine.rs`, replace the whole current function body of `generate_line` (from `let beat_dur = config.figure.beat_duration();` down to the final `events` / closing `}` of the function) so the function becomes:

```rust
pub fn generate_line(
    chart: &Chart,
    scale_overrides: &[Option<usize>],
    fretboard: &Fretboard,
    pair: &TriadPairSet,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    // Pre-resolve GMC triad-pair notes per chord, then walk the pattern.
    let triad_notes_per_chord: Vec<TriadNotes> = chart
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let scale = scale_overrides
                .get(i)
                .and_then(|opt| opt.and_then(|idx| Scale::ALL.get(idx)))
                .unwrap_or_else(|| scale_defaults::default_scale(change.quality));
            resolve_triad_notes(change.root_pc, scale, pair, &config.position, fretboard)
        })
        .collect();
    run_pattern(chart, config, &triad_notes_per_chord)
}

/// Walk the configured pattern over already-resolved per-chord note pools, voice-leading
/// each event to the previous. Shared by the GMC triad-pair line and the shell étude — the
/// ONLY thing that differs between them is how `triad_notes_per_chord` was resolved.
fn run_pattern(
    chart: &Chart,
    config: &LineConfig,
    triad_notes_per_chord: &[TriadNotes],
) -> Vec<NoteEvent> {
    let beat_dur = config.figure.beat_duration();
    let total_beats = chart.total_beats();
    let total_events = (total_beats / beat_dur).round() as usize;

    let mut events = Vec::with_capacity(total_events);
    let mut pattern_iter = config.pattern.iter();
    let mut current_direction = Direction::Ascending;
    let mut current_midi: i32 = 0;
    let mut first_note = true;

    // Pre-compute chord boundaries
    let mut chord_boundaries: Vec<(f32, f32, usize)> = Vec::new();
    let mut cumulative = 0.0_f32;
    for (i, change) in chart.changes.iter().enumerate() {
        chord_boundaries.push((cumulative, cumulative + change.beats, i));
        cumulative += change.beats;
    }

    let mut block_remaining = 0u8;
    let mut block_triad = TriadId::T1;
    let mut block_first = false;
    let mut block_shape = Shape::Monotonic;
    let mut block_anchor = Anchor::Nearest;
    let mut block_step = 0usize; // index of the note within the current block (for Shape::Order)

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
                block_shape = block.shape.clone();
                block_anchor = block.anchor;
                block_first = true;
                block_step = 0;
            }
        }

        let pool = triad_notes.notes_for(block_triad);

        if pool.is_empty() {
            block_remaining = block_remaining.saturating_sub(1);
            block_step += 1;
            continue;
        }
        let pcs = triad_notes.pcs_for(block_triad);
        let reference = if first_note { -1000 } else { current_midi };

        let chosen = match &block_shape {
            Shape::Order(order) => {
                let role = (order[block_step % order.len()] % 3) as usize;
                nearest_of_pc(pool, pcs[role], reference).or_else(|| pool.first())
            }
            Shape::Monotonic => {
                if first_note {
                    match block_anchor.role() {
                        Some(r) => nearest_of_pc(pool, pcs[r], reference).or_else(|| pool.first()),
                        None => pool.first(),
                    }
                } else if block_first {
                    match block_anchor.role() {
                        Some(r) => nearest_of_pc(pool, pcs[r], current_midi)
                            .or_else(|| find_closest(pool, current_midi)),
                        None => find_closest(pool, current_midi),
                    }
                } else {
                    let candidate = find_nearest(pool, current_midi, current_direction);
                    if candidate.is_some() {
                        candidate
                    } else {
                        current_direction = current_direction.invert();
                        let inverted = find_nearest(pool, current_midi, current_direction);
                        if inverted.is_some() {
                            inverted
                        } else {
                            find_closest(pool, current_midi)
                        }
                    }
                }
            }
        };
        block_first = false;

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
        block_step += 1;
    }

    events
}
```

- [ ] **Step 3: Run the safety-net tests to verify identical behavior**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib line_engine`
Expected: PASS — same count as Step 1 (refactor changed no behavior).

- [ ] **Step 4: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "refactor(gmc): extract run_pattern from generate_line"
```

---

## Task 3: `generate_shell_line` (shell notes → the shared walker)

**Files:**
- Modify: `src/theory/line_engine.rs` (imports; add `resolve_shell_notes` + `generate_shell_line`; add tests)

- [ ] **Step 1: Add imports**

At the top of `src/theory/line_engine.rs`, change:

```rust
use crate::theory::chart::Chart;
```
to:
```rust
use crate::theory::chart::{Chart, ChordChange};
use crate::theory::shells;
```

- [ ] **Step 2: Write the failing tests**

Add inside the `#[cfg(test)] mod tests { ... }` block (before its closing `}`):

```rust
    #[test]
    fn shell_line_outlines_each_chord_with_its_shells() {
        // Every generated event's pitch class must belong to one of the chord's two
        // shells — the étude never plays a note outside the distilled material.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("MN", "| Em7 A7 | Fm7 Bb7 | Ebmaj7 |").unwrap();
        let config = simple_config();
        let events = generate_shell_line(&chart, &fb, &config);
        assert!(!events.is_empty());

        // Rebuild the allowed pc set per chord and assert membership.
        let mut cumulative = 0.0_f32;
        let bounds: Vec<(f32, f32, usize)> = chart
            .changes
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let s = cumulative;
                cumulative += c.beats;
                (s, cumulative, i)
            })
            .collect();
        for e in &events {
            let idx = bounds
                .iter()
                .rposition(|&(s, _, _)| e.beat >= s)
                .unwrap_or(0);
            let change = &chart.changes[idx];
            let (a, b) = shells::resolve_shell_pair(change.root_pc, change.quality);
            let allowed: Vec<u8> = a.iter().chain(b.iter()).copied().collect();
            assert!(
                allowed.contains(&e.pitch_class),
                "pc {} at beat {} not in shells of chord {}",
                e.pitch_class,
                e.beat,
                idx
            );
        }
    }

    #[test]
    fn shell_line_event_count_matches_grid() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("MN", "| Em7 A7 | Fm7 Bb7 |").unwrap();
        let config = simple_config(); // Eighth figure → 8 events per 4/4 bar
        let events = generate_shell_line(&chart, &fb, &config);
        assert_eq!(events.len(), 16);
    }
```

- [ ] **Step 3: Run to verify failure**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib line_engine::tests::shell_line`
Expected: FAIL — `cannot find function generate_shell_line`.

- [ ] **Step 4: Implement the resolver + entry point**

In `src/theory/line_engine.rs`, add directly after `resolve_triad_notes` (it ends around line 65):

```rust
/// Resolve a chord's two guide-tone shells into fretboard note pools, mirroring
/// `resolve_triad_notes` but sourcing pitch classes from `shells` (chord-quality driven)
/// instead of a scale partition. The scale is irrelevant here, so none is passed.
fn resolve_shell_notes(
    change: &ChordChange,
    position: &NeckPosition,
    fretboard: &Fretboard,
) -> TriadNotes {
    let (pcs_a, pcs_b) = shells::resolve_shell_pair(change.root_pc, change.quality);
    TriadNotes {
        t1: position.find_notes(fretboard, &pcs_a),
        t2: position.find_notes(fretboard, &pcs_b),
        t1_pcs: pcs_a,
        t2_pcs: pcs_b,
    }
}

/// Generate a guide-tone "shell étude" line (Motor E) over a chart. Same pattern walker as
/// `generate_line`; the only difference is the per-chord note source — two chord-quality
/// shells instead of a GMC triad pair. No `scale_overrides`/`pair` apply.
pub fn generate_shell_line(
    chart: &Chart,
    fretboard: &Fretboard,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    let triad_notes_per_chord: Vec<TriadNotes> = chart
        .changes
        .iter()
        .map(|change| resolve_shell_notes(change, &config.position, fretboard))
        .collect();
    run_pattern(chart, config, &triad_notes_per_chord)
}
```

- [ ] **Step 5: Run to verify pass (and the whole library still green)**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
Expected: PASS — all tests including the two new `shell_line` ones.

- [ ] **Step 6: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "feat(gmc): generate_shell_line — shell étude over any chart"
```

---

## Task 4: WASM export `generate_shell_etude`

Mirrors `generate_gmc_line` but drops `pair_index`/`scale_overrides` (shells ignore the scale) and calls `generate_shell_line`. Returns the same `{events, changes, totalBeats}` shape so the web rendering is reused verbatim.

**Files:**
- Modify: `src/wasm_api.rs` (add the export after `generate_gmc_line`, ~line 718)
- Modify: `web/src/wasm.d.ts` (declare the export)

- [ ] **Step 1: Add the Rust export**

In `src/wasm_api.rs`, immediately after the closing `}` of `generate_gmc_line` (the `to_js(&serde_json::json!({ "events": ..., "changes": ..., "totalBeats": ... }))` block), add:

```rust
/// Generate a guide-tone "shell étude" line (Motor E) over a chart. Same controls as the
/// GMC line minus the triad pair and scale overrides — the two 3-note shells per chord are
/// derived from the chord quality (see `theory::shells`). Response shape matches
/// `generate_gmc_line` so the web renderer is shared.
#[wasm_bindgen]
pub fn generate_shell_etude(
    chart_text: &str,
    title: &str,
    figure_index: usize, // 0=Eighth, 1=Sixteenth, 2=Triplet
    position_fret: u8,   // base fret 1-12
    pattern_js: JsValue, // array of {count, direction, triad, shape?, anchor?}
) -> JsValue {
    use crate::theory::line_engine::{self, LineConfig};
    use crate::theory::line_pattern::{Anchor, Direction, Pattern, PatternBlock, RhythmicFigure, Shape, TriadId};
    use crate::theory::position::NeckPosition;
    use crate::theory::scale_defaults;

    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let blocks_raw: Vec<serde_json::Value> =
        serde_wasm_bindgen::from_value(pattern_js).unwrap_or_default();
    let blocks: Vec<PatternBlock> = blocks_raw
        .iter()
        .map(|b| {
            let count = b["count"].as_u64().unwrap_or(3).clamp(1, 6) as u8;
            let direction = if b["direction"].as_str() == Some("desc") {
                Direction::Descending
            } else {
                Direction::Ascending
            };
            let triad = if b["triad"].as_str() == Some("T2") {
                TriadId::T2
            } else {
                TriadId::T1
            };
            let shape = match b["shape"].as_array() {
                Some(arr) if !arr.is_empty() => {
                    let order: Vec<u8> =
                        arr.iter().filter_map(|v| v.as_u64()).map(|r| (r % 3) as u8).collect();
                    if order.is_empty() { Shape::Monotonic } else { Shape::Order(order) }
                }
                _ => Shape::Monotonic,
            };
            let anchor = match b["anchor"].as_str() {
                Some("root") => Anchor::Root,
                Some("third") => Anchor::Third,
                Some("fifth") => Anchor::Fifth,
                _ => Anchor::Nearest,
            };
            PatternBlock { count, direction, triad, shape, anchor }
        })
        .collect();

    if blocks.is_empty() {
        return to_js(&serde_json::json!({"error": "empty pattern"}));
    }

    let pattern = Pattern { name: "shell", blocks };
    let figure = match figure_index {
        1 => RhythmicFigure::Sixteenth,
        2 => RhythmicFigure::Triplet,
        _ => RhythmicFigure::Eighth,
    };

    let config = LineConfig { pattern, figure, position: NeckPosition::new(position_fret) };
    let fretboard = Fretboard::standard_tuning();
    let events = line_engine::generate_shell_line(&chart, &fretboard, &config);

    let changes_info: Vec<_> = chart
        .changes
        .iter()
        .map(|c| {
            let default_scale = scale_defaults::default_scale(c.quality);
            serde_json::json!({
                "chord": chords::chord_name(&c.root, c.quality),
                "rootPc": c.root_pc,
                "bassPc": c.bass_pc,
                "beats": c.beats,
                "defaultScale": default_scale.name,
                "defaultScaleIndex": Scale::ALL.iter().position(|s| s.name == default_scale.name),
                "activeScale": default_scale.name,
                "isOverride": false,
            })
        })
        .collect();

    let events_json: Vec<_> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "beat": e.beat,
                "string": e.string,
                "fret": e.fret,
                "triad": if e.triad == TriadId::T1 { "T1" } else { "T2" },
                "pitchClass": e.pitch_class,
                "midi": e.midi,
            })
        })
        .collect();

    to_js(&serde_json::json!({
        "events": events_json,
        "changes": changes_info,
        "totalBeats": chart.total_beats(),
    }))
}
```

- [ ] **Step 2: Type-check the wasm build**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: PASS (no errors).

- [ ] **Step 3: Declare the export in the hand-written ambient types**

In `web/src/wasm.d.ts`, after the `generate_gmc_line` declaration (line 20), add:

```typescript
  export function generate_shell_etude(chart_text: string, title: string, figure_index: number, position_fret: number, pattern_js: any): any;
```

- [ ] **Step 4: Rebuild the wasm package**

Run: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
Expected: build succeeds; `web/pkg/chordz.js` now exports `generate_shell_etude`.

- [ ] **Step 5: Commit**

```bash
git add src/wasm_api.rs web/src/wasm.d.ts web/pkg
git commit -m "feat(gmc): generate_shell_etude wasm export"
```

---

## Task 5: Typed web wrapper `generateShellEtude`

**Files:**
- Modify: `web/src/lib/wasm.ts` (after `generateGmcLine`, ~line 207)

- [ ] **Step 1: Add the wrapper**

In `web/src/lib/wasm.ts`, directly after the `generateGmcLine` function (it ends at the line returning `getWasm().generate_gmc_line(...)`), add:

```typescript
/**
 * Shell-étude (Motor E) line over a chart. Same result shape as generateGmcLine, so the
 * GMC-tune renderer is reused. No pair/scale-override — the two shells per chord come from
 * the chord quality (see Rust `theory::shells`).
 */
export function generateShellEtude(
  chartText: string,
  title: string,
  figureIndex: number,
  positionFret: number,
  pattern: GmcPatternBlock[],
): GmcLineResult {
  return getWasm().generate_shell_etude(chartText, title, figureIndex, positionFret, pattern);
}
```

- [ ] **Step 2: Type-check the web project**

Run: `cd web && npm run check`
Expected: PASS (svelte-check finds the new export typed; 0 errors).

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/wasm.ts
git commit -m "feat(gmc): generateShellEtude web wrapper"
```

---

## Task 6: Source toggle in the GMC-tune page

Add a "Shell Étude" toggle. When on, the generate call uses `generateShellEtude` (pair/scale ignored) and the pair selector is hidden; rendering, playback, and the existing bass toggle are reused unchanged.

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Import the wrapper**

In the `<script>` block, extend the import on line 4:

```typescript
  import { generateGmcLine, generateShellEtude, getPresets, getPairs, getAllScales } from '$lib/wasm';
```

- [ ] **Step 2: Add the mode state**

Next to the other control state (near `let pairIndex = $state(0);`, line 47), add:

```typescript
  let etudeMode = $state(false);
```

- [ ] **Step 3: Branch the two generate call sites**

There are two `generateGmcLine(...)` calls (≈ line 151 in `generate()` and ≈ line 179 in the live-regenerate path). Replace EACH call expression:

```typescript
    const res = generateGmcLine(
      presetText,
      title,
      pairIndex,
      scaleOverrides,
      figureIndex,
      positionFret,
      pattern,
    );
```

with the branch (keep each call's existing surrounding variable names — only the call expression changes):

```typescript
    const res = etudeMode
      ? generateShellEtude(presetText, title, figureIndex, positionFret, pattern)
      : generateGmcLine(presetText, title, pairIndex, scaleOverrides, figureIndex, positionFret, pattern);
```

> If the two sites use different local names than `presetText`/`title`/`pattern`, mirror the names already used at that site — do not rename anything.

- [ ] **Step 4: Add the toggle control and hide the pair selector in étude mode**

Find the pair `<select>` (line ≈ 433: `<select class="control-select" bind:value={pairIndex}>`). Wrap it so it only shows when NOT in étude mode, and add a toggle button just before it:

```svelte
        <button
          class="filter-btn"
          class:active={etudeMode}
          onclick={() => { etudeMode = !etudeMode; generate(); }}
          title="Guide-tone 7no5/7no3 shells per chord (Motor E)"
        >Shell Étude</button>
        {#if !etudeMode}
          <select class="control-select" bind:value={pairIndex}>
```

and add the matching `{/if}` immediately after that `<select>`'s closing `</select>` tag.

> Confirm the `<select …>` … `</select>` span you wrapped is balanced (the `{#if}` opens before `<select` and `{/if}` closes after `</select>`).

- [ ] **Step 5: Type-check and build the web app**

Run: `cd web && npm run check && npm run build`
Expected: PASS — 0 svelte-check errors; production build succeeds.

- [ ] **Step 6: Manual verification (the audible result)**

Run the dev server (`cd web && npm run dev`), open the GMC tune page, select the **Moment's Notice** preset, click **Shell Étude**, then **generate** and **play**. Confirm: a line renders as tab over every bar, plays back, and (with the Bass toggle) the bass sounds. Spot-check bar 1 (Em7) — the line's notes should be drawn from `{D,A,F#}`/`{G,B,C#}`-type guide-tone shells (b7-11-13 / b3-5-9 of Em7), not random scale tones.

- [ ] **Step 7: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): Shell Étude toggle in GMC tune page"
```

---

## Task 7 (optional, follow-up): faithful half-note shell bass

The spec's "root + guide-tone half-note bass" is deferred — v1 reuses the existing walking-bass toggle, which already works over any chart. Only do this task if the walking line feels too busy under the étude.

**Files:**
- Modify: `src/theory/walking_bass.rs` or new `src/theory/shell_bass.rs`; `src/wasm_api.rs`; `web/src/lib/wasm.ts`; `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Write a failing test** for `shell_bass_line(segments) -> Vec<BassNote>` asserting two half notes per chord — the root (low octave) on beat 0 and a guide tone (3rd or 5th from `quality.intervals`) on beat 2, each `beats: 2.0`.
- [ ] **Step 2:** Run it; verify it fails.
- [ ] **Step 3: Implement** `shell_bass_line`: for each segment emit `BassNote { midi: root in octave 2, beat: offset, beats: 2.0 }` and `BassNote { midi: root + (5th or 3rd), beat: offset + 2.0, beats: 2.0 }`, clamped to the existing bass register constants.
- [ ] **Step 4:** Run; verify pass.
- [ ] **Step 5: Expose** via a `bass` array on the `generate_shell_etude` response (shape `{midi, beat, beats}`) and schedule it in the page with the existing `scheduleBassLine` when étude mode + bass are on.
- [ ] **Step 6:** `npm run check && npm run build`; manual play-through.
- [ ] **Step 7: Commit.**

---

## Self-Review

**Spec coverage:**
- Distilled shell table (m7/7alt/maj7/maj7#5/m7b5) → Task 1 (`MAJ7`/`ALT`/`M7`/`M7B5`; `maj7#5` has no matching `ChordQuality`, folded into `maj7`/Lydian — noted).
- Upper-structure default + literal fallback → Task 1 (`table_for` / `literal`).
- Dominants default to alt → Task 1 (`starts_with("dom") → ALT`) + test.
- Minimal architecture, GMC untouched → Tasks 2-3 (`run_pattern` extraction; `generate_line` signature unchanged; safety-net tests).
- `.gp` as oracle → Task 1 unit tests (corpus labels) + Task 3 per-bar membership test.
- WASM + web exposure → Tasks 4-6.
- Bass voice → reused walking bass (Task 6) with faithful half-note bass deferred to Task 7 (a documented spec deviation for the user to veto).
- First target Moment's Notice → Task 3 tests + Task 6 manual verification.

**Placeholder scan:** No TBD/TODO in Tasks 1-6; every code step shows complete code. Task 7 is explicitly optional/deferred with concrete-enough steps.

**Type consistency:** `resolve_shell_pair(u8, &ChordQuality) -> ([u8;3],[u8;3])` used identically in `shells.rs`, the line-engine test, and `resolve_shell_notes`. `generate_shell_line(&Chart, &Fretboard, &LineConfig)` matches its caller in `generate_shell_etude`. `generateShellEtude(chartText, title, figureIndex, positionFret, pattern)` matches the wasm export arg order and the web call sites. `GmcLineResult` reused (same response shape).

**Open deviation flagged for the user:** v1 reuses the existing walking bass instead of writing the simple half-note bass from the spec (Task 7 covers the faithful version if wanted).
