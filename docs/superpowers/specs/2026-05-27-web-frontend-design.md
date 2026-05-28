# Web Frontend — Svelte + Rust WASM

## Status: Implemented (all phases delivered)

## Summary

Svelte web frontend consuming a Rust WASM core library. The Rust code (theory,
voicings, solver, synth) compiles to WASM via wasm-pack and exposes a typed API.
The Svelte app handles all rendering, layout, and interaction. The egui native app
is retained for dev/testing but receives no new visual features.

## Architecture

```
web/                          ← SvelteKit project
├── src/
│   ├── lib/
│   │   ├── wasm.ts           ← WASM init + typed wrapper
│   │   ├── audio.ts          ← Web Audio API playback (strum, arpeggio, bass, click)
│   │   ├── stores.ts         ← Svelte stores for GMC state
│   │   └── components/
│   │       ├── Rail.svelte         ← Vertical navigation rail
│   │       ├── SubTabs.svelte      ← Browse/Tune tabs (with actions slot)
│   │       ├── Fretboard.svelte    ← SVG panoramic fretboard (GMC)
│   │       ├── VoicingFretboard.svelte ← SVG zoomed fretboard (Chords)
│   │       ├── ChartGrid.svelte    ← Lead-sheet style chord grid
│   │       ├── PairDrawer.svelte   ← Collapsible pair list (GMC)
│   │       └── Select.svelte       ← Styled dropdown (supports optgroup)
│   ├── routes/
│   │   ├── +layout.svelte    ← Shell (WASM init + rail + content)
│   │   ├── +page.svelte      ← Redirect to /gmc/browse
│   │   ├── chords/
│   │   │   ├── browse/+page.svelte  ← Voicing browser
│   │   │   └── tune/+page.svelte    ← Tune mode (solver + chart grid)
│   │   └── gmc/
│   │       ├── browse/+page.svelte  ← GMC fretboard visualization
│   │       └── tune/+page.svelte    ← Placeholder (Phase 2)
│   ├── app.css               ← Global styles + Warm Amber theme tokens
│   └── app.html              ← HTML shell
├── static/fonts/             ← JetBrains Mono woff2
├── vite.config.ts            ← Vite + vite-plugin-wasm + SvelteKit
└── build.sh                  ← Full build script (wasm-pack + npm build)

src/                           ← Rust lib
├── theory/                    ← Scales, GMC, chords, intervals, notes, chart
├── voicings/                  ← Fretboard, generate, ranking, recipes, solver
├── audio/
│   ├── synth.rs              ← Sample generation (pluck, chord, stereo)
│   └── engine.rs             ← Native-only kira playback (cfg-gated)
└── wasm_api.rs               ← wasm-bindgen exports (cfg feature = "wasm")
```

## Visual Design

### Color Palette (Warm Amber)

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-base` | `#1a1a1a` | Page background |
| `--bg-surface` | `#242424` | Panels, cards |
| `--bg-raised` | `#2d2d2d` | Rail, elevated surfaces |
| `--border` | `#3d3d3d` | Dividers, outlines |
| `--primary` | `#d4a574` | Amber accent, active states |
| `--primary-hover` | `#e0b68a` | Hover on amber elements |
| `--primary-muted` | `#5c4033` | Selected backgrounds |
| `--secondary` | `#8ecae6` | Cool blue, triad B |
| `--secondary-muted` | `#2a4a5c` | Blue backgrounds |
| `--text` | `#f5e6d3` | Primary text (warm white) |
| `--text-muted` | `#999` | Secondary text |
| `--text-disabled` | `#555` | Disabled |

### Typography

- Font: JetBrains Mono (all text, loaded from static/fonts/)
- Sizes: heading 16px, body 13px, label 11px, dot-label 8px
- Line height: 1.4

### Navigation

Two-level navigation: **Chords | GMC** (world) → **Browse | Tune** (mode).

- **Vertical Rail** (56px, left): icon + label for Chords/GMC
  - Active: bg `--primary-muted`, text `--primary`
  - Inactive: transparent, text `--text-disabled`
- **Sub-tabs** (32px bar, top of content): Browse | Tune
  - Active: text `--primary`, 2px underline
  - Inactive: text `--text-disabled`
  - Supports an `actions` snippet slot for right-aligned controls (e.g. drawer toggle)

### Fretboard rendering

Two SVG fretboard components:

**Panoramic** (`Fretboard.svelte`, GMC):
- 15 frets, 6 strings, background `#222`
- Dots: 14px diameter, amber/blue per triad, label inside

**Zoomed** (`VoicingFretboard.svelte`, Chords):
- Auto-ranged frets around the voicing (~4-5 visible)
- Root notes: amber, non-root: blue
- Interval labels inside dots, note names below
- Muted strings shown as ×, open strings as hollow circle

## WASM API (`src/wasm_api.rs`)

All functions are `#[wasm_bindgen]` exports, data serialized via `serde_json` +
`js_sys::JSON::parse` (not `serde_wasm_bindgen::to_value`, which fails on nested
`serde_json::Value`).

### Scales & GMC
- `get_roots() → string[]`
- `get_all_scales() → {name, parent, degree, semitones}[]`
- `get_parent_scale_names() → string[]`
- `get_pairs() → {label, indicesA, indicesB}[]`
- `resolve_pair(root_pc, scale_index, pair_index) → {triadA, triadB}`
- `pair_display(root_pc, scale_index, pair_index) → string`
- `get_fretboard_notes() → FretNote[6][16]`
- `get_interval_name(semitone) → string`

### Chords
- `get_families() → {index, name}[]`
- `generate_voicings(root_index, family_index, note_count, prefer_crunch) → VoicingInfo[]`

### Solver
- `get_presets() → {title, chart}[]` (Stella, Just Friends, Moment's Notice, Giant Steps)
- `solve_chart(chart_text, title, config) → {changes, error?}`
- `solve_chart_with_locks(chart_text, title, config, locks) → {changes, error?}`

Config fields: `minStrings`, `maxStrings`, `maxFretSpan`, `maxFret`, `minFret`,
`tensionTarget`, `smoothnessWeight`, `jitter`, `allowOpenStrings`,
`expandBasicChords`, `recipes`, `allowedStrings`.

Solved changes include up to 10 `alternatives` per chord for manual swapping,
plus `rootPc`, `relaxation` label, and `tension` score.

### Audio
- `synth_chord(positions, duration) → f32[] interleaved stereo`
- `synth_arpeggio(positions, note_duration) → f32[] interleaved stereo`
- `synth_bass_note(root_pc, duration) → f32[] interleaved stereo`

## Features by page

### GMC Browse (`/gmc/browse`)
- Root selector, scale selector (grouped by parent family via optgroup)
- 10 triad pairs in collapsible drawer (toggle in sub-tab bar)
- SVG panoramic fretboard with two-color dots
- Toggle between note names and interval labels

### Chords Browse (`/chords/browse`)
- Root, family, note count selectors
- Crunch preference toggle (ranks voicings with m2/M2 intervals higher)
- Voicings grouped by chord + recipe + intervals
- j/k navigate groups, h/l cycle positions within group
- Space = strum, A = arpeggio
- Zoomed SVG fretboard with interval labels

### Chords Tune (`/chords/tune`)
- Chart text input + presets dropdown (4 standards)
- Solver constraints panel (collapsible):
  - Style: Grounded | Balanced | Open | Abstract
  - Movement: Free | Normal | Smooth | Tight
  - Variation slider (jitter)
  - Note filter (3-4 / 3 / 4 / 5 / 3-5)
  - Fret range, max span, open strings, expand basic chords
  - Recipe filter, string filter
- Chart grid (lead-sheet layout, 4 bars per line, clickable)
- Voicing detail panel (right side):
  - Fretboard diagram, intervals, recipe tag
  - Swap alternatives (◀▶ or h/l), counter
  - Lock checkbox per chord, clear locks button
  - Strum button (Space)
- Playback: Play All with adjustable BPM, optional click + bass track
- Keyboard: ←→ navigate, h/l swap, Space strum, Enter solve

## Tech Stack

- Svelte 5 (runes) + SvelteKit
- Vite 8 + vite-plugin-wasm
- TypeScript
- wasm-bindgen + js-sys + console_error_panic_hook
- Web Audio API for playback
- SVG for fretboard rendering
