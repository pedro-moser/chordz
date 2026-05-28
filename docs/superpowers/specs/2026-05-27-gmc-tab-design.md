# GMC Tab — Generic Modal Compression

## Status: Implemented

## Summary

App tab for visualizing triad pairs derived from the Tim Miller / Mick Goodrick
Generic Modal Compression system. Shows a panoramic SVG fretboard (15 frets) with
two triads from a pair colored differently, covering all occurrences across the neck.

Implemented as a Svelte page (`web/src/routes/gmc/browse/`) consuming the Rust
theory modules via WASM.

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

### Rust modules

- `src/theory/scales.rs` — `Scale`, `ParentScale`, 28 modes as `Scale::ALL`
- `src/theory/gmc.rs` — `TriadPairSet`, `PAIRS` const, `resolve_pair()`, `pair_display()`
- `src/wasm_api.rs` — WASM exports: `get_all_scales()`, `get_pairs()`, `resolve_pair()`,
  `pair_display()`, `get_fretboard_notes()`, `get_interval_name()`

### Svelte frontend

- `web/src/routes/gmc/browse/+page.svelte` — main GMC Browse page
- `web/src/lib/components/Fretboard.svelte` — SVG panoramic fretboard (15 frets)
- `web/src/lib/components/PairDrawer.svelte` — collapsible pair list (260px)
- `web/src/lib/stores.ts` — Svelte stores for root/scale/pair/interval state

## UI Layout

### Sub-tab bar

Browse | Tune tabs, with drawer toggle button ("▶ Show pairs" / "◀ Hide pairs")
aligned right.

### Controls (top of content area)

- **Root** — 12-note dropdown
- **Scale** — 28 modes grouped by ParentScale via `<optgroup>` (Major, Harm. Minor,
  Mel. Minor, Harm. Major)
- **Intervals** — checkbox to toggle dot labels between note names and intervals

### Pair drawer (collapsible, left)

- 260px wide when open, 0 when closed
- Toggle via button in sub-tab bar
- Lists all 10 pairs with label (bold) + display string (intervals = notes)
- Selected pair highlighted with `--primary-muted` background
- Slide transition 200ms

### Central panel

SVG panoramic fretboard: 15 frets × 6 strings.
- Background: `#222`
- Strings: `#555`, 1px
- Frets: `#3d3d3d`, 1px; nut: `#888`, 2.5px
- Dots: 14px diameter (7px radius)
  - Triad A: `--primary` (amber `#d4a574`)
  - Triad B: `--secondary` (blue `#8ecae6`)
  - Label: 8px bold `#1a1a1a` centered inside dot

## Future (Phase 2)

- GMC Tune: exercise/étude generation with triad pairs over chord changes
- Melodic exercises over triad pair material
- Voice-leading between pairs across chord changes

## Reference

Based on Tim Miller & Mick Goodrick's Generic Modal Compression system.
Pedro's complete study spreadsheet: `~/syncsyncs/GMC PAIRS.xlsx`
