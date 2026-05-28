<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import VoicingFretboard from '$lib/components/VoicingFretboard.svelte';
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

  onMount(() => {
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  function onKey(e: KeyboardEvent) {
    if (!solved) return;
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    switch (e.key) {
      case 'j': case 'ArrowRight':
        e.preventDefault();
        if (selectedChord < solved.length - 1) selectedChord++;
        break;
      case 'k': case 'ArrowLeft':
        e.preventDefault();
        if (selectedChord > 0) selectedChord--;
        break;
    }
  }

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
      <label class="input-label" for="tune-title">Title</label>
      <input id="tune-title" bind:value={titleInput} placeholder="Tune name" />
    </div>
    <div class="input-row">
      <label class="input-label" for="tune-chart">Chart</label>
      <textarea id="tune-chart" bind:value={chartInput} rows="3" placeholder="Dm7 | G7 | Cmaj7 | Cmaj7"></textarea>
    </div>
    <button class="solve-btn" onclick={solve}>Solve</button>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  {#if solved}
    <div class="tune-result">
      <div class="chord-nav">
        <button class="nav-btn" disabled={selectedChord === 0} onclick={() => selectedChord--}>◀</button>
        {#each solved as change, i}
          <button
            class="chord-btn"
            class:active={i === selectedChord}
            onclick={() => selectedChord = i}
          >
            {change.chord}
          </button>
        {/each}
        <button class="nav-btn" disabled={selectedChord >= solved.length - 1} onclick={() => selectedChord++}>▶</button>
        <span class="hint">{selectedChord + 1}/{solved.length} · ←→ navigate</span>
      </div>

      {#if selected}
        <div class="chord-detail">
          <h2>{selected.chord} <span class="recipe-tag">{selected.recipe}</span></h2>
          <p class="intervals-display">{selected.intervals?.join('  ') ?? ''}</p>
          <VoicingFretboard
            positions={selected.positions}
            notes={selected.notes}
            intervals={selected.intervals ?? []}
          />
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

  .nav-btn {
    padding: 4px 8px;
    font-size: var(--font-label);
    background: var(--bg-raised);
    border: 1px solid var(--border);
  }

  .nav-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .nav-btn:not(:disabled):hover {
    background: var(--primary-muted);
  }

  .hint {
    font-size: var(--font-label);
    color: var(--text-disabled);
    margin-left: 8px;
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
