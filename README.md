# chordz

A procedural harmony lab for jazz guitar. chordz generates voicings, voice-led
paths through chord charts, and single-note études from music theory and
guitar constraints — nothing comes from a static lookup table.

One Rust core, two frontends:

- **Web** (SvelteKit + WebAssembly): fretboard visualizations, triad-pair
  browser, étude generators, audio preview with real guitar samples.
- **Native desktop** (egui/eframe): keyboard-driven chord dictionary and
  chart voice-leading tool.

## What it does

- **Voicings** — a procedural voicing engine builds drop, shell, quartal, and
  upper-structure voicings from recipes plus playability rules, then ranks
  them by ergonomics, stability, and voice-leading distance (`src/voicings/`).
- **Tune mode** — parse a chord chart and solve a smooth voice-led path
  through the whole tune (`src/theory/chart.rs`, `src/voicings/solver.rs`).
- **GMC** — triad pairs from Tim Miller and Mick Goodrick's *Generic Modality
  Compression*: a panoramic fretboard browser for every pair across the neck,
  and a pattern-based étude generator with rhythm blocks and position
  selection (`src/theory/gmc.rs`, `line_engine.rs`, `line_pattern.rs`).
- **Audio** — on the web, a hand-rolled Web Audio sampler pitch-shifts real
  guitar samples through algorithmic amp/reverb effects; the native app uses
  synthesized playback via `kira`.

## Building

Prerequisites: a Rust toolchain with the `wasm32-unknown-unknown` target,
[`wasm-pack`](https://rustwasm.github.io/wasm-pack/), and Node.js.

```sh
# Native desktop app
cargo run

# Rust tests
cargo test --lib

# Web dev server (from the repo root)
wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
cd web && npm install && npm run dev

# Web production build (WASM + Svelte, output in web/build/)
./web/build.sh
```

Web checks, from `web/`: `npm run check` (svelte-check) and `npm run test`
(vitest).

## Documentation

Design docs live in `docs/` — start with `docs/ARCHITECTURE.md`. Feature
specs and implementation plans are under `docs/superpowers/`.

## Credits

- Guitar samples: *Shinyguitar* by
  [Karoryfer Samples](https://karoryfer.com) — CC0.
- Font: [JetBrains Mono](https://www.jetbrains.com/lp/mono/) — SIL OFL 1.1.
- The GMC material is based on concepts from Tim Miller and Mick Goodrick,
  *Creative Chordal Harmony for Guitar: Using Generic Modality Compression*
  (Berklee Press).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
