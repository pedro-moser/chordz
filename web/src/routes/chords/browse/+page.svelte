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

  interface VoicingGroup {
    chord: string;
    recipe: string;
    intervals: string[];
    entries: VoicingInfo[];
  }

  let roots = $state<string[]>([]);
  let families = $state<{index: number; name: string}[]>([]);
  let rootIndex = $state(0);
  let familyIndex = $state(0);
  let noteCount = $state(4);
  let groups = $state<VoicingGroup[]>([]);
  let selectedGroup = $state(0);
  let selectedPos = $state(0);

  onMount(() => {
    roots = getRoots();
    families = getFamilies();
    refresh();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  function refresh() {
    const flat = generateVoicings(rootIndex, familyIndex, noteCount);
    const map = new Map<string, VoicingGroup>();
    for (const v of flat) {
      const key = `${v.chord}|${v.recipe}|${(v.intervals ?? []).join(',')}`;
      if (map.has(key)) {
        map.get(key)!.entries.push(v);
      } else {
        map.set(key, {
          chord: v.chord,
          recipe: v.recipe,
          intervals: v.intervals ?? [],
          entries: [v],
        });
      }
    }
    groups = [...map.values()];
    selectedGroup = 0;
    selectedPos = 0;
  }

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    switch (e.key) {
      case 'j': case 'ArrowDown':
        e.preventDefault();
        if (selectedGroup < groups.length - 1) { selectedGroup++; selectedPos = 0; }
        break;
      case 'k': case 'ArrowUp':
        e.preventDefault();
        if (selectedGroup > 0) { selectedGroup--; selectedPos = 0; }
        break;
      case 'l': case 'ArrowRight':
        e.preventDefault();
        if (group && selectedPos < group.entries.length - 1) selectedPos++;
        break;
      case 'h': case 'ArrowLeft':
        e.preventDefault();
        if (selectedPos > 0) selectedPos--;
        break;
    }
  }

  let group = $derived(groups[selectedGroup] ?? null);
  let entry = $derived(group?.entries[selectedPos] ?? null);

  let rootOptions = $derived(roots.map((r, i) => ({ label: r, value: i })));
  let familyOptions = $derived(families.map((f) => ({ label: f.name, value: f.index })));
  let noteOptions = [
    { label: '3', value: 3 },
    { label: '4', value: 4 },
    { label: '5', value: 5 },
    { label: '6', value: 6 },
  ];
</script>

<SubTabs tabs={chordTabs} active="Browse" />

<div class="browser-layout">
  <div class="browser-controls">
    <Select label="Root" value={rootIndex} options={rootOptions} onchange={(v) => { rootIndex = v; refresh(); }} />
    <Select label="Family" value={familyIndex} options={familyOptions} onchange={(v) => { familyIndex = v; refresh(); }} />
    <Select label="Notes" value={noteCount} options={noteOptions} onchange={(v) => { noteCount = v; refresh(); }} />
    <span class="count">{groups.length} groups</span>
    <span class="hint">j/k navigate · h/l positions</span>
  </div>

  <div class="browser-body">
    <div class="voicing-list">
      {#each groups as g, i}
        <button
          class="voicing-item"
          class:selected={i === selectedGroup}
          onclick={() => { selectedGroup = i; selectedPos = 0; }}
        >
          <span class="v-chord">{g.chord}</span>
          <span class="v-recipe">{g.recipe}</span>
          <span class="v-intervals">{g.intervals.join(' ')}</span>
          {#if g.entries.length > 1}
            <span class="v-count">({g.entries.length})</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="voicing-detail">
      {#if entry && group}
        <h2>{group.chord} <span class="recipe-tag">{group.recipe}</span></h2>
        <p class="intervals-display">{group.intervals.join('  ')}</p>
        {#if group.entries.length > 1}
          <div class="pos-nav">
            <button class="pos-btn" disabled={selectedPos === 0} onclick={() => selectedPos--}>◀</button>
            <span class="pos-label">{selectedPos + 1} / {group.entries.length}</span>
            <button class="pos-btn" disabled={selectedPos >= group.entries.length - 1} onclick={() => selectedPos++}>▶</button>
          </div>
        {/if}
        <VoicingFretboard
          positions={entry.positions}
          notes={entry.notes}
          intervals={entry.intervals ?? []}
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

  .hint {
    font-size: var(--font-label);
    color: var(--text-disabled);
  }

  .browser-body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .voicing-list {
    width: 320px;
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

  .v-count {
    color: var(--text-disabled);
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
    margin-bottom: 12px;
  }

  .pos-nav {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .pos-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 2px 8px;
    font-size: var(--font-body);
    cursor: pointer;
  }

  .pos-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .pos-btn:not(:disabled):hover {
    background: var(--primary-muted);
  }

  .pos-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    min-width: 40px;
    text-align: center;
  }

  .empty {
    color: var(--text-disabled);
  }
</style>
