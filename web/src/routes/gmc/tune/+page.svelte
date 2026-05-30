<script lang="ts">
  import { onMount } from 'svelte';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import { generateGmcLine, getPresets, getPairs, getAllScales } from '$lib/wasm';
  import { scheduleNotes, scheduleBassLine, stopScheduled, getAudioTime, setAmbience } from '$lib/audio';
  import { generateWalkingBass } from '$lib/wasm';
  import type { GmcLineResult, GmcLineEvent, GmcChordInfo, GmcPatternBlock, Preset, PairInfo, ScaleInfo } from '$lib/wasm';

  const gmcTabs = [
    { label: 'Browse', href: '/gmc/browse' },
    { label: 'Tune', href: '/gmc/tune' },
  ];

  const FIGURE_LABELS = ['Eighth', 'Sixteenth', 'Triplet'];
  const POSITION_LABELS = ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X', 'XI', 'XII'];
  const PATTERN_PRESETS: { label: string; blocks: GmcPatternBlock[] }[] = [
    { label: 'Alternating 3+3', blocks: [{ count: 3, direction: 'asc', triad: 'T1' }, { count: 3, direction: 'desc', triad: 'T2' }] },
    { label: 'Continuous up', blocks: [{ count: 3, direction: 'asc', triad: 'T1' }, { count: 3, direction: 'asc', triad: 'T2' }] },
    { label: 'Short-long', blocks: [{ count: 2, direction: 'asc', triad: 'T1' }, { count: 4, direction: 'desc', triad: 'T2' }] },
  ];

  const T1_COLOR = '#64a0ff';
  const T2_COLOR = '#ff8c32';

  // Per-block "voicing": how the three triad voices (role 0,1,2 = 1/3/5 of a stacked-thirds
  // pair) are ordered. A monotonic Run uses the anchor to pick its landing note; the numbered
  // options play an explicit role order (1·5·3 etc. — the way triad-pair études rotate a shape).
  const VOICINGS: { label: string; title: string; shape?: number[]; anchor?: 'root' | 'third' | 'fifth' }[] = [
    { label: 'Run', title: 'Monotonic run (nearest connect)' },
    { label: 'Run→1', title: 'Run, land on the root', anchor: 'root' },
    { label: 'Run→3', title: 'Run, land on the 3rd', anchor: 'third' },
    { label: 'Run→5', title: 'Run, land on the 5th', anchor: 'fifth' },
    { label: '1·3·5', title: 'Ascending arpeggio (root–3rd–5th)', shape: [0, 1, 2] },
    { label: '1·5·3', title: 'Root–5th–3rd', shape: [0, 2, 1] },
    { label: '3·1·5', title: '3rd–root–5th', shape: [1, 0, 2] },
    { label: '5·3·1', title: 'Descending arpeggio (5th–3rd–root)', shape: [2, 1, 0] },
  ];

  // Data loaded once
  let presets = $state<Preset[]>([]);
  let pairs = $state<PairInfo[]>([]);
  let scales = $state<ScaleInfo[]>([]);

  // Input state
  let titleInput = $state('Stella by Starlight');
  let chartInput = $state('Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11 | Bbmaj7 | Em7b5 A7b9 | Dm7 | Bbm7 Eb7 | Fmaj7 | Em7b5 | Ebmaj7 | D7b9 | G7b13 | % | Cm7 | % | Ab7#11 | % | Bbmaj7 | % | Em7b5 | A7b9 | Dm7b5 | G7b9 | Cm7b5 | F7b9 | Bbmaj7 | %');
  let pairIndex = $state(0);
  let figureIndex = $state(0);
  let positionFret = $state(5);
  let pattern = $state<GmcPatternBlock[]>([
    { count: 3, direction: 'asc', triad: 'T1' },
    { count: 3, direction: 'desc', triad: 'T2' },
  ]);
  let scaleOverrides = $state<(number | null)[]>([]);
  // The chart text the current overrides were keyed to. Overrides are positional, so
  // they must be cleared when the chart changes — even to a different progression with the
  // same number of changes (otherwise stale scales reapply to the wrong chords).
  let overridesFor = $state('');

  // Result state
  let result = $state<GmcLineResult | null>(null);
  let error = $state<string | null>(null);
  let selectedMeasure = $state(0);
  let playing = $state(false);
  let controlsOpen = $state(true);
  let scaleModalOpen = $state(false);
  let fbLabelMode = $state<'order' | 'notes' | 'intervals'>('order');

  const PC_NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B'];
  const INTERVAL_NAMES = ['1', 'b2', '2', 'b3', '3', '4', 'b5', '5', 'b6', '6', 'b7', '7'];

  function noteLabel(event: GmcLineEvent, chordRootPc: number): string {
    if (fbLabelMode === 'notes') return PC_NAMES[event.pitchClass] ?? '?';
    if (fbLabelMode === 'intervals') return INTERVAL_NAMES[(event.pitchClass - chordRootPc + 12) % 12] ?? '?';
    return '';
  }

  // Derived: measures with events grouped by chord change boundaries
  let measures = $derived((() => {
    if (!result?.events || !result?.changes) return [];
    const changes = result.changes;
    const events = result.events;
    const out: { chord: GmcChordInfo; events: GmcLineEvent[]; startBeat: number; endBeat: number; index: number }[] = [];
    let cumBeat = 0;
    for (let i = 0; i < changes.length; i++) {
      const c = changes[i];
      const start = cumBeat;
      const end = cumBeat + c.beats;
      const measureEvents = events.filter(e => e.beat >= start - 0.001 && e.beat < end - 0.001);
      out.push({ chord: c, events: measureEvents, startBeat: start, endBeat: end, index: i });
      cumBeat = end;
    }
    return out;
  })());

  let selectedMeasureData = $derived(measures[selectedMeasure] ?? null);

  // Fretboard range for selected measure
  let fretRange = $derived((() => {
    const coreStart = positionFret;
    const coreEnd = positionFret + 3;
    const stretchStart = Math.max(0, positionFret - 1);
    const stretchEnd = positionFret + 4;
    return { coreStart, coreEnd, stretchStart, stretchEnd };
  })());

  onMount(() => {
    presets = getPresets();
    pairs = getPairs();
    scales = getAllScales();
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
      stopPlay(); // silence any scheduled melody/bass when leaving the page
    };
  });

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    // While the scale modal is open, only Escape (to close) is handled — don't let
    // Space/arrows drive playback or navigation behind the overlay.
    if (scaleModalOpen) {
      if (e.key === 'Escape') { e.preventDefault(); scaleModalOpen = false; }
      return;
    }
    if (!measures.length) {
      if (e.key === 'Enter') { e.preventDefault(); generate(); }
      return;
    }
    switch (e.key) {
      case 'ArrowRight': case 'l':
        e.preventDefault();
        if (selectedMeasure < measures.length - 1) selectedMeasure++;
        break;
      case 'ArrowLeft': case 'h':
        e.preventDefault();
        if (selectedMeasure > 0) selectedMeasure--;
        break;
      case ' ':
        e.preventDefault();
        if (playing) stopPlay(); else playThrough();
        break;
      case 'Enter':
        e.preventDefault();
        generate();
        break;
    }
  }

  function generate() {
    const res = generateGmcLine(
      chartInput,
      titleInput,
      pairIndex,
      scaleOverrides,
      figureIndex,
      positionFret,
      pattern,
    );
    if (res.error) {
      error = res.error;
      result = null;
    } else {
      error = null;
      result = res;
      // (Re)initialize scale overrides (all null = use defaults) whenever the chart text
      // changed or the change count differs — keeps positional overrides from leaking
      // across edits to a same-length but different progression.
      if (chartInput !== overridesFor || scaleOverrides.length !== (res.changes?.length ?? 0)) {
        scaleOverrides = (res.changes ?? []).map(() => null);
        overridesFor = chartInput;
      }
      selectedMeasure = 0;
    }
  }

  function regenerate() {
    // Regenerate with current scale overrides
    const res = generateGmcLine(
      chartInput,
      titleInput,
      pairIndex,
      scaleOverrides,
      figureIndex,
      positionFret,
      pattern,
    );
    if (!res.error) {
      result = res;
    }
  }

  function selectPreset(idx: number) {
    if (idx < 0 || idx >= presets.length) return;
    titleInput = presets[idx].title;
    chartInput = presets[idx].chart;
    result = null;
    scaleOverrides = [];
    error = null;
  }

  function setScaleOverride(chordIdx: number, scaleIdx: number | null) {
    scaleOverrides[chordIdx] = scaleIdx;
    scaleOverrides = [...scaleOverrides];
    regenerate();
  }

  // Pattern editing
  function addBlock() {
    pattern = [...pattern, { count: 3, direction: 'asc', triad: 'T1' }];
  }

  function removeBlock(idx: number) {
    pattern = pattern.filter((_, i) => i !== idx);
  }

  function setBlockCount(idx: number, val: number) {
    pattern[idx] = { ...pattern[idx], count: Math.max(1, Math.min(6, val)) };
    pattern = [...pattern];
  }

  function toggleBlockDirection(idx: number) {
    pattern[idx] = { ...pattern[idx], direction: pattern[idx].direction === 'asc' ? 'desc' : 'asc' };
    pattern = [...pattern];
  }

  function toggleBlockTriad(idx: number) {
    pattern[idx] = { ...pattern[idx], triad: pattern[idx].triad === 'T1' ? 'T2' : 'T1' };
    pattern = [...pattern];
  }

  function voicingIndexOf(block: GmcPatternBlock): number {
    const sameArr = (a?: number[], b?: number[]) => (a ?? []).join(',') === (b ?? []).join(',');
    const idx = VOICINGS.findIndex(
      v => sameArr(v.shape, block.shape) && (v.anchor ?? undefined) === (block.anchor ?? undefined),
    );
    return idx >= 0 ? idx : 0;
  }

  function setBlockVoicing(idx: number, vIdx: number) {
    const v = VOICINGS[vIdx];
    pattern[idx] = { ...pattern[idx], shape: v.shape, anchor: v.anchor };
    pattern = [...pattern];
  }

  function selectPatternPreset(idx: number) {
    if (idx >= 0 && idx < PATTERN_PRESETS.length) {
      pattern = [...PATTERN_PRESETS[idx].blocks];
    }
  }

  // Playback
  let bpm = $state(120);
  let bassEnabled = $state(false);
  let ambient = $state(true);
  $effect(() => { setAmbience(ambient ? 0.35 : 0.0); });
  let rafId: number | null = null;

  function playThrough() {
    if (!result?.events || !measures.length || playing) return;
    playing = true;

    // Clamp: the BPM <input> min/max are only HTML hints, so a typed 0 / cleared field
    // would otherwise yield Infinity timings (and crash the WASM synth on a huge buffer).
    const safeBpm = Number.isFinite(bpm) && bpm > 0 ? Math.min(300, Math.max(40, bpm)) : 120;
    const beatSecs = 60 / safeBpm;
    const startBeat = measures[selectedMeasure]?.startBeat ?? 0;

    // Melody: each note's length is the spacing to the next event (from the baked
    // beats), so it stays correct even if figureIndex changed without regenerating.
    const evs = result.events.filter(e => e.beat >= startBeat - 0.001);
    const audioNotes = evs.map((e, i) => {
      // Note length = gap to the next event. For the final note there is no next event,
      // so reuse the previous gap (the grid is uniform) instead of a hardcoded eighth —
      // otherwise Sixteenth/Triplet figures over-sustain the last note.
      const span = i + 1 < evs.length
        ? evs[i + 1].beat - e.beat
        : (evs.length > 1 ? e.beat - evs[i - 1].beat : 0.5);
      return {
        midi: e.midi,
        time: (e.beat - startBeat) * beatSecs,
        duration: Math.max(0.05, span) * beatSecs,
      };
    });

    // Single audio-clock origin shared by melody + bass so they stay phase-locked.
    const startTime = getAudioTime();
    scheduleNotes(audioNotes, startTime);

    if (bassEnabled) {
      // Walking bass over every chord from the playback start to the end of the chart.
      // generateWalkingBass (Rust core) needs each chord's symbol (for the quality's
      // 3rd/5th/7th), its rootPc/bassPc, and the *next* chord's bass (for the chromatic
      // approach), so feed it the contiguous segment list.
      const segments = measures
        .filter(m => m.startBeat >= startBeat - 0.001)
        .map(m => ({ rootPc: m.chord.rootPc, bassPc: m.chord.bassPc, symbol: m.chord.chord, beats: m.chord.beats }));
      const bassNotes = generateWalkingBass(segments).map(n => ({
        midi: n.midi,
        time: n.beat * beatSecs,
        duration: n.beats * beatSecs,
      }));
      scheduleBassLine(bassNotes, startTime);
    }

    // Drive the measure highlight off the audio clock (no wall-clock drift, no dangling
    // timer promise on stop). Self-terminates at the end of the chart.
    const totalBeats = measures.reduce((sum, m) => sum + m.chord.beats, 0);
    const tick = () => {
      if (!playing) return;
      const elapsedBeats = startBeat + (getAudioTime() - startTime) / beatSecs;
      if (elapsedBeats >= totalBeats - 0.001) { stopPlay(); return; }
      for (let i = 0; i < measures.length; i++) {
        if (elapsedBeats < measures[i].endBeat - 0.001) { selectedMeasure = i; break; }
      }
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  }

  function stopPlay() {
    playing = false;
    if (rafId !== null) { cancelAnimationFrame(rafId); rafId = null; }
    stopScheduled();
  }

  // Scale options for dropdown (grouped by parent)
  let scaleOptions = $derived(
    scales.map((s, i) => ({ label: s.name, value: i, group: s.parent }))
  );

  // --- TAB SVG constants ---
  const TAB_STRING_GAP = 18;
  const TAB_MEASURE_WIDTH = 140;
  const TAB_MARGIN_LEFT = 10;
  const TAB_MARGIN_TOP = 28;
  const TAB_CHORD_Y = 12;
  const TAB_SCALE_Y_OFFSET = 16;
  const STRING_LABELS = ['e', 'B', 'G', 'D', 'A', 'E'];

  let tabSvgWidth = $derived(TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH + 20);
  let tabSvgHeight = $derived(TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET + 14);

  // --- FRETBOARD SVG constants ---
  const FB_STRING_GAP = 28;
  const FB_FRET_WIDTH = 56;
  const FB_MARGIN_LEFT = 30;
  const FB_MARGIN_TOP = 24;
  const FB_NOTE_RADIUS = 11;

  let fbFretCount = $derived(fretRange.stretchEnd - fretRange.stretchStart + 1);
  let fbSvgWidth = $derived(FB_MARGIN_LEFT + fbFretCount * FB_FRET_WIDTH + 20);
  let fbSvgHeight = $derived(FB_MARGIN_TOP + 5 * FB_STRING_GAP + 30);

  // Helper: get tab Y position for a string (engine string 0=low E, tab line 0=high e at top)
  function tabY(engineString: number): number {
    const tabLine = 5 - engineString;
    return TAB_MARGIN_TOP + tabLine * TAB_STRING_GAP;
  }

  // Helper: get tab X position for a note event within its measure
  function tabX(event: GmcLineEvent, measure: typeof measures[0]): number {
    const measureStart = TAB_MARGIN_LEFT + measure.index * TAB_MEASURE_WIDTH;
    const beatDuration = measure.chord.beats;
    const relBeat = event.beat - measure.startBeat;
    const fraction = relBeat / beatDuration;
    return measureStart + 12 + fraction * (TAB_MEASURE_WIDTH - 24);
  }

  // Helper: fretboard note position
  function fbNoteX(fret: number): number {
    const relFret = fret - fretRange.stretchStart;
    return FB_MARGIN_LEFT + relFret * FB_FRET_WIDTH + FB_FRET_WIDTH / 2;
  }

  function fbStringY(engineString: number): number {
    const displayString = 5 - engineString;
    return FB_MARGIN_TOP + displayString * FB_STRING_GAP;
  }

  // Scroll tab to keep selected measure visible
  let tabContainer: HTMLDivElement | undefined = $state(undefined);

  function scrollToMeasure(idx: number) {
    if (!tabContainer) return;
    const x = TAB_MARGIN_LEFT + idx * TAB_MEASURE_WIDTH;
    const containerWidth = tabContainer.clientWidth;
    const scrollLeft = x - containerWidth / 3;
    tabContainer.scrollTo({ left: Math.max(0, scrollLeft), behavior: 'smooth' });
  }

  // Watch selectedMeasure changes
  $effect(() => {
    if (measures.length > 0) {
      scrollToMeasure(selectedMeasure);
    }
  });
</script>

<SubTabs tabs={gmcTabs} active="Tune" />

<div class="tune-layout">
  <!-- Left panel: controls -->
  <div class="tune-left">
    <div class="tune-input">
      <div class="input-row row-inline">
        <label class="input-label" for="gmc-title">Title</label>
        <input id="gmc-title" bind:value={titleInput} placeholder="Tune name" />
        {#if presets.length > 0}
          <select class="preset-select" onchange={(e) => { selectPreset(parseInt((e.target as HTMLSelectElement).value)); (e.target as HTMLSelectElement).value = '-1'; }}>
            <option value="-1">Presets...</option>
            {#each presets as preset, i}
              <option value={i}>{preset.title}</option>
            {/each}
          </select>
        {/if}
      </div>
      <div class="input-row">
        <label class="input-label" for="gmc-chart">Chart</label>
        <textarea id="gmc-chart" bind:value={chartInput} rows="2" placeholder="Dm7 | G7 | Cmaj7 | Cmaj7"></textarea>
      </div>
    </div>

    <button class="toggle-btn" onclick={() => controlsOpen = !controlsOpen}>
      {controlsOpen ? '▾' : '▸'} Controls
    </button>

    {#if controlsOpen}
    <div class="controls-panel">
      <!-- Pair selector -->
      <div class="control-row">
        <span class="control-label">Pair</span>
        <select class="control-select" bind:value={pairIndex}>
          {#each pairs as p, i}
            <option value={i}>{p.label}</option>
          {/each}
        </select>
      </div>

      <!-- Figure selector -->
      <div class="control-row">
        <span class="control-label">Figure</span>
        <div class="btn-group">
          {#each FIGURE_LABELS as label, i}
            <button class="filter-btn" class:active={figureIndex === i} onclick={() => figureIndex = i}>{label}</button>
          {/each}
        </div>
      </div>

      <!-- Position selector -->
      <div class="control-row">
        <span class="control-label">Position</span>
        <div class="btn-group position-group">
          {#each POSITION_LABELS as label, i}
            <button class="filter-btn pos-btn" class:active={positionFret === i + 1} onclick={() => positionFret = i + 1}>{label}</button>
          {/each}
        </div>
      </div>

      <!-- Pattern builder -->
      <div class="control-row">
        <span class="control-label">Pattern</span>
        <select class="control-select" onchange={(e) => { selectPatternPreset(parseInt((e.target as HTMLSelectElement).value)); (e.target as HTMLSelectElement).value = '-1'; }}>
          <option value="-1">Presets...</option>
          {#each PATTERN_PRESETS as p, i}
            <option value={i}>{p.label}</option>
          {/each}
        </select>
      </div>
      <div class="pattern-blocks">
        {#each pattern as block, i}
          <div class="pattern-block">
            <input type="number" min="1" max="6" value={block.count} class="count-input" oninput={(e) => setBlockCount(i, parseInt((e.target as HTMLInputElement).value) || 1)} />
            <button class="dir-btn" onclick={() => toggleBlockDirection(i)} title={block.direction === 'asc' ? 'Ascending' : 'Descending'}>
              {block.direction === 'asc' ? '↑' : '↓'}
            </button>
            <button class="triad-btn" class:t1={block.triad === 'T1'} class:t2={block.triad === 'T2'} onclick={() => toggleBlockTriad(i)}>
              {block.triad}
            </button>
            <select
              class="voicing-select"
              title="Voice order"
              value={voicingIndexOf(block)}
              onchange={(e) => setBlockVoicing(i, parseInt((e.target as HTMLSelectElement).value))}
            >
              {#each VOICINGS as v, vi}
                <option value={vi} title={v.title}>{v.label}</option>
              {/each}
            </select>
            {#if pattern.length > 1}
              <button class="remove-btn" onclick={() => removeBlock(i)} title="Remove block">&times;</button>
            {/if}
          </div>
        {/each}
        <button class="add-block-btn" onclick={addBlock}>+ Block</button>
      </div>
    </div>
    {/if}

    <div class="btn-row">
      <button class="generate-btn" onclick={generate}>Generate (Enter)</button>
      {#if error}
        <span class="error">{error}</span>
      {/if}
    </div>

    {#if result?.changes}
      <button class="scales-btn" onclick={() => scaleModalOpen = true}>
        Scales {scaleOverrides.some(o => o !== null) ? '(edited)' : ''}
      </button>
    {/if}
  </div>

  <!-- Center + bottom: tab + fretboard -->
  <div class="tune-center">
    {#if measures.length > 0}
      <!-- Playback controls -->
      <div class="playback-bar">
        {#if playing}
          <button class="action-btn playing" onclick={stopPlay}>Stop</button>
        {:else}
          <button class="action-btn" onclick={playThrough}>Play</button>
        {/if}
        <span class="measure-counter">{selectedMeasure + 1}/{measures.length}</span>
        <div class="bpm-control">
          <label class="bpm-label" for="gmc-bpm">BPM</label>
          <input id="gmc-bpm" type="number" min="40" max="300" bind:value={bpm} class="bpm-input" />
        </div>
        <label class="toggle-label">
          <input type="checkbox" bind:checked={bassEnabled} /> Bass
        </label>
        <label class="toggle-label">
          <input type="checkbox" bind:checked={ambient} /> Ambient
        </label>
        <span class="keyboard-hint">←/→ navigate  Space play/pause</span>
      </div>

      <!-- Tab SVG -->
      <div class="tab-container" bind:this={tabContainer}>
        <svg
          width={tabSvgWidth}
          height={tabSvgHeight}
          class="tab-svg"
        >
          <!-- String lines -->
          {#each STRING_LABELS as label, si}
            <line
              x1={0}
              y1={TAB_MARGIN_TOP + si * TAB_STRING_GAP}
              x2={tabSvgWidth}
              y2={TAB_MARGIN_TOP + si * TAB_STRING_GAP}
              stroke="var(--border)"
              stroke-width="1"
            />
            <text
              x={3}
              y={TAB_MARGIN_TOP + si * TAB_STRING_GAP + 4}
              fill="var(--text-disabled)"
              font-size="9"
              font-family="var(--font)"
            >{label}</text>
          {/each}

          <!-- Measures -->
          {#each measures as measure, mi}
            {@const mx = TAB_MARGIN_LEFT + mi * TAB_MEASURE_WIDTH}

            <!-- Selected measure highlight -->
            {#if mi === selectedMeasure}
              <rect
                x={mx}
                y={TAB_CHORD_Y - 4}
                width={TAB_MEASURE_WIDTH}
                height={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET + 8 - TAB_CHORD_Y + 4}
                fill="var(--primary-muted)"
                opacity="0.25"
                rx="3"
              />
            {/if}

            <!-- Click target (keyboard nav handled globally via onKey) -->
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
            <rect
              x={mx}
              y={0}
              width={TAB_MEASURE_WIDTH}
              height={tabSvgHeight}
              fill="transparent"
              style="cursor: pointer"
              role="button"
              tabindex="-1"
              onclick={() => selectedMeasure = mi}
            />

            <!-- Bar line -->
            <line
              x1={mx}
              y1={TAB_MARGIN_TOP}
              x2={mx}
              y2={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP}
              stroke="var(--text-disabled)"
              stroke-width="1"
            />

            <!-- Chord name -->
            <text
              x={mx + TAB_MEASURE_WIDTH / 2}
              y={TAB_CHORD_Y}
              text-anchor="middle"
              fill="var(--text)"
              font-size="11"
              font-weight="700"
              font-family="var(--font)"
            >{measure.chord.chord}</text>

            <!-- Scale name below strings -->
            <text
              x={mx + TAB_MEASURE_WIDTH / 2}
              y={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET}
              text-anchor="middle"
              fill={measure.chord.isOverride ? '#ffc83c' : 'var(--text-disabled)'}
              font-size="9"
              font-family="var(--font)"
            >{measure.chord.activeScale}</text>

            <!-- Notes -->
            {#each measure.events as event}
              {@const x = tabX(event, measure)}
              {@const y = tabY(event.string)}
              {@const color = event.triad === 'T1' ? T1_COLOR : T2_COLOR}
              <rect
                x={x - 7}
                y={y - 7}
                width="14"
                height="14"
                rx="2"
                fill="var(--bg-base)"
                opacity="0.85"
              />
              <text
                {x}
                {y}
                text-anchor="middle"
                dominant-baseline="central"
                fill={color}
                font-size="11"
                font-weight="700"
                font-family="var(--font)"
              >{event.fret}</text>
            {/each}
          {/each}

          <!-- Final bar line -->
          {#if measures.length > 0}
            <line
              x1={TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH}
              y1={TAB_MARGIN_TOP}
              x2={TAB_MARGIN_LEFT + measures.length * TAB_MEASURE_WIDTH}
              y2={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP}
              stroke="var(--text-disabled)"
              stroke-width="2"
            />
          {/if}
        </svg>
      </div>

      <!-- Fretboard for selected measure -->
      {#if selectedMeasureData}
        <div class="fretboard-section">
          <div class="fb-header">
            <span class="fb-chord">{selectedMeasureData.chord.chord}</span>
            <span class="fb-scale" class:override={selectedMeasureData.chord.isOverride}>{selectedMeasureData.chord.activeScale}</span>
            <span class="fb-position">Position {POSITION_LABELS[positionFret - 1]}</span>
            <div class="fb-label-toggle">
              <button class="fb-toggle-btn" class:active={fbLabelMode === 'order'} onclick={() => fbLabelMode = 'order'}>#</button>
              <button class="fb-toggle-btn" class:active={fbLabelMode === 'notes'} onclick={() => fbLabelMode = 'notes'}>Notes</button>
              <button class="fb-toggle-btn" class:active={fbLabelMode === 'intervals'} onclick={() => fbLabelMode = 'intervals'}>Intervals</button>
            </div>
          </div>
          <div class="fb-container">
            <svg
              width={fbSvgWidth}
              height={fbSvgHeight}
              class="fb-svg"
            >
              <!-- Fret lines (vertical) -->
              {#each Array(fbFretCount + 1) as _, fi}
                {@const x = FB_MARGIN_LEFT + fi * FB_FRET_WIDTH}
                <line
                  x1={x}
                  y1={FB_MARGIN_TOP}
                  x2={x}
                  y2={FB_MARGIN_TOP + 5 * FB_STRING_GAP}
                  stroke="var(--border)"
                  stroke-width={fi === 0 ? 3 : 1}
                />
              {/each}

              <!-- Fret numbers -->
              {#each Array(fbFretCount) as _, fi}
                {@const fretNum = fretRange.stretchStart + fi}
                {@const x = FB_MARGIN_LEFT + fi * FB_FRET_WIDTH + FB_FRET_WIDTH / 2}
                {@const isCore = fretNum >= fretRange.coreStart && fretNum <= fretRange.coreEnd}
                <text
                  {x}
                  y={FB_MARGIN_TOP - 8}
                  text-anchor="middle"
                  fill={isCore ? 'var(--text-muted)' : 'var(--text-disabled)'}
                  font-size="10"
                  font-weight={isCore ? '700' : '400'}
                  font-family="var(--font)"
                >{fretNum}</text>

                <!-- Core position highlight -->
                {#if isCore}
                  <rect
                    x={FB_MARGIN_LEFT + fi * FB_FRET_WIDTH}
                    y={FB_MARGIN_TOP}
                    width={FB_FRET_WIDTH}
                    height={5 * FB_STRING_GAP}
                    fill="var(--primary-muted)"
                    opacity="0.12"
                  />
                {/if}
              {/each}

              <!-- String lines (horizontal) -->
              {#each STRING_LABELS as label, si}
                {@const y = FB_MARGIN_TOP + si * FB_STRING_GAP}
                <line
                  x1={FB_MARGIN_LEFT}
                  y1={y}
                  x2={FB_MARGIN_LEFT + fbFretCount * FB_FRET_WIDTH}
                  y2={y}
                  stroke="var(--text-disabled)"
                  stroke-width="1"
                />
                <text
                  x={FB_MARGIN_LEFT - 12}
                  y={y + 4}
                  text-anchor="middle"
                  fill="var(--text-disabled)"
                  font-size="9"
                  font-family="var(--font)"
                >{label}</text>
              {/each}

              <!-- Notes for selected measure -->
              {#each selectedMeasureData.events as event, ei}
                {@const x = fbNoteX(event.fret)}
                {@const y = fbStringY(event.string)}
                {@const color = event.triad === 'T1' ? T1_COLOR : T2_COLOR}
                {@const label = fbLabelMode === 'order' ? String(ei + 1) : noteLabel(event, selectedMeasureData.chord.rootPc)}
                <circle
                  cx={x}
                  cy={y}
                  r={FB_NOTE_RADIUS}
                  fill={color}
                  opacity="0.9"
                />
                <text
                  x={x}
                  y={y}
                  text-anchor="middle"
                  dominant-baseline="central"
                  fill="var(--bg-base)"
                  font-size={fbLabelMode === 'order' ? '10' : '8'}
                  font-weight="700"
                  font-family="var(--font)"
                >{label}</text>
              {/each}
            </svg>
          </div>
        </div>
      {/if}
    {:else}
      <p class="empty-hint">Enter a chart and press Generate</p>
    {/if}
  </div>
</div>

<!-- Scale overrides modal -->
{#if scaleModalOpen && result?.changes}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-backdrop"
    onclick={(e) => { if (e.target === e.currentTarget) scaleModalOpen = false; }}
    onkeydown={(e) => e.key === 'Escape' && (scaleModalOpen = false)}
  >
    <div class="modal">
      <div class="modal-header">
        <span class="modal-title">Scale per chord</span>
        <button class="modal-close" onclick={() => scaleModalOpen = false}>✕</button>
      </div>
      <div class="modal-body">
        {#each result.changes as change, i}
          <div class="override-row">
            <span class="override-chord">{change.chord}</span>
            <select
              class="override-select"
              class:overridden={scaleOverrides[i] !== null}
              value={scaleOverrides[i] ?? change.defaultScaleIndex ?? 0}
              onchange={(e) => {
                const val = parseInt((e.target as HTMLSelectElement).value);
                const isDefault = val === change.defaultScaleIndex;
                setScaleOverride(i, isDefault ? null : val);
              }}
            >
              {#each scales as s, si}
                <option value={si}>{s.name}{si === change.defaultScaleIndex ? ' (default)' : ''}</option>
              {/each}
            </select>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .tune-layout {
    flex: 1;
    display: flex;
    gap: 0;
    overflow: hidden;
  }

  /* --- Left panel --- */
  .tune-left {
    width: 280px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }

  .tune-input {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row-inline {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .row-inline input {
    flex: 1;
    min-width: 0;
  }

  .input-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    white-space: nowrap;
  }

  textarea {
    font-family: var(--font);
    font-size: var(--font-body);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 8px;
    resize: vertical;
  }

  textarea:focus {
    border-color: var(--primary);
    outline: none;
  }

  .preset-select {
    font-family: var(--font);
    font-size: var(--font-label);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 6px;
    flex-shrink: 0;
  }

  .toggle-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 10px;
    font-size: var(--font-label);
    text-align: left;
  }

  .toggle-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  /* Controls panel */
  .controls-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: var(--font-label);
  }

  .control-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .control-label {
    font-size: var(--font-label);
    color: var(--text-muted);
  }

  .control-select {
    font-size: var(--font-label);
    padding: 3px 6px;
  }

  .btn-group {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
  }

  .filter-btn {
    padding: 3px 8px;
    font-size: var(--font-label);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .filter-btn.active {
    background: var(--primary-muted);
    border-color: var(--primary);
    color: var(--text);
  }

  .position-group {
    gap: 2px;
  }

  .pos-btn {
    padding: 3px 5px;
    font-size: 10px;
    min-width: 0;
  }

  /* Pattern builder */
  .pattern-blocks {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pattern-block {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .count-input {
    width: 40px;
    padding: 2px 4px;
    font-size: var(--font-label);
    text-align: center;
  }

  .dir-btn {
    width: 26px;
    height: 26px;
    padding: 0;
    font-size: 14px;
    text-align: center;
    line-height: 26px;
  }

  .triad-btn {
    padding: 2px 8px;
    font-size: var(--font-label);
    font-weight: 700;
  }

  .voicing-select {
    font-size: 10px;
    padding: 2px 2px;
    max-width: 62px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-muted);
    border-radius: 4px;
  }

  .triad-btn.t1 {
    color: #64a0ff;
    border-color: #64a0ff44;
  }

  .triad-btn.t2 {
    color: #ff8c32;
    border-color: #ff8c3244;
  }

  .remove-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    font-size: 14px;
    color: var(--text-disabled);
    background: transparent;
    border: none;
    line-height: 22px;
  }

  .remove-btn:hover {
    color: #e66;
    background: transparent;
  }

  .add-block-btn {
    font-size: var(--font-label);
    padding: 3px 8px;
    color: var(--text-muted);
    background: var(--bg-surface);
    align-self: flex-start;
  }

  .btn-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .generate-btn {
    background: var(--primary-muted);
    color: var(--text);
    padding: 6px 16px;
    font-weight: 700;
  }

  .generate-btn:hover {
    background: var(--primary);
    color: #1a1a1a;
  }

  .error {
    color: #e66;
    font-size: var(--font-label);
  }

  /* Scales button */
  .scales-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 12px;
    font-size: var(--font-label);
    cursor: pointer;
    width: 100%;
  }

  .scales-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg-surface, var(--bg-raised));
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 400px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .modal-title {
    font-size: var(--font-heading);
    font-weight: 700;
    color: var(--text);
  }

  .modal-close {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
  }

  .modal-close:hover {
    color: var(--text);
  }

  .modal-body {
    padding: 12px 16px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .override-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .override-chord {
    font-size: var(--font-body);
    color: var(--text);
    min-width: 80px;
    font-weight: 700;
  }

  .override-select {
    font-size: var(--font-label);
    padding: 3px 6px;
    flex: 1;
    min-width: 0;
    color: var(--text-muted);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .override-select.overridden {
    color: #ffc83c;
    border-color: #ffc83c44;
  }

  /* --- Center panel --- */
  .tune-center {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    overflow: hidden;
    min-width: 0;
  }

  .playback-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .bpm-control {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .bpm-label {
    font-size: var(--font-label);
    color: var(--text-muted);
  }

  .bpm-input {
    width: 52px;
    padding: 3px 6px;
    font-family: var(--font);
    font-size: var(--font-label);
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
    text-align: center;
  }

  .toggle-label {
    font-size: var(--font-label);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .toggle-label input[type="checkbox"] {
    accent-color: var(--primary);
  }

  .action-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 12px;
    font-size: var(--font-label);
  }

  .action-btn:hover {
    background: var(--primary-muted);
    color: var(--text);
  }

  .action-btn.playing {
    background: var(--primary-muted);
    color: var(--text);
    border-color: var(--primary);
  }

  .measure-counter {
    font-size: var(--font-label);
    color: var(--text-muted);
    font-weight: 700;
  }

  .keyboard-hint {
    font-size: 10px;
    color: var(--text-disabled);
    margin-left: auto;
  }

  /* Tab container */
  .tab-container {
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 0;
  }

  .tab-svg {
    display: block;
  }

  /* Fretboard section */
  .fretboard-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }

  .fb-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .fb-label-toggle {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }

  .fb-toggle-btn {
    padding: 2px 8px;
    font-size: 11px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }

  .fb-toggle-btn.active {
    background: var(--primary-muted);
    border-color: var(--primary);
    color: var(--text);
  }

  .fb-chord {
    font-size: var(--font-heading);
    font-weight: 700;
    color: var(--text);
  }

  .fb-scale {
    font-size: var(--font-body);
    color: var(--text-disabled);
  }

  .fb-scale.override {
    color: #ffc83c;
  }

  .fb-position {
    font-size: var(--font-label);
    color: var(--text-muted);
    margin-left: auto;
  }

  .fb-container {
    overflow-x: auto;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px;
  }

  .fb-svg {
    display: block;
  }

  .empty-hint {
    color: var(--text-disabled);
    font-size: var(--font-label);
    padding-top: 24px;
  }
</style>
