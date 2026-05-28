# Procedural Voicing Engine

## Summary

Replace the recipe-based voice set generation with a procedural approach that
generates ALL valid N-note subsets from a chord quality's interval pool, applies
voicing transformations (close, drop2, drop3, drop2&3), and ranks using
per-quality interval stability tables.

## Motivation

The current recipe system hardcodes specific interval combinations per recipe.
This misses many classic jazz shapes that are obvious to any guitarist:
- Em7b5 basic shape (x7878x) was missing because `expand_basic_chords` replaced
  the base quality instead of augmenting it
- Drop2 with extensions (9 replacing 5, 11 replacing R) wasn't generated
- Diatonic substitution voicings (Em7 over Cmaj7) only appeared if the recipe
  happened to generate that specific subset

## Architecture

### Interval Pool

Each chord family gets a stability table mapping all 12 chromatic semitones to a
stability score (0-4):

```rust
pub type StabilityTable = [u8; 12];  // index = semitone, value = stability 0-4

pub fn stability_for(quality: &ChordQuality, next_quality: Option<&ChordQuality>) -> StabilityTable
```

Tables (validated by Pedro):

- **Major (maj7)**: `[4,0,3,1,4,1,2,4,1,3,0,4]`
  R=4 b9=0 9=3 b3=1 3=4 11=1 #11=2 5=4 b13=1 13=3 b7=0 7=4

- **Minor (m7)**: `[4,1,3,4,0,4,2,4,2,2,4,2]`
  R=4 b9=1 9=3 b3=4 3=0 11=4 #11=2 5=4 b13=2 13=2 b7=4 maj7=2

- **Dominant natural (→major)**: `[4,2,3,2,4,3,2,4,2,3,4,0]`
  R=4 b9=2 9=3 #9=2 3=4 4/sus=3 #11=2 5=4 b13=2 13=3 b7=4 7=0

- **Dominant altered (→minor)**: `[4,3,2,3,4,2,2,4,3,1,4,0]`
  R=4 b9=3 9=2 #9=3 3=4 4/sus=2 #11=2 5=4 b13=3 13=1 b7=4 7=0

- **Half-diminished (m7b5)**: `[4,1,2,4,0,3,4,0,2,1,4,1]`
  R=4 b9=1 9=2 b3=4 3=0 11=3 b5=4 5=0 b13=2 13=1 b7=4 7=1

- **Diminished (dim7)**: `[4,2,2,4,1,2,4,1,2,4,2,1]`
  R=4 b9=2 9=2 b3=4 3=1 11=2 b5=4 5=1 b13=2 dim7=4 13=2 b7=1

Dominant profile chosen by looking at the next chord in the chart. Default to
natural if no context.

### Subset Generation

```rust
pub fn generate_subsets(
    pool: &StabilityTable,
    note_count: usize,
    min_total_stability: u8,
) -> Vec<(Vec<u8>, u16)>  // (semitones, total stability)
```

1. Collect all semitones with stability > 0
2. Generate all C(N, note_count) combinations
3. Compute total stability = sum of individual scores
4. Filter: discard subsets below `min_total_stability`
5. Sort by total stability descending

For 4-note voicings from 8 available tones: C(8,4) = 70 subsets.

### Voicing Transformations

Each subset of N semitones generates voice sets via 4 transformations:

```rust
pub enum VoicingTransform {
    Close,
    Drop2,
    Drop3,
    Drop2And3,
}
```

Each transformation produces N inversions (rotations of the close-position stack):

- **Close**: stack notes ascending, rotate N times
- **Drop 2**: from each close rotation, drop 2nd-from-top down an octave
- **Drop 3**: drop 3rd-from-top down an octave
- **Drop 2&3**: drop both 2nd and 3rd from top

Total per subset: 4 transforms × N inversions = 4N voice sets.
Total per chord (4 notes, 70 subsets): 70 × 16 = 1120 voice sets.

### Recipe Classification (post-hoc)

After generation, each voice set gets a label for UI display:

```rust
pub fn classify_voice_set(voice_set: &VoiceSet) -> &'static str
```

Rules:
- Only R + 3 + 7 (or b3 + b7): "shell"
- Close position, all chord tones: "closed"
- Drop2 transform applied: "drop2"
- Drop3: "drop3"
- Drop2&3: "drop2&3"
- Contains only 4ths between adjacent voices: "quartal"
- Contains a triad subset: "upper" or "triad-pair"
- Default: transform name

### Integration

#### New module: `src/voicings/procedural.rs`

Main entry point:

```rust
pub fn generate_all_voice_sets(
    root_pc: u8,
    quality: &'static ChordQuality,
    note_count: usize,
    stability_table: &StabilityTable,
) -> Vec<(VoiceSet, u16, &'static str)>  // (voice_set, stability_score, label)
```

#### New module: `src/voicings/stability.rs`

Stability tables and lookup:

```rust
pub fn get_stability_table(
    quality: &ChordQuality,
    next_quality: Option<&ChordQuality>,
) -> StabilityTable
```

#### Modified: `src/voicings/solver.rs`

`generate_candidates` calls `procedural::generate_all_voice_sets` instead of
iterating over recipes. The recipe filter in `SolverConfig` becomes a label
filter (post-hoc classification matches against allowed labels).

#### Modified: `src/wasm_api.rs`

`generate_voicings` calls the procedural generator. The `prefer_crunch` flag
continues to work as a ranking modifier.

### Ranking

Voice sets are ranked by:

1. **Stability score** (from subset selection) — higher = more consonant
2. **Guitar-idiomatic score** (existing ranking.rs) — span, bass note quality,
   guide tones, muddy penalty, crunch bonus
3. **Combined**: `total_score = stability_weight * stability + guitar_score`

The `tension_target` slider in the solver maps to stability filtering:
- Grounded (0.0): min_total_stability = 14 (only core tones)
- Balanced (0.3): min_total_stability = 12
- Open (0.6): min_total_stability = 10
- Abstract (1.0): min_total_stability = 8 (allow tense subsets)

### What Gets Removed

- `VoicingRecipe::generate_voice_sets()` and all recipe-specific generators
  (generate_shell, generate_closed, generate_drop2, generate_drop3,
  generate_rootless, generate_quartal, generate_upper_structure_triad,
  generate_triad_pair)
- `VoicingRecipe` enum stays but only as a label type
- `extended_for_voicing()` in solver.rs (replaced by stability pool)
- The `expand_basic_chords` config flag (always procedural now)

### What Stays

- `VoicingRules` (fret span, string count, fret range)
- `map_voice_set()` in generate.rs (maps voice sets to fretboard fingerings)
- `rank_fingerings()` in ranking.rs (guitar-idiomatic scoring)
- `Fingering`, `VoiceSet` structs
- Solver DP algorithm (unchanged, just gets candidates differently)

### Performance

Worst case: 70 subsets × 16 transforms × ~20 fingerings each = ~22,400 fingerings
per chord. With filtering (min_stability, rules, dedup): ~2,000-5,000 actual.
Current system generates ~500-2,000. Slightly more work but well within
interactive budget (solver runs in <100ms for 32-bar charts).

## Testing

- Verify Em7b5 x7878x appears with default settings
- Verify Em7 drop2 (x7978x) appears for Cmaj7
- Verify all 4 inversions of drop2 appear for any 4-note quality
- Verify stability filtering: Grounded shows only core tones, Abstract shows extensions
- Verify dominant profile switches based on next chord
- Regression: run existing solver tests (Stella 32 bars)
