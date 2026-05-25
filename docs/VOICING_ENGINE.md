# Voicing Engine Spec

## Goal

The voicing engine is the heart of chordz. It should generate useful modern jazz guitar voicings procedurally, then map them to playable fingerings.

It must not be a static dictionary and must not require every chord extension to be played. Guitar voicings are selective. A good Cmaj13#11 voicing might contain `3 7 9 #11 13` and omit the root and fifth. A useful G7alt voicing might be an upper-structure triad over a shell. A practical m11 voicing might be quartal and omit the fifth.

## Current Limitation

The current generator in `src/voicings/generate.rs` works like this:

1. Take every interval in `ChordQuality`.
2. Find one fretboard position for each interval.
3. Return fingerings that include all intervals exactly once.

This is acceptable as a prototype, but it is not the target engine.

Problems:

- Rootless voicings are impossible.
- Omitted fifths are impossible.
- Duplicated notes are impossible.
- Three-note shell voicings are impossible with current default `min_strings = 4`.
- Extended chords are treated like required full stacks.
- There is no concept of recipe, style, harmonic function, or ranking.

## Core Types

These names are proposed. Adjust them to fit Rust style and local code as implementation evolves.

### ChordFormula

The complete harmonic material available for a chord.

Examples:

- `maj7`: `1 3 5 7`
- `maj9`: `1 3 5 7 9`
- `maj13#11`: `1 3 5 7 9 #11 13`
- `m11`: `1 b3 5 b7 9 11`
- `7alt`: `1 3 b7 b9 #9 b5 #5`
- `13b9`: `1 3 5 b7 b9 13`

### VoicingRecipe

A musical strategy for selecting and arranging notes before mapping to guitar.

Initial recipes:

- `Closed`: simple close-position chord tones, mostly for baseline/testing.
- `Shell`: 3rd and 7th plus optional root, fifth, or color.
- `Drop2`: four-note close structure with second voice from top dropped.
- `Drop3`: four-note close structure with third voice from top dropped.
- `RootlessA`: common rootless dominant/major/minor forms based on 3rd and 7th.
- `RootlessB`: alternate inversion/family of rootless forms.
- `Quartal`: stacks of fourths derived from chord/scale material.
- `UpperStructureTriad`: triad over guide tones, common for altered dominants.
- `TriadPair`: two triads from a parent scale or harmonic color.

### VoiceSet

An abstract voicing before guitar mapping.

Fields to consider:

- `intervals`: selected interval names or IDs.
- `pitch_classes`: selected pitch classes.
- `octave_offsets`: spacing/inversion data.
- `recipe`: source recipe.
- `omissions`: root/fifth/etc. intentionally omitted.
- `tags`: rootless, shell, quartal, altered, bright, dense, sparse.

### Fingering

A playable mapping of a `VoiceSet` onto the guitar.

Fields to consider:

- `positions: [Option<u8>; 6]`
- `intervals: [Option<IntervalId>; 6]`
- `notes: [Option<Note>; 6]`
- `recipe`
- `score`
- `fret_span`
- `lowest_fret`
- `requires_barre`

### Ranking

Fingerings should be sorted by musical and ergonomic usefulness, not by raw position array.

Score dimensions:

- Fret span.
- Number of strings.
- Avoidance of awkward stretches.
- Region preference.
- Presence of guide tones.
- Presence of requested color tones.
- Bass note suitability.
- Recipe-specific priority.
- Avoid muddy low-register clusters.

## Generation Pipeline

### 1. Resolve Harmonic Material

Input:

- Root pitch class.
- Chord quality/formula.
- Optional harmonic context: tonic, function, scale source, bass note, target color.

Output:

- Complete interval material.
- Guide tones.
- Color tones.
- Avoid notes or low-priority notes for this context.

### 2. Expand Recipes

Each recipe receives the formula and returns one or more `VoiceSet` candidates.

Recipe examples:

- Shell dominant: `3 b7`, optional `1`, `13`, `b9`, `#9`.
- Rootless dominant: `3 b7 9 13`, or `b7 3 b13 b9`.
- Major rootless: `3 7 9 13`, optional `#11`.
- Minor rootless: `b3 b7 9 11`, optional `5`.
- Quartal minor: `11 b7 b3`, plus `9` or `5`.
- Upper-structure G7alt: guide tones `3 b7` plus upper triads from altered colors.
- Triad pair: alternate notes from two triads derived from the parent scale.

Recipes are allowed to:

- Omit roots.
- Omit fifths.
- Duplicate chord tones.
- Use 3, 4, 5, or 6 notes.
- Produce several inversions and octave spreads.

### 3. Map VoiceSets to Fretboard

For each voice in a `VoiceSet`, find candidate string/fret locations.

Constraints:

- Respect tuning and fret count.
- Prefer 3-5 note guitar voicings for jazz.
- Allow muted strings between played strings when musically/physically useful.
- Avoid generating every impossible permutation.

### 4. Filter Playability

Reject fingerings that violate hard constraints:

- Fret above max fret.
- Fret span too large.
- Too few or too many played strings.
- Impossible string ordering for the selected voice ordering, unless the recipe allows reordering.
- Low-register clusters that are too muddy.

### 5. Deduplicate

Deduplicate by musical and physical identity:

- Same string/fret positions.
- Same interval stack in the same register.
- Same shape transposed within equivalent local contexts, when appropriate.

### 6. Rank

Sort remaining fingerings by usefulness.

The first page in the TUI should contain playable, idiomatic voicings, not merely the first lexicographic results.

## Acceptance Examples

The engine should eventually satisfy behavior like this:

- A rootless dominant recipe for `G13` may return voicings with no G.
- A `Cmaj13#11` recipe may omit C and G while keeping E, B, D, F#, and A.
- A shell recipe for `G7b9` may return three-note or four-note voicings.
- A quartal recipe for `Dm11` may return stacks based on G-C-F or C-F-Bb-like colors depending on context.
- A triad-pair recipe should produce alternating material from two triads, not a single closed chord.
- The generator must not require every interval in an extended chord to appear in one fingering.

## Implementation Path

Recommended order:

1. Keep the existing generator as `legacy` or `closed` behavior while adding tests around its current behavior.
2. Add interval identity types instead of relying only on index into `ChordQuality`.
3. Introduce `VoicingRecipe` and `VoiceSet`.
4. Implement `Shell` recipes first; they are small and expose omissions/rootless behavior.
5. Implement mapping `VoiceSet -> Fingering`.
6. Add `RootlessA` and `RootlessB`.
7. Add ranking.
8. Add `Drop2`/`Drop3`.
9. Add quartal and upper-structure recipes.
10. Add triad-pair generation after scale/context primitives are strong enough.

## Tests Required Before Replacing the Current Generator

- Rootless recipes can return no-root fingerings.
- Shell recipes can return 3-string fingerings.
- Extended chords do not require all extensions to be present.
- Fifth omission is allowed where recipe permits.
- Required guide tones are present for shell/rootless dominant recipes.
- Fingerings respect fret span and max fret.
- Generated results are deterministic.

