<script lang="ts">
  import { base } from '$app/paths';

  interface Props {
    active: 'chords' | 'gmc';
  }

  let { active }: Props = $props();
</script>

<nav class="rail">
  <a href="{base}/chords/browse" class="rail-item" class:active={active === 'chords'}>
    <span class="rail-icon">♫</span>
    <span class="rail-label">Chords</span>
  </a>
  <a href="{base}/gmc/browse" class="rail-item" class:active={active === 'gmc'}>
    <span class="rail-icon">◆</span>
    <span class="rail-label">GMC</span>
  </a>
</nav>

<style>
  .rail {
    width: var(--rail-width);
    height: 100%;
    background: var(--bg-raised);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 16px;
    gap: 8px;
  }

  .rail-item {
    width: 44px;
    padding: 8px 4px;
    border-radius: 8px;
    text-align: center;
    text-decoration: none;
    color: var(--text-disabled);
    transition: all 150ms ease;
  }

  .rail-item:hover {
    background: var(--bg-surface);
    color: var(--text-muted);
  }

  .rail-item.active {
    background: var(--primary-muted);
    color: var(--primary);
  }

  .rail-icon {
    display: block;
    font-size: 18px;
    line-height: 1;
  }

  .rail-label {
    display: block;
    font-size: 9px;
    margin-top: 4px;
  }
</style>
