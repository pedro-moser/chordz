# Shell Étude Transparent Preset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the opaque Shell Étude engine with a transparent "Shell Étude" preset button that sets the existing GMC controls (pair "7no5/7no3" + per-chord characteristic scales + an Airegin-distilled arc pattern) and generates through the normal path.

**Architecture:** Add a core `etude_scale(quality)` map and a thin wasm `shell_etude_preset` descriptor that returns `{pairIndex, scaleOverrides, pattern}`. The web preset button applies those to the visible controls and calls the existing `generate()`. Then delete the now-redundant engine (`shells.rs`, `generate_shell_line`, `generate_shell_etude`, the `etudeMode` toggle). Ordering adds the new path first (everything stays green), then removes the old.

**Tech Stack:** Rust core (cargo, native nightly), `wasm-bindgen`/`wasm-pack`, SvelteKit (TypeScript, Svelte 5 runes).

**Spec:** `docs/superpowers/specs/2026-05-30-shell-etude-preset-design.md`

**Toolchain (cargo/rustc/wasm-pack NOT on PATH):**
- Native tests: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
- WASM type-check: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
- WASM build: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
- Web checks (in `web/`): `npm run check`, `npm run build`

> Confirm the dated toolchain name with `ls ~/.rustup/toolchains/` — the date drifts.

---

## File Structure

- **Modify** `src/theory/scale_defaults.rs` — add `etude_scale(quality)` (sibling of `default_scale`) + tests.
- **Modify** `src/wasm_api.rs` — add `shell_etude_preset` export (Task 2); later delete `generate_shell_etude` (Task 5).
- **Modify** `web/src/wasm.d.ts` — add `shell_etude_preset` decl (Task 3); later remove `generate_shell_etude` (Task 5).
- **Modify** `web/src/lib/wasm.ts` — add `shellEtudePreset` wrapper (Task 3); later remove `generateShellEtude` (Task 5).
- **Modify** `web/src/routes/gmc/tune/+page.svelte` — replace toggle with preset button; remove `etudeMode` + the `{#if !etudeMode}` sites (Task 4).
- **Delete** `src/theory/shells.rs`; **modify** `src/theory/mod.rs`, `src/theory/line_engine.rs` (remove `generate_shell_line`/`resolve_shell_notes`/shell tests) (Task 5).

---

## Task 1: Core `etude_scale` map

**Files:**
- Modify: `src/theory/scale_defaults.rs`

- [ ] **Step 1: Write the failing tests**

In `src/theory/scale_defaults.rs`, inside the existing `#[cfg(test)] mod tests { … }` block (it already has a `quality(name)` helper), add:

```rust
    #[test]
    fn etude_scales_per_quality() {
        assert_eq!(etude_scale(quality("maj7")).name, "Lydian");
        assert_eq!(etude_scale(quality("maj9")).name, "Lydian");
        assert_eq!(etude_scale(quality("maj7#11")).name, "Lydian");
        assert_eq!(etude_scale(quality("m7")).name, "Dorian");
        assert_eq!(etude_scale(quality("m11")).name, "Dorian");
        assert_eq!(etude_scale(quality("m7b5")).name, "Aeolian \u{266D}5");
        assert_eq!(etude_scale(quality("dom7")).name, "Altered");
        assert_eq!(etude_scale(quality("dom9")).name, "Altered");
        assert_eq!(etude_scale(quality("dom7b9")).name, "Altered");
        // Unmapped qualities fall back to the plain default scale.
        assert_eq!(
            etude_scale(quality("dim7")).name,
            default_scale(quality("dim7")).name
        );
    }

    #[test]
    fn etude_scale_with_7no5_7no3_pair_spells_the_shells() {
        // The whole premise: the etude scale, split by the "7no5/7no3" triad pair, reproduces
        // the Airegin guide-tone shells. Sets, because the partition order differs from the
        // old offset order.
        use crate::theory::gmc::{self, PAIRS};
        use std::collections::HashSet;
        let pair = PAIRS.iter().find(|p| p.label == "7no5/7no3").unwrap();

        // Fm7 (root F=5): shells {Eb,Bb,D}={3,10,2} + {Ab,C,G}={8,0,7}.
        let (a, b) = gmc::resolve_pair(5, etude_scale(quality("m7")), pair);
        assert_eq!(a.iter().copied().collect::<HashSet<u8>>(), HashSet::from([3, 10, 2]));
        assert_eq!(b.iter().copied().collect::<HashSet<u8>>(), HashSet::from([8, 0, 7]));

        // Cmaj7 (root C=0): shells {B,F#,A}={11,6,9} + {E,G,D}={4,7,2}.
        let (a, b) = gmc::resolve_pair(0, etude_scale(quality("maj7")), pair);
        assert_eq!(a.iter().copied().collect::<HashSet<u8>>(), HashSet::from([11, 6, 9]));
        assert_eq!(b.iter().copied().collect::<HashSet<u8>>(), HashSet::from([4, 7, 2]));
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib scale_defaults`
Expected: FAIL — `cannot find function etude_scale`.

- [ ] **Step 3: Implement `etude_scale`**

In `src/theory/scale_defaults.rs`, directly after the `default_scale` function (before the `#[cfg(test)]` module), add:

```rust
/// The characteristic guide-tone "shell" scale per quality: the scale that, split by the
/// `7no5/7no3` triad pair (`gmc::PAIRS`), spells the chord's two upper-structure shells.
/// Differs from `default_scale` for maj7 (Lydian), dominants (Altered), and m7b5 (Aeolian
/// ♭5) — that override is why a shell line sounds hipper than a default GMC line. Used by the
/// "Shell Étude" preset. Unmapped qualities (e.g. dim7) fall back to `default_scale`.
pub fn etude_scale(quality: &ChordQuality) -> &'static Scale {
    match quality.name {
        "maj7" | "maj9" | "maj13" | "maj7#11" => find_scale("Lydian"),
        "m7" | "m9" | "m11" | "m13" => find_scale("Dorian"),
        "m7b5" | "m9b11" => find_scale("Aeolian \u{266D}5"),
        "dom7" | "dom9" | "dom13" | "dom7#5" | "dom7b9" | "dom7#9" | "dom7#11" | "dom7b13" => {
            find_scale("Altered")
        }
        _ => default_scale(quality),
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib scale_defaults`
Expected: PASS (the two new tests + the existing default-scale tests).

- [ ] **Step 5: Commit**

```bash
git add src/theory/scale_defaults.rs
git commit -m "feat(gmc): etude_scale — characteristic shell scale per chord quality

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: WASM `shell_etude_preset` export

**Files:**
- Modify: `src/wasm_api.rs` (add after `generate_shell_etude`, before `generate_walking_bass` ≈ line 791)
- Modify: `web/src/wasm.d.ts`

- [ ] **Step 1: Add the Rust export**

In `src/wasm_api.rs`, immediately BEFORE the `/// Generate a quarter-note walking bass line…` doc comment of `generate_walking_bass`, add:

```rust
/// Describe the "Shell Étude" preset for a chart: the GMC controls that reproduce a
/// guide-tone (7no5/7no3) line. Returns `{ pairIndex, scaleOverrides, pattern }` so the web
/// UI can set its visible controls and generate through the normal `generate_gmc_line` path —
/// the preset is a transparent shortcut, not a separate engine.
#[wasm_bindgen]
pub fn shell_etude_preset(chart_text: &str, title: &str) -> JsValue {
    use crate::theory::gmc::PAIRS;
    use crate::theory::scale_defaults;

    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let pair_index = PAIRS.iter().position(|p| p.label == "7no5/7no3");

    let scale_overrides: Vec<Option<usize>> = chart
        .changes
        .iter()
        .map(|c| {
            let s = scale_defaults::etude_scale(c.quality);
            Scale::ALL.iter().position(|x| x.name == s.name)
        })
        .collect();

    // Airegin-distilled contour: a 6-up / 6-down arc through both shells (vs. the locked
    // 3-up/3-down). Same block shape the pattern editor consumes.
    let pattern = serde_json::json!([
        { "count": 3, "direction": "asc", "triad": "T1" },
        { "count": 3, "direction": "asc", "triad": "T2" },
        { "count": 3, "direction": "desc", "triad": "T1" },
        { "count": 3, "direction": "desc", "triad": "T2" },
    ]);

    to_js(&serde_json::json!({
        "pairIndex": pair_index,
        "scaleOverrides": scale_overrides,
        "pattern": pattern,
    }))
}
```

- [ ] **Step 2: Type-check the wasm build**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: 0 errors (the 2 pre-existing warnings in `ui/` are OK).

- [ ] **Step 3: Declare it in `web/src/wasm.d.ts`**

In `web/src/wasm.d.ts`, after the `generate_shell_etude` line (≈ line 21), add:

```typescript
  export function shell_etude_preset(chart_text: string, title: string): any;
```

- [ ] **Step 4: Rebuild the wasm package**

Run: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
Expected: build succeeds. Verify: `grep -c shell_etude_preset web/pkg/chordz.js` → ≥ 1.

- [ ] **Step 5: Commit**

```bash
git add src/wasm_api.rs web/src/wasm.d.ts web/pkg
git commit -m "feat(gmc): shell_etude_preset wasm descriptor (pair + scales + pattern)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
(`web/pkg` is gitignored — the `git add web/pkg` is a no-op; commit the source.)

---

## Task 3: Web wrapper `shellEtudePreset`

**Files:**
- Modify: `web/src/lib/wasm.ts` (after `generateShellEtude`, ≈ line 224)

- [ ] **Step 1: Add the typed wrapper + result type**

In `web/src/lib/wasm.ts`, directly AFTER the `generateShellEtude` function (it ends with the line returning `getWasm().generate_shell_etude(...)` and its closing `}`), add:

```typescript
/** The controls the Shell Étude preset sets. `pattern` reuses the GMC pattern-block shape. */
export interface ShellEtudePresetResult {
  pairIndex?: number;
  scaleOverrides?: (number | null)[];
  pattern?: GmcPatternBlock[];
  error?: string;
}

/**
 * Describe the Shell Étude preset for a chart: the triad pair, per-chord scale overrides, and
 * the arc pattern that reproduce a guide-tone (7no5/7no3) line through the normal GMC engine.
 * The caller applies these to the visible controls — the preset is transparent and editable.
 */
export function shellEtudePreset(chartText: string, title: string): ShellEtudePresetResult {
  return getWasm().shell_etude_preset(chartText, title);
}
```

- [ ] **Step 2: Type-check the web project**

Run: `cd web && npm run check`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/wasm.ts
git commit -m "feat(gmc): shellEtudePreset web wrapper

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Replace the toggle with a transparent preset button

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Update the import**

Change line 4 from:

```typescript
  import { generateGmcLine, generateShellEtude, getPresets, getPairs, getAllScales } from '$lib/wasm';
```
to:
```typescript
  import { generateGmcLine, shellEtudePreset, getPresets, getPairs, getAllScales } from '$lib/wasm';
```

- [ ] **Step 2: Remove the `etudeMode` state**

Delete line 48 (`let etudeMode = $state(false);`).

- [ ] **Step 3: Revert both generate call sites to the plain GMC call**

In `generate()` (≈ line 152-154), replace:

```typescript
    const res = etudeMode
      ? generateShellEtude(chartInput, titleInput, figureIndex, positionFret, pattern)
      : generateGmcLine(chartInput, titleInput, pairIndex, scaleOverrides, figureIndex, positionFret, pattern);
```
with:
```typescript
    const res = generateGmcLine(chartInput, titleInput, pairIndex, scaleOverrides, figureIndex, positionFret, pattern);
```

In `regenerate()` (≈ line 174-176), replace the identical `etudeMode ? … : …` block with the same single line:

```typescript
    const res = generateGmcLine(chartInput, titleInput, pairIndex, scaleOverrides, figureIndex, positionFret, pattern);
```

- [ ] **Step 4: Add the preset handler**

In the `<script>`, directly after the `regenerate()` function (≈ line 180), add:

```typescript
  function applyShellEtudePreset() {
    const p = shellEtudePreset(chartInput, titleInput);
    if (p.error) {
      error = p.error;
      return;
    }
    if (p.pairIndex != null) pairIndex = p.pairIndex;
    if (p.pattern) pattern = p.pattern;
    if (p.scaleOverrides) {
      scaleOverrides = p.scaleOverrides;
      // Mark these overrides as belonging to the current chart so generate()'s positional
      // reset guard doesn't immediately wipe them.
      overridesFor = chartInput;
    }
    generate();
  }
```

- [ ] **Step 5: Replace the toggle button and un-hide the pair selector**

Replace the button + `{#if !etudeMode}` wrap (≈ lines 421-433) — currently:

```svelte
        <button
          class="filter-btn"
          class:active={etudeMode}
          onclick={() => { etudeMode = !etudeMode; generate(); }}
          title="Guide-tone 7no5/7no3 shells per chord (Motor E)"
        >Shell Étude</button>
        {#if !etudeMode}
          <select class="control-select" bind:value={pairIndex}>
            {#each pairs as p, i}
              <option value={i}>{p.label}</option>
            {/each}
          </select>
        {/if}
```
with (button now applies the preset; select is always shown):

```svelte
        <button
          class="filter-btn"
          onclick={applyShellEtudePreset}
          title="Set pair 7no5/7no3 + characteristic scales + arc pattern (editable)"
        >Shell Étude</button>
        <select class="control-select" bind:value={pairIndex}>
          {#each pairs as p, i}
            <option value={i}>{p.label}</option>
          {/each}
        </select>
```

- [ ] **Step 6: Un-gate the Scales button**

Change (≈ line 504):
```svelte
    {#if result?.changes && !etudeMode}
```
to:
```svelte
    {#if result?.changes}
```

- [ ] **Step 7: Un-gate the two scale labels**

Remove the `{#if !etudeMode}` / `{/if}` around the tab scale `<text>` (≈ lines 614-623). Delete the line `{#if !etudeMode}` before the `<text` and the matching `{/if}` after that element's `</text>` — leaving the `<text …>{measure.chord.activeScale}</text>` unwrapped.

Then change the fretboard-header span (≈ line 671) from:
```svelte
            {#if !etudeMode}<span class="fb-scale" class:override={selectedMeasureData.chord.isOverride}>{selectedMeasureData.chord.activeScale}</span>{/if}
```
to:
```svelte
            <span class="fb-scale" class:override={selectedMeasureData.chord.isOverride}>{selectedMeasureData.chord.activeScale}</span>
```

- [ ] **Step 8: Type-check and build**

Run: `cd web && npm run check && npm run build`
Expected: 0 svelte-check errors; build succeeds. (Confirm no remaining `etudeMode` references: `grep -n etudeMode web/src/routes/gmc/tune/+page.svelte` → no output.)

- [ ] **Step 9: Manual check (the result)**

`cd web && npm run dev`, open the GMC tune page, pick **Moment's Notice**, click **Shell Étude**. Confirm: the Pair dropdown switches to **7no5/7no3**, each chord's scale label updates (Dorian/Lydian/Altered…), the pattern editor shows the 4-block arc, and the line plays and travels across the neck (not stuck). Everything stays editable.

- [ ] **Step 10: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): Shell Étude as a transparent preset button

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Delete the now-redundant opaque engine

Nothing references the old shell engine anymore (the UI uses the preset + `generate_gmc_line`). Remove it.

**Files:**
- Delete: `src/theory/shells.rs`
- Modify: `src/theory/mod.rs`, `src/theory/line_engine.rs`, `src/wasm_api.rs`, `web/src/lib/wasm.ts`, `web/src/wasm.d.ts`

- [ ] **Step 1: Delete the Rust shell module**

```bash
git rm src/theory/shells.rs
```
In `src/theory/mod.rs`, delete the line `pub mod shells;`.

- [ ] **Step 2: Remove the shell functions + imports from `line_engine.rs`**

In `src/theory/line_engine.rs`:
- Change line 1 from `use crate::theory::chart::{Chart, ChordChange};` to `use crate::theory::chart::Chart;`.
- Delete line 3 `use crate::theory::shells;`.
- Delete the entire `resolve_shell_notes` function (its doc comment through its closing `}`, ≈ lines 69-84) and the entire `generate_shell_line` function (its doc comment through closing `}`, ≈ lines 86-99).
- Fix the `TriadNotes` doc comment (≈ lines 26-27) from:
  ```rust
  /// Two 3-note pools plus their pitch classes in role order. Holds a GMC triad pair
  /// (`resolve_triad_notes`) or, reused as-is, a guide-tone shell pair (`resolve_shell_notes`).
  ```
  to:
  ```rust
  /// Two 3-note pools plus their pitch classes in role order (a GMC triad pair from
  /// `resolve_triad_notes`).
  ```
- Delete the two shell tests from the `#[cfg(test)] mod tests` block: `shell_line_outlines_each_chord_with_its_shells` and `shell_line_event_count_matches_grid` (≈ lines 502-551, including their `#[test]` attributes).

- [ ] **Step 3: Remove `generate_shell_etude` from `wasm_api.rs`**

In `src/wasm_api.rs`, delete the entire `generate_shell_etude` item — its doc comment (`/// Generate a guide-tone "shell étude" line (Motor E) over a chart…`), the `#[wasm_bindgen]`, and the function body through its closing `}` (≈ the block ending just before the `/// Describe the "Shell Étude" preset…` comment you added in Task 2). Leave `shell_etude_preset` and `generate_walking_bass` intact. The shared helpers (`parse_pattern_blocks`, `rhythmic_figure`, `line_events_json`) stay — `generate_gmc_line` still uses them.

- [ ] **Step 4: Remove the old web wrapper + decl**

In `web/src/lib/wasm.ts`, delete the `generateShellEtude` function (its JSDoc block through its closing `}`, ≈ lines 211-224).
In `web/src/wasm.d.ts`, delete the `generate_shell_etude` declaration line (≈ line 21).

- [ ] **Step 5: Verify everything green**

```bash
TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1)
PATH="$TC/bin:$PATH" cargo test --lib
PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib
TC2=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC2/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
cd web && npm run check && npm run build && cd ..
```
Expected: cargo test all green (shell tests gone, count drops by 2); wasm check 0 errors; wasm build ok; svelte-check 0 errors; web build ok. Confirm the dead export is gone: `grep -rc "generate_shell_etude\|generateShellEtude\|resolve_shell_pair\|generate_shell_line" src web/src` → 0.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(gmc): delete opaque shell engine, superseded by the preset

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:**
- `etude_scale` map (Lydian/Dorian/Altered/Aeolian ♭5 + fallback) → Task 1.
- Equivalence (pair 7no5/7no3 + etude_scale → shells), `.gp` oracle migrated → Task 1 test 2.
- `shell_etude_preset` returning `{pairIndex, scaleOverrides, pattern}` → Task 2.
- Arc pattern `[3↑T1,3↑T2,3↓T1,3↓T2]` → Task 2 (the `pattern` json).
- TS wrapper → Task 3. Preset button + transparent control updates + un-hidden controls → Task 4.
- Delete shells.rs / generate_shell_line / generate_shell_etude / etudeMode → Tasks 4-5.
- One-shot apply, editable, leaves figure/position/tune → Task 4 (handler sets pair+scales+pattern only).

**Placeholder scan:** No TBD/TODO; every code step shows complete code or an exact deletion target with anchors.

**Type consistency:** `etude_scale(&ChordQuality) -> &'static Scale` (Task 1) used in `shell_etude_preset` (Task 2). `shell_etude_preset(chart_text, title)` matches the `.d.ts` decl (Task 2) and the TS wrapper (Task 3). `ShellEtudePresetResult { pairIndex?, scaleOverrides?, pattern?, error? }` consumed by `applyShellEtudePreset` (Task 4). The handler sets the existing `pairIndex`/`pattern`/`scaleOverrides`/`overridesFor` state — names verified against the current component.

**Ordering:** new path added (Tasks 1-3) and switched in (Task 4) before the old path is deleted (Task 5), so every task leaves the build green. The `overridesFor = chartInput` line in Task 4 Step 4 is load-bearing — without it, `generate()`'s positional-reset guard wipes the preset scales.
