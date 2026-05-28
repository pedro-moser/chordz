<script lang="ts">
  import '../app.css';
  import Rail from '$lib/components/Rail.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { initWasm } from '$lib/wasm';

  let { children } = $props();
  let ready = $state(false);

  onMount(async () => {
    await initWasm();
    ready = true;
  });

  let activeWorld = $derived(
    $page.url.pathname.startsWith('/gmc') ? 'gmc' as const : 'chords' as const
  );
</script>

<div class="app-shell">
  <Rail active={activeWorld} />
  <main class="content">
    {#if ready}
      {@render children()}
    {:else}
      <div class="loading">Loading...</div>
    {/if}
    <footer class="attribution">
      <small>Guitar samples: tonejs-instruments (CC-BY 3.0).</small>
    </footer>
  </main>
</div>

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .attribution {
    flex-shrink: 0;
    padding: 2px 8px;
    text-align: right;
    color: var(--text-muted, #888);
    opacity: 0.6;
    font-size: 0.7rem;
    background: var(--bg, transparent);
  }
</style>
