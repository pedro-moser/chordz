# 🎲 Cores — Scale Shuffle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "🎲 Cores" button that sets each chord's scale override to a random scale valid for its quality, to explore color combinations.

**Architecture:** A pure core `valid_scales(quality)` (guide-tone rule: scale contains the 3rd + 7th, plus the 5th only when it is altered) → a thin wasm `valid_scales_for_chart` returning per-chord index lists → a TS wrapper → a UI button that picks a random valid index per chord (JS `Math.random`), sets `scaleOverrides` + `overridesFor`, and regenerates. Randomness in the front, validity in the core.

**Tech Stack:** Rust core (cargo, native nightly), `wasm-bindgen`/`wasm-pack`, SvelteKit (TypeScript, Svelte 5 runes).

**Spec:** `docs/superpowers/specs/2026-05-31-scale-shuffle-design.md`

**Toolchain (cargo/rustc/wasm-pack NOT on PATH):**
- Native tests: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib`
- WASM type-check: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
- WASM build: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
- Web checks (in `web/`): `npm run check`, `npm run build`

> Confirm the dated toolchain name with `ls ~/.rustup/toolchains/` — the date drifts.

---

## File Structure

- **Modify** `src/theory/scale_defaults.rs` — add `valid_scales(quality)` + tests (sibling of `default_scale`/`etude_scale`).
- **Modify** `src/wasm_api.rs` — add `valid_scales_for_chart` export (after `shell_etude_preset`).
- **Modify** `web/src/wasm.d.ts` — declare it.
- **Modify** `web/src/lib/wasm.ts` — `validScalesForChart` wrapper + `ValidScalesResult` type.
- **Modify** `web/src/routes/gmc/tune/+page.svelte` — "🎲 Cores" button + `applyScaleShuffle` handler.

---

## Task 1: Core `valid_scales`

**Files:**
- Modify: `src/theory/scale_defaults.rs`

- [ ] **Step 1: Write the failing tests**

In `src/theory/scale_defaults.rs`, inside the existing `#[cfg(test)] mod tests { … }` block (it has a `quality(name)` helper), add:

```rust
    #[test]
    fn valid_scales_match_chord_quality() {
        let names = |q: &str| -> Vec<&'static str> {
            valid_scales(quality(q)).iter().map(|&i| Scale::ALL[i].name).collect()
        };
        // maj7: bright major modes, never a minor mode.
        let maj7 = names("maj7");
        assert!(maj7.contains(&"Lydian"));
        assert!(maj7.contains(&"Ionian"));
        assert!(!maj7.contains(&"Dorian"));
        // m7: minor modes, never a major mode.
        let m7 = names("m7");
        assert!(m7.contains(&"Dorian"));
        assert!(m7.contains(&"Aeolian"));
        assert!(!m7.contains(&"Ionian"));
        // dom7: the full dominant palette includes Altered (max color).
        assert!(names("dom7").contains(&"Altered"));
        // m7b5: half-diminished family. The conditional-5th refinement EXCLUDES natural-5 Dorian.
        let m7b5 = names("m7b5");
        assert!(m7b5.contains(&"Locrian"));
        assert!(!m7b5.contains(&"Dorian"));
    }

    #[test]
    fn valid_scales_always_contains_the_default_and_is_non_empty() {
        for q in ChordQuality::ALL {
            let v = valid_scales(q);
            assert!(!v.is_empty(), "{} has no valid scales", q.name);
            let def = Scale::ALL.iter().position(|s| s.name == default_scale(q).name).unwrap();
            assert!(v.contains(&def), "{} valid set missing its default scale", q.name);
        }
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib scale_defaults`
Expected: FAIL — `cannot find function valid_scales`.

- [ ] **Step 3: Implement `valid_scales`**

In `src/theory/scale_defaults.rs`, directly after the `etude_scale` function (before `#[cfg(test)]`), add:

```rust
/// Indices into `Scale::ALL` of every scale valid for `quality` under the guide-tone rule: the
/// scale contains the chord's 3rd and 7th, plus the 5th **only when the 5th is altered** (not the
/// perfect 5th — semitone 7). This keeps the full color palette for plain maj7/m7/dom7 while keeping
/// the b5/°5/#5 family honest (e.g. no natural-5 Dorian over a m7b5). Always non-empty: the chord's
/// own `default_scale` satisfies the rule. Used by the "🎲 Cores" scale shuffle.
pub fn valid_scales(quality: &ChordQuality) -> Vec<usize> {
    let semitones = |i: usize| quality.intervals.get(i).map(|iv| iv.semitones);
    let third = semitones(1);
    let fifth = semitones(2);
    let seventh = semitones(3);
    Scale::ALL
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            let has = |st: Option<u8>| st.map_or(true, |v| s.semitones.contains(&v));
            // Constrain the 5th only when it is altered (a perfect 5th is left free for color).
            let fifth_ok = match fifth {
                None | Some(7) => true,
                Some(f) => s.semitones.contains(&f),
            };
            has(third) && has(seventh) && fifth_ok
        })
        .map(|(i, _)| i)
        .collect()
}
```

- [ ] **Step 4: Run to verify pass**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo test --lib scale_defaults`
Expected: PASS (the 2 new tests + the existing scale_defaults tests).

- [ ] **Step 5: Commit**

```bash
git add src/theory/scale_defaults.rs
git commit -m "feat(gmc): valid_scales — guide-tone scale set per chord quality"
```

---

## Task 2: WASM `valid_scales_for_chart`

**Files:**
- Modify: `src/wasm_api.rs` (add after `shell_etude_preset`, before `generate_walking_bass`)
- Modify: `web/src/wasm.d.ts`

- [ ] **Step 1: Add the Rust export**

In `src/wasm_api.rs`, locate `pub fn shell_etude_preset(...)` and add, immediately after its closing `}` (and before the `/// Generate a quarter-note walking bass…` doc comment of `generate_walking_bass`):

```rust
/// Per-chord lists of `Scale::ALL` indices valid for each chord's quality, for the "🎲 Cores"
/// scale shuffle. The web front picks one random index per chord. Returns
/// `{ validScales: number[][] }`, or `{ error }` on a parse failure.
#[wasm_bindgen]
pub fn valid_scales_for_chart(chart_text: &str, title: &str) -> JsValue {
    use crate::theory::scale_defaults;

    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let valid: Vec<Vec<usize>> = chart
        .changes
        .iter()
        .map(|c| scale_defaults::valid_scales(c.quality))
        .collect();

    to_js(&serde_json::json!({ "validScales": valid }))
}
```

If `Chart`/`to_js` are referred to differently in this file, mirror how `shell_etude_preset` uses them (read it). Do not invent names.

- [ ] **Step 2: Type-check the wasm build**

Run: `TC=$(ls -d ~/.rustup/toolchains/nightly-*-x86_64-unknown-linux-gnu | grep -v '/nightly-x86_64' | head -1); PATH="$TC/bin:$PATH" cargo check --no-default-features --features wasm --lib`
Expected: 0 errors (2 pre-existing `ui/` warnings OK).

- [ ] **Step 3: Declare it in `web/src/wasm.d.ts`**

After the `shell_etude_preset` line, add:

```typescript
  export function valid_scales_for_chart(chart_text: string, title: string): any;
```

- [ ] **Step 4: Rebuild the wasm package**

Run: `TC=~/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu; PATH="$TC/bin:$HOME/.cargo/bin:$PATH" wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm`
Expected: succeeds. Verify: `grep -c valid_scales_for_chart web/pkg/chordz.js` → ≥ 1.

- [ ] **Step 5: Commit**

```bash
git add src/wasm_api.rs web/src/wasm.d.ts web/pkg
git commit -m "feat(gmc): valid_scales_for_chart wasm export"
```
(`web/pkg` is gitignored — `git add web/pkg` is a no-op; commit the source.)

---

## Task 3: Web wrapper `validScalesForChart`

**Files:**
- Modify: `web/src/lib/wasm.ts` (after `shellEtudePreset`)

- [ ] **Step 1: Add the wrapper + result type**

In `web/src/lib/wasm.ts`, directly after the `shellEtudePreset` function, add:

```typescript
/** Per-chord lists of valid scale indices for the 🎲 Cores shuffle. */
export interface ValidScalesResult {
  validScales?: number[][];
  error?: string;
}

/**
 * For each chord in the chart, the indices into the scale list that are valid for that chord's
 * quality (guide-tone rule). The caller shuffles by picking one at random per chord.
 */
export function validScalesForChart(chartText: string, title: string): ValidScalesResult {
  return getWasm().valid_scales_for_chart(chartText, title);
}
```

- [ ] **Step 2: Type-check the web project**

Run: `cd web && npm run check`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add web/src/lib/wasm.ts
git commit -m "feat(gmc): validScalesForChart web wrapper"
```

---

## Task 4: "🎲 Cores" button + handler

**Files:**
- Modify: `web/src/routes/gmc/tune/+page.svelte`

- [ ] **Step 1: Import the wrapper**

In the import from `'$lib/wasm'` that already includes `shellEtudePreset` (top of the `<script>`), add `validScalesForChart`. Example (adapt to the real line):

```typescript
  import { generateGmcLine, shellEtudePreset, validScalesForChart, getPresets, getPairs, getAllScales } from '$lib/wasm';
```

- [ ] **Step 2: Add the shuffle handler**

Directly after the `applyShellEtudePreset()` function in the `<script>`, add:

```typescript
  function applyScaleShuffle() {
    const r = validScalesForChart(chartInput, titleInput);
    if (r.error || !r.validScales) {
      if (r.error) error = r.error;
      return;
    }
    // Pick one valid scale at random per chord (empty list → leave default).
    scaleOverrides = r.validScales.map((list) =>
      list.length ? list[Math.floor(Math.random() * list.length)] : null,
    );
    // Keep generate()'s positional reset guard from wiping the shuffled overrides.
    overridesFor = chartInput;
    generate();
  }
```

> `overridesFor` and `scaleOverrides` are existing `$state` in this component (confirmed in the shell-preset work). If a name differs, mirror the real one.

- [ ] **Step 3: Add the button next to "Scales"**

Find the Scales button (search for `scales-btn` / `scaleModalOpen = true`); it is wrapped in `{#if result?.changes}`. Add the shuffle button inside the same block, right after the Scales button:

```svelte
      <button
        class="scales-btn"
        onclick={applyScaleShuffle}
        title="Sortear uma escala válida (3ª+7ª, 5ª se alterada) para cada acorde"
      >🎲 Cores</button>
```

> Confirm it sits inside the `{#if result?.changes} … {/if}` that wraps the Scales button (so it only shows once a line exists), and that the block stays balanced.

- [ ] **Step 4: Type-check and build**

Run: `cd web && npm run check && npm run build`
Expected: 0 svelte-check errors; build succeeds.

- [ ] **Step 5: Manual check (optional, the human does this)**

`cd web && npm run dev`, open GMC → Tune, generate a tune, click **🎲 Cores** a few times. Confirm each chord's scale label changes to a quality-appropriate scale and the line regenerates; clicking again gives a new combination; the Scales modal still lets you hand-edit.

- [ ] **Step 6: Commit**

```bash
git add web/src/routes/gmc/tune/+page.svelte
git commit -m "feat(gmc): 🎲 Cores — shuffle valid scales per chord"
```

---

## Self-Review

**Spec coverage:**
- Guide-tone rule with conditional 5th → Task 1 (`valid_scales`) + tests (m7b5 excludes Dorian).
- Always non-empty (default fits) → Task 1 test 2.
- `valid_scales_for_chart` returning `{ validScales: number[][] }` → Task 2.
- TS wrapper + type → Task 3. Button + handler (random per chord, sets scaleOverrides + overridesFor, regenerates) → Task 4.
- Scope: scales only (no pair/pattern touched) → handler sets only `scaleOverrides`.

**Placeholder scan:** No TBD/TODO; every code step is complete.

**Type consistency:** `valid_scales(&ChordQuality) -> Vec<usize>` (Task 1) used in `valid_scales_for_chart` (Task 2). `valid_scales_for_chart(chart_text, title)` matches the `.d.ts` decl (Task 2) and the TS wrapper (Task 3). `ValidScalesResult { validScales?, error? }` consumed by `applyScaleShuffle` (Task 4), which sets the existing `scaleOverrides`/`overridesFor` state. `Math.random` is browser-side (fine in the Svelte app).
