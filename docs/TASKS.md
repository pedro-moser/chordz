# chordz Tasks

## Current Direction

Do not continue directly from the old "Phase 3 render" resume plan. The current priority is to repair the voicing engine design so it matches the product goal: modern procedural jazz guitar voicings.

Each milestone ends with the review gate from [HARNESS.md](./HARNESS.md): tests, diff inspection, OMP reviewer pass, fixes, and tests again.

Use `scripts/omp-task N` to launch a specific task and `scripts/omp-review` at milestone boundaries.

## Milestone 1: Stabilize the Core API

### Task 1: Add a Library Boundary

Goal: expose the core modules through `src/lib.rs` so tests and future UI code can use the same API.

Allowed files:

- `src/lib.rs`
- `src/main.rs` if needed

Acceptance:

- `cargo test --all-targets` passes.
- Existing module tests still run.
- No behavior change required.

### Task 2: Make Root Parsing Explicit

Goal: change invalid root handling from silent C fallback to explicit failure.

Allowed files:

- `src/theory/chords.rs`
- Callers/tests as needed

Acceptance:

- `root_to_pc("C") == Some(0)`.
- `root_to_pc("Db") == Some(1)`.
- `root_to_pc("H") == None`.
- Existing chord naming tests pass after updates.

### Task 3: Make Fretboard Access Fallible

Goal: prevent panics for invalid string/fret lookup.

Allowed files:

- `src/voicings/fretboard.rs`
- Callers/tests as needed

Acceptance:

- Safe API returns `None` for invalid string index.
- Safe API returns `None` for fret greater than `num_frets`.
- Existing generator still works with valid positions.

## Milestone 2: Replace Full-Stack Thinking With Recipes

### Task 4: Introduce VoicingRecipe

Goal: add an enum representing musical generation strategies.

Allowed files:

- `src/voicings/rules.rs` or a new `src/voicings/recipe.rs`
- `src/voicings/mod.rs`

Acceptance:

- Enum includes at least `Closed`, `Shell`, `RootlessA`, `RootlessB`, `Drop2`, `Drop3`, `Quartal`, `UpperStructureTriad`, `TriadPair`.
- No generator rewrite yet.
- Tests or compile-time use prove the enum is exported.

### Task 5: Introduce VoiceSet

Goal: represent selected musical intervals before mapping to the fretboard.

Allowed files:

- `src/voicings/generate.rs` or new `src/voicings/voice_set.rs`
- `src/voicings/mod.rs`

Acceptance:

- `VoiceSet` can represent fewer intervals than the source chord formula.
- `VoiceSet` stores its source `VoicingRecipe`.
- Unit test constructs a rootless major voice set with intervals `3 7 9 13`.

### Task 6: Implement Shell VoiceSet Generation

Goal: generate abstract shell voice sets without mapping them to guitar yet.

Allowed files:

- `src/voicings/recipe.rs`
- `src/voicings/voice_set.rs`
- `src/voicings/mod.rs`

Acceptance:

- Dominant shell includes guide tones `3` and `b7`.
- Minor shell includes `b3` and `b7`.
- Major shell includes `3` and `7`.
- Shell generation may produce 3-note voice sets.
- Tests prove fifth omission is allowed.

### Task 7: Implement Rootless VoiceSet Generation

Goal: generate abstract rootless voice sets for major, minor, and dominant families.

Allowed files:

- Same files as Task 6.

Acceptance:

- `G13` rootless can produce intervals without `1`.
- `Cmaj13` rootless can produce `3 7 9 13`.
- `Dm11` rootless can produce `b3 b7 9 11`.
- Tests prove root omission is intentional, not accidental.

## Milestone 3: Map Recipes to Guitar

### Task 8: Map VoiceSet to Fingering

Goal: convert a `VoiceSet` into playable guitar fingerings.

Allowed files:

- `src/voicings/fretboard.rs`
- `src/voicings/generate.rs`
- recipe/voice-set modules as needed

Acceptance:

- Mapping respects `max_fret`, `max_fret_span`, and string count.
- A shell voice set can map to a 3-string fingering.
- A rootless voice set can map to a fingering with no root.
- Results are deterministic.

### Task 9: Add Ranking

Goal: sort generated fingerings by usefulness.

Allowed files:

- `src/voicings/ranking.rs`
- `src/voicings/mod.rs`
- generator/tests as needed

Acceptance:

- Ranking is deterministic.
- Smaller fret spans generally rank better.
- Guide-tone-complete voicings outrank incomplete candidates.
- Very muddy low-register clusters are penalized.

## Milestone 4: First User-Visible Slice

### Task 10: ASCII Diagram Renderer

Goal: render a fingering as a stable text diagram.

Allowed files:

- `src/render/diagram.rs`
- `src/render/mod.rs`

Acceptance:

- Handles muted strings.
- Handles open strings.
- Shows fret region.
- Golden tests cover at least one closed voicing and one rootless/shell voicing.

### Task 11: CLI Smoke Output

Goal: make `cargo run` show a useful static example before the full TUI exists.

Allowed files:

- `src/main.rs`
- render/voicing modules as needed

Acceptance:

- `cargo run` prints a named chord and at least one diagram.
- `cargo test --all-targets` passes.

### Task 12: Minimal Ratatui Browser

Goal: create the first interactive TUI screen.

Allowed files:

- `src/ui/app.rs`
- `src/ui/events.rs`
- `src/ui/screens/browser.rs`
- `src/main.rs`

Acceptance:

- App opens in alternate screen and exits cleanly with `q`.
- `j/k` move selection.
- Display includes chord name and current voicing/diagram.
- Terminal state is restored on exit or panic path where practical.
