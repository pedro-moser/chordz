# Voicing Engine Spec

## Goal

The voicing engine is the heart of chordz. It generates useful modern jazz
guitar voicings procedurally, maps them to playable fingerings, ranks them, and
solves smooth paths through chord charts.

It must not be a static dictionary and must not require every chord extension to
be played. Guitar voicings are selective. A good Cmaj13#11 voicing might contain
`3 7 9 #11 13` and omit the root and fifth. A useful G7alt voicing might use
guide tones plus altered color. A practical m11 voicing might be quartal and
omit the fifth.

## Current Engine Shape

The engine uses a single recipe pipeline: expands a `VoicingRecipe` into
`VoiceSet` candidates, maps each `VoiceSet` to `Fingering` via `map_voice_set`,
ranks the results, and optionally solves chart-level voice leading.

```text
root_pc + ChordQuality
  -> VoicingRecipe::generate_voice_sets
  -> Vec<VoiceSet>
  -> map_voice_set
  -> Vec<Fingering>
  -> rank_fingerings or solver::solve / solver::solve_with_locks
```

## Core Types

### ChordQuality

Defined in `src/theory/chords.rs`. It represents the full harmonic material
available for a chord quality.

Examples:

- `maj7`: `1 3 5 maj7`
- `maj13`: `1 3 5 maj7 9 13`
- `m11`: `1 b3 5 b7 9 11`
- `dom7b9`: `1 3 5 b7 b9`
- `dom7#11`: `1 3 5 b7 #11`

### VoicingRecipe

Defined in `src/voicings/recipe.rs`. A musical strategy for selecting and
arranging chord tones before guitar mapping.

Current recipes:

- `Closed`: complete formula, mostly for simple closed/baseline behavior.
- `Shell`: guide tones with optional root.
- `RootlessA`: common rootless major/minor/dominant forms.
- `RootlessB`: alternate guide-tone ordering.
- `Drop2`: drop-2 voicings over multiple four-note cores and inversions.
- `Drop3`: drop-3 voicings over multiple four-note cores and inversions.
- `Quartal`: fourth-stack material where available.
- `UpperStructureTriad`: guide-tone-plus-triad dominant colors.
- `TriadPair`: alternating material from two available triads.

Call `VoicingRecipe::generate_voice_sets(root_pc, quality)` from callers. Do
not duplicate the recipe dispatch outside this type.

### VoiceSet

Defined in `src/voicings/voice_set.rs`. An abstract voicing before guitar
mapping.

Important fields:

- `root_pc`: root pitch class.
- `intervals`: selected interval identities.
- `octave_offsets`: spacing/inversion information.
- `recipe`: source recipe.
- `source_quality`: original harmonic material.

Voice sets may intentionally omit roots and fifths. They may contain fewer notes
than the source quality.

### Fingering

Defined in `src/voicings/generate.rs`. A playable mapping of a `VoiceSet` onto
the six guitar strings.

Important fields:

- `positions: [Option<u8>; 6]`: fret per string, or muted.
- `intervals: [Option<Interval>; 6]`: interval per string, or muted.

Important methods:

- `played_count`
- `fret_span`
- `lowest_fret`
- `played_intervals`
- `has_interval`
- `notes`

### SolverConfig

Defined in `src/voicings/solver.rs`. Controls chart-level candidate generation
and path solving.

Important fields:

- `rules`: hard `VoicingRules` for string count, span, max fret, root policy.
- `recipes`: recipe set to consider.
- `max_candidates`: cap per chord.
- `min_fret`: lower fret bound after mapping.
- `allowed_strings`: optional string mask.
- `allow_open_strings`: keep or reject fingerings that use fret 0.
- `expand_basic_chords`: when true, simple chart symbols such as `G7` can use
  richer voicing material such as altered or lydian dominant colors.
- `tension_target`: 0.0 grounded to 1.0 abstract.
- `tension_weight`: strength of tension matching.
- `rank_weight`: strength of standalone fingering rank in chart solving.
- `smoothness_weight`: multiplier for voice-leading movement cost.
- `jitter`: random-ish tie/noise amount; set to `0` for deterministic tests.

`SolverConfig::default()` is deterministic: `jitter` is `0`. The Tune UI maps
its Variation slider to this field when the user wants a less repeatable solve.

### SolvedChart, SolvedChange, and SolvedAlternative

Defined in `src/voicings/solver.rs`. `SolvedChart` contains:

- `fingerings`: the chosen path, one `SolvedChange` per chart change.
- `alternatives`: retained `SolvedAlternative` candidates per chart change,
  used by Tune mode for manual left/right swapping.

`SolvedChange` and `SolvedAlternative` both carry the selected `Fingering`,
`VoicingRecipe`, raw `tension`, per-chord `normalized_tension`, `rank_score`,
and `RelaxationLevel`.

The raw tension is calculated from recipe, chord quality, root omission, and
extensions. It is then normalized within each chord's candidate set before the
slider penalty is applied. This makes the Tension slider pick the lower or
higher tension options available for the current chord instead of depending on
absolute tension values across unrelated chord types.

`RelaxationLevel` documents whether a candidate matched the user's filters
exactly or came from a fallback:

- `Exact`: all filters were satisfied.
- `FewerNotes`: the solver allowed 2-note candidates.
- `IgnoreStringFilter`: the solver dropped the string mask.
- `WiderFretRange`: the solver widened the fret range by up to two frets.

## Generation Pipeline

### 1. Resolve Harmonic Material

Input:

- Root pitch class.
- `ChordQuality`.
- Optional chart context in higher-level callers.

Output:

- Complete interval material.
- Guide tones.
- Color tones.
- Recipe-specific cores.

Today this is mostly encoded directly in recipe helpers. Future scale/context
work should live in `theory` or a new domain module, not in UI code.

### 2. Expand Recipes

Each recipe returns one or more `VoiceSet` candidates. Recipes are allowed to:

- Omit roots.
- Omit fifths.
- Use 2, 3, 4, 5, or 6 notes.
- Use extensions without playing the full stack.
- Produce inversions and octave spreads.

Acceptance examples already covered by tests include:

- `G13` rootless forms omit `1`.
- `Cmaj13` rootless can produce `3 7 9 13`.
- `Dm11` rootless can produce `b3 b7 9 11`.
- Shells include guide tones and can omit fifths.
- Drop voicings generate multiple inversions.
- Quartal, upper-structure, and triad-pair recipes produce non-closed material.

### 3. Map VoiceSets to Fretboard

`map_voice_set` finds candidate string/fret locations for each voice and
backtracks through playable assignments.

Hard constraints:

- Respect fretboard tuning and `max_fret`.
- Respect `max_fret_span`.
- Respect min/max played strings.
- Respect `require_root`.
- Keep selected voices in ascending string order and non-descending MIDI order.

The mapper returns deterministic, deduplicated `Fingering` values.

### 4. Rank Fingerings

`ranking::score` rewards and penalizes musical/ergonomic traits:

- Smaller fret span is better.
- Complete guide tones are better.
- Muddy low-register clusters are penalized.

This ranking is intentionally simple. Future scoring should add region
preference, repeated-note handling, barre/stretch heuristics, and bass-note
suitability.

### 5. Solve Chart Voice Leading

`solver::solve` parses per-chord candidates and uses dynamic programming to find
a low-cost path through a chart. `solver::solve_with_locks` is the same solver
with selected chart positions forced to a previously chosen `SolvedAlternative`.

Cost inputs:

- Voice-leading distance between adjacent fingerings.
- Repeat-shape penalty.
- Tension mismatch penalty against normalized per-chord candidate tension.
- Standalone fingering rank penalty.
- Smoothness multiplier for voice-leading distance.
- Optional jitter.

Set `SolverConfig { jitter: 0, ..Default::default() }` in tests that need stable
paths.

Candidate generation order matters. Each mapped `VoiceSet` is ranked first, the
best three fingerings per voice set are retained, and candidate deduplication
preserves that ranked order until `max_candidates` is reached. Avoid replacing
this with lexicographic position sorting unless tests prove the musical ranking
is still preserved.

### 6. Chart Durations

`Chart::parse` distributes four beats across the chord tokens in each bar using
`f32` beat counts. A three-chord bar therefore stores `4.0 / 3.0` beats per
change instead of rounding down to one beat each. `SolvedChange.beats` and
`AudioEngine::play_progression` use the same floating-point beat durations, so
display, solving metadata, and playback stay aligned.

## Current Known Gaps

- Scale/source context is still implicit in recipe heuristics.
- Barre and difficult-fingering detection are not modeled.
- UI chart playback synthesis can block while a full progression buffer is
  generated.
- Tune mode is still implemented inside `src/ui/app.rs`; it should eventually
  move into a focused UI module.

## Test Expectations

Before replacing or changing generator behavior, keep these properties covered:

- Rootless recipes can return no-root fingerings.
- Shell recipes can return 3-string fingerings.
- Extended chords do not require all extensions to be present.
- Fifth omission is allowed where recipe permits.
- Required guide tones are present for shell/rootless dominant recipes.
- Fingerings respect fret span and max fret.
- Generated results are deterministic when jitter is zero.
- Locked Tune alternatives stay fixed when `solve_with_locks` is used.
- Auto-relaxed candidates expose their `RelaxationLevel`.
- Chart solver returns a complete path or explicit `None`, never a partial
  silently-successful result.
