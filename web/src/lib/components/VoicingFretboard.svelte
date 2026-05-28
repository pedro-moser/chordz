<script lang="ts">
  import { getIntervalName } from '$lib/wasm';

  interface Props {
    positions: (number | null)[];
    notes: ({ pc: number; name: string } | null)[];
    intervals: string[];
    rootPc?: number;
  }

  let { positions, notes, intervals, rootPc }: Props = $props();

  const numStrings = 6;
  const stringSpacing = 28;
  const fretSpacing = 40;
  const leftMargin = 40;
  const topMargin = 30;
  const dotRadius = 10;
  const stringLabels = ['E', 'A', 'D', 'G', 'B', 'e'];

  let played = $derived(positions.filter((p): p is number => p !== null));
  let hasOpen = $derived(played.includes(0));
  let minFret = $derived(Math.min(...played.filter(f => f > 0), 0) || 0);
  let maxFret = $derived(Math.max(...played, 0));
  let startFret = $derived(hasOpen ? 0 : minFret);
  let endFret = $derived(Math.max(maxFret, startFret + 3));
  let numFrets = $derived(endFret - startFret + 1);

  let width = $derived(leftMargin + stringSpacing * 5 + 40);
  let height = $derived(topMargin + fretSpacing * numFrets + 20);

  function fretY(fret: number): number {
    return topMargin + (fret - startFret) * fretSpacing + fretSpacing / 2;
  }

  function stringX(s: number): number {
    return leftMargin + s * stringSpacing;
  }

  function intervalForString(s: number): string | null {
    if (!intervals) return null;
    let idx = 0;
    for (let i = 0; i < s; i++) {
      if (positions[i] !== null) idx++;
    }
    if (positions[s] === null) return null;
    return intervals[idx] ?? null;
  }
</script>

<svg class="voicing-fretboard" {width} {height} viewBox="0 0 {width} {height}">
  <!-- String labels -->
  {#each stringLabels as label, s}
    <text
      x={stringX(s)}
      y={8}
      text-anchor="middle"
      fill="#999"
      font-size="13"
      font-family="var(--font)"
    >{label}</text>
  {/each}

  <!-- Fret lines -->
  {#each Array(numFrets + 1) as _, f}
    {@const y = topMargin + f * fretSpacing}
    <line
      x1={leftMargin - stringSpacing * 0.4}
      y1={y}
      x2={leftMargin + 5 * stringSpacing + stringSpacing * 0.4}
      y2={y}
      stroke={f === 0 && startFret === 0 ? '#ccc' : '#3d3d3d'}
      stroke-width={f === 0 && startFret === 0 ? 2.5 : 1}
    />
  {/each}

  <!-- Fret numbers -->
  {#each Array(numFrets) as _, f}
    {@const fretNum = startFret + f}
    {@const y = topMargin + f * fretSpacing + fretSpacing / 2}
    <text
      x={16}
      y={y + 4}
      text-anchor="middle"
      fill="#444"
      font-size="12"
      font-family="var(--font)"
    >{fretNum}</text>
  {/each}

  <!-- Strings -->
  {#each Array(numStrings) as _, s}
    <line
      x1={stringX(s)}
      y1={topMargin}
      x2={stringX(s)}
      y2={topMargin + numFrets * fretSpacing}
      stroke="#555"
      stroke-width="1"
    />
  {/each}

  <!-- Dots and mutes -->
  {#each positions as pos, s}
    {#if pos === null}
      <text
        x={stringX(s)}
        y={topMargin - 12}
        text-anchor="middle"
        fill="#666"
        font-size="14"
        font-family="var(--font)"
      >×</text>
    {:else if pos === 0}
      <circle
        cx={stringX(s)}
        cy={topMargin - 12}
        r={6}
        fill="none"
        stroke="#ccc"
        stroke-width="1.5"
      />
    {:else}
      {@const iv = intervalForString(s)}
      {@const isRoot = iv === '1' || iv === 'R'}
      <circle
        cx={stringX(s)}
        cy={fretY(pos)}
        r={dotRadius}
        fill={isRoot ? 'var(--primary)' : 'var(--secondary)'}
      />
      {#if iv}
        <text
          x={stringX(s)}
          y={fretY(pos) + 3}
          text-anchor="middle"
          fill="#1a1a1a"
          font-size="9"
          font-weight="bold"
          font-family="var(--font)"
        >{iv}</text>
      {/if}
      {#if notes[s]}
        <text
          x={stringX(s)}
          y={fretY(pos) + dotRadius + 12}
          text-anchor="middle"
          fill="#999"
          font-size="9"
          font-family="var(--font)"
        >{notes[s]?.name}</text>
      {/if}
    {/if}
  {/each}
</svg>

<style>
  .voicing-fretboard {
    background: #222;
    border-radius: 6px;
    display: block;
  }
</style>
