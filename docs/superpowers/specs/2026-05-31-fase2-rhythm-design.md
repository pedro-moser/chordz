# Fase 2 — Per-Block Rhythm: Held Note + Pickup Rest — Design

**Date:** 2026-05-31
**Status:** Approved (design), pending implementation plan

## Summary

Give the GMC line engine its first non-uniform rhythm: a **held landing note** (`hold_last`) and a
**pickup rest** (`lead_rest`) per pattern block. This retires the faked "repeated note" that ends
nearly every arch in Pedro's `.gp` études (the #1 gap from the pattern mining) and lets phrases
start off the downbeat. Minimal scope: two per-block controls on top of a small substrate (a time
cursor + a `duration` on each event). The substrate is exactly what a future full per-step rhythm
system would compile onto — a true subset, not throwaway.

## Background

Today the engine emits a **uniform grid**: `run_pattern` (`src/theory/line_engine.rs`) computes each
event's onset as `let beat = event_idx as f32 * beat_dur;` — step N always lands at N·beat_dur and
always emits exactly one note. `NoteEvent` carries `beat` but **no duration**; downstream code
recovers duration only because the grid is uniform (`next.beat − this.beat`). That invariant breaks
the moment a rest (a time gap with no event) or a hold (one note spanning multiple slots) exists.

## Architecture

### Substrate (the one structural refactor)

1. **`NoteEvent` gains `pub duration: f32`** (`line_engine.rs`, the struct). Set at the single
   `events.push` site. Legacy/uniform output sets `duration = beat_dur` → byte-identical.
2. **Time cursor.** Replace the index-derived onset (`let beat = event_idx * beat_dur`) with an
   accumulated `cursor: f32`, and the `for event_idx in 0..total_events` loop with
   `while cursor < total_beats { … }`. Each step advances the cursor by its own duration.
   `total_events` survives only as a `Vec::with_capacity` hint.

### The two controls (per `PatternBlock`)

Added as small integers (not booleans — a boolean 2× hold is musically anemic; a step count gives
real control over the landing/pickup length with the same UI cost):

```rust
// src/theory/line_pattern.rs — PatternBlock gains:
pub hold_last: u8,  // 0 = off. The block's LAST note sustains (1 + hold_last) grid steps.
pub lead_rest: u8,  // 0 = off. `lead_rest` grid steps of silence before the block's first note.
```

Semantics (confirmed with Pedro):
- **`hold_last`** — the block's final note is emitted **once** with `duration = (1 + hold_last) · beat_dur`
  (a sustained long note, **not** a restrike), and the cursor advances by that full duration. The
  phrase **dwells on the landing** (subsequent phrases start later) — exactly how a held landing
  reads. This replaces the repeated-note hack.
- **`lead_rest`** — before the block's first note, advance the cursor by `lead_rest · beat_dur`
  without emitting any event. A **pure gap** (no event on the wire — nothing downstream must "know"
  a rest occurred; the audio already ignores zero/none).

Both default `0` → identical to today's behavior.

### Downstream (small)

- **wasm serializer** (`wasm_api.rs`, `line_events_json`): add `"duration": e.duration`.
- **wasm parser** (`parse_pattern_blocks`): read `holdLast`/`leadRest` (default 0) into the block.
- **JS types** (`web/src/lib/wasm.ts`): `GmcLineEvent` gains `duration: number`; `GmcPatternBlock`
  gains `holdLast?: number` and `leadRest?: number`.
- **`playThrough`** (`web/src/routes/gmc/tune/+page.svelte`): replace the "gap-to-next" span
  (`evs[i+1].beat − e.beat`) with `e.duration · beatSecs`. (No rest events exist on the wire — pure
  gap — so no rest-skipping needed.)
- **`scheduleNotes`** (`web/src/lib/audio.ts`): **no change** — already consumes a per-note duration.
- **Tab render** (`+page.svelte`, `tabX`/glyph): **no change** — note length stays visually implicit;
  audio is correct. (A tie/rest glyph is deferred.)

### UI

Two small number inputs per block in the pattern editor (`+page.svelte`, by the per-block voicing
select): "Hold" (→ `holdLast`) and "Pickup" (→ `leadRest`), each 0–N, default 0.

## Testing

- **Legacy invariance:** with `hold_last = 0, lead_rest = 0`, the cursor loop yields the same beats
  as today and every event's `duration == beat_dur` (a regression guard; the existing line-engine
  tests must also pass unchanged).
- **`hold_last`:** a block with `hold_last = 1` emits its last note with `duration == 2·beat_dur`,
  and the next event's `beat` is pushed out by that extra step.
- **`lead_rest`:** a block with `lead_rest = 1` starts its first note one grid step later (a leading
  gap); the total event count drops accordingly.
- A golden test on a small chart asserting the exact `(beat, duration)` sequence for a `hold_last`
  block.

## Scope cuts (YAGNI)

- No rests/holds **inside** a block (mid-phrase) — only block-edge. That's the full per-step rhythm
  system (a `Vec<RhythmStep>` DSL + a per-step editor), deferred.
- No new Half/Whole `RhythmicFigure` values — `hold_last` covers longer landings.
- No tab length/tie/rest glyph — audio-correct, render-silent for now.
- Out of Fase 2 entirely (per the roadmap): off-pair chromatic approach/enclosure → Fase 5; the
  Sequence/rotation operator → Fase 3.

## Implementation note (legacy-compat caveat)

`PatternBlock` does **not** `#[derive(Default)]`, and presets/tests construct it via struct literals
(`line_pattern.rs` preset functions, `line_engine.rs` test helpers) plus `PatternBlock::legacy()`.
Adding the two fields means each literal gains `hold_last: 0, lead_rest: 0` (mechanical) and
`parse_pattern_blocks` sets them from the JS payload — the same wall the Fase-0/1 `shape`/`anchor`
additions hit; follow that pattern.

## Open items for the plan

- Confirm the exact `run_pattern` loop body and the single `events.push` site.
- Confirm every `PatternBlock` struct-literal site that needs the two new fields.
- Confirm the `playThrough` span computation and `beatSecs` derivation.
