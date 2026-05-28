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
      font-family="var(--font)"
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
            font-family="var(--font)"
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
