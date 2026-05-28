<script lang="ts">
  interface Props {
    label: string;
    value: number;
    options: { label: string; value: number }[];
    onchange: (value: number) => void;
  }

  let { label, value, options, onchange }: Props = $props();

  const id = $derived(`select-${label.toLowerCase().replace(/\s+/g, '-')}`);
</script>

<div class="select-group">
  <label class="select-label" for={id}>{label}</label>
  <select {id} {value} onchange={(e) => onchange(Number(e.currentTarget.value))}>
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
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
