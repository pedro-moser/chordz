# GMC Tune Mode — Single-Note Line Generator

Sub-mode within the existing GMC tab. Generates continuous melodic lines using triad pairs over a chord chart, output as tablature with fretboard visualization.

## User Flow

1. **Select chart** — same system as Tune mode (presets + manual input, existing parser)
2. **Scale per chord** — automatic defaults with click-to-override per chord
3. **Triad pair type** — global selector (T/T, T/7no5, etc.) — one type for the whole tune
4. **Rhythmic figure** — eighth note, sixteenth note, triplet
5. **Position** — neck position selector (I–XII), with strict/flexible toggle
6. **Pattern builder** — drag blocks to construct the melodic rule
7. **Generate** — engine runs, produces tab + fretboard output

## Scale Defaults

Automatic mapping from chord quality to scale:

| Quality | Default Scale |
|---------|---------------|
| maj7 | Ionian |
| maj7#11 | Lydian |
| m7, m9, m11, m13 | Dorian |
| m7b5 | Locrian |
| dom7, dom9, dom13 | Mixolydian |
| dom7#11 | Lydian Dominant |
| dom7b9, dom7#9, dom7alt | Altered |
| dom7b13 | Mixolydian b6 |
| dim7 | Diminished (HW) |
| m(maj7) | Melodic Minor |
| maj9, maj13 | Ionian |

**Fallback**: chord qualities not in this table default to the parent scale of their family (Major → Ionian, Minor → Dorian, Dominant → Mixolydian). If no match, Ionian.

User clicks on a chord to override with any scale from `Scale::ALL`.

## Pattern Builder

### Blocks

The user constructs a pattern by dragging blocks into a horizontal lane:

**Note block** (the core block):
- Note count: 1–6
- Direction: ↑ (ascending) / ↓ (descending)
- Triad: T1 / T2

Visual representation: `[3↑ T1]` `[2↓ T2]`

### Universal Rules (automatic, not blocks)

- **Connection between blocks**: next note is the nearest available note in the indicated triad, maintaining direction
- **Range inversion**: when the line hits the ceiling/floor of the position, direction reverses automatically
- **Chord change**: available notes change (new scale → new triads), connection by proximity, pattern does NOT restart

### Presets

- "Alternating 3+3": `[3↑ T1] [3↓ T2]`
- "Continuous up": `[3↑ T1] [3↑ T2]`
- "Short-long": `[2↑ T1] [4↓ T2]`

The pattern loops infinitely across the entire form.

## Engine

### Input (pre-processed)

- Parsed chart → list of chords with duration in beats
- Each chord with resolved scale → resolved triad pair (6 pitch classes: T1[3] + T2[3])
- Neck position → available notes per string (core 4 frets + stretch ±1)

### Starting Note

The first note is the lowest available note of T1 (from the first block's triad) within the core position. This provides a consistent, predictable starting point.

### Algorithm (note-by-note)

1. Read next block from pattern (e.g., "3↑ T1")
2. For each note in the block:
   - Get notes from the indicated triad that exist in the current position
   - Choose the nearest to the previous note in the indicated direction
   - If no note in that direction → invert (universal rule)
   - If flexible mode and no note fits the position → slide ±1–2 frets
3. Emit note event (string, fret, beat position, triad identity)
4. Advance time. If crossed a chord change, recalculate available triads
5. Continue with next block (or restart pattern from the beginning if exhausted)

### Position Logic

- **Strict**: 4-fret core (one finger per fret), stretch ±1 as exception only. Line stays in position.
- **Flexible**: prefers the chosen position, but slides to adjacent position when the new triad fits better there. Smooth movement, no jumps.

### Output

Vector of note events: `(beat, string, fret, triad_id)` — the full tab for the form.

## Output / UI

### Tablature

- Classic 6-line tab, horizontal scroll across the form
- Notes colored: color A for T1, color B for T2
- Bar lines with chord symbol above each chord
- Scale shown below the chord symbol (e.g., `Dm7 — Dorian`) — gray when default, highlighted when user-overridden

### Fretboard (measure by measure)

- Panoramic fretboard showing the selected measure
- Notes colored T1/T2, numbered in execution order
- Position indicator (core 4 frets highlighted)
- Clicking a measure in the tab updates the fretboard

### Controls

- **Click on measure**: select it, playback starts from there
- **Space**: play / pause
- **Pause**: returns to last clicked measure (or beginning if none clicked)
- **← / → (or h / l)**: navigate measure by measure when paused; updates fretboard and tab position

### Playback (native only)

- Cursor advances in tab, fretboard updates in sync
- Tempo based on BPM + chosen rhythmic figure

## Scope

This spec covers the **single-note (mono) engine only**. A polyphonic (chord) engine for triad pair voicings over charts is a separate future feature with different variables (spread vs closed, voice leading between shapes, register).
