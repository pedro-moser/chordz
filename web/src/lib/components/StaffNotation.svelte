<script lang="ts" module>
  /** Vertical space the staff block occupies, including ledger-line headroom. */
  export const STAFF_BLOCK_HEIGHT = 96;
</script>

<script lang="ts">
  import {
    layoutLine,
    STAFF_LINE_GAP,
    type Grid,
    type MeasureLayout,
  } from '$lib/notation';
  import { measureX, TAB_MEASURE_WIDTH, TAB_MARGIN_LEFT, type MeasureLike } from '$lib/tabLayout';
  import {
    TREBLE_CLEF_PATH,
    CLEF_OCTAVE_TEXT,
    REST_PATHS,
    ACCIDENTAL_PATHS,
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

  /** Half a staff step in pixels — the unit the glyph paths are authored in. */
  const UNIT = STAFF_LINE_GAP / 2;
  /** Y of the bottom line. Staff step 0 is the bottom line, 8 the top. */
  let bottomY = $derived(top + 4 * STAFF_LINE_GAP);

  function y(step: number): number {
    return bottomY - step * UNIT;
  }

  // One call for the whole line: a note held across a barline has to be filed into two
  // measures at once, which a per-measure call cannot do.
  let layouts = $derived(layoutLine(measures, grid) as MeasureLayout[]);

  let staffWidth = $derived(TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH);

  /** Stem length in pixels: a full octave of staff steps is the engraving default. */
  const STEM_LENGTH = 7 * UNIT;
</script>

<g class="staff">
  <!-- Staff lines -->
  {#each [0, 2, 4, 6, 8] as step}
    <line x1={0} y1={y(step)} x2={staffWidth} y2={y(step)} stroke="var(--border)" stroke-width="1" />
  {/each}

  <!-- Treble clef, 8vb -->
  <path
    d={TREBLE_CLEF_PATH}
    transform="translate({TAB_MARGIN_LEFT - 8}, {y(2)}) scale({UNIT})"
    fill="none"
    stroke="var(--text)"
    stroke-width={1.1 / UNIT}
    stroke-linecap="round"
  />
  <text
    x={TAB_MARGIN_LEFT - 4}
    y={y(-4)}
    text-anchor="middle"
    fill="var(--text)"
    font-size="8"
    font-family="var(--font)">{CLEF_OCTAVE_TEXT}</text
  >

  {#each layouts as layout, mi}
    <!-- Barline -->
    <line
      x1={measureX(mi)}
      y1={y(8)}
      x2={measureX(mi)}
      y2={y(0)}
      stroke="var(--text-disabled)"
      stroke-width="1"
    />

    <!-- Rests -->
    {#each layout.rests as rest}
      <path
        d={REST_PATHS[rest.value] ?? REST_PATHS[4]}
        transform="translate({rest.x}, {y(4)}) scale({UNIT})"
        fill="var(--text-disabled)"
      />
      {#if rest.dots === 1}
        <circle cx={rest.x + 1.6 * UNIT} cy={y(4) - UNIT} r={0.9} fill="var(--text-disabled)" />
      {/if}
    {/each}

    <!-- Ledger lines -->
    {#each layout.notes as note}
      {#each note.ledger as step}
        <line
          x1={note.x - 5}
          y1={y(step)}
          x2={note.x + 5}
          y2={y(step)}
          stroke="var(--border)"
          stroke-width="1"
        />
      {/each}
    {/each}

    <!-- Beams and stems -->
    {#each layout.beams as group}
      {@const up = group.stemUp}
      {@const first = group.notes[0]}
      {@const last = group.notes[group.notes.length - 1]}
      {@const tipY = up
        ? Math.min(...group.notes.map((n) => y(n.staffStep))) - STEM_LENGTH
        : Math.max(...group.notes.map((n) => y(n.staffStep))) + STEM_LENGTH}
      {#each group.notes as note}
        {#if note.value >= 2}
          <line
            x1={note.x + (up ? 4 : -4)}
            y1={y(note.staffStep)}
            x2={note.x + (up ? 4 : -4)}
            y2={tipY}
            stroke="var(--text)"
            stroke-width="1.1"
          />
        {/if}
      {/each}
      {#if group.notes.length > 1 && first.value >= 8}
        <line
          x1={first.x + (up ? 4 : -4)}
          y1={tipY}
          x2={last.x + (up ? 4 : -4)}
          y2={tipY}
          stroke="var(--text)"
          stroke-width="2.4"
        />
        {#if first.value >= 16}
          <line
            x1={first.x + (up ? 4 : -4)}
            y1={tipY + (up ? 4 : -4)}
            x2={last.x + (up ? 4 : -4)}
            y2={tipY + (up ? 4 : -4)}
            stroke="var(--text)"
            stroke-width="2.4"
          />
        {/if}
      {:else if first.value >= 8}
        <!-- Lone eighth or shorter: a flag, drawn as a short hook off the stem. -->
        <path
          d="M{first.x + (up ? 4 : -4)},{tipY} q4,2 3,6"
          fill="none"
          stroke="var(--text)"
          stroke-width="1.4"
        />
      {/if}
      {#if group.bracket}
        <text
          x={(first.x + last.x) / 2}
          y={tipY + (up ? -3 : 9)}
          text-anchor="middle"
          fill="var(--text-disabled)"
          font-size="8"
          font-family="var(--font)">3</text
        >
      {/if}
    {/each}

    <!-- Noteheads, accidentals, dots, ties -->
    {#each layout.notes as note, ni}
      {@const color = note.triad === 'T1' ? t1Color : t2Color}
      {#if note.accidental !== null}
        <path
          d={ACCIDENTAL_PATHS[note.accidental] ?? ACCIDENTAL_PATHS[0]}
          transform="translate({note.x - 8}, {y(note.staffStep)}) scale({UNIT})"
          fill={color}
        />
      {/if}
      <ellipse
        cx={note.x}
        cy={y(note.staffStep)}
        rx="4"
        ry="2.8"
        transform="rotate(-20 {note.x} {y(note.staffStep)})"
        fill={note.value <= 2 ? 'none' : color}
        stroke={color}
        stroke-width={note.value <= 2 ? 1.2 : 0}
      />
      {#if note.dots === 1}
        <circle cx={note.x + 7} cy={y(note.staffStep) - UNIT} r="1.1" fill={color} />
      {/if}
      {#if note.tiedToNext && layout.notes[ni + 1]}
        <path
          d="M{note.x + 5},{y(note.staffStep) + 5} Q{(note.x + layout.notes[ni + 1].x) / 2},{y(
            note.staffStep,
          ) + 10} {layout.notes[ni + 1].x - 5},{y(layout.notes[ni + 1].staffStep) + 5}"
          fill="none"
          stroke={color}
          stroke-width="1"
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
      stroke-width="2"
    />
  {/if}
</g>
