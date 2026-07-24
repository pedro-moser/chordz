# Pattern Presets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the six `.gp`-distilled line-pattern presets to the GMC pattern picker, extracting the preset list into a testable lib module that can also set the rhythmic figure.

**Architecture:** Move `PATTERN_PRESETS` out of `+page.svelte` into `web/src/lib/patternPresets.ts` (with an optional `figureIndex` per preset), add the 6 new presets alongside the existing 3, cover them with a vitest, and wire the component to import the list and apply the figure in `selectPatternPreset`. Front-end only — no Rust/wasm.

**Tech Stack:** SvelteKit (TypeScript, Svelte 5 runes), vitest.

**Spec:** `docs/superpowers/specs/2026-05-31-pattern-presets-design.md`

**Toolchain:** Web only, from `web/`: `npx vitest run <file>`, `npm run check`, `npm run build`.

---

## File Structure

- **Create** `web/src/lib/patternPresets.ts` — the `PatternPreset` type + `PATTERN_PRESETS` list (3 existing + 6 new). One responsibility: the pattern preset data.
- **Create** `web/src/lib/patternPresets.test.ts` — vitest validating every preset is well-formed.
- **Modify** `web/src/routes/gmc/tune/+page.svelte` — import the list, drop the inline const, set the figure in `selectPatternPreset`.

---

## Task 1: `patternPresets.ts` lib module + test

**Files:**
- Create: `web/src/lib/patternPresets.ts`
- Create: `web/src/lib/patternPresets.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/patternPresets.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { PATTERN_PRESETS } from './patternPresets';

describe('PATTERN_PRESETS', () => {
  it('has the 3 originals plus the 6 distilled presets', () => {
    const labels = PATTERN_PRESETS.map((p) => p.label);
    expect(labels).toContain('Alternating 3+3');
    expect(labels).toContain('Triad-Pair Arch');
    expect(labels).toContain('2-1-2 Run');
    expect(labels).toContain('3+3 Zigzag');
    expect(labels).toContain('3+3+2 Eighth Spine');
    expect(labels).toContain('Triplet 3+3');
    expect(labels).toContain('Arch (3rd-anchored)');
    expect(PATTERN_PRESETS.length).toBe(9);
  });

  it('every preset is well-formed', () => {
    for (const preset of PATTERN_PRESETS) {
      expect(preset.label.length).toBeGreaterThan(0);
      expect(preset.blocks.length).toBeGreaterThan(0);
      if (preset.figureIndex !== undefined) {
        expect([0, 1, 2]).toContain(preset.figureIndex);
      }
      for (const b of preset.blocks) {
        expect(b.count).toBeGreaterThanOrEqual(1);
        expect(b.count).toBeLessThanOrEqual(6);
        expect(['asc', 'desc']).toContain(b.direction);
        expect(['T1', 'T2']).toContain(b.triad);
        if (b.shape !== undefined) {
          expect(b.shape.length).toBeGreaterThan(0);
          for (const role of b.shape) expect([0, 1, 2]).toContain(role);
        }
        if (b.anchor !== undefined) {
          expect(['root', 'third', 'fifth']).toContain(b.anchor);
        }
      }
    }
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd web && npx vitest run src/lib/patternPresets.test.ts`
Expected: FAIL — cannot resolve `./patternPresets` (module doesn't exist yet).

- [ ] **Step 3: Create the lib module**

Create `web/src/lib/patternPresets.ts`:

```typescript
import type { GmcPatternBlock } from '$lib/wasm';

export interface PatternPreset {
  label: string;
  blocks: GmcPatternBlock[];
  /** 0=Eighth, 1=Sixteenth, 2=Triplet. Omitted → leave the current figure as-is. */
  figureIndex?: number;
}

// roles 0/1/2 = root/3rd/5th of the stacked-thirds pair. The first three are the original
// generic presets; the rest are distilled from Pedro's .gp études (2026-05-31 pattern mining).
export const PATTERN_PRESETS: PatternPreset[] = [
  {
    label: 'Alternating 3+3',
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1' },
      { count: 3, direction: 'desc', triad: 'T2' },
    ],
  },
  {
    label: 'Continuous up',
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1' },
      { count: 3, direction: 'asc', triad: 'T2' },
    ],
  },
  {
    label: 'Short-long',
    blocks: [
      { count: 2, direction: 'asc', triad: 'T1' },
      { count: 4, direction: 'desc', triad: 'T2' },
    ],
  },
  {
    label: 'Triad-Pair Arch',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [2, 0] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
      { count: 3, direction: 'desc', triad: 'T1', shape: [2, 1, 0] },
    ],
  },
  {
    label: '2-1-2 Run',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [2, 0] },
    ],
  },
  {
    label: '3+3 Zigzag',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
    ],
  },
  {
    label: '3+3+2 Eighth Spine',
    figureIndex: 0,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [0, 1, 2], anchor: 'root' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'desc', triad: 'T1', shape: [2, 1] },
    ],
  },
  {
    label: 'Triplet 3+3',
    figureIndex: 2,
    blocks: [
      { count: 3, direction: 'desc', triad: 'T1', shape: [1, 0, 2] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [0, 1, 2] },
    ],
  },
  {
    label: 'Arch (3rd-anchored)',
    figureIndex: 1,
    blocks: [
      { count: 3, direction: 'asc', triad: 'T1', shape: [1, 2, 0], anchor: 'third' },
      { count: 3, direction: 'asc', triad: 'T2', shape: [0, 1, 2] },
      { count: 2, direction: 'asc', triad: 'T1', shape: [0, 1] },
      { count: 3, direction: 'desc', triad: 'T2', shape: [2, 1, 0] },
      { count: 3, direction: 'desc', triad: 'T1', shape: [2, 1, 0] },
    ],
  },
];
```

- [ ] **Step 4: Run to verify it passes**

Run: `cd web && npx vitest run src/lib/patternPresets.test.ts`
Expected: PASS — 2 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/patternPresets.ts web/src/lib/patternPresets.test.ts
git commit -m "feat(gmc): pattern-preset lib module with the 6 distilled études"
```

---

## Task 2: Wire the lib into the GMC tune page

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Import the lib + remove the inline const**

In `web/src/routes/gmc/tune/+page.svelte`, add to the `<script>` imports (near the other `$lib` imports at the top):

```typescript
  import { PATTERN_PRESETS } from '$lib/patternPresets';
```

Then DELETE the inline `const PATTERN_PRESETS: { label: string; blocks: GmcPatternBlock[] }[] = [ … ];` block (currently lines ~16-20 — the three-entry array). The imported list replaces it.

- [ ] **Step 2: Make `selectPatternPreset` also set the figure**

Find `function selectPatternPreset(idx: number)` (≈ line 262) and replace its body:

```typescript
  function selectPatternPreset(idx: number) {
    if (idx >= 0 && idx < PATTERN_PRESETS.length) {
      const p = PATTERN_PRESETS[idx];
      pattern = [...p.blocks];
      if (p.figureIndex !== undefined) figureIndex = p.figureIndex;
    }
  }
```

> `figureIndex` is an existing `$state` in this component (0=Eighth, 1=Sixteenth, 2=Triplet). The `<select>` that calls `selectPatternPreset` already iterates `PATTERN_PRESETS`, so the six new entries appear automatically — no template change.

- [ ] **Step 3: Type-check and build**

Run: `cd web && npm run check && npm run build`
Expected: 0 svelte-check errors; build succeeds.

- [ ] **Step 4: Run the preset test once more (still green after the move)**

Run: `cd web && npx vitest run src/lib/patternPresets.test.ts`
Expected: PASS.

- [ ] **Step 5: Manual check (optional, the human does this)**

`cd web && npm run dev`, open GMC → Tune, generate a tune, then use the pattern-preset `<select>`: pick **Triad-Pair Arch** (the tab should redraw as the up-and-back arch and the figure switches to 16ths), **Triplet 3+3** (figure → triplet), etc. Each preset sets both the blocks and the figure.

- [ ] **Step 6: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): pattern presets set the figure; source from the lib module"
```

---

## Self-Review

**Spec coverage:**
- Extract `PATTERN_PRESETS` to `web/src/lib/patternPresets.ts` with optional `figureIndex` → Task 1.
- The 6 distilled presets with exact blocks + figures → Task 1 (the const).
- vitest validating well-formedness → Task 1 (the test).
- Component imports the list, drops the inline const, `selectPatternPreset` applies the figure → Task 2.
- Existing `<select>` shows them automatically → Task 2 (no template change needed).
- Front-end only, augment not replace → both tasks (3 originals kept; no Rust/wasm).

**Placeholder scan:** No TBD/TODO; every code step is complete (all 9 presets spelled out; the full test).

**Type consistency:** `PatternPreset { label, blocks: GmcPatternBlock[], figureIndex? }` defined in Task 1, consumed in Task 2's `selectPatternPreset` (`p.blocks`, `p.figureIndex`). `GmcPatternBlock` imported as a type from `$lib/wasm` (erased at runtime, so the test pulls no wasm). The test asserts 9 presets and the new labels, matching the const.
