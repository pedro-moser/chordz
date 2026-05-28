<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import Select from '$lib/components/Select.svelte';
  import Fretboard from '$lib/components/Fretboard.svelte';
  import PairDrawer from '$lib/components/PairDrawer.svelte';
  import { rootIndex, scaleIndex, pairIndex, showIntervals, drawerOpen, roots, scales, pairs, fretboardNotes, initStores } from '$lib/stores';
  import { resolvePair } from '$lib/wasm';

  onMount(() => {
    initStores();
  });

  let resolved = $derived(
    $scales.length > 0 ? resolvePair($rootIndex, $scaleIndex, $pairIndex) : { triadA: [], triadB: [] }
  );

  let scaleOptions = $derived(
    $scales.map((s, i) => ({ label: s.name, value: i }))
  );

  let rootOptions = $derived(
    $roots.map((r, i) => ({ label: r, value: i }))
  );

  const gmcTabs = [
    { label: 'Browse', href: '/gmc/browse' },
    { label: 'Tune', href: '/gmc/tune' },
  ];
</script>

<SubTabs tabs={gmcTabs} active="Browse" />

<div class="gmc-layout">
  <PairDrawer
    pairs={$pairs}
    selectedIndex={$pairIndex}
    rootPc={$rootIndex}
    scaleIndex={$scaleIndex}
    open={$drawerOpen}
    onselect={(i) => pairIndex.set(i)}
    ontoggle={() => drawerOpen.update(v => !v)}
  />

  <div class="gmc-main">
    <div class="gmc-controls">
      <Select label="Root" value={$rootIndex} options={rootOptions} onchange={(v) => rootIndex.set(v)} />
      <Select label="Scale" value={$scaleIndex} options={scaleOptions} onchange={(v) => scaleIndex.set(v)} />
      <label class="toggle">
        <input type="checkbox" bind:checked={$showIntervals} />
        Intervals
      </label>
      {#if !$drawerOpen}
        <button class="drawer-open-btn" onclick={() => drawerOpen.set(true)}>☰ Pairs</button>
      {/if}
    </div>

    <div class="gmc-heading">
      {$roots[$rootIndex] ?? ''} {$scales[$scaleIndex]?.name ?? ''} — {$pairs[$pairIndex]?.label ?? ''}
    </div>

    <div class="fretboard-container">
      <Fretboard
        notes={$fretboardNotes}
        triadA={resolved.triadA}
        triadB={resolved.triadB}
        rootPc={$rootIndex}
        showIntervals={$showIntervals}
      />
    </div>
  </div>
</div>

<style>
  .gmc-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .gmc-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 12px 16px;
    gap: 12px;
    overflow: auto;
  }

  .gmc-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .gmc-heading {
    font-size: var(--font-heading);
    color: var(--text);
  }

  .fretboard-container {
    flex: 1;
    display: flex;
    align-items: flex-start;
    overflow-x: auto;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-label);
    color: var(--text-muted);
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--primary);
  }

  .drawer-open-btn {
    background: var(--bg-raised);
    font-size: var(--font-label);
  }
</style>
