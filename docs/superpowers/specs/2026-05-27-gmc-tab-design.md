# GMC Tab — Generic Modal Compression (Phase 1)

## Summary

New app tab for visualizing triad pairs derived from the Tim Miller / Mick Goodrick
Generic Modal Compression system. Shows a panoramic fretboard (15 frets) with two
triads from a pair colored differently, covering all occurrences across the neck.

## Background

GMC takes a 7-note scale, removes the tonic, and partitions the remaining 6 notes
into pairs of 3. There are C(6,3)/2 = 10 such pairs. The book orders them by
structural type: Triad, Sus, Cluster, 7no5, 7no3 — combined systematically from
most consonant to most abstract.

The pairs follow a **fixed index pattern** applied to the 6 remaining scale tones
sorted ascending by semitone. The labels (T/T, Sus/7no5, etc.) describe the sound
but the formula is universal across all scales.

## Scale Sources

4 parent scales × 7 modes = 28 modes total:
- Major: Ionian, Dorian, Phrygian, Lydian, Mixolydian, Aeolian, Locrian
- Harmonic Minor: all 7 modes
- Melodic Minor: all 7 modes
- Harmonic Major: all 7 modes

## Pair Index Table

Applied to 6 notes at indices 0–5 (the scale tones minus root, ascending):

| #  | Label      | Group A   | Group B   |
|----|------------|-----------|-----------|
| 1  | T/T        | 0, 2, 4   | 1, 3, 5   |
| 2  | T/7no5     | 0, 3, 5   | 1, 2, 4   |
| 3  | T/7no3     | 0, 2, 5   | 1, 3, 4   |
| 4  | Sus/Sus    | 0, 3, 4   | 1, 2, 5   |
| 5  | Sus/7no5   | 0, 1, 4   | 2, 3, 5   |
| 6  | Sus/7no3   | 1, 4, 5   | 0, 2, 3   |
| 7  | Clus/Clus  | 0, 1, 2   | 3, 4, 5   |
| 8  | Clus/7no5  | 1, 2, 3   | 0, 4, 5   |
| 9  | Clus/7no3  | 2, 3, 4   | 0, 1, 5   |
| 10 | 7no5/7no3  | 2, 4, 5   | 0, 1, 3   |

## Architecture

### New files

- `src/theory/scales.rs` — Scale struct, ParentScale enum, 28 modes as const ALL
- `src/theory/gmc.rs` — PAIRS const (10 entries), resolve_pair() function

### Modified files

- `src/theory/mod.rs` — add `pub mod scales; pub mod gmc;`
- `src/ui/app.rs` — AppMode::Gmc variant, GmcState struct, show_gmc(), show_gmc_controls()
- `src/ui/fretboard.rs` — paint_panoramic_fretboard() function

## Data Model

### Scale

```rust
pub enum ParentScale { Major, HarmonicMinor, MelodicMinor, HarmonicMajor }

pub struct Scale {
    pub name: &'static str,
    pub parent: ParentScale,
    pub degree: u8,
    pub semitones: [u8; 7],
}

impl Scale {
    pub const ALL: &[Scale] = &[ /* 28 modes */ ];
}
```

### GMC Pairs

```rust
pub struct TriadPairSet {
    pub label: &'static str,
    pub indices: ([usize; 3], [usize; 3]),
}

pub const PAIRS: [TriadPairSet; 10] = [ /* fixed index table */ ];

pub fn resolve_pair(root_pc: u8, scale: &Scale, pair: &TriadPairSet) -> ([u8; 3], [u8; 3])
```

### App State

```rust
pub enum AppMode { Browser, Tune, Gmc }

struct GmcState {
    root_index: usize,      // 0..11
    scale_index: usize,     // 0..27
    pair_index: usize,      // 0..9
    show_intervals: bool,
}
```

## UI Layout

### Sidebar (left panel)

1. **Root** — 12-note selector (same as browser)
2. **Scale** — 28 modes grouped by ParentScale (Major, Harm. Minor, Mel. Minor, Harm. Major)
3. **Pair** — 10-item list showing label + intervals + concrete notes side by side
   - Example: `T/T  (2 4 6) + (b3 5 b7) = D F A + Eb G Bb`
4. **Toggle** — "Notes" / "Intervals" switch for dot labels

### Central panel

Panoramic fretboard: 15 frets × 6 strings, horizontal orientation.

For each (string, fret):
- Compute pitch class via `fretboard.get_note(s, fret)`
- If PC ∈ triad_a → dot in color A (blue)
- If PC ∈ triad_b → dot in color B (orange)
- Else → empty
- Dot label: note name or interval relative to root (per toggle)

Dots are smaller than the current voicing fretboard to fit the wider view.

## Non-goals (Phase 1)

- No exercise/étude generation (Phase 2)
- No voice-leading between pairs across chord changes (Phase 2)
- No audio playback
- No MIDI integration

## Reference

Based on Tim Miller & Mick Goodrick's Generic Modal Compression system.
Pedro's complete study spreadsheet: `~/syncsyncs/GMC PAIRS.xlsx`
