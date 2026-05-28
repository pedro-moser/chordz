# Web Frontend (Svelte + WASM) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Svelte web frontend for chordz that consumes the Rust theory/voicing logic via WASM, starting with the GMC Browse tab.

**Architecture:** Rust lib compiles to WASM via wasm-pack, exposing a JSON-based API through `src/wasm_api.rs`. A SvelteKit app in `web/` renders the UI with SVG fretboard, Warm Amber theme, and two-level navigation (Rail + Sub-tabs).

**Tech Stack:** Rust + wasm-bindgen + serde-wasm-bindgen, SvelteKit 5 (runes), TypeScript, Vite, SVG

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Add wasm-bindgen, serde-wasm-bindgen, crate-type cdylib |
| `src/wasm_api.rs` | Create | wasm-bindgen exported functions |
| `src/lib.rs` | Modify | Add `pub mod wasm_api;` |
| `web/package.json` | Create | SvelteKit project dependencies |
| `web/svelte.config.js` | Create | SvelteKit config |
| `web/vite.config.ts` | Create | Vite + WASM plugin |
| `web/tsconfig.json` | Create | TypeScript config |
| `web/src/app.html` | Create | HTML shell |
| `web/src/app.css` | Create | Global styles + theme tokens |
| `web/src/lib/wasm.ts` | Create | WASM init + typed wrapper |
| `web/src/lib/stores.ts` | Create | Svelte stores for state |
| `web/src/routes/+layout.svelte` | Create | Shell (rail + sub-tabs + slot) |
| `web/src/routes/+page.svelte` | Create | Redirect to /gmc/browse |
| `web/src/routes/gmc/browse/+page.svelte` | Create | GMC Browse page |
| `web/src/lib/components/Rail.svelte` | Create | Vertical nav rail |
| `web/src/lib/components/SubTabs.svelte` | Create | Browse/Tune tabs |
| `web/src/lib/components/Fretboard.svelte` | Create | SVG panoramic fretboard |
| `web/src/lib/components/PairDrawer.svelte` | Create | Collapsible pair list |
| `web/src/lib/components/Select.svelte` | Create | Styled dropdown |
| `web/static/fonts/` | Create | JetBrains Mono woff2 files |

---

### Task 1: Rust WASM API

**Files:**
- Modify: `Cargo.toml`
- Create: `src/wasm_api.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add WASM dependencies to Cargo.toml**

Add to `[dependencies]`:
```toml
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
```

Add to `[features]`:
```toml
wasm = ["dep:wasm-bindgen", "dep:serde-wasm-bindgen"]
```

Add to `[lib]`:
```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

- [ ] **Step 2: Create `src/wasm_api.rs`**

```rust
use wasm_bindgen::prelude::*;

use crate::theory::chords::{self, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;

#[wasm_bindgen]
pub fn get_roots() -> JsValue {
    let roots: Vec<&str> = chords::ROOTS.to_vec();
    serde_wasm_bindgen::to_value(&roots).unwrap()
}

#[wasm_bindgen]
pub fn get_all_scales() -> JsValue {
    let scales: Vec<_> = Scale::ALL
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "parent": s.parent.name(),
                "degree": s.degree,
                "semitones": s.semitones,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&scales).unwrap()
}

#[wasm_bindgen]
pub fn get_parent_scale_names() -> JsValue {
    use crate::theory::scales::ParentScale;
    let names: Vec<&str> = ParentScale::ALL.iter().map(|p| p.name()).collect();
    serde_wasm_bindgen::to_value(&names).unwrap()
}

#[wasm_bindgen]
pub fn get_pairs() -> JsValue {
    let pairs: Vec<_> = PAIRS
        .iter()
        .map(|p| {
            serde_json::json!({
                "label": p.label,
                "indicesA": p.indices.0,
                "indicesB": p.indices.1,
            })
        })
        .collect();
    serde_wasm_bindgen::to_value(&pairs).unwrap()
}

#[wasm_bindgen]
pub fn resolve_pair(root_pc: u8, scale_index: usize, pair_index: usize) -> JsValue {
    let scale = &Scale::ALL[scale_index];
    let pair = &PAIRS[pair_index];
    let (a, b) = gmc::resolve_pair(root_pc, scale, pair);
    let result = serde_json::json!({
        "triadA": a,
        "triadB": b,
    });
    serde_wasm_bindgen::to_value(&result).unwrap()
}

#[wasm_bindgen]
pub fn pair_display(root_pc: u8, scale_index: usize, pair_index: usize) -> String {
    let scale = &Scale::ALL[scale_index];
    let pair = &PAIRS[pair_index];
    gmc::pair_display(root_pc, scale, pair)
}

#[wasm_bindgen]
pub fn get_fretboard_notes() -> JsValue {
    let fb = Fretboard::standard_tuning();
    let mut notes: Vec<Vec<_>> = Vec::new();
    for s in 0..6 {
        let mut string_notes = Vec::new();
        for f in 0..=15 {
            if let Some(note) = fb.get_note(s, f) {
                string_notes.push(serde_json::json!({
                    "pc": note.pitch_class,
                    "name": PC_NAMES[note.pitch_class as usize],
                }));
            }
        }
        notes.push(string_notes);
    }
    serde_wasm_bindgen::to_value(&notes).unwrap()
}

#[wasm_bindgen]
pub fn get_interval_name(semitone: u8) -> String {
    let scale = &Scale::ALL[0]; // interval_name is scale-independent
    scale.interval_name(semitone).to_string()
}
```

- [ ] **Step 3: Register module in `src/lib.rs`**

Add (with cfg gate):
```rust
#[cfg(feature = "wasm")]
pub mod wasm_api;
```

Also add `serde_json` to dependencies in Cargo.toml:
```toml
serde_json = "1"  # already present
```

- [ ] **Step 4: Test WASM build**

Run: `wasm-pack build --target web --features wasm 2>&1 | tail -5`

If wasm-pack not installed: `cargo install wasm-pack`

Expected: `[INFO]: :-) Your wasm pkg is ready to publish`

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/wasm_api.rs src/lib.rs
git commit -m "feat: add WASM API via wasm-bindgen for web frontend"
```

---

### Task 2: SvelteKit project scaffold

**Files:**
- Create: `web/` directory with SvelteKit scaffold

- [ ] **Step 1: Create SvelteKit project**

```bash
cd /home/pedro/Projects/chordz
mkdir web && cd web
npm create svelte@latest . -- --template skeleton --types typescript --no-add-ons
npm install
```

Select: Skeleton project, TypeScript, no additional options.

- [ ] **Step 2: Install WASM integration deps**

```bash
cd /home/pedro/Projects/chordz/web
npm install -D vite-plugin-wasm vite-plugin-top-level-await
```

- [ ] **Step 3: Configure Vite for WASM**

Replace `web/vite.config.ts`:

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
	plugins: [wasm(), topLevelAwait(), sveltekit()]
});
```

- [ ] **Step 4: Create WASM wrapper**

Create `web/src/lib/wasm.ts`:

```typescript
import init, * as wasm from '../../pkg/chordz.js';

let initialized = false;

export async function initWasm() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export interface ScaleInfo {
  name: string;
  parent: string;
  degree: number;
  semitones: number[];
}

export interface PairInfo {
  label: string;
  indicesA: number[];
  indicesB: number[];
}

export interface ResolvedPair {
  triadA: number[];
  triadB: number[];
}

export interface FretNote {
  pc: number;
  name: string;
}

export function getRoots(): string[] {
  return wasm.get_roots();
}

export function getAllScales(): ScaleInfo[] {
  return wasm.get_all_scales();
}

export function getParentScaleNames(): string[] {
  return wasm.get_parent_scale_names();
}

export function getPairs(): PairInfo[] {
  return wasm.get_pairs();
}

export function resolvePair(rootPc: number, scaleIndex: number, pairIndex: number): ResolvedPair {
  return wasm.resolve_pair(rootPc, scaleIndex, pairIndex);
}

export function pairDisplay(rootPc: number, scaleIndex: number, pairIndex: number): string {
  return wasm.pair_display(rootPc, scaleIndex, pairIndex);
}

export function getFretboardNotes(): FretNote[][] {
  return wasm.get_fretboard_notes();
}

export function getIntervalName(semitone: number): string {
  return wasm.get_interval_name(semitone);
}
```

- [ ] **Step 5: Build WASM pkg into web-accessible location**

```bash
cd /home/pedro/Projects/chordz
wasm-pack build --target web --features wasm --out-dir web/pkg
```

Add to `web/.gitignore`:
```
pkg/
```

- [ ] **Step 6: Verify dev server starts**

```bash
cd /home/pedro/Projects/chordz/web
npm run dev -- --host 0.0.0.0 --port 5173
```

Expected: Vite dev server starts, page loads (may be blank).

- [ ] **Step 7: Commit**

```bash
cd /home/pedro/Projects/chordz
git add web/ -f
git commit -m "feat: scaffold SvelteKit project with WASM integration"
```

---

### Task 3: Theme and layout shell

**Files:**
- Create: `web/src/app.css`
- Create: `web/src/app.html`
- Create: `web/src/routes/+layout.svelte`
- Create: `web/src/routes/+page.svelte`
- Create: `web/src/lib/components/Rail.svelte`
- Create: `web/src/lib/components/SubTabs.svelte`
- Create: `web/static/fonts/` (JetBrains Mono)

- [ ] **Step 1: Download JetBrains Mono**

```bash
mkdir -p /home/pedro/Projects/chordz/web/static/fonts
cd /home/pedro/Projects/chordz/web/static/fonts
curl -Lo JetBrainsMono-Regular.woff2 "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/webfonts/JetBrainsMono-Regular.woff2"
curl -Lo JetBrainsMono-Bold.woff2 "https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/webfonts/JetBrainsMono-Bold.woff2"
```

- [ ] **Step 2: Create `web/src/app.css`**

```css
@font-face {
  font-family: 'JetBrains Mono';
  src: url('/fonts/JetBrainsMono-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'JetBrains Mono';
  src: url('/fonts/JetBrainsMono-Bold.woff2') format('woff2');
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}

:root {
  --bg-base: #1a1a1a;
  --bg-surface: #242424;
  --bg-raised: #2d2d2d;
  --border: #3d3d3d;
  --primary: #d4a574;
  --primary-hover: #e0b68a;
  --primary-muted: #5c4033;
  --secondary: #8ecae6;
  --secondary-muted: #2a4a5c;
  --text: #f5e6d3;
  --text-muted: #999;
  --text-disabled: #555;

  --font: 'JetBrains Mono', monospace;
  --font-heading: 16px;
  --font-body: 13px;
  --font-label: 11px;
  --font-dot: 8px;

  --rail-width: 56px;
  --subtab-height: 32px;
}

*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body {
  height: 100%;
  background: var(--bg-base);
  color: var(--text);
  font-family: var(--font);
  font-size: var(--font-body);
  line-height: 1.4;
  overflow: hidden;
}

select, button, input {
  font-family: var(--font);
  font-size: var(--font-body);
  color: var(--text);
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 4px 8px;
  outline: none;
}

select:focus, button:focus, input:focus {
  border-color: var(--primary);
}

button {
  cursor: pointer;
}

button:hover {
  background: var(--primary-muted);
}
```

- [ ] **Step 3: Create `web/src/app.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>chordz</title>
    %sveltekit.head%
  </head>
  <body data-sveltekit-prerender="true">
    %sveltekit.body%
  </body>
</html>
```

- [ ] **Step 4: Create `web/src/lib/components/Rail.svelte`**

```svelte
<script lang="ts">
  interface Props {
    active: 'chords' | 'gmc';
  }

  let { active }: Props = $props();
</script>

<nav class="rail">
  <a href="/chords/browse" class="rail-item" class:active={active === 'chords'}>
    <span class="rail-icon">♫</span>
    <span class="rail-label">Chords</span>
  </a>
  <a href="/gmc/browse" class="rail-item" class:active={active === 'gmc'}>
    <span class="rail-icon">◆</span>
    <span class="rail-label">GMC</span>
  </a>
</nav>

<style>
  .rail {
    width: var(--rail-width);
    height: 100%;
    background: var(--bg-raised);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 16px;
    gap: 8px;
  }

  .rail-item {
    width: 44px;
    padding: 8px 4px;
    border-radius: 8px;
    text-align: center;
    text-decoration: none;
    color: var(--text-disabled);
    transition: all 150ms ease;
  }

  .rail-item:hover {
    background: var(--bg-surface);
    color: var(--text-muted);
  }

  .rail-item.active {
    background: var(--primary-muted);
    color: var(--primary);
  }

  .rail-icon {
    display: block;
    font-size: 18px;
    line-height: 1;
  }

  .rail-label {
    display: block;
    font-size: 9px;
    margin-top: 4px;
  }
</style>
```

- [ ] **Step 5: Create `web/src/lib/components/SubTabs.svelte`**

```svelte
<script lang="ts">
  interface Tab {
    label: string;
    href: string;
  }

  interface Props {
    tabs: Tab[];
    active: string;
  }

  let { tabs, active }: Props = $props();
</script>

<div class="subtabs">
  {#each tabs as tab}
    <a href={tab.href} class="subtab" class:active={active === tab.label}>
      {tab.label}
    </a>
  {/each}
</div>

<style>
  .subtabs {
    height: var(--subtab-height);
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }

  .subtab {
    font-size: var(--font-body);
    color: var(--text-disabled);
    text-decoration: none;
    padding: 6px 0;
    border-bottom: 2px solid transparent;
    transition: all 150ms ease;
  }

  .subtab:hover {
    color: var(--text-muted);
  }

  .subtab.active {
    color: var(--primary);
    border-bottom-color: var(--primary);
  }
</style>
```

- [ ] **Step 6: Create `web/src/routes/+layout.svelte`**

```svelte
<script lang="ts">
  import '../app.css';
  import Rail from '$lib/components/Rail.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { initWasm } from '$lib/wasm';

  let { children } = $props();
  let ready = $state(false);

  onMount(async () => {
    await initWasm();
    ready = true;
  });

  let activeWorld = $derived(
    $page.url.pathname.startsWith('/gmc') ? 'gmc' as const : 'chords' as const
  );
</script>

<div class="app-shell">
  <Rail active={activeWorld} />
  <main class="content">
    {#if ready}
      {@render children()}
    {:else}
      <div class="loading">Loading...</div>
    {/if}
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 7: Create `web/src/routes/+page.svelte`**

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    goto('/gmc/browse', { replaceState: true });
  });
</script>
```

- [ ] **Step 8: Verify shell renders**

```bash
cd /home/pedro/Projects/chordz
wasm-pack build --target web --features wasm --out-dir web/pkg
cd web && npm run dev -- --host 0.0.0.0 --port 5173
```

Expected: Browser shows the Rail on the left with "Chords" and "GMC" icons, redirects to /gmc/browse.

- [ ] **Step 9: Commit**

```bash
cd /home/pedro/Projects/chordz
git add web/src/app.css web/src/app.html web/src/routes/ web/src/lib/components/Rail.svelte web/src/lib/components/SubTabs.svelte web/static/fonts/
git commit -m "feat: add theme, layout shell with Rail and SubTabs"
```

---

### Task 4: GMC Browse page with SVG fretboard

**Files:**
- Create: `web/src/routes/gmc/browse/+page.svelte`
- Create: `web/src/lib/components/Fretboard.svelte`
- Create: `web/src/lib/components/PairDrawer.svelte`
- Create: `web/src/lib/components/Select.svelte`
- Create: `web/src/lib/stores.ts`

- [ ] **Step 1: Create GMC store**

Create `web/src/lib/stores.ts`:

```typescript
import { writable, derived } from 'svelte/store';
import { getRoots, getAllScales, getPairs, resolvePair, pairDisplay, getFretboardNotes } from './wasm';
import type { ScaleInfo, PairInfo, ResolvedPair, FretNote } from './wasm';

export const rootIndex = writable(0);
export const scaleIndex = writable(1); // Dorian
export const pairIndex = writable(0);
export const showIntervals = writable(false);
export const drawerOpen = writable(true);

export const roots = writable<string[]>([]);
export const scales = writable<ScaleInfo[]>([]);
export const pairs = writable<PairInfo[]>([]);
export const fretboardNotes = writable<FretNote[][]>([]);

export function initStores() {
  roots.set(getRoots());
  scales.set(getAllScales());
  pairs.set(getPairs());
  fretboardNotes.set(getFretboardNotes());
}
```

- [ ] **Step 2: Create Select component**

Create `web/src/lib/components/Select.svelte`:

```svelte
<script lang="ts">
  interface Props {
    label: string;
    value: number;
    options: { label: string; value: number; group?: string }[];
    onchange: (value: number) => void;
  }

  let { label, value, options, onchange }: Props = $props();
</script>

<div class="select-group">
  <label class="select-label">{label}</label>
  <select {value} onchange={(e) => onchange(Number(e.currentTarget.value))}>
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
  </select>
</div>

<style>
  .select-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .select-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    white-space: nowrap;
  }

  select {
    min-width: 120px;
  }
</style>
```

- [ ] **Step 3: Create SVG Fretboard component**

Create `web/src/lib/components/Fretboard.svelte`:

```svelte
<script lang="ts">
  import type { FretNote } from '$lib/wasm';
  import { getIntervalName } from '$lib/wasm';

  interface Props {
    notes: FretNote[][];
    triadA: number[];
    triadB: number[];
    rootPc: number;
    showIntervals: boolean;
    numFrets?: number;
  }

  let { notes, triadA, triadB, rootPc, showIntervals, numFrets = 15 }: Props = $props();

  const numStrings = 6;
  const stringSpacing = 22;
  const fretSpacing = 50;
  const leftMargin = 30;
  const topMargin = 24;
  const dotRadius = 7;

  let width = $derived(leftMargin + fretSpacing * numFrets + 20);
  let height = $derived(topMargin + stringSpacing * (numStrings - 1) + 30);

  function dotColor(pc: number): string | null {
    if (triadA.includes(pc)) return 'var(--primary)';
    if (triadB.includes(pc)) return 'var(--secondary)';
    return null;
  }

  function dotLabel(pc: number): string {
    if (showIntervals) {
      const semitone = (pc - rootPc + 12) % 12;
      return getIntervalName(semitone);
    }
    return notes[0]?.[0]?.name ?? ''; // fallback; real lookup below
  }

  function fretX(fret: number): number {
    if (fret === 0) return leftMargin;
    return leftMargin + (fret - 1) * fretSpacing + fretSpacing / 2;
  }

  function stringY(s: number): number {
    return topMargin + s * stringSpacing;
  }
</script>

<svg class="fretboard" {width} {height} viewBox="0 0 {width} {height}">
  <!-- Fret lines -->
  {#each Array(numFrets + 1) as _, f}
    <line
      x1={leftMargin + f * fretSpacing}
      y1={topMargin}
      x2={leftMargin + f * fretSpacing}
      y2={topMargin + (numStrings - 1) * stringSpacing}
      stroke={f === 0 ? '#888' : '#3d3d3d'}
      stroke-width={f === 0 ? 2.5 : 1}
    />
  {/each}

  <!-- Strings -->
  {#each Array(numStrings) as _, s}
    <line
      x1={leftMargin}
      y1={stringY(s)}
      x2={leftMargin + numFrets * fretSpacing}
      y2={stringY(s)}
      stroke="#555"
      stroke-width="1"
    />
  {/each}

  <!-- Fret numbers -->
  {#each Array(numFrets) as _, f}
    <text
      x={leftMargin + f * fretSpacing + fretSpacing / 2}
      y={12}
      text-anchor="middle"
      fill="#444"
      font-size="10"
    >{f}</text>
  {/each}

  <!-- Dots -->
  {#each notes as stringNotes, s}
    {#each stringNotes as note, f}
      {#if f <= numFrets}
        {@const color = dotColor(note.pc)}
        {#if color}
          <circle
            cx={fretX(f)}
            cy={stringY(s)}
            r={dotRadius}
            fill={color}
          />
          <text
            x={fretX(f)}
            y={stringY(s) + 3}
            text-anchor="middle"
            fill="#1a1a1a"
            font-size="8"
            font-weight="bold"
          >{showIntervals ? getIntervalName((note.pc - rootPc + 12) % 12) : note.name}</text>
        {/if}
      {/if}
    {/each}
  {/each}
</svg>

<style>
  .fretboard {
    background: #222;
    border-radius: 6px;
    display: block;
    max-width: 100%;
    height: auto;
  }
</style>
```

- [ ] **Step 4: Create PairDrawer component**

Create `web/src/lib/components/PairDrawer.svelte`:

```svelte
<script lang="ts">
  import { pairDisplay } from '$lib/wasm';
  import type { PairInfo } from '$lib/wasm';

  interface Props {
    pairs: PairInfo[];
    selectedIndex: number;
    rootPc: number;
    scaleIndex: number;
    open: boolean;
    onselect: (index: number) => void;
    ontoggle: () => void;
  }

  let { pairs, selectedIndex, rootPc, scaleIndex, open, onselect, ontoggle }: Props = $props();
</script>

<aside class="drawer" class:open>
  <div class="drawer-header">
    <span class="drawer-title">Pairs</span>
    <button class="drawer-toggle" onclick={ontoggle}>
      {open ? '◀' : '▶'}
    </button>
  </div>
  {#if open}
    <ul class="pair-list">
      {#each pairs as pair, i}
        <li>
          <button
            class="pair-item"
            class:selected={i === selectedIndex}
            onclick={() => onselect(i)}
          >
            <span class="pair-label">{pair.label}</span>
            <span class="pair-display">{pairDisplay(rootPc, scaleIndex, i)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .drawer {
    width: 0;
    overflow: hidden;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    transition: width 200ms ease;
    display: flex;
    flex-direction: column;
  }

  .drawer.open {
    width: 260px;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .drawer-title {
    font-size: var(--font-label);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .drawer-toggle {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 6px;
  }

  .pair-list {
    list-style: none;
    overflow-y: auto;
    flex: 1;
  }

  .pair-item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 0;
    padding: 6px 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pair-item:hover {
    background: var(--bg-raised);
  }

  .pair-item.selected {
    background: var(--primary-muted);
  }

  .pair-label {
    font-size: var(--font-body);
    color: var(--text);
    font-weight: 700;
  }

  .pair-display {
    font-size: var(--font-label);
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 5: Create GMC Browse page**

Create `web/src/routes/gmc/browse/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import Select from '$lib/components/Select.svelte';
  import Fretboard from '$lib/components/Fretboard.svelte';
  import PairDrawer from '$lib/components/PairDrawer.svelte';
  import { rootIndex, scaleIndex, pairIndex, showIntervals, drawerOpen, roots, scales, pairs, fretboardNotes, initStores } from '$lib/stores';
  import { resolvePair } from '$lib/wasm';

  onMount(() => {
    initStores();
  });

  let resolved = $derived(
    $scales.length > 0 ? resolvePair($rootIndex, $scaleIndex, $pairIndex) : { triadA: [], triadB: [] }
  );

  let scaleOptions = $derived(
    $scales.map((s, i) => ({ label: `${s.name}`, value: i, group: s.parent }))
  );

  let rootOptions = $derived(
    $roots.map((r, i) => ({ label: r, value: i }))
  );

  const gmcTabs = [
    { label: 'Browse', href: '/gmc/browse' },
    { label: 'Tune', href: '/gmc/tune' },
  ];
</script>

<SubTabs tabs={gmcTabs} active="Browse" />

<div class="gmc-layout">
  <PairDrawer
    pairs={$pairs}
    selectedIndex={$pairIndex}
    rootPc={$rootIndex}
    scaleIndex={$scaleIndex}
    open={$drawerOpen}
    onselect={(i) => pairIndex.set(i)}
    ontoggle={() => drawerOpen.update(v => !v)}
  />

  <div class="gmc-main">
    <div class="gmc-controls">
      <Select label="Root" value={$rootIndex} options={rootOptions} onchange={(v) => rootIndex.set(v)} />
      <Select label="Scale" value={$scaleIndex} options={scaleOptions} onchange={(v) => scaleIndex.set(v)} />
      <label class="toggle">
        <input type="checkbox" bind:checked={$showIntervals} />
        Intervals
      </label>
      {#if !$drawerOpen}
        <button class="drawer-open-btn" onclick={() => drawerOpen.set(true)}>☰ Pairs</button>
      {/if}
    </div>

    <div class="gmc-heading">
      {$roots[$rootIndex]} {$scales[$scaleIndex]?.name ?? ''} — {$pairs[$pairIndex]?.label ?? ''}
    </div>

    <div class="fretboard-container">
      <Fretboard
        notes={$fretboardNotes}
        triadA={resolved.triadA}
        triadB={resolved.triadB}
        rootPc={$rootIndex}
        showIntervals={$showIntervals}
      />
    </div>
  </div>
</div>

<style>
  .gmc-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .gmc-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 12px 16px;
    gap: 12px;
    overflow: auto;
  }

  .gmc-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .gmc-heading {
    font-size: var(--font-heading);
    color: var(--text);
  }

  .fretboard-container {
    flex: 1;
    display: flex;
    align-items: flex-start;
    overflow-x: auto;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-label);
    color: var(--text-muted);
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--primary);
  }

  .drawer-open-btn {
    background: var(--bg-raised);
    font-size: var(--font-label);
  }
</style>
```

- [ ] **Step 6: Create gmc layout for sub-tabs routing**

Create `web/src/routes/gmc/+layout.svelte`:

```svelte
<script lang="ts">
  let { children } = $props();
</script>

{@render children()}
```

Create `web/src/routes/gmc/tune/+page.svelte`:

```svelte
<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';

  const gmcTabs = [
    { label: 'Browse', href: '/gmc/browse' },
    { label: 'Tune', href: '/gmc/tune' },
  ];
</script>

<SubTabs tabs={gmcTabs} active="Tune" />

<div style="padding: 24px; color: var(--text-muted);">
  Coming soon — Phase 2
</div>
```

- [ ] **Step 7: Build and test end-to-end**

```bash
cd /home/pedro/Projects/chordz
wasm-pack build --target web --features wasm --out-dir web/pkg
cd web && npm run dev -- --host 0.0.0.0 --port 5173
```

Expected: Browser shows Rail (Chords/GMC), GMC Browse with fretboard rendering colored dots, pair drawer, root/scale selectors working.

- [ ] **Step 8: Commit**

```bash
cd /home/pedro/Projects/chordz
git add web/src/
git commit -m "feat: implement GMC Browse page with SVG fretboard and pair drawer"
```

---

### Task 5: Chords placeholder routes

**Files:**
- Create: `web/src/routes/chords/+layout.svelte`
- Create: `web/src/routes/chords/browse/+page.svelte`
- Create: `web/src/routes/chords/tune/+page.svelte`

- [ ] **Step 1: Create placeholder pages**

Create `web/src/routes/chords/+layout.svelte`:

```svelte
<script lang="ts">
  let { children } = $props();
</script>

{@render children()}
```

Create `web/src/routes/chords/browse/+page.svelte`:

```svelte
<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];
</script>

<SubTabs tabs={chordTabs} active="Browse" />

<div style="padding: 24px; color: var(--text-muted);">
  Voicing Browser — Phase 2
</div>
```

Create `web/src/routes/chords/tune/+page.svelte`:

```svelte
<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];
</script>

<SubTabs tabs={chordTabs} active="Tune" />

<div style="padding: 24px; color: var(--text-muted);">
  Tune Mode — Phase 2
</div>
```

- [ ] **Step 2: Verify navigation works**

Open browser, click "Chords" in rail → should show "Voicing Browser — Phase 2". Click "Tune" sub-tab → shows "Tune Mode — Phase 2". Click "GMC" in rail → back to GMC Browse with fretboard.

- [ ] **Step 3: Commit**

```bash
cd /home/pedro/Projects/chordz
git add web/src/routes/chords/
git commit -m "feat: add Chords placeholder routes for Phase 2"
```

---

### Task 6: Build script and cleanup

**Files:**
- Create: `web/build.sh`
- Modify: `web/.gitignore`

- [ ] **Step 1: Create build script**

Create `web/build.sh`:

```bash
#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building WASM..."
wasm-pack build --target web --features wasm --out-dir web/pkg

echo "Building Svelte..."
cd web
npm run build

echo "Done! Output in web/build/"
```

```bash
chmod +x /home/pedro/Projects/chordz/web/build.sh
```

- [ ] **Step 2: Update .gitignore**

Ensure `web/.gitignore` contains:

```
node_modules/
pkg/
build/
.svelte-kit/
```

And project root `.gitignore`:

```
dist/
web/node_modules/
web/pkg/
web/build/
web/.svelte-kit/
```

- [ ] **Step 3: Final full test**

```bash
cd /home/pedro/Projects/chordz
cargo test  # Rust tests still pass
./web/build.sh  # Full build works
```

- [ ] **Step 4: Commit**

```bash
git add web/build.sh web/.gitignore .gitignore
git commit -m "chore: add build script and gitignore for web frontend"
```
