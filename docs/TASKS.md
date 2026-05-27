# chordz Tasks

## Current Direction

The project has moved past the original minimal TUI milestones. The current app
is an `eframe` / `egui` desktop application with a procedural voicing engine,
chart parser, voice-leading solver, ASCII renderer, and synthesized playback.

Use this file as the current backlog. Use [ARCHITECTURE.md](./ARCHITECTURE.md),
[VOICING_ENGINE.md](./VOICING_ENGINE.md), and [AGENT_GUIDE.md](./AGENT_GUIDE.md)
as implementation context.

Each completed task must pass:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

## Completed Baseline

These capabilities exist and should be preserved unless a task explicitly says
otherwise:

- Library boundary through `src/lib.rs`.
- Explicit root parsing with `Option<u8>`.
- Safe fretboard note lookup.
- `VoicingRecipe` and `VoiceSet`.
- Shell, rootless, drop, quartal, upper-structure, and triad-pair recipes.
- `VoiceSet -> Fingering` mapping.
- Basic deterministic ranking.
- Chart parser.
- Chart parser with fractional beat distribution inside each 4/4 bar.
- Dynamic-programming chart solver with tension, smoothness, ranking,
  alternatives, locks, automatic filter relaxation, and optional jitter.
- ASCII diagram rendering with golden tests.
- Native egui browser/tune UI with Tune filters for note count, fret range,
  stretch, strings, open strings, extension expansion, recipes, tension,
  smoothness, variation, alternatives, and locks.
- Synthesized strum, arpeggio, and progression playback.

## Priority 1: Keep Gates Green

### Task A: Preserve Clean Formatting And Lints

Goal: keep the repository ready for handoff to other agents.

Allowed files:

- Any files touched by the current feature/fix.

Acceptance:

- `cargo fmt --check` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- `cargo test --all-targets` passes.

## Priority 2: UI Decomposition

### Task B: Split Browser Mode From `app.rs`

Goal: move browser-specific state/view/helpers out of `src/ui/app.rs`.

Suggested files:

- `src/ui/app.rs`
- new `src/ui/browser.rs`
- `src/ui/mod.rs`

Acceptance:

- Browser behavior remains unchanged.
- Root/family/note-count selection still refreshes voicings.
- `j/k/h/l` and arrow navigation still work.
- Quality gates pass.

### Task C: Split Tune Mode From `app.rs`

Goal: move chart/tune-specific state/view/helpers out of `src/ui/app.rs`.

Suggested files:

- `src/ui/app.rs`
- new `src/ui/tune.rs`
- `src/ui/mod.rs`

Acceptance:

- Chart presets, solve, selected chord navigation, alternative swapping, and
  playback controls still work.
- Solver config construction remains testable outside widget code where
  practical.
- Quality gates pass.

## Priority 3: Solver Candidate Quality

### Task D: Preserve Musical Ranking During Candidate Capping - Done

Goal: avoid discarding useful candidates in `solver::generate_candidates` due to
lexicographic position sorting before truncation.

Suggested files:

- `src/voicings/solver.rs`
- `src/voicings/ranking.rs` if score data needs to be exposed

Acceptance:

- Deduplication remains deterministic.
- Candidate truncation prefers ranked/tension-appropriate options over raw
  position order.
- Existing solver tests pass.
- Add a focused test proving a lower-ranked lexicographic shape does not
  incorrectly displace a better candidate under a small `max_candidates`.

Status: implemented. Candidate generation now ranks per `VoiceSet`, keeps the
top mapped fingerings in that order, deduplicates while preserving order, and
tracks `rank_score` for the chart-level cost.

### Task E: Expand Ranking Heuristics

Goal: make top results more guitar-idiomatic.

Suggested files:

- `src/voicings/ranking.rs`
- tests in the same module

Acceptance:

- Add at least one tested heuristic for region preference, repeated tones,
  awkward stretches, or barre-like shapes.
- Existing ranking tests still pass.

## Priority 4: Chart Model

### Task F: Improve Beat Distribution - Done

Goal: replace integer-only `4 / tokens.len()` chart beat allocation with a model
that can represent three chords per bar and other common divisions.

Suggested files:

- `src/theory/chart.rs`
- `src/voicings/solver.rs`
- `src/audio/engine.rs`
- UI code that displays or plays beats

Acceptance:

- Existing charts parse the same or with documented, intentional duration
  changes.
- A 3-chord bar has nonzero durations that sum to the bar length.
- Progression playback uses the new duration model.
- Quality gates pass.

Status: implemented. `ChordChange.beats`, `SolvedChange.beats`, and progression
playback now use `f32` beat durations.

## Priority 5: Storage

### Task G: Implement Favorites Or Remove Placeholder

Goal: make `src/storage/favorites.rs` real or remove the unused public module
until needed.

Suggested files:

- `src/storage/favorites.rs`
- `src/storage/mod.rs`
- UI files if favorites become user-visible

Acceptance:

- If implemented, favorites persist as JSON under the platform config directory.
- If deferred, unused placeholder API is removed and docs mention storage as
  future work.
- Quality gates pass.

## Priority 6: Audio Responsiveness

### Task H: Move Long Progression Synthesis Off The UI Thread

Goal: keep the egui app responsive when generating full progression playback.

Suggested files:

- `src/audio/engine.rs`
- `src/ui/tune.rs` or `src/ui/app.rs`, depending on UI decomposition state

Acceptance:

- Full progression playback does not block the UI while samples are generated.
- Errors are surfaced or ignored gracefully without panics.
- Quality gates pass.
