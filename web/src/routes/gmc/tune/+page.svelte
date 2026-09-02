<script lang="ts">
  import { onMount } from 'svelte';
  import { base } from '$app/paths';
  import SubTabs from '$lib/components/SubTabs.svelte';
  import { generateGmcLine, shellEtudePreset, validScalesForChart, getPresets, getPairs, getAllScales } from '$lib/wasm';
  import { scheduleNotes, scheduleBassLine, stopScheduled, getAudioTime, setAmbience, getBassVolume, setBassVolume } from '$lib/audio';
  import { generateWalkingBass } from '$lib/wasm';
  import type { GmcLineResult, GmcLineEvent, GmcChordInfo, GmcPatternBlock, Preset, PairInfo, ScaleInfo } from '$lib/wasm';
  import { PATTERN_PRESETS } from '$lib/patternPresets';
  import {
    deleteGmcUserPreset,
    loadGmcUserPresets,
    resolvePatternPresetPairIndex,
    resolveScalePresetOverrides,
    saveGmcUserPreset,
    type GmcUserPresetLibrary,
    type GmcUserPresetKind,
  } from '$lib/gmcUserPresets';
  import {
    TAB_STRING_GAP,
    TAB_MEASURE_WIDTH,
    TAB_MARGIN_LEFT,
    TAB_MARGIN_TOP,
    TAB_CHORD_Y,
    TAB_SCALE_Y_OFFSET,
    STRING_LABELS,
    tabX,
    tabY,
    measureX,
    splitSystems,
    systemOf,
    SYSTEM_GAP,
  } from '$lib/tabLayout';
  import StaffNotation, { STAFF_BLOCK_HEIGHT } from '$lib/components/StaffNotation.svelte';
  import { GRIDS, type GridKind } from '$lib/notation';
  import { CONTOURS, contourPoints } from '$lib/contour';

  const gmcTabs = [
    { label: 'Browse', href: `${base}/gmc/browse` },
    { label: 'Tune', href: `${base}/gmc/tune` },
  ];

  const FIGURE_LABELS = ['Eighth', 'Sixteenth', 'Triplet'];
  const POSITION_LABELS = ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X', 'XI', 'XII'];

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

  // Per-block "connector": how this block's grip links to the next one (inter-grip movement on
  // the triad's inversion ladder). The within-grip note order stays the Voicing/direction above.
  type ConnectorValue = NonNullable<GmcPatternBlock['connector']>;
  const CONNECTORS: { value: ConnectorValue; label: string; title: string }[] = [
    { value: 'invertUp', label: '↑inv', title: 'Invert up — step one inversion up (the staircase)' },
    { value: 'invertDown', label: '↓inv', title: 'Invert down — step one inversion down' },
    { value: 'nearestUp', label: '↑run', title: 'Continue up — leap to the next grip past this one (continuous climb)' },
    { value: 'nearestDown', label: '↓run', title: 'Continue down' },
    { value: 'voiceLead', label: 'voice', title: 'Voice-lead — least hand movement, never repeats a note' },
    { value: 'random', label: 'rand', title: 'Random rung (generative variety)' },
  ];
  const DEFAULT_CONNECTOR: ConnectorValue = 'invertUp';

  // Data loaded once
  let presets = $state<Preset[]>([]);
  let pairs = $state<PairInfo[]>([]);
  let scales = $state<ScaleInfo[]>([]);
  let userPresets = $state<GmcUserPresetLibrary>({
    version: 1,
    harmonies: [],
    scales: [],
    patterns: [],
  });
  let selectedHarmonyPresetId = $state('');
  let selectedScalePresetId = $state('');
  let selectedPatternPresetId = $state('');
  let presetStatus = $state<{ text: string; error: boolean } | null>(null);

  // Input state
  let titleInput = $state('Stella by Starlight');
  let chartInput = $state('Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11 | Bbmaj7 | Em7b5 A7b9 | Dm7 | Bbm7 Eb7 | Fmaj7 | Em7b5 | Ebmaj7 | D7b9 | G7b13 | % | Cm7 | % | Ab7#11 | % | Bbmaj7 | % | Em7b5 | A7b9 | Dm7b5 | G7b9 | Cm7b5 | F7b9 | Bbmaj7 | %');
  let pairIndex = $state(0);
  let figureIndex = $state(0);
  // The figure a line was actually GENERATED with. `figureIndex` is a live control that can
  // change while a line is on screen (button clicks, preset selection) without regenerating —
  // the staff must keep rendering under the grid it was built for, not whatever is selected
  // now, or it silently misreads the existing events under the wrong grid. Set only on the
  // success path of generate()/regenerate(), never on error.
  let renderedFigureIndex = $state(0);
  // Selected neck positions (base frets 1-12). Empty = no restriction: the line may use the
  // whole neck. Defaults to a single position (V) to match the previous behavior.
  let selectedPositions = $state<number[]>([5]);
  let pattern = $state<GmcPatternBlock[]>([
    { count: 3, direction: 'asc', triad: 'T1', connector: DEFAULT_CONNECTOR },
    { count: 3, direction: 'desc', triad: 'T2', connector: DEFAULT_CONNECTOR },
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
  let fbLabelMode = $state<'notes' | 'intervals'>('notes');

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

  const GRID_KINDS: GridKind[] = ['eighth', 'sixteenth', 'triplet'];
  // Derived from renderedFigureIndex, not the live figureIndex control — the staff must draw
  // the grid the on-screen line was generated with, even if the Figure buttons have since been
  // clicked to something else without regenerating.
  let staffGrid = $derived(GRIDS[GRID_KINDS[renderedFigureIndex] ?? 'eighth']);

  const NUM_FRETS = 24;

  // Fretboard window for the selected measure's diagram.
  // - 1+ positions selected → the union of their stretch boxes (stable across measures), with
  //   each core box (`cores`) highlighted.
  // - Free (none selected) → fit the window to the notes actually played in this measure, since
  //   there's no fixed box to anchor on. No core highlight.
  let fretRange = $derived((() => {
    if (selectedPositions.length > 0) {
      const sorted = [...selectedPositions].sort((a, b) => a - b);
      const stretchStart = Math.max(0, sorted[0] - 1);
      const stretchEnd = Math.min(NUM_FRETS, sorted[sorted.length - 1] + 4);
      const cores = sorted.map(p => ({ start: p, end: p + 3 }));
      return { stretchStart, stretchEnd, cores };
    }
    const frets = (selectedMeasureData?.events ?? []).map(e => e.fret);
    if (frets.length === 0) return { stretchStart: 0, stretchEnd: 5, cores: [] };
    const lo = Math.max(0, Math.min(...frets) - 1);
    const hi = Math.min(NUM_FRETS, Math.max(...frets) + 1);
    return { stretchStart: lo, stretchEnd: Math.max(hi, lo + 4), cores: [] };
  })());

  function isCoreFret(fretNum: number): boolean {
    return fretRange.cores.some(c => fretNum >= c.start && fretNum <= c.end);
  }

  function togglePosition(fret: number) {
    selectedPositions = selectedPositions.includes(fret)
      ? selectedPositions.filter(p => p !== fret)
      : [...selectedPositions, fret];
    // Re-place the line for the new region right away (mirrors the scale-override flow), so the
    // diagram window and its notes never disagree.
    if (result) regenerate();
  }

  onMount(() => {
    presets = getPresets();
    pairs = getPairs();
    scales = getAllScales();
    userPresets = loadGmcUserPresets(window.localStorage);
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
    // Overrides are positional. If the chart changed, never generate once with the previous
    // chart's scales before the reset below; that can put a perfectly legal pair note in the
    // wrong harmonic context and was especially visible when two charts had equal length.
    const chartChanged = chartInput !== overridesFor;
    const generationOverrides = chartChanged ? [] : scaleOverrides;
    const res = generateGmcLine(chartInput, titleInput, pairIndex, generationOverrides, figureIndex, selectedPositions, pattern);
    if (res.error) {
      error = res.error;
      result = null;
    } else {
      error = null;
      result = res;
      renderedFigureIndex = figureIndex;
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
    const res = generateGmcLine(chartInput, titleInput, pairIndex, scaleOverrides, figureIndex, selectedPositions, pattern);
    if (!res.error) {
      result = res;
      renderedFigureIndex = figureIndex;
    }
  }

  function applyShellEtudePreset() {
    const p = shellEtudePreset(chartInput, titleInput);
    if (p.error) {
      error = p.error;
      return;
    }
    selectedPatternPresetId = '';
    selectedScalePresetId = '';
    if (p.pairIndex != null) pairIndex = p.pairIndex;
    if (p.pattern) pattern = p.pattern;
    if (p.scaleOverrides) {
      scaleOverrides = p.scaleOverrides;
      // Mark these overrides as belonging to the current chart so generate()'s positional
      // reset guard doesn't immediately wipe them.
      overridesFor = chartInput;
    }
    generate();
  }

  function applyScaleShuffle() {
    const r = validScalesForChart(chartInput, titleInput);
    if (r.error || !r.validScales) {
      if (r.error) error = r.error;
      return;
    }
    selectedScalePresetId = '';
    // Pick one valid scale at random per chord (empty list → leave default).
    scaleOverrides = r.validScales.map((list) =>
      list.length ? list[Math.floor(Math.random() * list.length)] : null,
    );
    // Keep generate()'s positional reset guard from wiping the shuffled overrides.
    overridesFor = chartInput;
    generate();
  }

  function selectPreset(idx: number) {
    if (idx < 0 || idx >= presets.length) return;
    titleInput = presets[idx].title;
    chartInput = presets[idx].chart;
    result = null;
    scaleOverrides = [];
    overridesFor = '';
    selectedHarmonyPresetId = '';
    selectedScalePresetId = '';
    error = null;
  }

  function clonePatternBlocks(blocks: GmcPatternBlock[]): GmcPatternBlock[] {
    return blocks.map((block) => ({
      ...block,
      shape: block.shape ? [...block.shape] : undefined,
      contour: block.contour ? [...block.contour] : undefined,
    }));
  }

  function askPresetName(message: string, suggestion: string): string | null {
    const entered = window.prompt(message, suggestion);
    if (entered === null) return null;
    const name = entered.trim();
    if (!name) {
      presetStatus = { text: 'Preset name cannot be empty.', error: true };
      return null;
    }
    return name;
  }

  function confirmPresetOverwrite(presets: readonly { name: string }[], name: string): boolean {
    const exists = presets.some((preset) => preset.name.toLocaleLowerCase() === name.toLocaleLowerCase());
    return !exists || window.confirm(`Replace the existing preset “${name}”?`);
  }

  function saveHarmonyUserPreset() {
    const name = askPresetName('Name this harmony preset', titleInput.trim() || 'Untitled harmony');
    if (!name || !confirmPresetOverwrite(userPresets.harmonies, name)) return;
    try {
      userPresets = saveGmcUserPreset(window.localStorage, 'harmony', {
        name,
        title: titleInput,
        chart: chartInput,
      });
      selectedHarmonyPresetId = userPresets.harmonies.find(
        (preset) => preset.name.toLocaleLowerCase() === name.toLocaleLowerCase(),
      )?.id ?? '';
      presetStatus = { text: `Harmony “${name}” saved.`, error: false };
    } catch (saveError) {
      presetStatus = { text: saveError instanceof Error ? saveError.message : 'Could not save harmony.', error: true };
    }
  }

  function saveScaleUserPreset() {
    if (!result?.changes || overridesFor !== chartInput || scaleOverrides.length !== result.changes.length) {
      presetStatus = { text: 'Generate the current harmony before saving its scale selection.', error: true };
      return;
    }
    const scaleRefs: ({ parent: string; degree: number } | null)[] = [];
    for (const index of scaleOverrides) {
      if (index === null) {
        scaleRefs.push(null);
        continue;
      }
      const scale = scales[index];
      if (!scale) {
        presetStatus = { text: 'One selected scale is no longer available.', error: true };
        return;
      }
      scaleRefs.push({ parent: scale.parent, degree: scale.degree });
    }
    const name = askPresetName('Name this scale-selection preset', `${titleInput.trim() || 'Untitled'} scales`);
    if (!name || !confirmPresetOverwrite(userPresets.scales, name)) return;
    try {
      userPresets = saveGmcUserPreset(window.localStorage, 'scales', {
        name,
        title: titleInput,
        chart: chartInput,
        chordSymbols: result.changes.map((change) => change.chord),
        scaleRefs,
      });
      selectedScalePresetId = userPresets.scales.find(
        (preset) => preset.name.toLocaleLowerCase() === name.toLocaleLowerCase(),
      )?.id ?? '';
      presetStatus = { text: `Scale selection “${name}” saved with its harmony.`, error: false };
    } catch (saveError) {
      presetStatus = { text: saveError instanceof Error ? saveError.message : 'Could not save scales.', error: true };
    }
  }

  function savePatternUserPreset() {
    const pair = pairs[pairIndex];
    if (!pair) {
      presetStatus = { text: 'The selected triad pair is no longer available.', error: true };
      return;
    }
    const summary = pattern.map((block) => block.count).join('+');
    const name = askPresetName('Name this pattern preset', `Pattern ${summary}`);
    if (!name || !confirmPresetOverwrite(userPresets.patterns, name)) return;
    try {
      userPresets = saveGmcUserPreset(window.localStorage, 'pattern', {
        name,
        pairLabel: pair.label,
        figureIndex,
        blocks: clonePatternBlocks(pattern),
      });
      selectedPatternPresetId = userPresets.patterns.find(
        (preset) => preset.name.toLocaleLowerCase() === name.toLocaleLowerCase(),
      )?.id ?? '';
      presetStatus = { text: `Pattern “${name}” saved with its pair and figure.`, error: false };
    } catch (saveError) {
      presetStatus = { text: saveError instanceof Error ? saveError.message : 'Could not save pattern.', error: true };
    }
  }

  function loadHarmonyUserPreset(id: string) {
    selectedHarmonyPresetId = id;
    const preset = userPresets.harmonies.find((candidate) => candidate.id === id);
    if (!preset) return;
    titleInput = preset.title;
    chartInput = preset.chart;
    result = null;
    scaleOverrides = [];
    overridesFor = '';
    selectedScalePresetId = '';
    error = null;
    presetStatus = { text: `Harmony “${preset.name}” loaded.`, error: false };
  }

  function loadScaleUserPreset(id: string) {
    const preset = userPresets.scales.find((candidate) => candidate.id === id);
    if (!preset) {
      selectedScalePresetId = '';
      return;
    }

    // Scale choices are positional. Parse the saved harmony once with defaults, then verify the
    // exact normalized chord sequence before applying any override. This prevents even one stale
    // render if the parser/catalog changes in a later app version.
    const parsed = generateGmcLine(
      preset.chart,
      preset.title,
      pairIndex,
      [],
      figureIndex,
      selectedPositions,
      pattern,
    );
    if (parsed.error || !parsed.changes) {
      selectedScalePresetId = '';
      presetStatus = { text: parsed.error ?? 'The saved harmony could not be parsed.', error: true };
      return;
    }
    const resolution = resolveScalePresetOverrides(
      preset,
      scales,
      parsed.changes.map((change) => change.chord),
    );
    if (!resolution.ok) {
      selectedScalePresetId = '';
      presetStatus = {
        text: resolution.reason === 'harmony-mismatch'
          ? 'The saved scale selection no longer matches its harmony.'
          : 'One scale in this preset is no longer available.',
        error: true,
      };
      return;
    }

    selectedScalePresetId = id;
    titleInput = preset.title;
    chartInput = preset.chart;
    scaleOverrides = resolution.overrides;
    overridesFor = preset.chart;
    selectedHarmonyPresetId = '';
    result = null;
    error = null;
    generate();
    if (!error) presetStatus = { text: `Scale selection “${preset.name}” loaded with its harmony.`, error: false };
  }

  function loadPatternUserPreset(id: string) {
    const preset = userPresets.patterns.find((candidate) => candidate.id === id);
    if (!preset) {
      selectedPatternPresetId = '';
      return;
    }
    const savedPairIndex = resolvePatternPresetPairIndex(preset, pairs);
    if (savedPairIndex === null) {
      selectedPatternPresetId = '';
      presetStatus = { text: 'The triad pair in this preset is no longer available.', error: true };
      return;
    }
    selectedPatternPresetId = id;
    pairIndex = savedPairIndex;
    figureIndex = preset.figureIndex;
    pattern = clonePatternBlocks(preset.blocks);
    presetStatus = { text: `Pattern “${preset.name}” loaded.`, error: false };
  }

  function deleteUserPreset(kind: GmcUserPresetKind, id: string) {
    if (!id) return;
    const preset = kind === 'harmony'
      ? userPresets.harmonies.find((candidate) => candidate.id === id)
      : kind === 'scales'
        ? userPresets.scales.find((candidate) => candidate.id === id)
        : userPresets.patterns.find((candidate) => candidate.id === id);
    if (!preset || !window.confirm(`Delete preset “${preset.name}”?`)) return;
    try {
      userPresets = deleteGmcUserPreset(window.localStorage, kind, id);
      if (kind === 'harmony') selectedHarmonyPresetId = '';
      else if (kind === 'scales') selectedScalePresetId = '';
      else selectedPatternPresetId = '';
      presetStatus = { text: `Preset “${preset.name}” deleted.`, error: false };
    } catch (deleteError) {
      presetStatus = { text: deleteError instanceof Error ? deleteError.message : 'Could not delete preset.', error: true };
    }
  }

  function setScaleOverride(chordIdx: number, scaleIdx: number | null) {
    selectedScalePresetId = '';
    scaleOverrides[chordIdx] = scaleIdx;
    scaleOverrides = [...scaleOverrides];
    regenerate();
  }

  // Pattern editing
  function addBlock() {
    pattern = [...pattern, { count: 3, direction: 'asc', triad: 'T1', connector: DEFAULT_CONNECTOR }];
    selectedPatternPresetId = '';
  }

  function removeBlock(idx: number) {
    pattern = pattern.filter((_, i) => i !== idx);
    selectedPatternPresetId = '';
  }

  function setBlockCount(idx: number, val: number) {
    const count = Math.max(1, Math.min(6, val));
    pattern[idx] = { ...pattern[idx], count, contour: count === 3 ? pattern[idx].contour : undefined };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  function toggleBlockDirection(idx: number) {
    pattern[idx] = { ...pattern[idx], direction: pattern[idx].direction === 'asc' ? 'desc' : 'asc' };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  function toggleBlockTriad(idx: number) {
    pattern[idx] = { ...pattern[idx], triad: pattern[idx].triad === 'T1' ? 'T2' : 'T1' };
    pattern = [...pattern];
    selectedPatternPresetId = '';
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
    selectedPatternPresetId = '';
  }

  function setBlockConnector(idx: number, value: ConnectorValue) {
    pattern[idx] = { ...pattern[idx], connector: value };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  // Contour: ordinal register shape for a 3-note cell (1 = lowest .. n = highest). Undefined
  // restores the plain directional walk. Mirrors setBlockConnector — a pattern-state edit that
  // the user applies with the Generate button, like every other per-block control.
  function setBlockContour(idx: number, ranks: number[] | undefined) {
    pattern[idx] = { ...pattern[idx], contour: ranks };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  function setBlockHold(idx: number, val: number) {
    pattern[idx] = { ...pattern[idx], holdLast: Math.max(0, Math.min(16, val || 0)) };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  function setBlockLeadRest(idx: number, val: number) {
    pattern[idx] = { ...pattern[idx], leadRest: Math.max(0, Math.min(16, val || 0)) };
    pattern = [...pattern];
    selectedPatternPresetId = '';
  }

  function selectPatternPreset(idx: number) {
    if (idx >= 0 && idx < PATTERN_PRESETS.length) {
      const p = PATTERN_PRESETS[idx];
      selectedPatternPresetId = '';
      pattern = clonePatternBlocks(p.blocks);
      if (p.figureIndex !== undefined) figureIndex = p.figureIndex;
      if (p.pairIndex !== undefined) pairIndex = p.pairIndex;
      if (p.resetScales) {
        selectedScalePresetId = '';
        scaleOverrides = [];
        overridesFor = chartInput;
      }
    }
  }

  // Playback
  let bpm = $state(120);
  let bassEnabled = $state(true);
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

    // Melody: use the baked per-event duration so held notes (holdLast) actually sustain.
    const evs = result.events.filter(e => e.beat >= startBeat - 0.001);
    const audioNotes = evs.map((e) => ({
      midi: e.midi,
      time: (e.beat - startBeat) * beatSecs,
      duration: Math.max(0.05, e.duration) * beatSecs,
    }));

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
  /**
   * Systems, not one endless strip. A sixteenth-note measure needs roughly 280px to engrave
   * and a 32-bar tune would run 9000px wide, showing four bars at a time. So the line wraps
   * to the container's width and stacks, and the container scrolls DOWN instead of across.
   */
  let containerWidth = $state(0);
  let systems = $derived(splitSystems(measures.length, containerWidth || 1200));
  /** Height of one system's tab block, below its staff. */
  const TAB_BLOCK_HEIGHT = TAB_MARGIN_TOP + 5 * TAB_STRING_GAP + TAB_SCALE_Y_OFFSET + 14;
  const SYSTEM_HEIGHT = STAFF_BLOCK_HEIGHT + TAB_BLOCK_HEIGHT + SYSTEM_GAP;
  /** Every system but the last is full width, so the first one sets the SVG width. */
  let systemWidth = $derived(
    TAB_MARGIN_LEFT + (systems[0]?.count ?? 1) * TAB_MEASURE_WIDTH + 20,
  );
  let tabSvgHeight = $derived(Math.max(1, systems.length) * SYSTEM_HEIGHT);

  // --- FRETBOARD SVG constants ---
  const FB_STRING_GAP = 28;
  const FB_FRET_WIDTH = 56;
  const FB_MARGIN_LEFT = 30;
  const FB_MARGIN_TOP = 24;
  const FB_NOTE_RADIUS = 11;

  let fbFretCount = $derived(fretRange.stretchEnd - fretRange.stretchStart + 1);
  let fbSvgWidth = $derived(FB_MARGIN_LEFT + fbFretCount * FB_FRET_WIDTH + 20);
  let fbSvgHeight = $derived(FB_MARGIN_TOP + 5 * FB_STRING_GAP + 30);

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

  /** Bring the SYSTEM holding a measure into view. Systems stack, so this scrolls down. */
  function scrollToMeasure(idx: number) {
    if (!tabContainer) return;
    const si = systemOf(systems, idx);
    const top = si * SYSTEM_HEIGHT - SYSTEM_HEIGHT / 3;
    tabContainer.scrollTo({ top: Math.max(0, top), behavior: 'smooth' });
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
        <input
          id="gmc-title"
          bind:value={titleInput}
          oninput={() => selectedHarmonyPresetId = ''}
          placeholder="Tune name"
        />
        {#if presets.length > 0}
          <select class="preset-select" onchange={(e) => { selectPreset(parseInt((e.target as HTMLSelectElement).value)); (e.target as HTMLSelectElement).value = '-1'; }}>
            <option value="-1">Built-in...</option>
            {#each presets as preset, i}
              <option value={i}>{preset.title}</option>
            {/each}
          </select>
        {/if}
      </div>
      <div class="input-row">
        <label class="input-label" for="gmc-chart">Chart</label>
        <textarea
          id="gmc-chart"
          bind:value={chartInput}
          oninput={() => { selectedHarmonyPresetId = ''; selectedScalePresetId = ''; }}
          rows="2"
          placeholder="Dm7 | G7 | Cmaj7 | Cmaj7"
        ></textarea>
      </div>
    </div>

    <div class="user-preset-row">
      <span class="user-preset-label">Harmony</span>
      <select
        class="user-preset-select"
        aria-label="Saved harmony presets"
        value={selectedHarmonyPresetId}
        onchange={(e) => loadHarmonyUserPreset((e.target as HTMLSelectElement).value)}
      >
        <option value="">Saved...</option>
        {#each userPresets.harmonies as preset}
          <option value={preset.id}>{preset.name}</option>
        {/each}
      </select>
      <button class="preset-action" aria-label="Save harmony preset" onclick={saveHarmonyUserPreset}>Save…</button>
      <button
        class="preset-action danger"
        aria-label="Delete selected harmony preset"
        disabled={!selectedHarmonyPresetId}
        onclick={() => deleteUserPreset('harmony', selectedHarmonyPresetId)}
      >Delete</button>
    </div>
    {#if presetStatus}
      <div class="preset-status" class:error={presetStatus.error} role={presetStatus.error ? 'alert' : 'status'}>
        {presetStatus.text}
      </div>
    {/if}

    <button class="toggle-btn" onclick={() => controlsOpen = !controlsOpen}>
      {controlsOpen ? '▾' : '▸'} Controls
    </button>

    {#if controlsOpen}
    <div class="controls-panel">
      <!-- Pair selector -->
      <div class="control-row">
        <span class="control-label">Pair</span>
        <button
          class="filter-btn"
          onclick={applyShellEtudePreset}
          title="Set pair 7no5/7no3 + characteristic scales + arc pattern (editable)"
        >Shell Étude</button>
        <select
          class="control-select"
          bind:value={pairIndex}
          onchange={() => selectedPatternPresetId = ''}
        >
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
            <button
              class="filter-btn"
              class:active={figureIndex === i}
              onclick={() => { figureIndex = i; selectedPatternPresetId = ''; }}
            >{label}</button>
          {/each}
        </div>
      </div>

      <!-- Position selector (multi-select; none = no restriction) -->
      <div class="control-row">
        <span class="control-label">
          Position
          {#if selectedPositions.length === 0}<span class="free-tag">— free (whole neck)</span>{/if}
        </span>
        <div class="btn-group position-group">
          {#each POSITION_LABELS as label, i}
            <button class="filter-btn pos-btn" class:active={selectedPositions.includes(i + 1)} onclick={() => togglePosition(i + 1)}>{label}</button>
          {/each}
        </div>
      </div>

      <!-- Pattern builder -->
      <div class="control-row">
        <span class="control-label">Pattern</span>
        <select class="control-select" onchange={(e) => { selectPatternPreset(parseInt((e.target as HTMLSelectElement).value)); (e.target as HTMLSelectElement).value = '-1'; }}>
          <option value="-1">Built-in...</option>
          {#each PATTERN_PRESETS as p, i}
            <option value={i}>{p.label}</option>
          {/each}
        </select>
        <div class="user-preset-row inset">
          <span class="user-preset-label">Pattern</span>
          <select
            class="user-preset-select"
            aria-label="Saved pattern presets"
            value={selectedPatternPresetId}
            onchange={(e) => loadPatternUserPreset((e.target as HTMLSelectElement).value)}
          >
            <option value="">Saved...</option>
            {#each userPresets.patterns as preset}
              <option value={preset.id}>{preset.name}</option>
            {/each}
          </select>
          <button class="preset-action" aria-label="Save pattern preset" onclick={savePatternUserPreset}>Save…</button>
          <button
            class="preset-action danger"
            aria-label="Delete selected pattern preset"
            disabled={!selectedPatternPresetId}
            onclick={() => deleteUserPreset('pattern', selectedPatternPresetId)}
          >Delete</button>
        </div>
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
            <select
              class="voicing-select"
              title="Connector — how this block links to the next grip"
              value={block.connector ?? 'voiceLead'}
              onchange={(e) => setBlockConnector(i, (e.target as HTMLSelectElement).value as ConnectorValue)}
            >
              {#each CONNECTORS as c}
                <option value={c.value} title={c.title}>{c.label}</option>
              {/each}
            </select>
            {#if block.count === 3}
              <div class="contour-row">
                <span class="contour-label">Contour</span>
                <button
                  class="filter-btn"
                  class:active={!block.contour}
                  aria-label="No contour — use the direction walk"
                  title="No contour — use the direction walk"
                  onclick={() => setBlockContour(i, undefined)}>—</button>
                {#each CONTOURS as c}
                  <button
                    class="filter-btn contour-btn"
                    class:active={block.contour?.join() === c.ranks.join()}
                    aria-label={`Contour ${c.ranks.join(' ')}`}
                    title={c.title}
                    onclick={() => setBlockContour(i, [...c.ranks])}>
                    <svg viewBox="0 0 30 20" width="30" height="20" aria-hidden="true">
                      <polyline
                        points={contourPoints(c.ranks, 30, 20).map((p) => `${p.x},${p.y}`).join(' ')}
                        fill="none" stroke="currentColor" stroke-width="2"
                        stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                  </button>
                {/each}
              </div>
            {/if}
            <input
              type="number" min="0" max="16" class="count-input"
              title="Hold the last note (extra grid steps)"
              value={block.holdLast ?? 0}
              oninput={(e) => setBlockHold(i, parseInt((e.target as HTMLInputElement).value))}
            />
            <input
              type="number" min="0" max="16" class="count-input"
              title="Pickup rest before this block (grid steps)"
              value={block.leadRest ?? 0}
              oninput={(e) => setBlockLeadRest(i, parseInt((e.target as HTMLInputElement).value))}
            />
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
      <div class="scale-actions">
        <button class="scales-btn" onclick={() => scaleModalOpen = true}>
          Scales {scaleOverrides.some(o => o !== null) ? '(edited)' : ''}
        </button>
        <button
          class="scales-btn"
          onclick={applyScaleShuffle}
          title="Sortear uma escala válida (3ª+7ª, 5ª se alterada) para cada acorde"
        >🎲 Cores</button>
      </div>
    {/if}
    <div class="user-preset-row">
      <span class="user-preset-label">Scales</span>
      <select
        class="user-preset-select"
        aria-label="Saved scale-selection presets"
        value={selectedScalePresetId}
        onchange={(e) => loadScaleUserPreset((e.target as HTMLSelectElement).value)}
      >
        <option value="">Saved...</option>
        {#each userPresets.scales as preset}
          <option value={preset.id}>{preset.name}</option>
        {/each}
      </select>
      <button
        class="preset-action"
        aria-label="Save scale-selection preset"
        disabled={!result?.changes}
        onclick={saveScaleUserPreset}
      >Save…</button>
      <button
        class="preset-action danger"
        aria-label="Delete selected scale-selection preset"
        disabled={!selectedScalePresetId}
        onclick={() => deleteUserPreset('scales', selectedScalePresetId)}
      >Delete</button>
    </div>
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
        {#if bassEnabled}
          <label class="bass-volume-control" for="gmc-bass-volume">Bass volume</label>
          <input
            id="gmc-bass-volume"
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={getBassVolume()}
            oninput={(e) => setBassVolume(Number((e.target as HTMLInputElement).value))}
            class="bass-vol"
            aria-label="Bass volume"
            title="Bass volume"
          />
        {/if}
        <label class="toggle-label">
          <input type="checkbox" bind:checked={ambient} /> Ambient
        </label>
        <span class="keyboard-hint">←/→ navigate  Space play/pause</span>
      </div>

      <!-- Tab SVG -->
      <div class="tab-container" bind:this={tabContainer} bind:clientWidth={containerWidth}>
        <svg width={systemWidth} height={tabSvgHeight} class="tab-svg">
          {#each systems as sys, si}
            <g transform="translate(0, {si * SYSTEM_HEIGHT})">
              <!--
                The staff's top line sits 34px down inside its 104px block, not at the very
                top. Stems reach about an octave beyond their notehead and beams sit at the
                stem tip, so a staff pinned near y=0 pushes the beams of high stem-up groups
                (and, on the sixteenth/triplet grids, flats and triplet brackets above the
                beam) to negative y, where the SVG clips them. Measured across all three
                rhythmic grids, 34 is the smallest offset that keeps every glyph at y >= 0.

                `measures` is the WHOLE line on purpose — layoutLine needs every measure to
                place a note held across a barline. The component draws only `sys`.
              -->
              <StaffNotation
                measures={measures}
                grid={staffGrid}
                system={sys}
                isLast={si === systems.length - 1}
                top={34}
                t1Color={T1_COLOR}
                t2Color={T2_COLOR}
              />

              <g transform="translate(0, {STAFF_BLOCK_HEIGHT})">
                <!-- String lines -->
                {#each STRING_LABELS as label, sti}
                  <line
                    x1={0}
                    y1={TAB_MARGIN_TOP + sti * TAB_STRING_GAP}
                    x2={measureX(sys.count)}
                    y2={TAB_MARGIN_TOP + sti * TAB_STRING_GAP}
                    stroke="var(--border)"
                    stroke-width="1"
                  />
                  <text
                    x={3}
                    y={TAB_MARGIN_TOP + sti * TAB_STRING_GAP + 4}
                    fill="var(--text-disabled)"
                    font-size="9"
                    font-family="var(--font)"
                  >{label}</text>
                {/each}

                <!-- This system's measures, redrawn at local indices -->
                {#each Array.from({ length: sys.count }) as _, li}
                  {@const mi = sys.first + li}
                  {@const measure = measures[mi]}
                  {@const local = { ...measure, index: li }}
                  {@const mx = measureX(li)}

                  <!-- Selected measure highlight -->
                  {#if mi === selectedMeasure}
                    <rect
                      x={mx}
                      y={TAB_CHORD_Y - 4 - STAFF_BLOCK_HEIGHT}
                      width={TAB_MEASURE_WIDTH}
                      height={TAB_BLOCK_HEIGHT + STAFF_BLOCK_HEIGHT - TAB_CHORD_Y + 4}
                      fill="var(--primary-muted)"
                      opacity="0.25"
                      rx="3"
                    />
                  {/if}

                  <!-- Click target (keyboard nav handled globally via onKey) -->
                  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
                  <rect
                    x={mx}
                    y={-STAFF_BLOCK_HEIGHT}
                    width={TAB_MEASURE_WIDTH}
                    height={STAFF_BLOCK_HEIGHT + TAB_BLOCK_HEIGHT}
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
                    {@const x = tabX(event, local)}
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

                <!-- Closing bar line for this system; heavy only on the last one -->
                <line
                  x1={measureX(sys.count)}
                  y1={TAB_MARGIN_TOP}
                  x2={measureX(sys.count)}
                  y2={TAB_MARGIN_TOP + 5 * TAB_STRING_GAP}
                  stroke="var(--text-disabled)"
                  stroke-width={si === systems.length - 1 ? 2 : 1}
                />
              </g>
            </g>
          {/each}
        </svg>
      </div>

      <!-- Fretboard for selected measure -->
      {#if selectedMeasureData}
        <div class="fretboard-section">
          <div class="fb-header">
            <span class="fb-chord">{selectedMeasureData.chord.chord}</span>
            <span class="fb-scale" class:override={selectedMeasureData.chord.isOverride}>{selectedMeasureData.chord.activeScale}</span>
            <span class="fb-position">{selectedPositions.length ? 'Position ' + [...selectedPositions].sort((a, b) => a - b).map(p => POSITION_LABELS[p - 1]).join(', ') : 'Free'}</span>
            <div class="fb-label-toggle">
              <button class="fb-toggle-btn" class:active={fbLabelMode === 'notes'} onclick={() => fbLabelMode = 'notes'} title="Nome da nota">Notes</button>
              <button class="fb-toggle-btn" class:active={fbLabelMode === 'intervals'} onclick={() => fbLabelMode = 'intervals'} title="Intervalo em relação à tônica do acorde">Intervals</button>
            </div>
          </div>
          <div class="fb-help">
            <span><b class="legend-t1">T1</b>/<b class="legend-t2">T2</b> = triades do par</span>
            <span>rótulo do ponto = <b>nota</b> ou intervalo</span>
            <span>número acima = <b>casa</b></span>
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
                {@const isCore = isCoreFret(fretNum)}
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
              {#each selectedMeasureData.events as event}
                {@const x = fbNoteX(event.fret)}
                {@const y = fbStringY(event.string)}
                {@const color = event.triad === 'T1' ? T1_COLOR : T2_COLOR}
                {@const label = noteLabel(event, selectedMeasureData.chord.rootPc)}
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
                  font-size="8"
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
    width: 440px;
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

  .user-preset-row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }

  .user-preset-row.inset {
    margin-top: 2px;
  }

  .user-preset-label {
    width: 52px;
    flex: 0 0 52px;
    color: var(--text-muted);
    font-size: 10px;
  }

  .user-preset-select {
    flex: 1;
    min-width: 0;
    padding: 3px 5px;
    font-family: var(--font);
    font-size: 10px;
    color: var(--text);
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .preset-action {
    flex: 0 0 auto;
    padding: 3px 6px;
    font-size: 10px;
    color: var(--text-muted);
    background: var(--bg-raised);
    border: 1px solid var(--border);
  }

  .preset-action:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--primary);
  }

  .preset-action.danger:hover:not(:disabled) {
    color: #e66;
    border-color: #e666;
  }

  .preset-action:disabled {
    cursor: default;
    opacity: 0.4;
  }

  .preset-status {
    padding: 3px 6px;
    color: var(--text-muted);
    font-size: 10px;
    background: var(--bg-raised);
    border-left: 2px solid var(--primary);
  }

  .preset-status.error {
    color: #e66;
    border-left-color: #e66;
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

  .free-tag {
    color: var(--text-disabled);
    font-style: italic;
  }

  /* Pattern builder */
  .pattern-blocks {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pattern-block {
    display: flex;
    flex-wrap: wrap;
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
  .scale-actions {
    display: flex;
    gap: 4px;
  }

  .scales-btn {
    flex: 1;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 4px 12px;
    font-size: var(--font-label);
    cursor: pointer;
    width: auto;
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

  .bass-volume-control {
    font-size: 10px;
    color: var(--text-disabled);
  }

  .bass-vol {
    width: 60px;
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
    /* Systems stack, so this scrolls down. Horizontal overflow would mean a system
       wider than the container, which splitSystems exists to prevent. */
    overflow-x: hidden;
    overflow-y: auto;
    max-height: 52vh;
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

  /* Contour picker (3-note cells only) */
  .contour-row {
    display: flex;
    flex: 1 0 100%;
    align-items: center;
    gap: 2px;
  }

  .contour-label {
    margin-right: 2px;
    color: var(--text-disabled);
    font-size: 10px;
  }

  .contour-btn {
    width: 34px;
    height: 24px;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .contour-btn svg {
    display: block;
  }

  .fb-help {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    padding: 2px 4px 5px;
    color: var(--text-disabled);
    font-size: 10px;
  }

  .legend-t1 {
    color: #64a0ff;
  }

  .legend-t2 {
    color: #ff8c32;
  }

  @media (max-width: 900px) {
    .tune-layout {
      flex-direction: column;
      overflow-y: auto;
    }

    .tune-left {
      width: auto;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }

    .tune-center {
      min-height: 0;
    }
  }
</style>
