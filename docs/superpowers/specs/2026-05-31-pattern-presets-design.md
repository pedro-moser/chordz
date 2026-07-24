# Pattern Presets from the .gp Library — Design

**Date:** 2026-05-31
**Status:** Approved (design), pending implementation plan

## Summary

Add the six line-pattern presets distilled from Pedro's `.gp` library (the 2026-05-31 pattern
mining) to the GMC tune page's pattern picker, so the characteristic melodic shapes he actually
plays are one click away. Pure front-end data + a test — no Rust/wasm change. Along the way,
extract the preset list out of the Svelte component into an importable, testable lib module, and
let a preset also set the rhythmic figure (the shapes are figure-specific).

## Background

The web pattern picker already works off a JS array `PATTERN_PRESETS` in
`web/src/routes/gmc/tune/+page.svelte` (`{ label, blocks }[]`, 3 entries) selected via a `<select>`
that calls `selectPatternPreset(idx)` → `pattern = [...PATTERN_PRESETS[idx].blocks]`. The mining
produced six presets as exact `GmcPatternBlock` lists (validated against the engine grammar). A
`GmcPatternBlock` is `{ count: 1-6, direction: 'asc'|'desc', triad: 'T1'|'T2', shape?: number[] (roles 0-2), anchor?: 'root'|'third'|'fifth' }`. The rhythmic figure is a separate `figureIndex` state (0=Eighth, 1=Sixteenth, 2=Triplet).

## Architecture

### New: `web/src/lib/patternPresets.ts`

Move the preset list here (importable + testable) and extend the entry type with an optional figure:

```typescript
import type { GmcPatternBlock } from '$lib/wasm';

export interface PatternPreset {
  label: string;
  blocks: GmcPatternBlock[];
  /** 0=Eighth, 1=Sixteenth, 2=Triplet. Omitted → leave the current figure as-is. */
  figureIndex?: number;
}

export const PATTERN_PRESETS: PatternPreset[] = [ /* the existing 3 + the 6 new */ ];
```

The existing three (`Alternating 3+3`, `Continuous up`, `Short-long`) move verbatim (no
`figureIndex`). The six new ones (exact blocks from the mining; roles 0/1/2 = root/3rd/5th of the
stacked-thirds pair):

1. **Triad-Pair Arch** (figure 1 / 16ths) — the house motor: up T1→T2 to the octave, mirror down.
   ```
   [ {count:3,direction:'asc',triad:'T1',shape:[0,1,2],anchor:'root'},
     {count:3,direction:'asc',triad:'T2',shape:[0,1,2]},
     {count:2,direction:'asc',triad:'T1',shape:[2,0]},
     {count:3,direction:'desc',triad:'T2',shape:[2,1,0]},
     {count:3,direction:'desc',triad:'T1',shape:[2,1,0]} ]
   ```
2. **2-1-2 Run** (figure 1) — the ascending half of the arch.
   ```
   [ {count:3,direction:'asc',triad:'T1',shape:[0,1,2],anchor:'root'},
     {count:3,direction:'asc',triad:'T2',shape:[0,1,2]},
     {count:2,direction:'asc',triad:'T1',shape:[2,0]} ]
   ```
3. **3+3 Zigzag** (figure 1) — T1 up / T2 down, looping (the blues engine).
   ```
   [ {count:3,direction:'asc',triad:'T1',shape:[0,1,2]},
     {count:3,direction:'desc',triad:'T2',shape:[2,1,0]} ]
   ```
4. **3+3+2 Eighth Spine** (figure 0 / 8ths) — two ascending arpeggios + a 2-note return tag.
   ```
   [ {count:3,direction:'asc',triad:'T1',shape:[0,1,2],anchor:'root'},
     {count:3,direction:'asc',triad:'T2',shape:[0,1,2]},
     {count:2,direction:'desc',triad:'T1',shape:[2,1]} ]
   ```
5. **Triplet 3+3** (figure 2 / triplet) — 3 of T1, 3 of T2, descending.
   ```
   [ {count:3,direction:'desc',triad:'T1',shape:[1,0,2]},
     {count:3,direction:'desc',triad:'T2',shape:[0,1,2]} ]
   ```
6. **Arch (3rd-anchored)** (figure 1) — the arch entering on the 3rd.
   ```
   [ {count:3,direction:'asc',triad:'T1',shape:[1,2,0],anchor:'third'},
     {count:3,direction:'asc',triad:'T2',shape:[0,1,2]},
     {count:2,direction:'asc',triad:'T1',shape:[0,1]},
     {count:3,direction:'desc',triad:'T2',shape:[2,1,0]},
     {count:3,direction:'desc',triad:'T1',shape:[2,1,0]} ]
   ```

### Modify: `web/src/routes/gmc/tune/+page.svelte`

- Remove the inline `const PATTERN_PRESETS = [...]`; `import { PATTERN_PRESETS } from '$lib/patternPresets';`.
- `selectPatternPreset` also applies the figure when the preset specifies one:
  ```typescript
  function selectPatternPreset(idx: number) {
    if (idx >= 0 && idx < PATTERN_PRESETS.length) {
      const p = PATTERN_PRESETS[idx];
      pattern = [...p.blocks];
      if (p.figureIndex !== undefined) figureIndex = p.figureIndex;
    }
  }
  ```
- The existing `<select>` already iterates `PATTERN_PRESETS`, so the six show up automatically.

## Testing

`web/src/lib/patternPresets.test.ts` (vitest — the repo runs `npx vitest run`): for every preset,
assert each block is well-formed — `count` in 1-6, `direction` ∈ {asc, desc}, `triad` ∈ {T1, T2},
every `shape` role ∈ {0,1,2}, `anchor` (if present) ∈ {root, third, fifth}, and `figureIndex` (if
present) ∈ {0,1,2}. This guards the ~30 hand-entered blocks against typos.

## Scope

Front-end only: a new lib module, the component edit, and a vitest. No Rust, no wasm rebuild. The
six are added alongside the existing three (augment, not replace). The presets set the pattern
blocks + figure only; they do not touch pair, scales, or position.

## Scope cuts (YAGNI)

- The core `line_pattern.rs` presets (`preset_alternating`, etc.) are a separate Rust-native path —
  not touched; the web picker is the JS list.
- No "save my own preset" UI — just the curated six.
- The "held landing note" that ends the arch is faked (a repeated note) today; the faithful version
  needs Fase 2 (a separate, later feature).

## Open items for the plan

- Confirm the exact current `selectPatternPreset` body and the import line in `+page.svelte`.
- Confirm vitest config picks up `web/src/lib/*.test.ts` (existing tests live there).
