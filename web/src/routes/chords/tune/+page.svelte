<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';
  import { solveChart } from '$lib/wasm';
  import type { SolvedChange } from '$lib/wasm';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];

  let chartInput = $state('Dm7 | G7 | Cmaj7 | Cmaj7');
  let titleInput = $state('Untitled');
  let solved = $state<SolvedChange[] | null>(null);
  let error = $state<string | null>(null);
  let selectedChord = $state(0);

  function solve() {
    const result = solveChart(chartInput, titleInput);
    if (result.error) {
      error = result.error;
      solved = null;
    } else {
      error = null;
      solved = result.changes ?? null;
      selectedChord = 0;
    }
  }

  let selected = $derived(solved ? solved[selectedChord] ?? null : null);
</script>

<SubTabs tabs={chordTabs} active="Tune" />

<div class="tune-layout">
  <div class="tune-input">
    <div class="input-row">
      <label class="input-label">Title</label>
      <input bind:value={titleInput} placeholder="Tune name" />
    </div>
    <div class="input-row">
      <label class="input-label">Chart</label>
      <textarea bind:value={chartInput} rows="3" placeholder="Dm7 | G7 | Cmaj7 | Cmaj7"></textarea>
    </div>
    <button class="solve-btn" onclick={solve}>Solve</button>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  {#if solved}
    <div class="tune-result">
      <div class="chord-nav">
        {#each solved as change, i}
          <button
            class="chord-btn"
            class:active={i === selectedChord}
            onclick={() => selectedChord = i}
          >
            {change.chord}
          </button>
        {/each}
      </div>

      {#if selected}
        <div class="chord-detail">
          <h2>{selected.chord} <span class="recipe-tag">{selected.recipe}</span></h2>
          <p class="intervals-display">{selected.intervals.join('  ')}</p>
          <div class="fret-diagram">
            {#each selected.positions as pos, s}
              <div class="string-row">
                <span class="string-label">{['E','A','D','G','B','e'][s]}</span>
                <span class="fret-value">{pos !== null ? pos : '×'}</span>
                {#if pos !== null && selected.notes[s]}
                  <span class="note-name">{selected.notes[s]?.name}</span>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tune-layout {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 16px;
    overflow-y: auto;
  }

  .tune-input {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 600px;
  }

  .input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-label {
    font-size: var(--font-label);
    color: var(--text-muted);
  }

  textarea {
    font-family: var(--font);
    font-size: var(--font-body);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px;
    resize: vertical;
  }

  textarea:focus {
    border-color: var(--primary);
    outline: none;
  }

  .solve-btn {
    align-self: flex-start;
    background: var(--primary-muted);
    color: var(--text);
    padding: 6px 16px;
    font-weight: 700;
  }

  .solve-btn:hover {
    background: var(--primary);
    color: #1a1a1a;
  }

  .error {
    color: #e66;
    font-size: var(--font-label);
  }

  .tune-result {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .chord-nav {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .chord-btn {
    padding: 4px 10px;
    font-size: var(--font-label);
    background: var(--bg-raised);
    border: 1px solid var(--border);
  }

  .chord-btn.active {
    background: var(--primary-muted);
    border-color: var(--primary);
    color: var(--text);
  }

  .chord-detail h2 {
    font-size: var(--font-heading);
    color: var(--text);
    margin-bottom: 8px;
  }

  .recipe-tag {
    font-size: var(--font-body);
    color: var(--primary);
    font-weight: 400;
  }

  .intervals-display {
    color: var(--text-muted);
    margin-bottom: 16px;
  }

  .fret-diagram {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .string-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .string-label {
    width: 16px;
    color: var(--text-muted);
  }

  .fret-value {
    width: 24px;
    text-align: center;
    color: var(--text);
    font-weight: 700;
  }

  .note-name {
    color: var(--secondary);
    font-size: var(--font-label);
  }
</style>
