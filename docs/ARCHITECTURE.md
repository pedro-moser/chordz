# chordz Architecture

## Product Goal

chordz is a keyboard-driven Rust TUI for guitarists studying and using modern jazz harmony. It should feel like a practical chord dictionary, but its voicings are generated from music theory and guitar constraints instead of stored as a static lookup table.

The app should help a player answer questions like:

- What are useful Cmaj9 voicings around frets 5-9?
- Show me rootless dominant voicings for a ii-V-I.
- Compare drop, shell, quartal, and upper-structure sounds for the same chord.
- Give me modern colors: #11, b9, #9, b13, 13, altered, diminished, and triad-pair material.

## Current State

- `src/theory/` has the first theory primitives: notes, intervals, chord qualities, roots, and chord naming.
- `src/voicings/` has a first-pass fretboard model, simple rules, and a backtracking generator.
- `src/render/`, `src/audio/`, `src/ui/`, and `src/storage/` are placeholders.
- `src/main.rs` is not yet a real TUI entry point.

The current voicing generator is useful as a prototype, but it is not the final design. It enumerates fingerings that contain every interval in a chord quality exactly once. Modern jazz guitar voicings need a richer model: omissions, rootless forms, shell voicings, upper structures, quartal stacks, triad pairs, inversions, and ranking.

## Architecture

```text
src/
  main.rs              TUI entry point and app bootstrap
  theory/              Pitch, interval, chord, scale, and harmonic context primitives
  voicings/            Procedural voicing engine and fretboard mapping
  render/              Text/ratatui diagram rendering
  audio/               Optional synthesized playback
  ui/                  App state, screens, widgets, and event handling
  storage/             Favorites and user config persistence
```

## Module Responsibilities

### theory

Pure music theory. No terminal, audio, storage, or guitar UI concerns.

Expected responsibilities:

- Notes and pitch classes.
- Intervals with enharmonic/display names.
- Chord formulas and qualities.
- Scale/chord relationships needed for altered, melodic minor, diminished, and modal colors.
- Harmonic context for choosing rootless and upper-structure recipes.

### voicings

The procedural guitar engine. It should not draw UI, play audio, or store user data.

Expected responsibilities:

- Fretboard model and tuning.
- Musical voicing recipes.
- Candidate voice-set generation before guitar mapping.
- Mapping voice sets to playable fingerings.
- Playability filtering.
- Musical and ergonomic ranking.

See [VOICING_ENGINE.md](./VOICING_ENGINE.md) for the canonical design.

### render

Converts domain data into terminal-friendly representations.

Expected responsibilities:

- ASCII/ratatui fretboard diagrams.
- Interval labels, note labels, muted/open strings, barre hints.
- Stable layout for different fret regions.
- Snapshot/golden tests for diagrams.

### ui

Ratatui application layer.

Expected responsibilities:

- App state and navigation.
- Browser, detail, search, compare, and favorites screens.
- Vim-style key bindings.
- Event loop and terminal lifecycle.

Preferred first layout:

```text
+----------------+-----------------------------+----------------+
| Chords         | Diagram / voicing list      | Details        |
| roots/quality  | selected fretboard region   | recipe, notes  |
| filters        | generated alternatives      | intervals      |
+----------------+-----------------------------+----------------+
| / search  j/k navigate  h/l pane/root  enter select  p play  q quit |
+---------------------------------------------------------------------+
```

### audio

Optional playback. Keep this later than render/UI unless the user asks otherwise.

Expected responsibilities:

- Convert fingerings to pitches/frequencies.
- Simple synthesized pluck/strum.
- Non-blocking playback.
- Graceful failure when no audio device is available.

### storage

User data only.

Expected responsibilities:

- Favorites.
- Maybe user presets for filters and tuning.
- JSON under the platform config directory via `dirs`.

## Interaction Model

Use vim-style bindings as the default:

- `j` / `k`: move down/up
- `h` / `l`: move left/right or transpose root down/up depending on focused pane
- `g` / `G`: first/last item
- `/`: search
- `n` / `N`: next/previous search match
- `enter`: select
- `tab`: cycle pane
- `f`: favorite
- `p`: play
- `c`: compare
- `q`: quit

Arrow keys may also work, but the app should not require them.

## Non-Goals

- Do not build an Electron app.
- Do not depend on a static chord dictionary as the primary source of voicings.
- Do not optimize audio before the voicing engine and visual browsing are useful.
- Do not treat extended chords as "play every listed interval on guitar".

## Quality Bar

Every completed slice should have:

- Focused unit tests for theory and voicing behavior.
- Golden/snapshot-style tests for text diagrams once render exists.
- A small observable smoke path through `cargo run` once the TUI shell exists.
- `cargo test --all-targets` passing before calling work complete.

