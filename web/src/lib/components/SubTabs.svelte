<script lang="ts">
  interface Tab {
    label: string;
    href: string;
  }

  interface Props {
    tabs: Tab[];
    active: string;
  }

  let { tabs, active }: Props = $props();
</script>

<div class="subtabs">
  {#each tabs as tab}
    <a href={tab.href} class="subtab" class:active={active === tab.label}>
      {tab.label}
    </a>
  {/each}
</div>

<style>
  .subtabs {
    height: var(--subtab-height);
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }

  .subtab {
    font-size: var(--font-body);
    color: var(--text-disabled);
    text-decoration: none;
    padding: 6px 0;
    border-bottom: 2px solid transparent;
    transition: all 150ms ease;
  }

  .subtab:hover {
    color: var(--text-muted);
  }

  .subtab.active {
    color: var(--primary);
    border-bottom-color: var(--primary);
  }
</style>
