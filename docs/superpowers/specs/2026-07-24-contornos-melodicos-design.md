# Melodic Contour (CSEG) as a Line-Engine Axis — Design

**Date:** 2026-07-24
**Status:** Approved (design), pending implementation plan

## Summary

Give the GMC line engine an explicit **melodic contour** axis: the ordinal shape of a cell
(low-high-mid, high-low-mid, …), controlled independently of *which* notes the cell plays. For a
3-note cell there are exactly six contours — `<1 2 3>`, `<1 3 2>`, `<2 1 3>`, `<2 3 1>`, `<3 1 2>`,
`<3 2 1>`.

Today the engine has two of the six, spelled as `Direction::Ascending` / `Direction::Descending`, and
for `Shape::Order` blocks the contour is **not controlled at all** — it falls out of whichever rung
the connector happened to land on. This adds the missing four and makes the axis deterministic.

## Provenance

Suggested by Julio Herrlein (author of *Harmonia Combinatória*, UFRGS), who prototyped it as a
`contour-mel` function in Opusmodus/Common Lisp. The core of his implementation:

```lisp
(oct 0)                                   ; começa na oitava base (C4 = 0)
(candidate (+ (* oct 12) pc))
;; Garante que a nota de rank N seja sempre maior que a de rank N-1
(while (<= candidate last-val)
  do (setf oct (1+ oct))
     (setf candidate (+ (* oct 12) pc)))
```

Sort the cell's positions by requested rank, then walk in rank order pushing each pitch class up by
octaves until it clears the previous rank. The pitch classes stay fixed; the **register** is what
moves. That octave displacement is what turns a repetitive arpeggio into a line with leaps.

Two adaptations are required for guitar, both detailed below: register is not free (the line is
confined to a `PositionSet` region), and Julio's version always resolves upward from a fixed floor,
so every cell is anchored to the same register.

## Background — what exists today

### The two axes that are already there

`PatternBlock` (`src/theory/line_pattern.rs:141`) carries `direction: Direction` and `shape: Shape`:

```rust
pub enum Shape {
    Monotonic,        // walk the grip by pitch, in `direction`
    Order(Vec<u8>),   // play triad voice roles in an explicit cyclic sequence
}
```

`note_at` (`src/theory/line_engine.rs:135`) realizes them one note at a time:

- `Monotonic` sorts the grip's notes ascending, reverses for `Descending`, and indexes via
  `pingpong(k, len)` when `count > 3`.
- `Order(o)` looks up `pcs[o[k % o.len()] % 3]` and finds that pitch class inside the grip.

**`Direction` is already contour theory with four of six cases missing.** `Monotonic` ascending on a
3-note grip emits `notes[0], notes[1], notes[2]` — that *is* the contour `<1 2 3>`. Descending is
`<3 2 1>`.

### The leak this fixes

`Shape::Order` fixes *identity* (which scale degrees) and lets the shape fall where it falls. Worked
example with `Order([0, 2, 1])` — "play root, fifth, third":

| Grip the connector landed on | Pitch-ascending contents | Emitted order | Resulting contour |
|---|---|---|---|
| root position | R, 3, 5 | R, 5, 3 | `<1 3 2>` |
| first inversion | 3, 5, R | R, 5, 3 | `<3 2 1>` |

Same exercise, different melodic shape, depending on a rung the player did not choose. A student
practicing "1-5-3" gets a different figure on every rung. Contour closes this.

### The invariant this relaxes

`run_pattern` (`src/theory/line_engine.rs:271-273`) documents:

> Each block plays one grip (so the `1+1+1`/`2+1`/`1+2`, no-open-string guarantee holds)

A block lives inside a single grip, which is what guarantees the string distribution that makes the
cell playable. **Contour with register displacement breaks this by construction**, because realizing
`<3 1 2>` needs notes from more than one rung. Resolution: the invariant becomes a *cost preference*
rather than a law (see "Guitaristic cost").

## Architecture

### Data model

`PatternBlock` gains one optional field. The identity axis (`shape`) and the contour axis are
independent:

```rust
// src/theory/line_pattern.rs — PatternBlock gains:
/// Ordinal register shape of each cell, 1-based (1 = lowest). `None` = today's behaviour.
/// Length is the cell size; cycles over the block like `Shape::Order` does.
pub contour: Option<Vec<u8>>,
```

`contour: None` reproduces current behaviour exactly, in every code path. This is the migration
story: nothing changes until the field is set.

### Semantics of the two axes together

The identity axis supplies the sequence of pitch classes; the contour axis supplies their register.

| `shape` | `contour` | Identity sequence for the cell |
|---|---|---|
| `Monotonic` | `None` | today's directional walk + `pingpong` |
| `Monotonic` | `Some(c)` | **the current grip's 3 pitch classes in the block's `direction` order** — ascending for `Ascending`, reversed for `Descending` |
| `Order(o)` | `None` | today's role cycle, contour uncontrolled |
| `Order(o)` | `Some(c)` | roles per `o`, register per `c` |

The `Monotonic + Some(c)` row is deliberate and load-bearing. Defining it as "the grip's notes in
pitch order" (rather than as `Order([0,1,2])`) is what makes `<1 2 3>` reproduce today's ascending
walk **byte for byte**, including on inverted grips — that equivalence is the non-regression test.
Defining it via roles would force root-position spelling and silently change existing output.

`direction` must be honoured, not ignored. An earlier draft of this spec said the opposite and was
wrong: `Direction::Descending` is not "the contour `<3 2 1>` over an ascending identity", it is the
same walk with the **identity reversed**. Legacy descending plays `notes[2], notes[1], notes[0]`. If
the identity stayed ascending, `<3 2 1>` would instead ask the grip's *lowest* pitch class to sound
as the cell's *highest* note — a different operation, requiring displacement, and not a
reproduction of anything. With the identity reversed, the grip's own notes realize `<3 2 1>`
directly and the non-regression property holds on both monotonic contours.

### Cell resolution

A *contour* of length `n` is a permutation vector where `c[i] ∈ 1..=n` is the ordinal rank of
position `i`. `<3 1 2>` means: first note highest, second lowest, third middle.

The block is divided into consecutive cells of `n` notes starting at `k = 0`. Each cell resolves as
a unit:

1. **Identity.** Per the table above, produce `pc[0..n]` — the pitch class wanted at each position.
2. **Candidates.** For each position, the region's occurrences of its pitch class:
   `positions.find_notes(fretboard, &[pc])`, sorted by midi. Typically 2–3 per class inside a
   position window; at most ~4.
   **No octave arithmetic is needed** — the fretboard already *is* the quantized register axis, and
   the region bound comes free.
3. **Enumerate realizations.** Let `by_rank[r]` be the position whose rank is `r + 1`. A realization
   assigns one candidate per position such that midi is strictly increasing in rank order:
   `midi(by_rank[0]) < midi(by_rank[1]) < … < midi(by_rank[n-1])`.
   Worst case for `n = 3` is 4³ = 64 assignments before the monotonicity filter, which kills most.
   Cap the enumeration at 512 assignments (a guard, not a tuning knob) and keep the best found;
   candidates are visited in sorted order, so the result is deterministic.
4. **Score and choose.** Take the argmin of the cost below.
5. **Emit** in positional order.

### Guitaristic cost

Hard filter: every note comes from the region. Among the survivors, minimize:

```
cost = SAME_STRING * (adjacent pairs in PLAYING order sharing a string)
     + (max_fret - min_fret)                      // keep the hand compact
     + OFF_GRIP * (notes not drawn from the cursor grip)
     + |midi(position 0) - prev_midi|             // continuity with the previous block, 0 if none
```

`OFF_GRIP = 6` — a per-note toll for leaving the hand shape the connector chose.

**Why the grip-affinity term is required, not decorative.** Inside one grip each pitch class occurs
at exactly one midi, hence at exactly one rank: the contour is *fully determined* by the identity
axis, with no freedom left. Two consequences follow.

- A **monotonic** contour is always realizable in-grip (it is what the grip already produces), so it
  pays `OFF_GRIP = 0` and wins outright. This is what makes `<1 2 3>` reproduce today's ascending
  walk exactly. Without the term, a realization elsewhere in the region could win on the
  `|midi - prev_midi|` term alone and silently change existing output — the non-regression guarantee
  would be false.
- A **scrambled** contour is *not* realizable in-grip (the ranking cannot match), so it must draw
  from elsewhere, and the toll makes it depart as little as possible.

`SAME_STRING = 24`, chosen to exceed the fret span of any realistic position window, so inside a
region a string reuse can never be bought back with compactness. With `positions` empty (no region
restriction, per `LineConfig`) a cell could in principle span more of the neck than that; the
ordering still behaves, since avoiding a reuse costs at most the extra span while the reuse costs a
flat 24. Ties break on lowest total midi.

Adjacency is measured in **playing** order, not rank order — that is what the picking hand actually
does.

**The grip short-circuit — a rule, not a weighting.** Before scoring anything, check whether the
cursor grip's own three notes already realize the requested identity and contour. If they do, return
them and run no search at all.

An earlier draft tried to obtain this from the cost function alone, reasoning that a single grip is
"by construction three notes on three distinct strings" and therefore always cheapest. **That
premise is false.** Real grips reuse strings: `[(str4,fret5,midi64), (str5,fret5,midi69),
(str5,fret8,midi72)]` is a grip the engine actually produces, and it pays `SAME_STRING` itself. In
that case a displaced alternative scored 30 against the grip's 32 and won — correctly, by the stated
cost function, even for `<1 2 3>`. A guarantee that depends on weights ordering the way you hoped is
not a guarantee.

With the short-circuit, the non-regression property is structural: `<1 2 3>` and `<3 2 1>` are
exactly what the grip already plays, so they always take this path and never reach the search. The
cost function then governs only the cases the grip genuinely cannot express — which is where
displacement is the point.

**The anchor — an improvement on the Lisp.** Julio's version starts every cell at `(oct 0)`, the
base octave, so cells are pinned to a register floor and register never becomes expressive. Here the
`|midi(position 0) - prev_midi|` term makes the resolver choose the placement that voice-leads from
the previous block. Contour therefore *composes* with the existing `Connector` and voice-leading
machinery instead of fighting it.

### When the region admits only one realization

A narrow region plus a scrambled contour can collapse the feasible set to exactly one assignment.
The cost function then decides nothing — grip affinity, compactness and repeat-avoidance all become
inert, because there is nothing to choose between. If that single assignment happens to open on the
pitch that just sounded, the line repeats a note across the boundary and no rung choice can prevent
it: the constraint is feasibility, not search.

The engine's answer is to **keep the contour and stay in the region**, accepting the repeat. Both
alternatives are worse: breaking the contour silently would make the exercise a lie about its own
shape, and leaving the region would break the position discipline the whole line generator is built
on. A repeat is audible and self-explanatory to the player; the other two failures are invisible.

This is why the resolver still carries a repeat penalty (`REPEAT = 48`) even though it cannot help
here — it avoids repeats in every case where an alternative exists, which is the common one. The
forced case is covered by its own test, `a_single_position_region_can_force_a_repeated_pitch`, so
the trade-off is recorded rather than rediscovered as a bug.

### Degradation

If no realization satisfies strict monotonicity within the region, the contour is unrealizable
there. Fall back to the assignment achieving the **longest correct rank prefix**, tie-broken by the
normal cost. Deterministic, and it degrades gradually rather than snapping to an unrelated shape.
Log nothing; surface it in the UI as described below.

### Partial cells

When `count` is not a multiple of `n`, resolve the full cell and emit only its first `count % n`
notes. Predictable, and it avoids inventing a re-ranking rule for truncated contours.

### Mid-block chord changes

A cell may straddle a chord change (`line_engine.rs:372`). A chord change **truncates the current
cell**: emit what has been resolved, then begin a fresh cell on the new chord's ladder, anchored by
the glue pitch. Resolving a cell across two harmonies would make its register arrangement meaningless.

### The rung, for contour blocks

The rung keeps two jobs and loses one.

- **Keeps:** for `Monotonic + Some(c)`, the identity sequence *is* the current grip's three pitch
  classes, so the rung still decides **which pitch classes** the cell plays. (`Order(o)` takes its
  classes from `ladder.pcs` and does not need the rung for this.)
- **Keeps:** it supplies the placement anchor via `grip_center` when `prev_midi` is `None`.
- **Loses:** it no longer constrains **which occurrence** of a pitch class — that is, the register.
  Contour blocks draw occurrences from the whole region, and placement is decided by the cost
  function.

The cursor and `Connector` state still advance normally so that patterns mixing contour and
non-contour blocks stay coherent, and so `Anchor` keeps working.

## Refactor surface

This is the bulk of the work, and it is a **change of granularity**, not a new arm in a `match`.
`note_at` returns the k-th note independently; contour makes note `k` depend on notes `0..k` of the
same cell.

```rust
// replaces the per-note note_at for contour blocks
fn resolve_cell(
    ladder: &TriadLadder,
    rung: usize,
    positions: &PositionSet,
    fretboard: &Fretboard,
    block: &PatternBlock,
    cell_index: usize,
    prev_midi: Option<i32>,
) -> Vec<FretNote>;
```

Three call sites consume `note_at` today, and all three need the cell resolver when a contour is set:

1. **Emission loop** (`line_engine.rs:387`) — resolve on cell boundaries, memoize, serve by `k`.
2. **`glue_rung`** (`line_engine.rs:159-171`) — scores each rung by the note it *would* produce at
   index `k`. Becomes "the first note of the cell this rung would produce".
3. **No-repeat probe** (`line_engine.rs:346-356`) — same substitution.

`note_at` stays unchanged for `contour: None`, so both paths coexist and the legacy path keeps its
exact behaviour.

Cost check: resolving a cell is ~64 filtered assignments; `glue_rung` does it per rung across ~3–4
rungs. Negligible against the existing per-chord ladder construction.

## Downstream

- **`wasm_api.rs`, `parse_pattern_blocks`** (~`:586`): read optional `contour` array into the block,
  absent → `None`.
- **`web/src/lib/wasm.ts`**: `GmcPatternBlock` gains `contour?: number[]`.
- **`web/src/routes/gmc/tune/+page.svelte`**: the per-block control described below.
- **Audio, tab render, notation**: no change. Contour alters which pitches are chosen, not the
  event shape.

> Note: the standard-notation view (`docs/superpowers/plans/2026-07-24-gmc-partitura.md`) is being
> built concurrently. It consumes `NoteEvent`, which this design does not modify, so the two do not
> interact.

## UI

For `count == 3`, six buttons carrying the contour drawn as a polyline — the notation Julio used,
and far more readable than the numbers. The glyph is an SVG polyline over **three** vertical levels;
two levels are not enough, since at two levels `<1 3 2>` and `<2 3 1>` collapse onto the same shape,
as do `<2 1 3>` and `<3 1 2>`. The point heights are exactly the contour vector:

| Contour | 1st note | 2nd note | 3rd note |
|---|---|---|---|
| `<1 2 3>` | low | mid | high |
| `<1 3 2>` | low | high | mid |
| `<2 1 3>` | mid | low | high |
| `<2 3 1>` | mid | high | low |
| `<3 1 2>` | high | low | mid |
| `<3 2 1>` | high | mid | low |

Plus an "off" state (`contour: None`) which restores the `↑`/`↓` toggle. Cells of other sizes take a
free-text CSEG vector; the six glyphs are the 3-note affordance, not the whole model.

When a block's contour degrades (no realization fits the region), mark the block — the honest signal
is "this shape does not fit this position", and the fix is a wider region or a different contour.

## Testing

1. **Non-regression:** `Monotonic + <1 2 3>` produces output identical to `Monotonic + Ascending`
   for every preset, including on inverted grips; likewise `<3 2 1>` and `Descending`. Assert on
   full event vectors, not spot checks.
2. **`contour: None` is inert:** every existing line-engine test passes unchanged.
3. **Ordinal correctness:** for each of the six contours, the emitted cell's midi ranking equals the
   requested contour.
4. **The leak, closed:** `Order([0,2,1]) + <2 3 1>` yields the same ordinal shape starting from a
   root-position rung and from a first-inversion rung. This is the regression test for the table in
   "The leak this fixes".
5. **Guitaristic cost:** given a contour satisfiable within one grip, the resolver picks the
   single-grip realization — the old invariant, asserted as behaviour.
6. **Degradation:** a region too narrow to realize a contour yields the longest-correct-prefix
   fallback, deterministically, and never panics or emits out-of-region notes.
7. **Cycling and partial cells:** `count = 6` with a 3-contour gives two identical cells;
   `count = 4` gives one full cell plus the first note of the next.
8. **Chord-change truncation:** a cell straddling a change emits the resolved prefix and restarts on
   the new ladder.

## Non-goals

- **A `spread`/amplitude knob** (how many rungs a contour may borrow). Considered and dropped —
  YAGNI. The minimal-displacement rule already yields sane defaults, and the cost function bounds
  the spread implicitly. Natural later extension.
- **Contour operations from the literature** (inversion, retrograde, contour-class reduction,
  COM-matrix similarity). The six 3-note contours are the deliverable; the `Vec<u8>` model does not
  preclude adding operators later.
- **Contour as a sampled dimension in the probabilistic étude generator.** Contour is a clean new
  orthogonal axis and is obviously valuable there, but that is a separate piece of work.
- **Migrating `Direction` away.** `Direction` stays; it is subsumed but not removed, so the legacy
  path and its tests remain untouched.

## References

- Julio Herrlein's `contour-mel` (Opusmodus / Common Lisp), the origin of this design.
- Marcos Sampaio, contour theory (Brazilian musicology), suggested by Julio as further reading.
- Robert Morris, *Composition with Pitch-Classes* (1987) — the canonical origin of CSEG and
  contour space.
