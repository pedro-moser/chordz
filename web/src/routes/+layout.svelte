<script lang="ts">
  import '../app.css';
  import Rail from '$lib/components/Rail.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { initWasm } from '$lib/wasm';
  import { attemptWasmBoot, type WasmBootState } from '$lib/wasmBoot';
  import { initGuitarAudio } from '$lib/audio';

  let { children } = $props();
  let bootState = $state<WasmBootState>({ status: 'loading' });

  async function initializeApp() {
    bootState = { status: 'loading' };
    const result = await attemptWasmBoot(initWasm);
    bootState = result;

    if (result.status === 'error') {
      console.error('Failed to initialize the WASM music engine', result.error);
      return;
    }

    try {
      initGuitarAudio(); // preload guitar samples + effects app-wide (all routes)
    } catch (error) {
      // Audio can still initialize on first playback; do not block the whole app.
      console.warn('Failed to preload guitar audio', error);
    }
  }

  onMount(() => {
    void initializeApp();
  });

  let activeWorld = $derived(
    $page.url.pathname.startsWith('/gmc') ? 'gmc' as const : 'chords' as const
  );
</script>

<div class="app-shell">
  <Rail active={activeWorld} />
  <main class="content">
    {#if bootState.status === 'ready'}
      {@render children()}
    {:else if bootState.status === 'error'}
      <section class="boot-state boot-error" role="alert" aria-live="assertive">
        <h1>Couldn't load Chordz</h1>
        <p>The music engine failed to start. Check your connection and try again.</p>
        <button type="button" onclick={initializeApp}>Retry</button>
      </section>
    {:else}
      <div class="boot-state loading" role="status" aria-live="polite">
        Loading music engine…
      </div>
    {/if}
    <footer class="attribution">
      <small>Guitar: Karoryfer Shinyguitar (CC0).</small>
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

  .boot-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    padding: 24px;
    text-align: center;
  }

  .loading {
    color: var(--text-muted);
  }

  .boot-error h1,
  .boot-error p {
    margin: 0;
  }

  .boot-error p {
    max-width: 36rem;
    color: var(--text-muted);
  }

  .boot-error button {
    margin-top: 4px;
    padding: 8px 16px;
    border: 1px solid var(--primary);
    border-radius: 6px;
    background: var(--primary);
    color: var(--bg-base);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .boot-error button:hover {
    filter: brightness(1.08);
  }

  .boot-error button:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 3px;
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
