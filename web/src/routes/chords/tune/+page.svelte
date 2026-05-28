<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import VoicingFretboard from '$lib/components/VoicingFretboard.svelte';
  import ChartGrid from '$lib/components/ChartGrid.svelte';
  import { solveChart, solveChartWithLocks, getPresets } from '$lib/wasm';
  import { playStrum, playClick, playBass, stopAll, bassVolume } from '$lib/audio';
  import * as audio from '$lib/audio';
  import type { SolvedChange, SolverConfig, Preset } from '$lib/wasm';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];

  const ALL_RECIPES = ['shell', 'closed', 'drop2', 'drop3', 'rless-a', 'rless-b', 'quartal', 'upper', 'triads'];
  const ABSTRACTION_PRESETS = [
    { label: 'Grounded', value: 0.0 },
    { label: 'Balanced', value: 0.3 },
    { label: 'Open', value: 0.6 },
    { label: 'Abstract', value: 1.0 },
  ];
  const TENSION_PRESETS = [
    { label: 'Low', value: 0.0 },
    { label: 'Medium', value: 0.3 },
    { label: 'High', value: 0.7 },
    { label: 'Max', value: 1.0 },
  ];
  const SMOOTHNESS_PRESETS = [
    { label: 'Free', value: 0.0 },
    { label: 'Normal', value: 1.0 },
    { label: 'Smooth', value: 2.0 },
    { label: 'Tight', value: 3.0 },
  ];
  const NOTE_FILTERS: { label: string; min: number; max: number }[] = [
    { label: '3-4', min: 3, max: 4 },
    { label: '3', min: 3, max: 3 },
    { label: '4', min: 4, max: 4 },
    { label: '5', min: 5, max: 5 },
    { label: '3-5', min: 3, max: 5 },
  ];

  // State
  let presets = $state<Preset[]>([]);
  let chartInput = $state('Dm7 | G7 | Cmaj7 | Cmaj7');
  let titleInput = $state('Untitled');
  let solved = $state<SolvedChange[] | null>(null);
  let error = $state<string | null>(null);
  let selectedChord = $state(0);
  let altIndices = $state<number[]>([]);
  let locked = $state<boolean[]>([]);
  let constraintsOpen = $state(false);
  let playingAll = $state(false);
  let bpm = $state(120);
  let clickEnabled = $state(true);
  let bassEnabled = $state(false);

  // Solver constraints
  let abstraction = $state(0.0);
  let tension = $state(0.3);
  let smoothness = $state(1.0);
  let variation = $state(0);
  let noteFilterIdx = $state(0);
  let fretMin = $state(0);
  let fretMax = $state(15);
  let maxSpan = $state(5);
  let allowOpenStrings = $state(true);
  let recipeFilterOn = $state(false);
  let recipes = $state(ALL_RECIPES.map(() => true));
  let stringFilterOn = $state(false);
  let strings = $state([true, true, true, true, true, true]);

  onMount(() => {
    presets = getPresets();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    if (!solved) {
      if (e.key === 'Enter') { e.preventDefault(); solve(); }
      return;
    }
    switch (e.key) {
      case 'j': case 'ArrowRight':
        e.preventDefault();
        if (selectedChord < solved.length - 1) selectedChord++;
        break;
      case 'k': case 'ArrowLeft':
        e.preventDefault();
        if (selectedChord > 0) selectedChord--;
        break;
      case 'h':
        e.preventDefault();
        swapAlt(-1);
        break;
      case 'l':
        e.preventDefault();
        swapAlt(1);
        break;
      case ' ':
        e.preventDefault();
        if (selected) playStrum(selected.positions);
        break;
      case 'Enter':
        e.preventDefault();
        solve();
        break;
    }
  }

  function buildConfig(): SolverConfig {
    const nf = NOTE_FILTERS[noteFilterIdx];
    const cfg: SolverConfig = {
      minStrings: nf.min,
      maxStrings: nf.max,
      maxFretSpan: maxSpan,
      maxFret: fretMax,
      minFret: fretMin,
      tensionTarget: tension,
      abstraction,
      smoothnessWeight: smoothness,
      jitter: variation,
      allowOpenStrings,
    };
    if (stringFilterOn) {
      cfg.allowedStrings = strings;
    }
    if (recipeFilterOn) {
      const selected = ALL_RECIPES.filter((_, i) => recipes[i]);
      if (selected.length > 0 && selected.length < ALL_RECIPES.length) {
        cfg.recipes = selected;
      }
    }
    return cfg;
  }

  function solve() {
    const config = buildConfig();
    const hasLocks = locked.some(Boolean);

    let result;
    if (hasLocks && solved) {
      const locks = solved.map((change, i) => {
        if (!locked[i]) return null;
        return { positions: change.positions, recipe: change.recipe };
      });
      result = solveChartWithLocks(chartInput, titleInput, config, locks);
    } else {
      result = solveChart(chartInput, titleInput, config);
    }

    if (result.error) {
      error = result.error;
      solved = null;
    } else {
      error = null;
      solved = result.changes ?? null;
      if (solved) {
        altIndices = solved.map(() => 0);
        // Preserve locks that are within bounds
        const newLocked = solved.map((_, i) => locked[i] ?? false);
        locked = newLocked;
      }
      selectedChord = 0;
    }
  }

  function selectPreset(idx: number) {
    if (idx < 0 || idx >= presets.length) return;
    titleInput = presets[idx].title;
    chartInput = presets[idx].chart;
    solved = null;
    locked = [];
    altIndices = [];
    error = null;
  }

  function swapAlt(direction: number) {
    if (!solved) return;
    const change = solved[selectedChord];
    if (!change || !change.alternatives || change.alternatives.length <= 1) return;

    const total = change.alternatives.length;
    const current = altIndices[selectedChord] ?? 0;
    const next = ((current + direction) % total + total) % total;

    const alt = change.alternatives[next];
    solved[selectedChord] = {
      ...change,
      positions: alt.positions,
      notes: alt.notes,
      intervals: alt.intervals,
      recipe: alt.recipe,
      relaxation: alt.relaxation,
      tension: alt.tension,
    };
    altIndices[selectedChord] = next;
  }

  function toggleLock(idx: number) {
    locked[idx] = !locked[idx];
  }

  function clearLocks() {
    locked = locked.map(() => false);
  }

  let lockedCount = $derived(locked.filter(Boolean).length);

  async function playAll() {
    if (!solved || playingAll) return;
    playingAll = true;
    const beatMs = 60000 / bpm;
    for (let i = 0; i < solved.length; i++) {
      if (!playingAll) break;
      selectedChord = i;
      if (bassEnabled) playBass(solved[i].rootPc);
      playStrum(solved[i].positions);
      const beats = solved[i].beats;
      for (let b = 0; b < beats; b++) {
        if (!playingAll) break;
        if (clickEnabled && b > 0) playClick();
        await new Promise(r => setTimeout(r, beatMs));
      }
    }
    playingAll = false;
  }

  function stopPlayAll() {
    playingAll = false;
    stopAll();
  }

  let selected = $derived(solved ? solved[selectedChord] ?? null : null);
  let selectedAltIdx = $derived(altIndices[selectedChord] ?? 0);
  let selectedAltCount = $derived(selected?.alternatives?.length ?? 0);
</script>

<SubTabs tabs={chordTabs} active="Tune" />

<div class="tune-layout">
  <div class="tune-left">
    <!-- Input section -->
    <div class="tune-input">
      <div class="input-row row-inline">
        <label class="input-label" for="tune-title">Title</label>
        <input id="tune-title" bind:value={titleInput} placeholder="Tune name" />
        {#if presets.length > 0}
          <select class="preset-select" onchange={(e) => { selectPreset(parseInt((e.target as HTMLSelectElement).value)); (e.target as HTMLSelectElement).value = '-1'; }}>
            <option value="-1">Presets...</option>
            {#each presets as preset, i}
              <option value={i}>{preset.title}</option>
            {/each}
          </select>
        {/if}
      </div>
      <div class="input-row">
        <label class="input-label" for="tune-chart">Chart</label>
        <textarea id="tune-chart" bind:value={chartInput} rows="2" placeholder="Dm7 | G7 | Cmaj7 | Cmaj7"></textarea>
      </div>
      <div class="btn-row">
        <button class="solve-btn" onclick={solve}>Solve (Enter)</button>
        <button class="toggle-btn" onclick={() => constraintsOpen = !constraintsOpen}>
          {constraintsOpen ? '▾' : '▸'} Constraints
        </button>
        {#if error}
          <span class="error">{error}</span>
        {/if}
      </div>
    </div>

    <!-- Constraints panel -->
    {#if constraintsOpen}
    <div class="constraints-panel">
      <div class="constraint-row">
        <span class="constraint-label">Abstraction</span>
        <div class="btn-group">
          {#each ABSTRACTION_PRESETS as ap}
            <button class="filter-btn" class:active={abstraction === ap.value} onclick={() => abstraction = ap.value}>{ap.label}</button>
          {/each}
        </div>
      </div>
      <div class="constraint-row">
        <span class="constraint-label">Tension</span>
        <div class="btn-group">
          {#each TENSION_PRESETS as tp}
            <button class="filter-btn" class:active={tension === tp.value} onclick={() => tension = tp.value}>{tp.label}</button>
          {/each}
        </div>
      </div>
      <div class="constraint-row">
        <span class="constraint-label">Movement</span>
        <div class="btn-group">
          {#each SMOOTHNESS_PRESETS as sp}
            <button class="filter-btn" class:active={smoothness === sp.value} onclick={() => smoothness = sp.value}>{sp.label}</button>
          {/each}
        </div>
      </div>
      <div class="constraint-row">
        <label for="variation-slider">Variation <span class="val">{variation}</span></label>
        <input id="variation-slider" type="range" min="0" max="100" step="1" bind:value={variation} />
      </div>
      <div class="constraint-row">
        <span class="constraint-label">Notes</span>
        <div class="btn-group">
          {#each NOTE_FILTERS as nf, i}
            <button class="filter-btn" class:active={noteFilterIdx === i} onclick={() => noteFilterIdx = i}>{nf.label}</button>
          {/each}
        </div>
      </div>
      <div class="constraint-row">
        <label for="fret-min">Fret range</label>
        <div class="num-inputs">
          <input id="fret-min" type="number" min="0" max="15" bind:value={fretMin} class="num-input" />
          <span>to</span>
          <input id="fret-max" type="number" min="0" max="15" bind:value={fretMax} class="num-input" />
        </div>
      </div>
      <div class="constraint-row">
        <label for="max-span">Max span</label>
        <input id="max-span" type="number" min="2" max="8" bind:value={maxSpan} class="num-input" />
      </div>
      <div class="constraint-row">
        <label>
          <input type="checkbox" bind:checked={allowOpenStrings} /> Open strings
        </label>
      </div>
      <div class="constraint-row">
        <label>
          <input type="checkbox" bind:checked={recipeFilterOn} /> Recipe filter
        </label>
        {#if recipeFilterOn}
          <div class="recipe-checks">
            {#each ALL_RECIPES as name, i}
              <label class="recipe-label">
                <input type="checkbox" bind:checked={recipes[i]} /> {name}
              </label>
            {/each}
          </div>
        {/if}
      </div>
      <div class="constraint-row">
        <label>
          <input type="checkbox" bind:checked={stringFilterOn} /> String filter
        </label>
        {#if stringFilterOn}
          <div class="recipe-checks">
            {#each ['E', 'A', 'D', 'G', 'B', 'e'] as name, i}
              <label class="recipe-label">
                <input type="checkbox" bind:checked={strings[i]} /> {name}
              </label>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}

    <!-- Results: chart grid -->
    {#if solved}
      <div class="result-actions">
        <div class="bpm-control">
          <label class="bpm-label" for="bpm-input">BPM</label>
          <input id="bpm-input" type="number" min="40" max="300" bind:value={bpm} class="bpm-input" />
          <label class="click-label">
            <input type="checkbox" bind:checked={clickEnabled} /> Click
          </label>
          <label class="click-label">
            <input type="checkbox" bind:checked={bassEnabled} /> Bass
          </label>
          {#if bassEnabled}
            <input
              type="range"
              min="0"
              max="1"
              step="0.1"
              value={audio.bassVolume}
              oninput={(e) => audio.bassVolume = Number((e.target as HTMLInputElement).value)}
              class="bass-vol"
              title="Bass volume"
            />
          {/if}
        </div>
        {#if playingAll}
          <button class="action-btn playing" onclick={stopPlayAll}>Stop</button>
        {:else}
          <button class="action-btn" onclick={playAll}>Play all</button>
        {/if}
        {#if lockedCount > 0}
          <span class="lock-count">{lockedCount} locked</span>
          <button class="action-btn" onclick={clearLocks}>Clear</button>
        {/if}
      </div>

      <ChartGrid
        changes={solved}
        selectedIndex={selectedChord}
        {locked}
        onselect={(i) => selectedChord = i}
      />
    {/if}
  </div>

  <!-- Right: fretboard + detail -->
  <div class="tune-right">
    {#if selected}
      <div class="detail-header">
        <h2>{selected.chord} <span class="recipe-tag">{selected.recipe}</span></h2>
        {#if selected.relaxation !== 'exact'}
          <span class="relaxation-tag">{selected.relaxation}</span>
        {/if}
      </div>

      <div class="detail-controls">
        <button class="play-btn" onclick={() => playStrum(selected!.positions)}>Strum</button>
        <div class="swap-controls">
          <button class="swap-btn" disabled={selectedAltCount <= 1} onclick={() => swapAlt(-1)}>◀</button>
          <span class="alt-counter">{selectedAltIdx + 1}/{selectedAltCount}</span>
          <button class="swap-btn" disabled={selectedAltCount <= 1} onclick={() => swapAlt(1)}>▶</button>
        </div>
        <label class="lock-toggle">
          <input type="checkbox" checked={locked[selectedChord] ?? false} onchange={() => toggleLock(selectedChord)} />
          Lock
        </label>
      </div>

      <p class="intervals-display">{selected.intervals?.join('  ') ?? ''}</p>

      <VoicingFretboard
        positions={selected.positions}
        notes={selected.notes}
        intervals={selected.intervals ?? []}
      />

      <p class="keyboard-hint">←→ navigate · h/l swap · Space strum</p>
    {:else if !solved}
      <p class="empty-hint">Enter a chart and press Solve</p>
    {/if}
  </div>
</div>

<style>
  .tune-layout {
    flex: 1;
    display: flex;
    gap: 16px;
    padding: 12px;
    overflow: hidden;
  }

  .tune-left {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    min-width: 0;
  }

  .tune-right {
    width: 260px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding-top: 2px;
  }

  .tune-input {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row-inline {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .row-inline input {
    flex: 1;
  }

  .input-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    white-space: nowrap;
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

  .preset-select {
    font-family: var(--font);
    font-size: var(--font-label);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
  }

  .btn-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .solve-btn {
    background: var(--primary-muted);
    color: var(--text);
    padding: 6px 16px;
    font-weight: 700;
  }

  .solve-btn:hover {
    background: var(--primary);
    color: #1a1a1a;
  }

  .toggle-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 6px 12px;
    font-size: var(--font-label);
  }

  .toggle-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  .error {
    color: #e66;
    font-size: var(--font-label);
  }

  /* Constraints panel */
  .constraints-panel {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: var(--font-label);
  }

  .constraint-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .constraint-label,
  .constraint-row label {
    font-size: var(--font-label);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .constraint-row .val {
    color: var(--primary);
    font-weight: 700;
    margin-left: 4px;
  }

  .constraint-row input[type="range"] {
    width: 100%;
    max-width: 300px;
    accent-color: var(--primary);
  }

  .btn-group {
    display: flex;
    gap: 4px;
  }

  .filter-btn {
    padding: 3px 10px;
    font-size: var(--font-label);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .filter-btn.active {
    background: var(--primary-muted);
    border-color: var(--primary);
    color: var(--text);
  }

  .num-inputs {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: var(--font-label);
  }

  .num-input {
    width: 56px;
    padding: 3px 6px;
    font-family: var(--font);
    font-size: var(--font-label);
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    text-align: center;
  }

  .recipe-checks {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 4px;
  }

  .recipe-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .empty-hint {
    color: var(--text-disabled);
    font-size: var(--font-label);
    padding-top: 24px;
  }

  .keyboard-hint {
    font-size: 10px;
    color: var(--text-disabled);
    margin-top: 8px;
  }

  .result-actions {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  .bpm-control {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .bpm-label {
    font-size: var(--font-label);
    color: var(--text-muted);
  }

  .bpm-input {
    width: 56px;
    padding: 3px 6px;
    font-family: var(--font);
    font-size: var(--font-label);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    text-align: center;
  }

  .click-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .click-label input[type="checkbox"] {
    accent-color: var(--primary);
  }

  .bass-vol {
    width: 60px;
    accent-color: var(--primary);
  }

  .lock-count {
    font-size: var(--font-label);
    color: var(--primary);
    font-weight: 700;
  }

  .action-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 12px;
    font-size: var(--font-label);
  }

  .action-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  .action-btn.playing {
    background: var(--primary-muted);
    color: var(--text);
    border-color: var(--primary);
  }

  /* Detail panel */

  .detail-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .recipe-tag {
    font-size: var(--font-body);
    color: var(--primary);
    font-weight: 400;
  }

  .relaxation-tag {
    font-size: var(--font-label);
    color: #e66;
    background: rgba(238, 102, 102, 0.1);
    padding: 1px 6px;
    border-radius: 3px;
  }

  .intervals-display {
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .detail-controls {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .play-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 12px;
    font-size: var(--font-label);
  }

  .play-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  .swap-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .swap-btn {
    padding: 3px 8px;
    font-size: 11px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .swap-btn:not(:disabled):hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  .swap-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .alt-counter {
    font-size: var(--font-label);
    color: var(--text-muted);
    min-width: 40px;
    text-align: center;
  }

  .lock-toggle {
    font-size: var(--font-label);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .keyboard-hint {
    font-size: 11px;
    color: var(--text-disabled);
    margin-top: 12px;
  }
</style>
