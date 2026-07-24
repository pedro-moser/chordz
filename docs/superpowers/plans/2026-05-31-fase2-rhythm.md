# Fase 2 — Per-Block Rhythm Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a held landing note (`hold_last`) and a pickup rest (`lead_rest`) per pattern block, on top of a `NoteEvent.duration` + time-cursor substrate that decouples a pattern step from time.

**Architecture:** `NoteEvent` gains a `duration`; `run_pattern` swaps its index-derived onset for an accumulated `cursor` (a `while cursor < total_beats` loop). Two `u8` fields on `PatternBlock` — `hold_last` (last note sustains `1+n` grid steps) and `lead_rest` (n grid steps of pure-gap silence before the block) — drive the new durations/gaps. Downstream just reads `duration`; audio is already duration-native.

**Tech Stack:** Rust core (cargo, native nightly), `wasm-bindgen`/`wasm-pack`, SvelteKit (TypeScript), vitest.

**Spec:** `docs/superpowers/specs/2026-05-31-fase2-rhythm-design.md`

**Toolchain (cargo/rustc/wasm-pack NOT on PATH):**
- Native tests: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
- WASM type-check: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
- WASM build: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
- Web checks (in `web/`): `npm run check`, `npm run build`

> Confirm the dated toolchain name with `ls ~/.rustup/toolchains/`.

**Key facts:** `RhythmicFigure::beat_duration()` = Eighth 0.5, Sixteenth 0.25, Triplet 1/3 (`line_pattern.rs:48-54`). `PatternBlock` does NOT derive `Default`; it's built via `PatternBlock::legacy()` (covers all 3 core presets), one struct literal in the line_engine tests (`:257`), and `parse_pattern_blocks` (`wasm_api.rs:611`). `NoteEvent` is constructed at exactly one site (the `events.push` in `run_pattern`).

---

## File Structure

- **Modify** `src/theory/line_pattern.rs` — add `hold_last`/`lead_rest` to `PatternBlock`; update `legacy()`.
- **Modify** `src/theory/line_engine.rs` — `NoteEvent.duration`; refactor `run_pattern` to a cursor; read the two fields; tests.
- **Modify** `src/wasm_api.rs` — `parse_pattern_blocks` reads the two fields; `line_events_json` serializes `duration`.
- **Modify** `web/src/lib/wasm.ts` — `GmcLineEvent.duration`; `GmcPatternBlock.holdLast?/leadRest?`.
- **Modify** `web/src/routes/gmc/tune/+page.svelte` — `playThrough` uses `e.duration`; two number inputs + mutators per block.

---

## Task 1: `PatternBlock` gains `hold_last` / `lead_rest` (mechanical, no behavior change)

**Files:**
- Modify: `src/theory/line_pattern.rs` (struct + `legacy()`)
- Modify: `src/theory/line_engine.rs:257` (the one test literal)
- Modify: `src/wasm_api.rs` (`parse_pattern_blocks`)

- [ ] **Step 1: Add the two fields to the struct**

In `src/theory/line_pattern.rs`, the `PatternBlock` struct (the `pub struct PatternBlock { … }` with fields count/direction/triad/shape/anchor) gains two fields at the end:

```rust
    /// The block's landing/first note. Defaults to `Nearest` (legacy connect).
    pub anchor: Anchor,
    /// The block's LAST note sustains `1 + hold_last` grid steps (a held landing). 0 = off.
    pub hold_last: u8,
    /// `lead_rest` grid steps of silence before the block's first note (a pickup). 0 = off.
    pub lead_rest: u8,
}
```

In the same file, `PatternBlock::legacy(...)` gains the two fields:

```rust
    pub fn legacy(count: u8, direction: Direction, triad: TriadId) -> Self {
        Self {
            count,
            direction,
            triad,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 0,
        }
    }
```

- [ ] **Step 2: Update the test struct-literal**

In `src/theory/line_engine.rs`, line ~257 (inside `shaped_config`), change:

```rust
                blocks: vec![PatternBlock { count, direction: Direction::Ascending, triad, shape, anchor }],
```
to:
```rust
                blocks: vec![PatternBlock { count, direction: Direction::Ascending, triad, shape, anchor, hold_last: 0, lead_rest: 0 }],
```

- [ ] **Step 3: Read the fields in `parse_pattern_blocks`**

In `src/wasm_api.rs`, `parse_pattern_blocks`, just before the final `PatternBlock { … }` construction (after the `anchor` match), add:

```rust
            // Pickup-rest and held-landing per block (Fase 2). Clamp at the trust boundary.
            let hold_last = b["holdLast"].as_u64().unwrap_or(0).min(16) as u8;
            let lead_rest = b["leadRest"].as_u64().unwrap_or(0).min(16) as u8;
            PatternBlock { count, direction, triad, shape, anchor, hold_last, lead_rest }
```
(replacing the existing `PatternBlock { count, direction, triad, shape, anchor }` line).

- [ ] **Step 4: Verify native + wasm compile, existing tests pass**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
Expected: PASS (no behavior change — `run_pattern` doesn't read the new fields yet).
Run: `PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: 0 errors.

- [ ] **Step 5: Commit**

```bash
git add src/theory/line_pattern.rs src/theory/line_engine.rs src/wasm_api.rs
git commit -m "feat(gmc): add hold_last/lead_rest fields to PatternBlock"
```

---

## Task 2: `NoteEvent.duration` + cursor refactor of `run_pattern`

**Files:**
- Modify: `src/theory/line_engine.rs` (NoteEvent struct; `run_pattern` body; new tests)

- [ ] **Step 1: Confirm the safety-net tests pass first**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib line_engine`
Expected: PASS (record the count).

- [ ] **Step 2: Add `duration` to `NoteEvent`**

In `src/theory/line_engine.rs`, the `NoteEvent` struct gains a field:

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
}
```

- [ ] **Step 3: Write the failing tests**

In `src/theory/line_engine.rs`'s `#[cfg(test)] mod tests`, add (the test module already imports `Pattern, PatternBlock, Direction, RhythmicFigure, Shape, TriadId, Anchor, NeckPosition, Fretboard, PAIRS`):

```rust
    fn one_block_config(block: PatternBlock, figure: RhythmicFigure) -> LineConfig {
        LineConfig {
            pattern: Pattern { name: "t", blocks: vec![block] },
            figure,
            position: NeckPosition::new(5),
        }
    }

    #[test]
    fn legacy_blocks_have_uniform_duration() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config(); // hold_last/lead_rest = 0 (preset_alternating), Eighth
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 8); // unchanged grid count
        for e in &events {
            assert_eq!(e.duration, 0.5); // beat_dur for Eighth
        }
    }

    #[test]
    fn hold_last_sustains_the_final_note_of_the_block() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let block = PatternBlock {
            count: 2, direction: Direction::Ascending, triad: TriadId::T1,
            shape: Shape::Monotonic, anchor: Anchor::Nearest, hold_last: 1, lead_rest: 0,
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &one_block_config(block, RhythmicFigure::Eighth));
        // block = [note, note(held)]. The 2nd note sustains (1+1)*0.5 = 1.0 and pushes the next.
        assert_eq!(events[0].duration, 0.5);
        assert_eq!(events[1].duration, 1.0);
        assert_eq!(events[2].beat, 1.5);
    }

    #[test]
    fn lead_rest_inserts_a_pickup_gap() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let block = PatternBlock {
            count: 4, direction: Direction::Ascending, triad: TriadId::T1,
            shape: Shape::Monotonic, anchor: Anchor::Nearest, hold_last: 0, lead_rest: 1,
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &one_block_config(block, RhythmicFigure::Eighth));
        // A 1-step rest precedes the first note → it lands at 0.5, not 0.
        assert_eq!(events[0].beat, 0.5);
    }
```

- [ ] **Step 4: Run to verify failure**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib line_engine`
Expected: FAIL — `duration` field missing on the push site (compile error) and the new tests' assertions.

- [ ] **Step 5: Refactor `run_pattern` to a cursor + apply the fields + set duration**

In `src/theory/line_engine.rs`, replace the body of `run_pattern` from `let beat_dur = …` through the final `events` return with:

```rust
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
    let mut block_hold_last = 0u8;

    // Time cursor (replaces `event_idx * beat_dur`). The epsilon absorbs f32 accumulation
    // drift (notably the 1/3 triplet grid) so the legacy event count is unchanged.
    let mut cursor = 0.0_f32;
    const EPS: f32 = 1e-3;

    while cursor < total_beats - EPS {
        // Advance pattern if needed: load the next block and apply its pickup rest.
        if block_remaining == 0 {
            match pattern_iter.next() {
                Some(block) => {
                    block_remaining = block.count;
                    block_triad = block.triad;
                    current_direction = block.direction;
                    block_shape = block.shape.clone();
                    block_anchor = block.anchor;
                    block_hold_last = block.hold_last;
                    block_first = true;
                    block_step = 0;
                    cursor += block.lead_rest as f32 * beat_dur; // pickup rest (pure gap)
                    if cursor >= total_beats - EPS {
                        break;
                    }
                }
                None => break, // empty pattern (guarded upstream); avoid a non-advancing loop
            }
        }

        // The block's last note sustains when hold_last is set.
        let step_dur = if block_remaining == 1 && block_hold_last > 0 {
            (1 + block_hold_last as u32) as f32 * beat_dur
        } else {
            beat_dur
        };

        let beat = cursor;

        // Find which chord we're in
        let chord_idx = chord_boundaries
            .iter()
            .rposition(|&(start, _, _)| beat >= start)
            .unwrap_or(0);

        let triad_notes = &triad_notes_per_chord[chord_idx];
        let pool = triad_notes.notes_for(block_triad);

        if pool.is_empty() {
            block_remaining = block_remaining.saturating_sub(1);
            block_step += 1;
            cursor += step_dur;
            continue;
        }
        let pcs = triad_notes.pcs_for(block_triad);
        // No previous pitch on the very first note: anchor low so the line starts at the
        // bottom of the position (matches the legacy `pool.first()`).
        let reference = if first_note { -1000 } else { current_midi };

        let chosen = match &block_shape {
            Shape::Order(order) => {
                // Play the triad voices in the explicit cyclic role order, each voice-led
                // to the previous note.
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
                    // Connect to the new triad: the anchored voice if requested, else the
                    // nearest distinct note (legacy), then continue in direction.
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
                duration: step_dur,
            });
            current_midi = note.midi;
            first_note = false;
        }

        block_remaining = block_remaining.saturating_sub(1);
        block_step += 1;
        cursor += step_dur;
    }

    events
```

- [ ] **Step 6: Run to verify all pass**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
Expected: PASS — the existing line_engine tests (legacy invariance: counts/beats/pcs unchanged) plus the 3 new rhythm tests.

- [ ] **Step 7: Commit**

```bash
git add src/theory/line_engine.rs
git commit -m "feat(gmc): NoteEvent.duration + cursor; hold_last/lead_rest rhythm"
```

---

## Task 3: Serialize `duration` to the web + rebuild wasm

**Files:**
- Modify: `src/wasm_api.rs` (`line_events_json`)

- [ ] **Step 1: Add `duration` to the event JSON**

In `src/wasm_api.rs`, the `line_events_json` helper builds each event object (with `beat/string/fret/triad/pitchClass/midi`). Add the duration:

```rust
            serde_json::json!({
                "beat": e.beat,
                "string": e.string,
                "fret": e.fret,
                "triad": if e.triad == TriadId::T1 { "T1" } else { "T2" },
                "pitchClass": e.pitch_class,
                "midi": e.midi,
                "duration": e.duration,
            })
```
(add only the `"duration": e.duration,` line to the existing object — match the real field set in the file.)

- [ ] **Step 2: Type-check wasm**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: 0 errors.

- [ ] **Step 3: Rebuild the wasm package**

Run: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
Expected: build succeeds. Verify the field is emitted: `grep -c '"duration"' web/pkg/chordz.js` → ≥ 1 (or confirm the build ran clean — the JSON is built at runtime, so a source grep of wasm_api isn't in chordz.js; just confirm the build succeeded).

- [ ] **Step 4: Commit**

```bash
git add src/wasm_api.rs web/pkg
git commit -m "feat(gmc): serialize NoteEvent.duration to the web"
```
(`web/pkg` is gitignored — the add is a no-op; commit the source.)

---

## Task 4: Web types + duration-driven playback

**Files:**
- Modify: `web/src/lib/wasm.ts` (types)
- Modify: `web/src/routes/gmc/tune/+page.svelte` (`playThrough`)

- [ ] **Step 1: Extend the TS types**

In `web/src/lib/wasm.ts`, add `duration` to `GmcLineEvent`:

```typescript
export interface GmcLineEvent {
  beat: number;
  string: number;
  fret: number;
  triad: 'T1' | 'T2';
  pitchClass: number;
  midi: number;
  duration: number;
}
```

And add the two optional fields to `GmcPatternBlock`:

```typescript
export interface GmcPatternBlock {
  count: number;
  direction: 'asc' | 'desc';
  triad: 'T1' | 'T2';
  /** Voice order by triad role (0,1,2 = scale-index order). Absent/empty = monotonic walk. */
  shape?: number[];
  /** Landing note for the block's first note. Absent = nearest (legacy connect). */
  anchor?: 'root' | 'third' | 'fifth';
  /** Held landing: the block's last note sustains 1+holdLast grid steps. Absent/0 = off. */
  holdLast?: number;
  /** Pickup rest: grid steps of silence before the block's first note. Absent/0 = off. */
  leadRest?: number;
}
```

- [ ] **Step 2: Use `e.duration` in `playThrough`**

In `web/src/routes/gmc/tune/+page.svelte`, `playThrough`, replace the `span` computation (the block that does `const span = i + 1 < evs.length ? evs[i + 1].beat - e.beat : …`) so the note length comes from the baked duration:

```typescript
    const audioNotes = evs.map((e) => ({
      midi: e.midi,
      time: (e.beat - startBeat) * beatSecs,
      duration: Math.max(0.05, e.duration) * beatSecs,
    }));
```
(remove the now-unused index `i` from the `.map((e, i) =>` signature and the gap-to-next logic; `e.duration` already encodes held notes and the uniform grid.)

- [ ] **Step 3: Type-check and build**

Run: `cd web && npm run check && npm run build`
Expected: 0 svelte-check errors; build succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/lib/wasm.ts web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): play notes by their baked duration (held notes sound held)"
```

---

## Task 5: Per-block Hold / Pickup inputs in the pattern editor

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte` (mutators + the pattern-block markup)

- [ ] **Step 1: Add the two mutators**

In the `<script>`, near the other block mutators (`setBlockCount`, `toggleBlockDirection`, …), add:

```typescript
  function setBlockHold(idx: number, val: number) {
    pattern[idx] = { ...pattern[idx], holdLast: Math.max(0, Math.min(16, val || 0)) };
    pattern = [...pattern];
  }

  function setBlockLeadRest(idx: number, val: number) {
    pattern[idx] = { ...pattern[idx], leadRest: Math.max(0, Math.min(16, val || 0)) };
    pattern = [...pattern];
  }
```

- [ ] **Step 2: Add the two inputs to each pattern block**

In the `{#each pattern as block, i}` block markup (the `<div class="pattern-block">` that holds the count input, dir/triad buttons, and voicing select), add — after the voicing `<select>` and before the remove button:

```svelte
            <input
              type="number" min="0" max="16" class="count-input"
              title="Hold the last note (extra grid steps)"
              value={block.holdLast ?? 0}
              oninput={(e) => setBlockHold(i, parseInt((e.target as HTMLInputElement).value))}
            />
            <input
              type="number" min="0" max="16" class="count-input"
              title="Pickup rest before this block (grid steps)"
              value={block.leadRest ?? 0}
              oninput={(e) => setBlockLeadRest(i, parseInt((e.target as HTMLInputElement).value))}
            />
```

(reuse the existing `count-input` class for styling consistency.)

- [ ] **Step 3: Type-check and build**

Run: `cd web && npm run check && npm run build`
Expected: 0 svelte-check errors; build succeeds.

- [ ] **Step 4: Manual check (optional, the human does this)**

`cd web && npm run dev`, GMC → Tune, generate a tune, pick the **Triad-Pair Arch** preset, set the last block's **Hold** to 3, **Generate** + **Play** — the arch's landing note should now sustain (not restrike), and a **Pickup** of 1 on the first block should start the phrase off the downbeat.

- [ ] **Step 5: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): per-block Hold/Pickup inputs in the pattern editor"
```

---

## Self-Review

**Spec coverage:**
- `NoteEvent.duration` substrate → Task 2.
- Time cursor decouple (while loop, EPS for legacy count) → Task 2.
- `hold_last` (sustained landing, `1+n` steps) → Task 1 (field) + Task 2 (logic + test).
- `lead_rest` (pure-gap pickup) → Task 1 (field) + Task 2 (logic + test).
- wasm serialize duration + parse holdLast/leadRest → Task 1 (parse) + Task 3 (serialize).
- JS types + playThrough by duration → Task 4. `scheduleNotes` unchanged (not in any task). Tab unchanged (not in any task).
- UI inputs → Task 5.
- Legacy invariance (counts/beats/pcs unchanged) → Task 2 test `legacy_blocks_have_uniform_duration` + the existing line_engine tests must stay green.

**Placeholder scan:** No TBD/TODO; the full refactored `run_pattern` is spelled out; tests are complete code.

**Type consistency:** `hold_last: u8`/`lead_rest: u8` on `PatternBlock` (Task 1) read in `run_pattern` (Task 2) and `parse_pattern_blocks` (Task 1, keys `holdLast`/`leadRest`). `NoteEvent.duration: f32` (Task 2) serialized as `"duration"` (Task 3), typed `duration: number` (Task 4), consumed in `playThrough` (Task 4). `GmcPatternBlock.holdLast?/leadRest?` (Task 4) set by `setBlockHold`/`setBlockLeadRest` (Task 5) and sent to the wasm parser (Task 1). The cursor `EPS = 1e-3` preserves the legacy triplet count (existing `figure_triplet` test asserts 12).
