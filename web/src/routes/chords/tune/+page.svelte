<script lang="ts">
  import SubTabs from '$lib/components/SubTabs.svelte';

  const chordTabs = [
    { label: 'Browse', href: '/chords/browse' },
    { label: 'Tune', href: '/chords/tune' },
  ];
</script>

<SubTabs tabs={chordTabs} active="Tune" />

<div style="padding: 24px; color: var(--text-muted);">
  Tune Mode — Phase 2
</div>
