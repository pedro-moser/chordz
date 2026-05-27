# Agent Guide

This guide is for coding agents working on chordz. It describes the current
working shape of the app and the expectations for safe changes.

## First Commands

Start every nontrivial turn by checking the local state:

```bash
git status --short --branch
rg --files
cargo test --all-targets
```

Before finishing code changes, run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Use `cargo fmt` to fix formatting. Do not claim completion while any of the
three final commands fail.

## Current App Behavior

`cargo run` opens a native egui window.

The app has two user-facing modes:

- Browser: explore generated voicings for a selected root, chord family, and
  note count.
- Tune: parse a chord chart, solve a smooth fingering path, inspect each chord,
  swap alternatives, and play the current voicing or full progression.

Audio is optional. `ChordzApp::new` attempts `AudioEngine::new().ok()`, so UI
work must handle `audio: None`.

## Data Flow Cheat Sheet

Browser mode:

```text
root selector + ChordFamily + note count
  -> ChordQuality names
  -> VoicingRecipe::generate_voice_sets
  -> map_voice_set
  -> rank_fingerings
  -> VoicingGroup / VoicingEntry
  -> egui fretboard widget
```

Tune mode:

```text
chart text
  -> Chart::parse
  -> TuneConstraints::to_solver_config
  -> locked alternatives from current SolvedChart, when enabled
  -> solver::solve or solver::solve_with_locks
  -> SolvedChart
  -> progression list + fretboard widget + audio playback
```

`SolvedChart` carries both the chosen path and all retained alternatives. Tune
mode uses the alternatives for `h/l` swapping and stores a parallel `locked`
vector so selected positions survive a re-solve. Each solved change also carries
raw tension, normalized per-chord tension, rank score, and relaxation metadata.

Audio:

```text
Fingering
  -> fingering.notes(Fretboard)
  -> synth::generate_chord / generate_pluck
  -> in-memory WAV
  -> kira StaticSoundData
```

## Module Boundaries

Keep dependencies flowing in one direction:

```text
ui -> audio/render/voicings/theory
audio -> voicings/theory
render -> voicings
voicings -> theory
theory -> standard library only
```

Do not import `egui`, `eframe`, or `kira` from `theory`, `voicings`, or
`render`.

Do not put chart parsing or music theory shortcuts in UI code. If the behavior
is musical, it belongs in `theory` or `voicings` with tests.

## Important Invariants

- `root_to_pc` returns `None` for invalid roots.
- `Fretboard::get_note` returns `None` for invalid strings/frets.
- `VoiceSet` may intentionally omit root and fifth.
- Extended chords do not imply every listed interval must appear in one guitar
  fingering.
- `VoicingRecipe::generate_voice_sets` is the single recipe dispatch API for
  callers.
- `ChartChange.beats` is `f32`; playback should pass those durations through
  instead of rounding to integers.
- `SolverConfig::default()` uses `jitter: 0`; UI Variation is the opt-in source
  of non-deterministic solves.
- Solver deduplication preserves ranked candidate order before applying
  `max_candidates`.
- Locked Tune positions should be represented as `SolvedAlternative` values and
  solved through `solver::solve_with_locks`.
- Solver tests that need repeatable results must set `jitter: 0`.
- Lowercase interval constants such as `Interval::m3` are intentional music
  notation.

## Where To Add Tests

- Chord spelling/parsing: `src/theory/chords.rs` or `src/theory/chart.rs`.
- Recipe behavior: `src/voicings/recipe.rs`.
- Guitar mapping: `src/voicings/generate.rs`.
- Ranking behavior: `src/voicings/ranking.rs`.
- Chart path solving: `src/voicings/solver.rs`.
- ASCII diagrams: `src/render/diagram.rs`.
- Synth math: `src/audio/synth.rs`.

UI code currently has no interaction test harness. Keep UI changes small and
back domain logic with tests in lower modules whenever possible.

## Common Pitfalls

- Do not reintroduce old ratatui/crossterm assumptions. The active UI is egui.
- Do not sort and truncate candidates in a way that loses musical ranking unless
  the behavior is explicitly tested and documented.
- Do not unwrap user-entered chart/root data in UI code unless an earlier parser
  validation proves it safe.
- Do not add static chord dictionaries as the primary source of voicings.
- Do not make audio mandatory for startup.

## Suggested Work Pattern

1. Read `docs/ARCHITECTURE.md` and the relevant section of
   `docs/VOICING_ENGINE.md`.
2. Inspect the exact files to be edited.
3. Make the smallest coherent change.
4. Add/update focused tests for domain behavior.
5. Run the final quality gates.
6. Summarize changed files, tests, and remaining caveats.
