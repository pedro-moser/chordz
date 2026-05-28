<script lang="ts">
  import { pairDisplay } from '$lib/wasm';
  import type { PairInfo } from '$lib/wasm';

  interface Props {
    pairs: PairInfo[];
    selectedIndex: number;
    rootPc: number;
    scaleIndex: number;
    open: boolean;
    onselect: (index: number) => void;
    ontoggle: () => void;
  }

  let { pairs, selectedIndex, rootPc, scaleIndex, open, onselect, ontoggle }: Props = $props();
</script>

<aside class="drawer" class:open>
  <div class="drawer-header">
    <span class="drawer-title">Pairs</span>
  </div>
  {#if open}
    <ul class="pair-list">
      {#each pairs as pair, i}
        <li>
          <button
            class="pair-item"
            class:selected={i === selectedIndex}
            onclick={() => onselect(i)}
          >
            <span class="pair-label">{pair.label}</span>
            <span class="pair-display">{pairDisplay(rootPc, scaleIndex, i)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .drawer {
    width: 0;
    overflow: hidden;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    transition: width 200ms ease;
    display: flex;
    flex-direction: column;
  }

  .drawer.open {
    width: 260px;
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .drawer-title {
    font-size: var(--font-label);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .drawer-toggle {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px 6px;
  }

  .pair-list {
    list-style: none;
    overflow-y: auto;
    flex: 1;
  }

  .pair-item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-radius: 0;
    padding: 6px 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pair-item:hover {
    background: var(--bg-raised);
  }

  .pair-item.selected {
    background: var(--primary-muted);
  }

  .pair-label {
    font-size: var(--font-body);
    color: var(--text);
    font-weight: 700;
  }

  .pair-display {
    font-size: var(--font-label);
    color: var(--text-muted);
  }
</style>
