<script lang="ts" module>
  /** Vertical space the staff block occupies, including ledger-line and stem headroom. */
  export const STAFF_BLOCK_HEIGHT = 104;
</script>

<script lang="ts">
  import { layoutLine, STAFF_LINE_GAP, type Grid, type MeasureLayout } from '$lib/notation';
  import { measureX, TAB_MEASURE_WIDTH, TAB_MARGIN_LEFT, type MeasureLike } from '$lib/tabLayout';
  import {
    SMUFL_FONT,
    CLEF_8VB,
    REST_GLYPH,
    REST_ANCHOR_STEP,
    ACCIDENTAL_GLYPH,
    ACCIDENTAL_WIDTH_SP,
  } from '$lib/notationGlyphs';
  import type { GmcLineEvent } from '$lib/wasm';

  interface Props {
    measures: Array<MeasureLike & { events: GmcLineEvent[] }>;
    grid: Grid;
    /** Y of the staff's top line within the parent SVG. */
    top: number;
    t1Color: string;
    t2Color: string;
  }

  let { measures, grid, top, t1Color, t2Color }: Props = $props();

  // ---------------------------------------------------------------------------
  // Engraving metrics.
  //
  // Music notation sizes everything in STAFF SPACES — the distance between two
  // adjacent staff lines — so a score scales without ever re-tuning proportions.
  // The figures below are SMuFL `engravingDefaults` and `glyphBBoxes` from Bravura,
  // the same metadata MuseScore reads. Nothing here should be a bare pixel guess;
  // the first draft of this component was, and the clef came out a third too big.
  // ---------------------------------------------------------------------------

  /** One staff space. */
  const SP = STAFF_LINE_GAP;
  /** One staff step — half a space, the line-to-space distance. Glyph paths use this. */
  const UNIT = SP / 2;

  const STAFF_LINE_THICKNESS = 0.13 * SP; // staffLineThickness
  const THIN_BARLINE = 0.16 * SP; // thinBarlineThickness
  const THICK_BARLINE = 0.5 * SP; // thickBarlineThickness
  const STEM_THICKNESS = 0.12 * SP; // stemThickness
  const BEAM_THICKNESS = 0.5 * SP; // beamThickness
  const BEAM_STEP = (0.5 + 0.25) * SP; // beamThickness + beamSpacing
  const LEGER_THICKNESS = 0.16 * SP; // legerLineThickness
  const LEGER_EXTENSION = 0.4 * SP; // legerLineExtension, each side of the notehead
  const TIE_THICKNESS = 0.22 * SP; // tieMidpointThickness
  const NOTEHEAD_RX = 0.59 * SP; // noteheadBlack is 1.18 wide
  const NOTEHEAD_RY = 0.5 * SP; // …and 1.0 tall, so it fills a space exactly
  const NOTEHEAD_WHOLE_RX = 0.844 * SP; // noteheadWhole is 1.688 wide
  const DOT_R = 0.2 * SP; // augmentationDot is 0.4 across
  const DOT_OFFSET = NOTEHEAD_RX + 0.3 * SP;
  /** A stem is an octave — seven staff steps. From Gould, not the font metadata. */
  const STEM_LENGTH = 3.5 * SP;
  /** Stems meet the notehead at its edge, not its centre. */
  const STEM_X = NOTEHEAD_RX - STEM_THICKNESS / 2;
  /** Breathing room between an accidental and the notehead it belongs to. */
  const ACCIDENTAL_GAP = 0.16 * SP;
  /**
   * SMuFL fonts are drawn so one em spans four staff spaces. At this size every glyph
   * comes out engraved-correct with no per-glyph scaling, and its origin — the text
   * baseline — is the staff position it attaches to.
   */
  const SMUFL_SIZE = 4 * SP;
  /** For the small non-SMuFL labels, like the triplet 3. */
  const GLYPH_TEXT_SIZE = 0.95 * SP;

  /** Y of the bottom line. Staff step 0 is the bottom line, 8 the top. */
  let bottomY = $derived(top + 4 * SP);

  function y(step: number): number {
    return bottomY - step * UNIT;
  }

  // One call for the whole line: a note held across a barline has to be filed into two
  // measures at once, which a per-measure call cannot do.
  let layouts = $derived(layoutLine(measures, grid) as MeasureLayout[]);

  let staffWidth = $derived(TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH);

  /**
   * An accidental hangs to the LEFT of its notehead, clearing it by its own width — a
   * natural is two thirds a sharp and a double flat nearly twice one. The glyph's origin
   * is its left edge, so subtract the full width, not half of it.
   */
  function accidentalX(noteX: number, alter: number): number {
    const width = (ACCIDENTAL_WIDTH_SP[alter] ?? 1) * SP;
    return noteX - NOTEHEAD_RX - ACCIDENTAL_GAP - width;
  }
</script>

<g class="staff">
  <!-- Staff lines -->
  {#each [0, 2, 4, 6, 8] as step}
    <line
      x1={0}
      y1={y(step)}
      x2={staffWidth}
      y2={y(step)}
      stroke="var(--border)"
      stroke-width={STAFF_LINE_THICKNESS}
    />
  {/each}

  <!-- Treble clef with the 8 below. Its baseline sits on the G line, staff step 2. -->
  <text
    x={0.3 * SP}
    y={y(2)}
    fill="var(--text)"
    font-size={SMUFL_SIZE}
    font-family={SMUFL_FONT}>{CLEF_8VB}</text
  >

  {#each layouts as layout, mi}
    <!-- Barline -->
    <line
      x1={measureX(mi)}
      y1={y(8)}
      x2={measureX(mi)}
      y2={y(0)}
      stroke="var(--text-disabled)"
      stroke-width={THIN_BARLINE}
    />

    <!-- Rests -->
    {#each layout.rests as rest}
      <text
        x={rest.x - 0.5 * SP}
        y={y(REST_ANCHOR_STEP[rest.value] ?? 4)}
        fill="var(--text-disabled)"
        font-size={SMUFL_SIZE}
        font-family={SMUFL_FONT}>{REST_GLYPH[rest.value] ?? REST_GLYPH[4]}</text
      >
      {#if rest.dots === 1}
        <circle cx={rest.x + DOT_OFFSET} cy={y(5)} r={DOT_R} fill="var(--text-disabled)" />
      {/if}
    {/each}

    <!-- Ledger lines: they run past the notehead on both sides. -->
    {#each layout.notes as note}
      {#each note.ledger as step}
        <line
          x1={note.x - NOTEHEAD_RX - LEGER_EXTENSION}
          y1={y(step)}
          x2={note.x + NOTEHEAD_RX + LEGER_EXTENSION}
          y2={y(step)}
          stroke="var(--border)"
          stroke-width={LEGER_THICKNESS}
        />
      {/each}
    {/each}

    <!-- Beams and stems -->
    {#each layout.beams as group}
      {@const up = group.stemUp}
      {@const first = group.notes[0]}
      {@const last = group.notes[group.notes.length - 1]}
      {@const stemDx = up ? STEM_X : -STEM_X}
      {@const tipY = up
        ? Math.min(...group.notes.map((n) => y(n.staffStep))) - STEM_LENGTH
        : Math.max(...group.notes.map((n) => y(n.staffStep))) + STEM_LENGTH}
      {#each group.notes as note}
        {#if note.value >= 2}
          <line
            x1={note.x + stemDx}
            y1={y(note.staffStep)}
            x2={note.x + stemDx}
            y2={tipY}
            stroke="var(--text)"
            stroke-width={STEM_THICKNESS}
          />
        {/if}
      {/each}
      {#if group.notes.length > 1}
        <!-- Primary beam spans the whole group: every note in a group carries a flag. -->
        <line
          x1={first.x + stemDx}
          y1={tipY}
          x2={last.x + stemDx}
          y2={tipY}
          stroke="var(--text)"
          stroke-width={BEAM_THICKNESS}
        />
        <!--
          Secondary beam is per adjacent PAIR, not per group. A group may legitimately mix
          an eighth with sixteenths, and a group-wide flag would either drop the second beam
          (making sixteenths read as eighths) or run it under the eighth's stem.
        -->
        {#each group.notes.slice(0, -1) as n, bi}
          {#if n.value >= 16 && group.notes[bi + 1].value >= 16}
            <line
              x1={n.x + stemDx}
              y1={tipY + (up ? BEAM_STEP : -BEAM_STEP)}
              x2={group.notes[bi + 1].x + stemDx}
              y2={tipY + (up ? BEAM_STEP : -BEAM_STEP)}
              stroke="var(--text)"
              stroke-width={BEAM_THICKNESS}
            />
          {/if}
        {/each}
      {:else if first.value >= 8}
        <!-- Lone eighth or shorter: a flag, drawn as a short hook off the stem. -->
        <path
          d="M{first.x + stemDx},{tipY} q{0.6 * SP},{0.35 * SP} {0.45 * SP},{1.0 * SP}"
          fill="none"
          stroke="var(--text)"
          stroke-width={BEAM_THICKNESS * 0.55}
        />
      {/if}
      {#if group.bracket}
        <text
          x={(first.x + last.x) / 2}
          y={tipY + (up ? -0.4 * SP : 1.2 * SP)}
          text-anchor="middle"
          fill="var(--text-disabled)"
          font-size={GLYPH_TEXT_SIZE}
          font-family="var(--font)">3</text
        >
      {/if}
    {/each}

    <!-- Noteheads, accidentals, dots, ties -->
    {#each layout.notes as note, ni}
      {@const color = note.triad === 'T1' ? t1Color : t2Color}
      {@const open = note.value <= 2}
      {#if note.accidental !== null}
        <text
          x={accidentalX(note.x, note.accidental)}
          y={y(note.staffStep)}
          fill={color}
          font-size={SMUFL_SIZE}
          font-family={SMUFL_FONT}
          >{ACCIDENTAL_GLYPH[note.accidental] ?? ACCIDENTAL_GLYPH[0]}</text
        >
      {/if}
      <ellipse
        cx={note.x}
        cy={y(note.staffStep)}
        rx={note.value === 1 ? NOTEHEAD_WHOLE_RX : NOTEHEAD_RX}
        ry={NOTEHEAD_RY}
        transform="rotate(-20 {note.x} {y(note.staffStep)})"
        fill={open ? 'none' : color}
        stroke={color}
        stroke-width={open ? 0.13 * SP : 0}
      />
      {#if note.dots === 1}
        <!--
          The dot always sits in a SPACE. A note on a line (even staff step) pushes it to
          the space above; a note already in a space keeps its own height. A fixed offset
          would put the dot of a space-note straight onto the line above.
        -->
        <circle
          cx={note.x + DOT_OFFSET}
          cy={y(note.staffStep % 2 === 0 ? note.staffStep + 1 : note.staffStep)}
          r={DOT_R}
          fill={color}
        />
      {/if}
      <!--
        The tie's partner is usually the next note in this measure, but for a note held
        across a barline it is the FIRST note of the next measure — that continuation is
        exactly what layoutLine exists to produce, and looking only within this measure
        would silently drop the curve on the one case the whole design is about.
        Measures render into the same coordinate space, so the lookup just crosses arrays.
      -->
      {@const tiePartner = layout.notes[ni + 1] ?? layouts[mi + 1]?.notes[0]}
      {#if note.tiedToNext && tiePartner}
        <path
          d="M{note.x + NOTEHEAD_RX},{y(note.staffStep) + 0.6 * SP} Q{(note.x +
            tiePartner.x) /
            2},{y(note.staffStep) + 1.4 * SP} {tiePartner.x - NOTEHEAD_RX},{y(
            tiePartner.staffStep,
          ) + 0.6 * SP}"
          fill="none"
          stroke={color}
          stroke-width={TIE_THICKNESS}
        />
      {/if}
    {/each}
  {/each}

  <!-- Final barline -->
  {#if measures.length > 0}
    <line
      x1={measureX(measures.length)}
      y1={y(8)}
      x2={measureX(measures.length)}
      y2={y(0)}
      stroke="var(--text-disabled)"
      stroke-width={THICK_BARLINE}
    />
  {/if}
</g>
