# Cola Cromática na Troca de Acorde (line_engine) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pattern block that straddles a chord change must play its remaining notes from the NEW chord's triad pair, entering it by chromatic voice-leading (±1/±2 semitones preferred), instead of leaking the old chord's pitch classes onto the new chord's downbeat.

**Architecture:** `run_pattern` in `src/theory/line_engine.rs` currently resolves the sounding chord ONCE per block (from the block's start beat) and emits all `count` notes from that chord's `TriadLadder`. The fix restructures the emit loop to re-check the sounding chord per note; when the chord changes mid-block, it re-anchors the cursor on the new chord's ladder via a new `glue_rung` chooser that prefers a ±1/±2-semitone move from the last sounded pitch (the "chromatic glue" rule distilled from Pedro's étude corpus). `block_notes` (whole-block upfront) is replaced by a per-note `note_at`.

**Tech Stack:** Rust (stable, native tests) + wasm-pack (nightly) for the web pkg. No new dependencies.

## Global Constraints

- `cargo`/`rustc` are NOT on PATH. Native tests: `TC=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu; PATH="$TC/bin:$PATH" cargo test --lib` (run from `/home/pedro/Projects/chordz`).
- WASM rebuild (generic nightly only has the wasm32 target): `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
- `web/pkg/` is gitignored — never commit it.
- Commit messages: lowercase `tipo(escopo): descricao` in pt-BR **sem acentos** (match `git log` style). **NEVER add a Co-Authored-By or any Claude trailer** (repo history was rewritten on 2026-07-21 to strip them).
- Comments in code stay in English, matching the file's existing voice.
- Work on branch `fix/cola-cromatica` (created in Task 1 Step 0 from `main`).

---

### Task 1: Chromatic glue in `run_pattern` (TDD, one commit)

**Files:**
- Modify: `src/theory/line_engine.rs` (fn `block_notes` ~line 121, fn `run_pattern` ~lines 232–339, tests module)

**Interfaces:**
- Consumes: existing `TriadLadder { grips: Vec<TriadShape>, pcs: [u8; 3] }`, `pingpong`, `advance_cursor`, `gmc::resolve_pair`, `scale_defaults::default_scale` (all already in the file / imported).
- Produces: private `fn note_at(grip: &TriadShape, pcs: &[u8; 3], block: &PatternBlock, k: usize) -> FretNote` and private `fn glue_rung(ladder: &TriadLadder, block: &PatternBlock, k: usize, prev: i32) -> usize`. Public API (`generate_line`) unchanged — no wasm.d.ts work needed.

- [ ] **Step 0: Create the branch**

```bash
cd /home/pedro/Projects/chordz && git switch -c fix/cola-cromatica
```

- [ ] **Step 1: Write the two failing tests**

Add to the `mod tests` block of `src/theory/line_engine.rs` (it already has `use super::*;`, which brings `gmc`, `scale_defaults`, `Chart`, `PAIRS` into scope; `simple_config` is the existing helper at ~line 426):

```rust
#[test]
fn straddling_blocks_switch_to_the_new_chords_pair_at_the_boundary() {
    // A block that starts under one chord and crosses the barline must play its remaining
    // notes from the NEW chord's triads. Reproduces the Giant Steps opening leak: with the
    // 3+3 eighth pattern, the T2 block at beats 1.5-2.5 used to carry Bmaj7's D#/A# onto D7.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Bmaj7 D7 | Gmaj7 Bb7 | Ebmaj7 |").unwrap();
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &simple_config());

    let mut starts: Vec<f32> = Vec::new();
    let mut legal: Vec<Vec<u8>> = Vec::new();
    let mut cumul = 0.0_f32;
    for c in &chart.changes {
        starts.push(cumul);
        cumul += c.beats;
        let scale = scale_defaults::default_scale(c.quality);
        let (a, b) = gmc::resolve_pair(c.root_pc, scale, &PAIRS[0]);
        legal.push(a.iter().chain(b.iter()).copied().collect());
    }
    for e in &events {
        let idx = starts.iter().rposition(|&s| e.beat >= s - 1e-4).unwrap_or(0);
        assert!(
            legal[idx].contains(&e.pitch_class),
            "pc {} at beat {} is not in the sounding chord's pair {:?}",
            e.pitch_class, e.beat, legal[idx]
        );
    }
}

#[test]
fn boundary_glue_prefers_a_chromatic_step_when_available() {
    // Unrestricted region: the new chord's ladder offers candidate notes in every octave,
    // and | Cmaj7 D7 | is built so every old-triad pitch class has a new-triad pitch class
    // a half/whole step away (T2: {E,G,B} -> {F#,A,C}; T1: {E,G,B} -> {D,F,A}). Note the
    // glue targets a specific within-grip VOICE (the block's k-th note), so register floors
    // can rule the chromatic pitch class out — the per-boundary bounds below reflect that.
    let fb = Fretboard::standard_tuning();
    let chart = Chart::parse("Test", "| Cmaj7 D7 | Cmaj7 D7 |").unwrap();
    let config = LineConfig {
        pattern: Pattern::preset_alternating(),
        figure: RhythmicFigure::Eighth,
        positions: PositionSet::unrestricted(),
    };
    let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
    // Boundary 2.0 (Cmaj7 -> D7) crosses on a MID voice: a 1-2 st target exists in register,
    // so the glue must be chromatic. Boundary 4.0 (D7 -> Cmaj7) crosses on an ascending
    // block's TOP voice, and the incoming B2 has no {D,F,A} top note within 2 st (A2 is
    // never a top voice; the lowest top is D3) — the best available move is 3 st, so assert
    // the glue still takes that minimal step rather than a leap or a repeat.
    for (boundary, max_d) in [(2.0_f32, 2), (4.0_f32, 3)] {
        let cross = events
            .windows(2)
            .find(|w| w[0].beat < boundary && w[1].beat >= boundary)
            .expect("no event pair crossing the boundary");
        let d = (cross[1].midi - cross[0].midi).abs();
        assert!(
            (1..=max_d).contains(&d),
            "glue into beat {} moved {} semitones (want 1-{}): {} -> {}",
            boundary, d, max_d, cross[0].midi, cross[1].midi
        );
    }
}
```

- [ ] **Step 2: Run the new tests, verify both FAIL**

```bash
cd /home/pedro/Projects/chordz && TC=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu; PATH="$TC/bin:$PATH" cargo test --lib line_engine::tests::straddling line_engine::tests::boundary_glue 2>&1 | tail -20
```

Expected: `straddling_blocks_...` fails on an assert like `pc 3 at beat 2 is not in the sounding chord's pair` (D# leaking over D7). `boundary_glue_...` fails with a 3+-semitone move. If either PASSES, stop — the test is wrong; fix the test before touching the engine.

- [ ] **Step 3: Replace `block_notes` with per-note `note_at`**

Delete fn `block_notes` (~line 121) and add in its place:

```rust
/// The k-th fretboard note a block plays from one grip (0-based within the block).
/// `Monotonic` walks the grip by pitch in the block's direction (folding back via `pingpong`
/// if `count` > 3); `Order` plays the explicit cyclic role sequence.
fn note_at(grip: &TriadShape, pcs: &[u8; 3], block: &PatternBlock, k: usize) -> FretNote {
    match &block.shape {
        Shape::Order(order) => {
            let role = (order[k % order.len()] % 3) as usize;
            let pc = pcs[role];
            grip.notes.iter().find(|n| n.pitch_class == pc).copied().unwrap_or(grip.notes[0])
        }
        Shape::Monotonic => {
            let mut base = grip.notes.to_vec(); // grip.notes is sorted ascending
            if block.direction == Direction::Descending {
                base.reverse();
            }
            base[pingpong(k, base.len())]
        }
    }
}

/// At a mid-block chord change, the rung of the NEW chord's ladder whose next note (the k-th
/// of the block) best glues to the previous pitch: chromatic steps (±1/±2 semitones) first,
/// then the smallest move; the same pitch only when there is no alternative.
fn glue_rung(ladder: &TriadLadder, block: &PatternBlock, k: usize, prev: i32) -> usize {
    let cost = |i: usize| {
        let midi = note_at(&ladder.grips[i], &ladder.pcs, block, k).midi;
        match (midi - prev).abs() {
            0 => (2, 0),
            d @ (1 | 2) => (0, d),
            d => (1, d),
        }
    };
    (0..ladder.grips.len()).min_by_key(|&i| cost(i)).unwrap_or(0)
}
```

- [ ] **Step 4: Restructure the emit loop in `run_pattern`**

Add near the top of the file (next to the other helpers):

```rust
/// Float slack for beat-vs-chord-start comparisons (triplet grids accumulate f32 error).
const BEAT_EPS: f32 = 1e-4;
```

In `run_pattern`, change the block-start chord lookup (~line 271) to use the epsilon:

```rust
let chord_idx = chord_starts.iter().rposition(|&s| beat >= s - BEAT_EPS).unwrap_or(0);
```

Change the no-repeat nudge (~lines 295–307) to use `note_at` instead of `block_notes(...).first()` — the condition becomes:

```rust
while len > 1
    && tries < len
    && note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, 0).midi == prev
```

Replace the whole emit section (from `let notes = block_notes(...)` through the end of the `for (k, note)` loop, ~lines 309–329) AND the connector advance (~lines 331–335) with:

```rust
let count = block.count as usize;
let mut active_chord = chord_idx;
let mut silenced = false;
for k in 0..count {
    if slots >= total_events {
        break 'blocks;
    }
    let beat_now = slots as f32 * beat_dur;
    // Re-resolve mid-block chord changes: the rest of the block plays the NEW chord's
    // triad, entered from the last sounded pitch by chromatic glue.
    let chord_now = chord_starts
        .iter()
        .rposition(|&s| beat_now >= s - BEAT_EPS)
        .unwrap_or(0);
    if chord_now != active_chord {
        let new_ladder = &ladders[chord_now][ti];
        if new_ladder.grips.is_empty() {
            slots += count - k; // no notes available — the rest of the block is silent
            silenced = true;
            break;
        }
        cursor[ti] = match last_midi {
            Some(prev) => glue_rung(new_ladder, block, k, prev),
            None => new_ladder.nearest_rung(None),
        };
        active_chord = chord_now;
        cursor_chord[ti] = chord_now;
    }
    let ladder = &ladders[active_chord][ti];
    let note = note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, k);
    // The block's last note sustains `1 + hold_last` grid slots; others take one.
    let hold = if k + 1 == count { block.hold_last as usize } else { 0 };
    let step_slots = 1 + hold;
    events.push(NoteEvent {
        beat: beat_now,
        string: note.string,
        fret: note.fret,
        triad: block.triad,
        pitch_class: note.pitch_class,
        midi: note.midi,
        duration: step_slots as f32 * beat_dur,
    });
    last_midi = Some(note.midi);
    slots += step_slots;
}
if silenced {
    if slots >= total_events {
        break;
    }
    continue;
}

// Choose the next grip for this triad via the block's connector — on the ladder of the
// chord the block ENDED in (it may have glued into a new one mid-block).
let ladder = &ladders[active_chord][ti];
let (nc, nf) =
    advance_cursor(&ladder.grips, cursor[ti], flip[ti], block.connector, last_midi, &mut rng);
cursor[ti] = nc;
flip[ti] = nf;
```

Note: the earlier `let ladder = &ladders[chord_idx][ti];` binding (~line 272) and the block-start anchor/no-repeat logic stay as they are; the loop above shadows `ladder` per note on purpose. Do NOT change `advance_cursor`, `TriadLadder`, or any public signature.

- [ ] **Step 5: Run the two new tests, verify both PASS**

Same command as Step 2. Expected: both PASS.

- [ ] **Step 6: Run the whole native suite**

```bash
cd /home/pedro/Projects/chordz && TC=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu; PATH="$TC/bin:$PATH" cargo test --lib 2>&1 | tail -15
```

Expected: all tests pass, zero failures. If an existing test fails, fix the ENGINE (or report back if you believe the test encodes the old leak as correct — do not weaken tests silently).

- [ ] **Step 7: Type-check the wasm feature**

```bash
cd /home/pedro/Projects/chordz && TC=~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu; PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib 2>&1 | tail -5
```

Expected: `Finished` with no errors.

- [ ] **Step 8: Commit**

```bash
cd /home/pedro/Projects/chordz && git add src/theory/line_engine.rs && git commit -m "fix(line): bloco que cruza a troca de acorde entra no par novo por cola cromatica

O acorde soante era resolvido uma vez por bloco (no beat inicial), entao um
bloco que atravessava a barra tocava as triades do acorde anterior sobre o
novo, inclusive no tempo forte (ex.: D#/A# de Bmaj7 sobre o D7 em Giant
Steps). Agora o loop de emissao re-resolve o acorde por nota e, ao cruzar a
fronteira, re-ancora o cursor no ladder do acorde novo preferindo um movimento
cromatico de 1-2 semitons a partir da ultima nota (regra de cola dos etudes)."
```

(No trailer. Verify with `git log -1` that no Co-Authored-By slipped in.)

### Task 2: Rebuild the web pkg and run web checks

**Files:**
- Regenerate: `web/pkg/` (gitignored — do not commit)
- No source changes expected.

**Interfaces:**
- Consumes: Task 1's engine change (same `generate_line` signature; no new wasm exports, so `web/src/wasm.d.ts` needs no edits).
- Produces: a fresh `web/pkg` so `npm run dev` / vitest exercise the fixed engine.

- [ ] **Step 1: Rebuild the wasm pkg**

```bash
cd /home/pedro/Projects/chordz && TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm 2>&1 | tail -5
```

Expected: `[INFO]: :-) Your wasm pkg is ready to publish at .../web/pkg.`

- [ ] **Step 2: Run the web checks**

```bash
cd /home/pedro/Projects/chordz/web && npm run check 2>&1 | tail -5 && npx vitest run 2>&1 | tail -10
```

Expected: svelte-check 0 errors; all vitest tests pass. If a web test pinned the old leaking note sequence, report it back instead of editing it yourself.

- [ ] **Step 3: Confirm nothing to commit**

```bash
cd /home/pedro/Projects/chordz && git status --short
```

Expected: empty (web/pkg is gitignored). If anything else shows up, report it.

---

## Verification (orchestrator, after both tasks)

- Re-run the full native suite; spot-check the generated line over `| Bmaj7 D7 | Gmaj7 Bb7 | Ebmaj7 |` (diag print) to confirm 0 leaks and small boundary intervals.
- Present branch-integration options (merge to main / keep branch) per superpowers:finishing-a-development-branch.
