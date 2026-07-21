# Contributing

chordz is a work in progress. I built it to study harmony on the guitar, and
opened it because other guitarists asked for it. Suggestions, algorithm ideas
and fixes are all welcome.

## Where to start

The most open surface is the **line engine**: the pattern blocks and connectors
that generate a melodic étude over a chart (`src/theory/line_pattern.rs`,
`src/theory/line_engine.rs`). Proposing a new pattern shape or a new connector
does not require understanding the voicing solver, and it is where musical
ideas turn into code most directly.

Other areas:

- **Voicing recipes** (`src/voicings/`): the recipes that build drop, shell,
  quartal and upper-structure voicings, and the ranking that orders them by
  ergonomics, stability and voice-leading distance.
- **Chart parsing** (`src/theory/chart.rs`): the chord chart grammar, plus the
  built-in tune presets.
- **Web frontend** (`web/`): SvelteKit and the Web Audio sampler. The layout is
  desktop only today.

Issues labelled `good first issue` are concrete entry points.

## Ideas are contributions

If you play and have an idea about what the app should do musically, open an
issue and describe it. A clear description of the musical behaviour is worth
more than a patch that guesses at it.

## Building and testing

Build instructions live in the [README](README.md). Before opening a pull
request:

```sh
cargo test --lib            # Rust core
cd web && npm run check     # svelte-check
cd web && npm run test      # vitest
```

Design docs and specs are under `docs/`. Larger changes usually start with a
short design note there.

## Licence

Contributions are dual licensed under MIT and Apache-2.0, matching the project.
