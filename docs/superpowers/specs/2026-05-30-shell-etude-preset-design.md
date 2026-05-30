# Shell Étude as a Transparent Preset — Design

**Date:** 2026-05-30
**Status:** Approved (design), pending implementation plan
**Supersedes the engine approach in:** `2026-05-30-shell-etude-generator-design.md`

## Summary

Replace the opaque "Shell Étude" mode (a separate `generate_shell_etude` engine that
bypassed and hid the user's controls) with a **transparent preset**: a button that *sets the
existing GMC controls* — triad pair, per-chord scale overrides, and the pattern — then
generates through the normal `generate_gmc_line` path. The user sees exactly what the preset
changed and can edit any of it afterward.

This is possible because of a discovery: **Motor E (guide-tone shells) is mathematically
identical to the existing GMC engine driven by the triad pair `"7no5/7no3"` (`PAIRS[9]`) plus
each chord's characteristic scale.** So no separate engine is needed — the preset is just a
shortcut for settings the user already has.

## Why this works (the equivalence)

The GMC engine splits a chord's scale's 6 non-root tones into two 3-note groups per the chosen
`TriadPairSet` partition. `PAIRS[9] = "7no5/7no3"` has indices `([2,4,5],[0,1,3])`. Applying
that partition to each chord's **characteristic scale** yields exactly the guide-tone shells:

| Quality | Characteristic scale (`Scale::ALL` idx) | `7no5/7no3` partition → shells |
|---------|------------------------------------------|--------------------------------|
| m7 family | Dorian (1) | b7,11,13 + b3,5,9 |
| maj7 family | Lydian (3) | 7,#11,13 + 3,5,9 |
| dominant family (all) | Altered (20) | b7,3,b13 + #9,#11,b9 |
| m7b5 / m9b11 | Aeolian ♭5 = Locrian ♮2 (19) | b7,11,b13 + b3,b5,9 |

Verified: e.g. Fm7 + Dorian + `PAIRS[9]` → `{Bb,D,Eb}` + `{G,Ab,C}` = the Airegin label
`{Eb,Bb,D}` + `{Ab,C,G}` (same sets). Note the preset deliberately **overrides the engine
defaults** for maj7 (default Ionian → Lydian), dominants (default Mixolydian → Altered), and
m7b5 (default Locrian → Aeolian ♭5) — that override is *why* the shells sound hipper than a
plain GMC line, and making it visible is the whole point of the transparent preset.

## The pattern (distilled from the Airegin contour)

Analysis of the Airegin melody's contour (each note classified by shell membership + direction):

- **Grouping:** ~3 notes per shell, alternating A↔B (same as the current default — not the issue).
- **Direction:** sustained across a whole bar — long ascending arcs through *both* shells, then
  descending arcs (`AAAABBBB ↑↑↑↑↑↑↑`, then descending bars). The current `preset_alternating`
  (3↑T1 / 3↓T2) reverses every 3 notes → nets ~zero motion → "stuck in one region."

**Preset default pattern — a 6-up / 6-down arc:**

```
[ {count:3, direction:asc, triad:T1},
  {count:3, direction:asc, triad:T2},
  {count:3, direction:desc, triad:T1},
  {count:3, direction:desc, triad:T2} ]
```

Ascends through both shells (6 notes up), then descends through both (6 down) — double the
directional span before reversing, so the line travels the neck like the Airegin étude instead
of hovering. The user can edit these blocks afterward (it's just the pattern editor).

## Architecture

### Core (`src/theory/scale_defaults.rs`)

Add a sibling to the existing `default_scale`:

```rust
/// The characteristic "guide-tone shell" scale per quality — the scale that, partitioned by
/// the `7no5/7no3` triad pair, spells the chord's two upper-structure shells. Differs from
/// `default_scale` for maj7 (Lydian), dominants (Altered), and m7b5 (Aeolian ♭5).
pub fn etude_scale(quality: &ChordQuality) -> &'static Scale
```

Mapping: `maj*` → Lydian; `m7|m9|m11|m13` → Dorian; `m7b5|m9b11` → Aeolian ♭5;
`dom*` → Altered; anything else → `default_scale(quality)` (covers dim7 etc.).

**Delete** `src/theory/shells.rs` and its `pub mod shells;` registration — the pitch-class
table is replaced by this one quality→scale map (single source of truth).

### Core (`src/theory/line_engine.rs`)

**Delete** `resolve_shell_notes` and `generate_shell_line` (the GMC path now covers shells via
the preset), the `use ...::shells` import, the now-unused `ChordChange` import if orphaned, and
the two `shell_line_*` tests. Keep `run_pattern` (used by `generate_line`). Update the
`TriadNotes` doc comment to drop the now-removed "shell pair" mention.

### WASM (`src/wasm_api.rs`)

**Delete** `generate_shell_etude`. Keep the shared helpers (`parse_pattern_blocks`,
`rhythmic_figure`, `line_events_json`) — now used by `generate_gmc_line` alone, still fine.

**Add** a preset descriptor export:

```rust
#[wasm_bindgen]
pub fn shell_etude_preset(chart_text: &str, title: &str) -> JsValue
```

Returns `{ pairIndex, scaleOverrides, pattern }`:
- `pairIndex` = index of `"7no5/7no3"` in `PAIRS` (computed, not hardcoded).
- `scaleOverrides` = per chord, the index of `etude_scale(change.quality)` in `Scale::ALL`.
- `pattern` = the fixed 4-block arc above (`[{count,direction,triad}, …]`), so the entire
  preset is defined in one core place and reflected verbatim in the UI's pattern editor.

On a parse error, return `{ "error": "…" }` (consistent with the other exports).

### Web

- `web/src/wasm.d.ts`: **remove** `generate_shell_etude` decl; **add** `shell_etude_preset`.
- `web/src/lib/wasm.ts`: **remove** `generateShellEtude`; **add**
  `shellEtudePreset(chartText, title): { pairIndex: number; scaleOverrides: (number|null)[]; pattern: GmcPatternBlock[] } | { error: string }`.
- `web/src/routes/gmc/tune/+page.svelte`:
  - **Remove** `etudeMode` state, the `generateShellEtude` import/branches in `generate()`/
    `regenerate()` (revert to the plain `generateGmcLine` call), and the `{#if !etudeMode}`
    wraps around the pair `<select>` and the scale labels (controls are always visible now).
  - **Add** a **"Shell Étude"** button near the pair selector. On click: call
    `shellEtudePreset(chartInput, titleInput)`; if not an error, set `pairIndex`,
    `scaleOverrides`, and `pattern` from the result, then call `generate()`. The pair dropdown,
    the per-chord scale labels, and the pattern editor visibly update to the preset values.

## Scope

The preset sets **pair + per-chord scales + pattern**. It leaves figure and neck position as
the user's current settings. It is a **one-shot apply** (not a live mode): after clicking, all
values live in the normal controls and are fully editable; switching tunes means re-clicking.

## Testing

- `etude_scale` returns the documented scale per quality (Dorian/Lydian/Altered/Aeolian ♭5,
  and a fallback for dim7), at the core level.
- **Equivalence oracle (the `.gp` migrates here):** for Fm7/Cmaj7/G7/Cm7b5,
  `gmc::resolve_pair(root, etude_scale(quality), &PAIRS[9])` equals the Airegin-labelled shell
  sets (asserted as sets, since the partition order differs from the old offset order).
- A core test that the `"7no5/7no3"` label resolves to a real `PAIRS` index (guards the lookup).

## Scope cuts (YAGNI)

- No separate engine, no pitch-class shell table (deleted — the scale map + existing partition
  cover it).
- The preset doesn't touch figure/position/tune.
- dim7 falls back to its default scale (no Moment's Notice impact; the prior literal-shell
  fallback is dropped).

## Open items for the plan

- Confirm the exact UI state variable names (`pattern`, `scaleOverrides`, `chartInput`,
  `titleInput`) at the two call sites and the pattern-editor binding.
- Decide whether to keep the wasm DRY helpers as-is (single caller) — default: keep.
