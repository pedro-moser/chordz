# Web Frontend — Svelte + Rust WASM

## Summary

Replace the egui UI with a Svelte web frontend consuming a Rust WASM core library.
The Rust code (theory, voicings, solver) compiles to WASM and exposes a typed API
via wasm-bindgen. The Svelte app handles all rendering, layout, and interaction.

## Architecture

```
web/                          ← Svelte project (new)
├── src/
│   ├── lib/wasm.ts           ← WASM bindings (auto-generated types)
│   ├── components/
│   │   ├── Rail.svelte       ← Vertical navigation rail
│   │   ├── SubTabs.svelte    ← Browse/Tune tabs
│   │   ├── Fretboard.svelte  ← SVG panoramic fretboard
│   │   ├── PairDrawer.svelte ← Collapsible pair list
│   │   └── Selectors.svelte  ← Root/Scale/Family combos
│   ├── routes/
│   │   ├── +layout.svelte    ← Shell (rail + content)
│   │   ├── chords/
│   │   │   ├── browse/       ← Voicing browser
│   │   │   └── tune/         ← Tune mode
│   │   └── gmc/
│   │       ├── browse/        ← GMC fretboard
│   │       └── tune/          ← (Phase 2)
│   └── styles/
│       ├── theme.css          ← Warm Amber palette + tokens
│       └── fretboard.css      ← Fretboard-specific styles
├── static/
│   └── fonts/                 ← JetBrains Mono
└── vite.config.ts             ← WASM plugin config

src/                           ← Existing Rust lib (unchanged logic)
├── theory/
├── voicings/
└── wasm_api.rs               ← NEW: wasm-bindgen exports
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

- Font: JetBrains Mono (all text)
- Sizes: heading 16px, body 13px, label 11px, dot-label 8px
- Line height: 1.4

### Navigation

- **Vertical Rail** (56px, left): icon + label for Chords/GMC
  - Active: bg `--primary-muted`, text `--primary`
  - Inactive: transparent, text `--text-disabled`
- **Sub-tabs** (32px bar, top of content): Browse | Tune
  - Active: text `--primary`, 2px underline
  - Inactive: text `--text-disabled`

### Fretboard (SVG, minimal dots)

- Background: `#222`
- Strings: `#555`, 1px horizontal lines
- Frets: `#3d3d3d`, 1px vertical lines
- Nut: `#888`, 2.5px
- Fret numbers: `#444`, 10px, above fretboard
- Dots: 14px diameter circles
  - Triad A / Root: `--primary` (amber)
  - Triad B / Non-root: `--secondary` (blue)
  - Label: 8px bold `#1a1a1a` centered inside dot
- 15 frets visible for GMC panoramic view
- ~5 frets visible for voicing browser (zoomed)

### Drawer (GMC pairs)

- Width: 240px when open, 0 when closed
- Toggle button in sub-tab bar (hamburger or panel icon)
- Background: `--bg-surface`
- Border-right: `--border`
- Pair items: label + intervals + notes
- Selected pair: bg `--primary-muted`, text `--text`
- Transition: slide 200ms ease

### Components

- **ComboBox**: bg `--bg-raised`, border `--border`, dropdown bg `--bg-surface`
- **Buttons**: primary bg `--primary-muted` text `--text`, ghost no-bg text `--primary`
- **Lists**: selected bg `--primary-muted`, hover bg `--bg-raised`
- **Checkbox on**: bg `--primary`, check `#1a1a1a`

## WASM API (wasm-bindgen exports)

Functions the Svelte frontend needs:

```rust
// Scales
pub fn get_all_scales() -> JsValue;  // Vec<{name, parent, semitones}>
pub fn get_parent_scale_names() -> JsValue;

// GMC
pub fn get_pairs() -> JsValue;  // Vec<{label, indices}>
pub fn resolve_pair(root_pc: u8, scale_index: usize, pair_index: usize) -> JsValue;
pub fn pair_display(root_pc: u8, scale_index: usize, pair_index: usize) -> String;

// Chords
pub fn get_roots() -> JsValue;
pub fn get_families() -> JsValue;
pub fn get_quality_names(family_index: usize) -> JsValue;
pub fn generate_voicings(root_pc: u8, family_index: usize, note_count: usize) -> JsValue;

// Fretboard
pub fn get_note_at(string: usize, fret: usize) -> JsValue;  // {pc, octave, name}

// Solver (tune mode)
pub fn solve_chart(chart_text: &str, config: JsValue) -> JsValue;
```

Data returned as JSON-serializable structs via serde + wasm-bindgen.

## Scope — Phase 1 (this spec)

1. Rust WASM API (`src/wasm_api.rs`)
2. Svelte project scaffold (`web/`)
3. Theme + layout shell (rail, sub-tabs)
4. GMC Browse tab (fretboard SVG + selectors + drawer)

## Out of scope

- Chords Browser port (Phase 2)
- Chords Tune port (Phase 2)
- GMC Tune (Phase 3)
- Audio playback in browser
- Mobile/responsive (later)
- PWA/offline (later)

## Tech Stack

- Svelte 5 (runes) + SvelteKit (file routing)
- Vite + vite-plugin-wasm
- wasm-bindgen + serde-wasm-bindgen
- TypeScript
- SVG for fretboard rendering
