<script lang="ts">
  import type { SolvedChange } from '$lib/wasm';

  interface Props {
    changes: SolvedChange[];
    selectedIndex: number;
    locked: boolean[];
    barsPerLine?: number;
    onselect: (index: number) => void;
  }

  let { changes, selectedIndex, locked, barsPerLine = 4, onselect }: Props = $props();

  interface Bar {
    chords: { index: number; name: string; beats: number }[];
  }

  let bars = $derived((() => {
    const result: Bar[] = [];
    let currentBar: Bar = { chords: [] };
    let beatsInBar = 0;

    for (let i = 0; i < changes.length; i++) {
      const c = changes[i];
      currentBar.chords.push({ index: i, name: c.chord, beats: c.beats });
      beatsInBar += c.beats;
      if (beatsInBar >= 4) {
        result.push(currentBar);
        currentBar = { chords: [] };
        beatsInBar = 0;
      }
    }
    if (currentBar.chords.length > 0) {
      result.push(currentBar);
    }
    return result;
  })());

  let lines = $derived((() => {
    const result: Bar[][] = [];
    for (let i = 0; i < bars.length; i += barsPerLine) {
      result.push(bars.slice(i, i + barsPerLine));
    }
    return result;
  })());
</script>

<div class="chart-grid">
  {#each lines as line}
    <div class="chart-line">
      {#each line as bar}
        <div class="chart-bar">
          {#each bar.chords as chord}
            <button
              class="chart-chord"
              class:active={chord.index === selectedIndex}
              class:locked={locked[chord.index]}
              style="flex: {chord.beats}"
              onclick={() => onselect(chord.index)}
            >
              {#if locked[chord.index]}<span class="lock-icon">🔒</span>{/if}
              {chord.name}
            </button>
          {/each}
        </div>
      {/each}
      {#if line.length < barsPerLine}
        {#each Array(barsPerLine - line.length) as _}
          <div class="chart-bar empty"></div>
        {/each}
      {/if}
    </div>
  {/each}
</div>

<style>
  .chart-grid {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-family: var(--font);
  }

  .chart-line {
    display: flex;
    gap: 2px;
  }

  .chart-bar {
    flex: 1;
    display: flex;
    min-height: 36px;
    border: 1px solid var(--border);
    border-radius: 3px;
    overflow: hidden;
  }

  .chart-bar.empty {
    border-color: transparent;
  }

  .chart-chord {
    flex: 1;
    background: var(--bg-surface);
    border: none;
    border-radius: 0;
    color: var(--text-muted);
    font-size: var(--font-body);
    font-family: var(--font);
    padding: 6px 4px;
    cursor: pointer;
    text-align: center;
    white-space: nowrap;
    transition: background 100ms ease;
  }

  .chart-chord:hover {
    background: var(--bg-raised);
  }

  .chart-chord.active {
    background: var(--primary-muted);
    color: var(--text);
    font-weight: 700;
  }

  .chart-chord.locked {
    border-bottom: 2px solid var(--primary);
  }

  .lock-icon {
    font-size: 8px;
    margin-right: 2px;
  }
</style>
