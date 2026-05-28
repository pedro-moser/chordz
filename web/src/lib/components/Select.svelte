<script lang="ts">
  interface Props {
    label: string;
    value: number;
    options: { label: string; value: number; group?: string }[];
    onchange: (value: number) => void;
  }

  let { label, value, options, onchange }: Props = $props();

  const id = $derived(`select-${label.toLowerCase().replace(/\s+/g, '-')}`);

  let grouped = $derived((() => {
    const hasGroups = options.some(o => o.group);
    if (!hasGroups) return null;
    const groups: { name: string; items: typeof options }[] = [];
    let current: string | null = null;
    for (const opt of options) {
      if (opt.group !== current) {
        current = opt.group ?? '';
        groups.push({ name: current, items: [] });
      }
      groups[groups.length - 1].items.push(opt);
    }
    return groups;
  })());
</script>

<div class="select-group">
  <label class="select-label" for={id}>{label}</label>
  <select {id} {value} onchange={(e) => onchange(Number(e.currentTarget.value))}>
    {#if grouped}
      {#each grouped as group}
        <optgroup label={group.name}>
          {#each group.items as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </optgroup>
      {/each}
    {:else}
      {#each options as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    {/if}
  </select>
</div>

<style>
  .select-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .select-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    white-space: nowrap;
  }

  select {
    min-width: 120px;
  }
</style>
