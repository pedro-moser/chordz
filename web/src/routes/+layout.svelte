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
</style>
