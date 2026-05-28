<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';

  const gmcTabs = [
    { label: 'Browse', href: '/gmc/browse' },
    { label: 'Tune', href: '/gmc/tune' },
  ];
</script>

<SubTabs tabs={gmcTabs} active="Tune" />

<div style="padding: 24px; color: var(--text-muted);">
  Coming soon — Phase 2
</div>
