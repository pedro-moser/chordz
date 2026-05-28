# chordz Architecture

## Product Goal

chordz is a keyboard-driven Rust desktop app for guitarists studying and using
modern jazz harmony. It should feel like a practical chord dictionary and chart
voice-leading tool, but its voicings are generated from music theory and guitar
constraints instead of stored as a static lookup table.

The app should help a player answer questions like:

- What are useful Cmaj9 voicings around frets 5-9?
- Show me rootless dominant voicings for a ii-V-I.
- Compare drop, shell, quartal, and upper-structure sounds for the same chord.
- Find a playable path through Stella by Starlight with smooth voice leading.
- Hear a voicing or full progression as a quick synthesized reference.

## Current Stack

- UI: `eframe` / `egui` native desktop window.
- Audio: `kira` playback with generated in-memory WAV buffers.
- Tests: focused Rust unit tests across theory, rendering, voicings, solver, and
  synth modules.

`src/main.rs` is intentionally small. It creates the `eframe::NativeOptions` and
launches `ChordzApp`.

## Source Map

```text
src/
  main.rs              Native egui app bootstrap.
  lib.rs               Library boundary exposing core modules to tests/UI.
  theory/              Notes, intervals, chord qualities, roots, chart parsing.
  voicings/            Procedural guitar engine, recipes, mapping, ranking,
                       chart solver, and voice-leading distance.
  render/              Pure ASCII diagram renderer and golden tests.
  audio/               Synth sample generation and kira playback engine.
  ui/                  egui application state, views, widgets, and commands.
```

## Module Responsibilities

### theory

Pure music theory. No UI, audio, storage, or guitar rendering concerns.

Important files:

- `notes.rs`: `Note`, MIDI conversion, pitch-class display names.
- `intervals.rs`: interval constants. Lowercase constants such as `m3` and `m7`
  are intentional music notation, not Rust style drift.
- `chords.rs`: supported `ChordQuality` values, jazz root names, root parsing,
  and display names.
- `chart.rs`: parsing chart text into timed `ChordChange` values.

Rules:

- Invalid roots should fail explicitly through `Option`/`Result`.
- Chart beat durations are `f32`; each bar distributes four beats across its
  chord tokens so odd divisions such as three chords in a bar remain playable.
- Keep chord spelling and parsing rules test-backed.
- Do not introduce UI display assumptions here beyond stable domain names.

### voicings

The procedural guitar engine. It should not draw UI, play audio, or store user
data.

Important files:

- `fretboard.rs`: standard tuning and safe string/fret note lookup.
- `recipe.rs`: `VoicingRecipe` plus recipe-to-`VoiceSet` generation.
- `voice_set.rs`: abstract selected intervals before guitar mapping.
- `generate.rs`: `VoiceSet -> Fingering` mapper (`map_voice_set`).
- `ranking.rs`: musical and ergonomic scoring for mapped fingerings.
- `voice_leading.rs`: distance metric between two fingerings.
- `solver.rs`: chart-level dynamic programming solver with tension and optional
  jitter, retained alternatives, locks, and automatic filter relaxation.

Core data flow:

```text
ChordQuality + root
  -> VoicingRecipe::generate_voice_sets
  -> VoiceSet
  -> map_voice_set
  -> Fingering
  -> rank_fingerings / solver::solve / solver::solve_with_locks
```

Rules:

- Extended chords must not require every extension in one fingering.
- Rootless and fifth-omitting voicings are first-class behavior.
- Recipe dispatch belongs in `VoicingRecipe::generate_voice_sets`; avoid
  duplicating that matrix in UI or solver code.
- Mapping and ranking should stay deterministic unless solver jitter is
  intentionally nonzero.
- Solver candidates should retain recipe, raw/normalized tension, rank score,
  and relaxation metadata so UI code can inspect and swap choices without
  reconstructing musical state.

### render

Pure text rendering. It converts a `Fingering` into stable ASCII diagrams for
tests, logs, and possible future CLI output.

Rules:

- Keep golden tests stable.
- Do not depend on egui here.

### ui

Desktop egui application layer.

Important files:

- `app.rs`: app state, browser mode, chart/tune mode, keyboard handling, and
  commands.
- `fretboard.rs`: stateless egui fretboard widget and compact interval labels.
- `mod.rs`: UI module exports.

Current modes:

- Browser mode: choose root, chord family, note count, recipe/quality groups,
  and cycle voicing positions.
- Tune mode: parse a chart, solve a smooth fingering path, inspect each chord,
  lock choices, swap alternatives, adjust solver filters, and play
  strums/progressions.

Keyboard model:

- Browser: `j/k` or arrows move groups, `h/l` cycle positions, space strums.
- Tune: `j/k` or arrows move through solved changes, `h/l` swap alternatives,
  space strums current chord, full progression playback is available in the UI.

Tune constraints map directly into `SolverConfig`: note count, fret range,
maximum span, string mask, open-string policy, basic-chord extension expansion,
recipe filter, tension target, smoothness, and variation. Re-solving preserves
locked chord positions by passing their current `SolvedAlternative` values to
`solver::solve_with_locks`.

Rules:

- Keep egui code out of `theory` and `voicings`.
- Keep reusable widgets outside `app.rs` when possible.
- `app.rs` is still larger than ideal; future UI work should continue splitting
  browser state/view, tune state/view, and command handling.

### audio

Optional playback. The app must continue to function if audio initialization
fails.

Important files:

- `synth.rs`: deterministic generated plucked-string-like samples.
- `engine.rs`: `kira` manager, active sound handles, strum/arpeggio/progression
  playback, in-memory WAV encoding.

Rules:

- Playback errors should not crash the UI.
- Any sound started through `AudioEngine` should be tracked so `stop_all` can
  stop it.
- Long generated progressions may block the UI today; moving synthesis off the
  UI thread is a good future improvement.

## Quality Gates

Before calling a code change complete, run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

For musical behavior changes, add focused tests in the relevant domain module.
For rendering changes, update or add golden tests. For UI-only changes, keep the
domain behavior untouched and verify the app still builds.

## Non-Goals

- Do not add Electron or web runtime dependencies.
- Do not replace procedural voicing generation with a static chord dictionary.
- Do not force extended chords into full chord stacks.
- Do not let UI convenience leak into core theory/voicing APIs.
