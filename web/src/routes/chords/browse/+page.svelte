<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import Select from '$lib/components/Select.svelte';
  import { getRoots, getFamilies, generateVoicings } from '$lib/wasm';
  import type { VoicingInfo } from '$lib/wasm';
  import VoicingFretboard from '$lib/components/VoicingFretboard.svelte';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];

  let roots = $state<string[]>([]);
  let families = $state<{index: number; name: string}[]>([]);
  let rootIndex = $state(0);
  let familyIndex = $state(0);
  let noteCount = $state(4);
  let voicings = $state<VoicingInfo[]>([]);
  let selectedIndex = $state(0);

  onMount(() => {
    roots = getRoots();
    families = getFamilies();
    refresh();
  });

  function refresh() {
    voicings = generateVoicings(rootIndex, familyIndex, noteCount);
    selectedIndex = 0;
  }

  let rootOptions = $derived(roots.map((r, i) => ({ label: r, value: i })));
  let familyOptions = $derived(families.map((f) => ({ label: f.name, value: f.index })));
  let noteOptions = [
    { label: '3', value: 3 },
    { label: '4', value: 4 },
    { label: '5', value: 5 },
    { label: '6', value: 6 },
  ];

  let selected = $derived(voicings[selectedIndex] ?? null);
</script>

<SubTabs tabs={chordTabs} active="Browse" />

<div class="browser-layout">
  <div class="browser-controls">
    <Select label="Root" value={rootIndex} options={rootOptions} onchange={(v) => { rootIndex = v; refresh(); }} />
    <Select label="Family" value={familyIndex} options={familyOptions} onchange={(v) => { familyIndex = v; refresh(); }} />
    <Select label="Notes" value={noteCount} options={noteOptions} onchange={(v) => { noteCount = v; refresh(); }} />
    <span class="count">{voicings.length} voicings</span>
  </div>

  <div class="browser-body">
    <div class="voicing-list">
      {#each voicings as v, i}
        <button
          class="voicing-item"
          class:selected={i === selectedIndex}
          onclick={() => selectedIndex = i}
        >
          <span class="v-chord">{v.chord}</span>
          <span class="v-recipe">{v.recipe}</span>
          <span class="v-intervals">{v.intervals?.join(' ') ?? ''}</span>
        </button>
      {/each}
    </div>

    <div class="voicing-detail">
      {#if selected}
        <h2>{selected.chord} <span class="recipe-tag">{selected.recipe}</span></h2>
        <p class="intervals-display">{selected.intervals?.join('  ') ?? ''}</p>
        <VoicingFretboard
          positions={selected.positions}
          notes={selected.notes}
          intervals={selected.intervals ?? []}
        />
      {:else}
        <p class="empty">No voicings for this combination</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .browser-layout {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .browser-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }

  .count {
    font-size: var(--font-label);
    color: var(--text-muted);
    margin-left: auto;
  }

  .browser-body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .voicing-list {
    width: 300px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }

  .voicing-item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 0;
    padding: 6px 12px;
    cursor: pointer;
    display: flex;
    gap: 8px;
    align-items: baseline;
    font-size: var(--font-body);
  }

  .voicing-item:hover {
    background: var(--bg-raised);
  }

  .voicing-item.selected {
    background: var(--primary-muted);
  }

  .v-chord {
    color: var(--text);
    font-weight: 700;
    min-width: 60px;
  }

  .v-recipe {
    color: var(--primary);
    min-width: 50px;
  }

  .v-intervals {
    color: var(--text-muted);
    font-size: var(--font-label);
  }

  .voicing-detail {
    flex: 1;
    padding: 16px 24px;
    overflow-y: auto;
  }

  .voicing-detail h2 {
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
    font-size: var(--font-body);
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

  .empty {
    color: var(--text-disabled);
  }
</style>
