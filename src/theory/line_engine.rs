use crate::theory::chart::Chart;
use crate::theory::contour::resolve_cell;
use crate::theory::gmc::{self, TriadPairSet};
use crate::theory::line_pattern::{
    Connector, Direction, Pattern, PatternBlock, RhythmicFigure, Shape, TriadId,
};
use crate::theory::position::{FretNote, PositionSet};
use crate::theory::scale_defaults;
use crate::theory::scales::Scale;
use crate::theory::spelling::{self, Spelled};
use crate::theory::triad_shape::{inversion_ladder, TriadShape};
use crate::voicings::fretboard::Fretboard;

#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub beat: f32,
    pub string: u8,
    pub fret: u8,
    pub triad: TriadId,
    pub pitch_class: u8,
    pub midi: i32,
    /// Sounding length in beats. Uniform (= the figure's grid) unless `hold_last` extends it.
    pub duration: f32,
    /// Notation spelling: 0=C … 6=B.
    pub step: u8,
    /// Notation spelling: -2=bb … +2=##.
    pub alter: i8,
    /// Notation spelling: sounding octave, middle C is C4.
    pub octave: i8,
}

pub struct LineConfig {
    pub pattern: Pattern,
    pub figure: RhythmicFigure,
    /// The neck region(s) the line may use. Empty set = no position restriction.
    pub positions: PositionSet,
}

/// One triad's inversion ladder for a chord: grips sorted ascending by pitch, plus the triad's
/// role-ordered pitch classes (so `Shape::Order` / `Anchor` can target a specific voice).
struct TriadLadder {
    grips: Vec<TriadShape>,
    pcs: [u8; 3],
}

impl TriadLadder {
    /// The rung nearest a previous note's pitch (by grip centre) — voice-leads the hand into a
    /// new chord; the lowest rung when there is no previous note.
    fn nearest_rung(&self, to: Option<i32>) -> usize {
        match to {
            None => 0,
            Some(midi) => self
                .grips
                .iter()
                .enumerate()
                .min_by_key(|(_, g)| (grip_center(g) - midi).abs())
                .map(|(i, _)| i)
                .unwrap_or(0),
        }
    }

    /// The lowest rung whose bottom note is the given pitch class — lets `Anchor::Third` start the
    /// climb on a first-inversion grip, etc. Falls back to rung 0.
    fn rung_on_pc(&self, pc: u8) -> usize {
        self.grips
            .iter()
            .position(|g| g.notes[0].pitch_class == pc)
            .unwrap_or(0)
    }
}

/// A grip's mean pitch — its hand height for voice-leading comparisons.
fn grip_center(g: &TriadShape) -> i32 {
    (g.notes[0].midi + g.notes[1].midi + g.notes[2].midi) / 3
}

/// Build a triad's inversion ladder in the region. If no legal grip fits (a window too narrow
/// for any shape), fall back to a single pseudo-grip from the lowest in-region fretted notes so
/// the line still generates.
fn triad_ladder(fretboard: &Fretboard, positions: &PositionSet, pcs: [u8; 3]) -> TriadLadder {
    let grips = inversion_ladder(fretboard, positions, &pcs);
    if !grips.is_empty() {
        return TriadLadder { grips, pcs };
    }
    let notes = fallback_notes(positions, fretboard, &pcs);
    let grips = if notes.is_empty() {
        Vec::new()
    } else {
        let mut g = [
            notes[0],
            notes.get(1).copied().unwrap_or(notes[0]),
            notes.get(2).copied().unwrap_or(notes[0]),
        ];
        g.sort_by_key(|n| n.midi);
        vec![TriadShape { notes: g }]
    };
    TriadLadder { grips, pcs }
}

/// In-region notes with open strings removed — used only to build the degenerate fallback grip.
/// If even that is empty, allow open strings rather than produce no notes at all.
fn fallback_notes(positions: &PositionSet, fretboard: &Fretboard, pcs: &[u8; 3]) -> Vec<FretNote> {
    let fretted: Vec<FretNote> = positions
        .find_notes(fretboard, pcs)
        .into_iter()
        .filter(|n| n.fret >= 1)
        .collect();
    if fretted.is_empty() {
        positions.find_notes(fretboard, pcs)
    } else {
        fretted
    }
}

/// Float slack for beat-vs-chord-start comparisons (triplet grids accumulate f32 error).
const BEAT_EPS: f32 = 1e-4;

/// Ping-pong index into a `len`-element sequence: 0,1,…,len-1,len-2,…,1,0,1,… — how a block of
/// more than `len` notes folds back inside a single grip.
fn pingpong(k: usize, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let period = 2 * (len - 1);
    let m = k % period;
    if m < len {
        m
    } else {
        period - m
    }
}

/// The k-th fretboard note a block plays from one grip (0-based within the block).
/// `Monotonic` walks the grip by pitch in the block's direction (folding back via `pingpong`
/// if `count` > 3); `Order` plays the explicit cyclic role sequence.
fn note_at(grip: &TriadShape, pcs: &[u8; 3], block: &PatternBlock, k: usize) -> FretNote {
    match &block.shape {
        Shape::Order(order) => {
            let role = (order[k % order.len()] % 3) as usize;
            let pc = pcs[role];
            grip.notes
                .iter()
                .find(|n| n.pitch_class == pc)
                .copied()
                .unwrap_or(grip.notes[0])
        }
        Shape::Monotonic => {
            let mut base = grip.notes.to_vec(); // grip.notes is sorted ascending
            if block.direction == Direction::Descending {
                base.reverse();
            }
            base[pingpong(k, base.len())]
        }
    }
}

/// The note a rung would produce at block-position `cell_index` — the prediction both
/// `glue_rung` and the no-repeat probe need before they commit to a rung.
///
/// `cell_index` is the note's position within the whole block (both call sites pass the same
/// value for `cell_index` and `k`; `k` alone feeds the `note_at` fallback). It must be split into
/// a resolve-cell index and an intra-cell offset the SAME way the per-note cache in `run_pattern`
/// does (`cell_start = pos - pos % cell_len`, then `cell.get(pos - cell_start)`) — otherwise this
/// predicts the cell's position-0 note while the block actually plays a different offset,
/// picking a rung by a voice that is never sounded.
fn first_note_of(
    ladder: &TriadLadder,
    rung: usize,
    positions: &PositionSet,
    fretboard: &Fretboard,
    block: &PatternBlock,
    cell_index: usize,
    k: usize,
    prev_midi: Option<i32>,
) -> FretNote {
    if let Some(contour) = &block.contour {
        let cell_len = contour.len().max(1);
        let cell = resolve_cell(
            &ladder.grips[rung],
            &ladder.pcs,
            positions,
            fretboard,
            block,
            cell_index / cell_len,
            prev_midi,
        );
        if let Some(note) = cell.get(cell_index % cell_len) {
            return *note;
        }
    }
    note_at(&ladder.grips[rung], &ladder.pcs, block, k)
}

/// At a mid-block chord change, the rung of the NEW chord's ladder whose next note (the k-th
/// of the block) best glues to the previous pitch: chromatic steps (±1/±2 semitones) first,
/// then the smallest move; the same pitch only when there is no alternative.
fn glue_rung(
    ladder: &TriadLadder,
    block: &PatternBlock,
    k: usize,
    prev: i32,
    positions: &PositionSet,
    fretboard: &Fretboard,
) -> usize {
    let cost = |i: usize| {
        let midi = first_note_of(ladder, i, positions, fretboard, block, k, k, Some(prev)).midi;
        match (midi - prev).abs() {
            0 => (2, 0),
            d @ (1 | 2) => (0, d),
            d => (1, d),
        }
    };
    (0..ladder.grips.len())
        .min_by_key(|&i| cost(i))
        .unwrap_or(0)
}

/// Move the cursor to the next grip per the block's connector. Returns the new `(cursor, flip)`;
/// `flip` ping-pongs the directional connectors so the line bounces at the region's ends.
fn advance_cursor(
    grips: &[TriadShape],
    cursor: usize,
    flip: bool,
    connector: Connector,
    last_midi: Option<i32>,
    rng: &mut u64,
) -> (usize, bool) {
    let len = grips.len();
    if len <= 1 {
        return (cursor, flip);
    }
    let lowest = |i: usize| grips[i].notes[0].midi;
    let highest = |i: usize| grips[i].notes[2].midi;

    match connector {
        Connector::VoiceLead => {
            // Stay near the previous note; nudge a rung if that would land on the same grip.
            let anchor = last_midi.unwrap_or_else(|| grip_center(&grips[cursor]));
            let near = grips
                .iter()
                .enumerate()
                .min_by_key(|(_, g)| (grip_center(g) - anchor).abs())
                .map(|(i, _)| i)
                .unwrap_or(cursor);
            let next = if near == cursor {
                (cursor + 1).min(len - 1)
            } else {
                near
            };
            (next, flip)
        }
        Connector::Random => {
            *rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (((*rng >> 33) as usize) % len, flip)
        }
        // Directional connectors: `invert` steps one rung; `nearest` leaps to the next grip clear
        // of the current one's range. `flip` reverses at the region edge (bounce).
        Connector::InvertUp
        | Connector::InvertDown
        | Connector::NearestUp
        | Connector::NearestDown => {
            let base_up = matches!(connector, Connector::InvertUp | Connector::NearestUp);
            let invert = matches!(connector, Connector::InvertUp | Connector::InvertDown);
            let step = |up: bool| -> Option<usize> {
                if invert {
                    if up {
                        (cursor + 1 < len).then_some(cursor + 1)
                    } else {
                        cursor.checked_sub(1)
                    }
                } else if up {
                    (cursor + 1..len).find(|&j| lowest(j) > highest(cursor))
                } else {
                    (0..cursor).rev().find(|&j| highest(j) < lowest(cursor))
                }
            };
            match step(base_up ^ flip) {
                Some(j) => (j, flip),
                None => {
                    let nf = !flip;
                    (step(base_up ^ nf).unwrap_or(cursor), nf)
                }
            }
        }
    }
}

pub fn generate_line(
    chart: &Chart,
    scale_overrides: &[Option<usize>],
    fretboard: &Fretboard,
    pair: &TriadPairSet,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    // Per chord, build each triad's inversion ladder (the rungs a connector steps through)
    // and the spelling table notation reads the line through, then walk the pattern.
    let mut ladders: Vec<[TriadLadder; 2]> = Vec::with_capacity(chart.changes.len());
    let mut spellings: Vec<[Option<Spelled>; 12]> = Vec::with_capacity(chart.changes.len());
    for (i, change) in chart.changes.iter().enumerate() {
        let scale = scale_overrides
            .get(i)
            .and_then(|opt| opt.and_then(|idx| Scale::ALL.get(idx)))
            .unwrap_or_else(|| scale_defaults::default_scale(change.quality));
        let (pcs_a, pcs_b) = gmc::resolve_pair(change.root_pc, scale, pair);
        ladders.push([
            triad_ladder(fretboard, &config.positions, pcs_a),
            triad_ladder(fretboard, &config.positions, pcs_b),
        ]);
        spellings.push(spelling::spell_scale(&change.root, change.quality, scale));
    }
    run_pattern(chart, config, fretboard, &ladders, &spellings)
}

/// Walk the pattern over the per-chord inversion ladders. Each block plays one grip (so the
/// `1+1+1`/`2+1`/`1+2`, no-open-string guarantee holds); its connector chooses the next grip,
/// and the hand voice-leads across chord changes. Timing (grid slots, holds, pickups) is the
/// same integer-slot scheme as before.
fn run_pattern(
    chart: &Chart,
    config: &LineConfig,
    fretboard: &Fretboard,
    ladders: &[[TriadLadder; 2]],
    spellings: &[[Option<Spelled>; 12]],
) -> Vec<NoteEvent> {
    let beat_dur = config.figure.beat_duration();
    let total_events = (chart.total_beats() / beat_dur).round() as usize;

    // Chord start beats, for mapping a slot to its chord.
    let mut chord_starts: Vec<f32> = Vec::with_capacity(chart.changes.len());
    let mut cumulative = 0.0_f32;
    for change in &chart.changes {
        chord_starts.push(cumulative);
        cumulative += change.beats;
    }

    let mut events = Vec::with_capacity(total_events);
    let mut pattern_iter = config.pattern.iter();

    // Per-triad walk state: cursor (rung in the current chord's ladder), the chord that cursor
    // belongs to, and the ping-pong flip for bouncing at the region edges.
    let mut cursor = [0usize; 2];
    let mut cursor_chord = [usize::MAX; 2];
    let mut flip = [false; 2];
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15; // deterministic seed for the Random connector
    let mut last_midi: Option<i32> = None;
    let mut slots = 0usize;

    'blocks: loop {
        let block = match pattern_iter.next() {
            Some(b) => b,
            None => break, // empty pattern (guarded upstream)
        };
        slots += block.lead_rest as usize; // pickup rest
        if slots >= total_events {
            break;
        }

        let ti = match block.triad {
            TriadId::T1 => 0,
            TriadId::T2 => 1,
        };
        let beat = slots as f32 * beat_dur;
        let chord_idx = chord_starts
            .iter()
            .rposition(|&s| beat >= s - BEAT_EPS)
            .unwrap_or(0);
        let ladder = &ladders[chord_idx][ti];

        if ladder.grips.is_empty() {
            slots += block.count as usize; // no notes available — silent block
            if slots >= total_events {
                break;
            }
            continue;
        }

        // New chord for this triad: voice-lead the cursor to the nearest rung, or honour an
        // explicit anchor (Run→1/3/5 chooses the starting inversion).
        if cursor_chord[ti] != chord_idx {
            cursor[ti] = match block.anchor.role() {
                Some(r) => ladder.rung_on_pc(ladder.pcs[r]),
                None => ladder.nearest_rung(last_midi),
            };
            cursor_chord[ti] = chord_idx;
        }

        // No-repeat: never start a block on the previous pitch. Nudge rung-by-rung until the
        // start differs (consecutive grips can share the role's pitch, so one nudge isn't always
        // enough); give up after a full lap rather than loop forever.
        if let Some(prev) = last_midi {
            let len = ladder.grips.len();
            let mut tries = 0;
            while len > 1
                && tries < len
                && first_note_of(
                    ladder,
                    cursor[ti],
                    &config.positions,
                    fretboard,
                    block,
                    0,
                    0,
                    last_midi,
                )
                .midi
                    == prev
            {
                cursor[ti] = (cursor[ti] + 1) % len;
                tries += 1;
            }
        }

        let count = block.count as usize;
        let mut active_chord = chord_idx;
        let mut silenced = false;
        // Contour blocks resolve a whole cell at once (note k depends on notes 0..k of its
        // cell), so cache the current cell and serve it by offset. `None` keeps the legacy
        // per-note path untouched.
        let cell_len = block.contour.as_ref().map(|c| c.len()).unwrap_or(0);
        let mut cell: Vec<FretNote> = Vec::new();
        // `usize::MAX` means "no cell resolved yet". Emptiness cannot carry that meaning:
        // `resolve_cell` legitimately returns an empty vector when the contour is not
        // expressible in this region, and that answer must not trigger a re-resolve per note.
        let mut cell_start = usize::MAX;
        // Set when a mid-block chord change moves the cursor onto a different grip while `k`
        // is still inside the cached cell's span. The `k`-vs-`cell_start` check alone can't see
        // this (the span hasn't been exhausted), so it needs its own signal to force a re-resolve
        // against the new grip rather than silently serving notes computed from the old one.
        let mut cell_stale = false;
        for k in 0..count {
            if slots >= total_events {
                break 'blocks;
            }
            let beat_now = slots as f32 * beat_dur;
            // Re-resolve mid-block chord changes: the rest of the block plays the NEW chord's
            // triad, entered from the last sounded pitch by chromatic glue.
            let chord_now = chord_starts
                .iter()
                .rposition(|&s| beat_now >= s - BEAT_EPS)
                .unwrap_or(0);
            if chord_now != active_chord {
                let new_ladder = &ladders[chord_now][ti];
                if new_ladder.grips.is_empty() {
                    slots += count - k; // no notes available — the rest of the block is silent
                    silenced = true;
                    break;
                }
                cursor[ti] = match last_midi {
                    Some(prev) => {
                        glue_rung(new_ladder, block, k, prev, &config.positions, fretboard)
                    }
                    None => new_ladder.nearest_rung(None),
                };
                active_chord = chord_now;
                cursor_chord[ti] = chord_now;
                // The cached cell was resolved against the OLD chord's grip; a mid-block glue
                // just moved the cursor onto a different grip (possibly a different chord's
                // ladder entirely), so the cache is stale regardless of where k sits relative to
                // cell_start. Force a fresh resolve on the next note.
                cell_stale = true;
            }
            let ladder = &ladders[active_chord][ti];
            let note = if cell_len > 0 {
                if cell_start == usize::MAX || k >= cell_start + cell_len || cell_stale {
                    // Re-align to the cell's own boundary (a multiple of cell_len from the
                    // block's start) rather than to `k` itself: a stale-cache re-resolve can
                    // land mid-cell (k not a boundary), and resolving from `k` would shift which
                    // rank of the contour each remaining note gets.
                    cell_start = k - (k % cell_len);
                    cell = resolve_cell(
                        &ladder.grips[cursor[ti]],
                        &ladder.pcs,
                        &config.positions,
                        fretboard,
                        block,
                        cell_start / cell_len,
                        last_midi,
                    );
                    cell_stale = false;
                }
                // An empty cell means the contour is not expressible in this region — the
                // resolver refuses rather than inventing an out-of-region note. Fall back to
                // the legacy per-note walk, which always yields a note from the current grip.
                cell.get(k - cell_start)
                    .copied()
                    .unwrap_or_else(|| note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, k))
            } else {
                note_at(&ladder.grips[cursor[ti]], &ladder.pcs, block, k)
            };
            // The block's last note sustains `1 + hold_last` grid slots; others take one.
            let hold = if k + 1 == count {
                block.hold_last as usize
            } else {
                0
            };
            let step_slots = 1 + hold;
            let spelled = spelling::spell_midi(&spellings[active_chord], note.midi);
            events.push(NoteEvent {
                beat: beat_now,
                string: note.string,
                fret: note.fret,
                triad: block.triad,
                pitch_class: note.pitch_class,
                midi: note.midi,
                duration: step_slots as f32 * beat_dur,
                step: spelled.step,
                alter: spelled.alter,
                octave: spelled.octave,
            });
            last_midi = Some(note.midi);
            slots += step_slots;
        }
        if silenced {
            if slots >= total_events {
                break;
            }
            continue;
        }

        // Choose the next grip for this triad via the block's connector — on the ladder of the
        // chord the block ENDED in (it may have glued into a new one mid-block).
        let ladder = &ladders[active_chord][ti];
        let (nc, nf) = advance_cursor(
            &ladder.grips,
            cursor[ti],
            flip[ti],
            block.connector,
            last_midi,
            &mut rng,
        );
        cursor[ti] = nc;
        flip[ti] = nf;
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chart::Chart;
    use crate::theory::gmc::PAIRS;
    use crate::theory::line_pattern::{
        Anchor, Connector, Direction, Pattern, PatternBlock, RhythmicFigure, Shape, TriadId,
    };
    use crate::theory::position::{NeckPosition, PositionSet};
    use crate::voicings::fretboard::Fretboard;

    /// A one-block pattern with an explicit shape + anchor, for the Fase-1 tests.
    fn shaped_config(count: u8, triad: TriadId, shape: Shape, anchor: Anchor) -> LineConfig {
        LineConfig {
            pattern: Pattern {
                name: "test",
                blocks: vec![PatternBlock {
                    count,
                    direction: Direction::Ascending,
                    triad,
                    shape,
                    anchor,
                    hold_last: 0,
                    lead_rest: 0,
                    connector: Connector::default(),
                    contour: None,
                }],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        }
    }

    // T1 of Dm7 with the default Dorian scale and the T/T pair (PAIRS[0]) resolves to the
    // role-ordered pitch classes [E=4, G=7, B=11] (role 0, 1, 2).
    #[test]
    fn order_shape_plays_voices_in_the_given_role_sequence() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = shaped_config(6, TriadId::T1, Shape::Order(vec![0, 1, 2]), Anchor::Nearest);
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let pcs: Vec<u8> = events.iter().take(6).map(|e| e.pitch_class).collect();
        assert_eq!(pcs, vec![4, 7, 11, 4, 7, 11]);
    }

    #[test]
    fn order_shape_respects_a_rotated_one_five_three_sequence() {
        // 1-5-3 = roles [0, 2, 1] -> E, B, G.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = shaped_config(3, TriadId::T1, Shape::Order(vec![0, 2, 1]), Anchor::Nearest);
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let pcs: Vec<u8> = events.iter().take(3).map(|e| e.pitch_class).collect();
        assert_eq!(pcs, vec![4, 11, 7]);
    }

    #[test]
    fn order_blocks_never_repeat_a_pitch() {
        // `nearest_of_pc` (used by Order shapes + anchors) must skip the current pitch, like
        // `find_closest` does — otherwise a targeted voice whose nearest note IS the just-played
        // note repeats it. Reproduces the "D3 D3" the arch preset produced over a held Cm7.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cm7 | % |").unwrap();
        let arch = vec![
            PatternBlock {
                count: 3,
                direction: Direction::Ascending,
                triad: TriadId::T1,
                shape: Shape::Order(vec![0, 1, 2]),
                anchor: Anchor::Root,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: None,
            },
            PatternBlock {
                count: 3,
                direction: Direction::Ascending,
                triad: TriadId::T2,
                shape: Shape::Order(vec![0, 1, 2]),
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: None,
            },
            PatternBlock {
                count: 2,
                direction: Direction::Ascending,
                triad: TriadId::T1,
                shape: Shape::Order(vec![2, 0]),
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: None,
            },
            PatternBlock {
                count: 3,
                direction: Direction::Descending,
                triad: TriadId::T2,
                shape: Shape::Order(vec![2, 1, 0]),
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: None,
            },
            PatternBlock {
                count: 3,
                direction: Direction::Descending,
                triad: TriadId::T1,
                shape: Shape::Order(vec![2, 1, 0]),
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: None,
            },
        ];
        let config = LineConfig {
            pattern: Pattern {
                name: "arch",
                blocks: arch,
            },
            figure: RhythmicFigure::Sixteenth,
            positions: PositionSet::from_base_frets(&[5]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        for i in 1..events.len() {
            assert_ne!(
                events[i].midi,
                events[i - 1].midi,
                "repeated pitch {} at event {}",
                events[i].midi,
                i
            );
        }
    }

    #[test]
    fn anchor_third_starts_a_monotonic_block_on_the_triad_third() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = shaped_config(3, TriadId::T1, Shape::Monotonic, Anchor::Third);
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        // role 1 of T1 = G (7).
        assert_eq!(events[0].pitch_class, 7);
    }

    fn simple_config() -> LineConfig {
        LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        }
    }

    fn one_block_config(block: PatternBlock, figure: RhythmicFigure) -> LineConfig {
        LineConfig {
            pattern: Pattern {
                name: "t",
                blocks: vec![block],
            },
            figure,
            positions: PositionSet::from_base_frets(&[5]),
        }
    }

    #[test]
    fn legacy_blocks_have_uniform_duration() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 8);
        for e in &events {
            assert_eq!(e.duration, 0.5);
        }
    }

    #[test]
    fn hold_last_sustains_the_final_note_of_the_block() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let block = PatternBlock {
            count: 2,
            direction: Direction::Ascending,
            triad: TriadId::T1,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
            hold_last: 1,
            lead_rest: 0,
            connector: Connector::default(),
            contour: None,
        };
        let events = generate_line(
            &chart,
            &[],
            &fb,
            &PAIRS[0],
            &one_block_config(block, RhythmicFigure::Eighth),
        );
        assert_eq!(events[0].duration, 0.5);
        assert_eq!(events[1].duration, 1.0);
        assert_eq!(events[2].beat, 1.5);
    }

    #[test]
    fn lead_rest_inserts_a_pickup_gap() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let block = PatternBlock {
            count: 4,
            direction: Direction::Ascending,
            triad: TriadId::T1,
            shape: Shape::Monotonic,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 1,
            connector: Connector::default(),
            contour: None,
        };
        let events = generate_line(
            &chart,
            &[],
            &fb,
            &PAIRS[0],
            &one_block_config(block, RhythmicFigure::Eighth),
        );
        assert_eq!(events[0].beat, 0.5);
    }

    #[test]
    fn triplet_count_is_exact_over_a_long_chart() {
        // Regression guard: the integer slot counter must reproduce the legacy count exactly even
        // where a float accumulation of the 1/3 triplet grid would drift past EPS and add a note.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse(
            "Test",
            "| Dm7 | G7 | Cmaj7 | Am7 | Dm7 | G7 | Cmaj7 | Am7 | Dm7 | G7 | Cmaj7 | Am7 |",
        )
        .unwrap();
        let mut config = simple_config();
        config.figure = RhythmicFigure::Triplet;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let expected = (chart.total_beats() / (1.0_f32 / 3.0)).round() as usize;
        assert_eq!(events.len(), expected); // 12 bars × 4 beats × 3 = 144
    }

    #[test]
    fn generates_correct_number_of_events() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 8);
    }

    #[test]
    fn events_have_sequential_beats() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        for i in 1..events.len() {
            assert!(
                events[i].beat > events[i - 1].beat,
                "beat {} not after {}",
                events[i].beat,
                events[i - 1].beat
            );
        }
    }

    #[test]
    fn events_stay_in_position() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        // simple_config() restricts to a single position at base fret 5.
        let (lo, hi) = NeckPosition::new(5).stretch_range();
        for e in &events {
            assert!(
                e.fret >= lo && e.fret <= hi,
                "fret {} outside stretch range {}-{}",
                e.fret,
                lo,
                hi
            );
        }
    }

    #[test]
    fn pattern_does_not_restart_on_chord_change() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 16);
        // Pattern is 3+3 (T1,T2). Block pattern continuous:
        // Notes 0-2: T1, 3-5: T2, 6-8: T1 (crosses bar), 9-11: T2, 12-14: T1, 15: T2
        assert_eq!(events[0].triad, TriadId::T1);
        assert_eq!(events[3].triad, TriadId::T2);
        assert_eq!(events[6].triad, TriadId::T1);
        assert_eq!(events[9].triad, TriadId::T2);
    }

    #[test]
    fn all_notes_belong_to_indicated_triad() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        for e in &events {
            assert!(e.triad == TriadId::T1 || e.triad == TriadId::T2);
        }
    }

    #[test]
    fn scale_override_changes_available_notes() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events_default = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let events_override = generate_line(&chart, &[Some(2)], &fb, &PAIRS[0], &config);
        let pcs_default: Vec<u8> = events_default.iter().map(|e| e.pitch_class).collect();
        let pcs_override: Vec<u8> = events_override.iter().map(|e| e.pitch_class).collect();
        assert_ne!(pcs_default, pcs_override);
    }

    #[test]
    fn out_of_range_scale_override_falls_back_to_default_without_panicking() {
        // The override index originates from JS and is not bounds-checked there.
        // An index past Scale::ALL must degrade to the chord's default scale
        // instead of panicking (slice index would trap the wasm call).
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events_oob = generate_line(&chart, &[Some(9999)], &fb, &PAIRS[0], &config);
        let events_default = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let pcs_oob: Vec<u8> = events_oob.iter().map(|e| e.pitch_class).collect();
        let pcs_default: Vec<u8> = events_default.iter().map(|e| e.pitch_class).collect();
        assert_eq!(pcs_oob, pcs_default);
    }

    #[test]
    fn sixteenth_notes_produce_double_events() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let mut config = simple_config();
        config.figure = RhythmicFigure::Sixteenth;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 16);
    }

    #[test]
    fn triplets_produce_twelve_events_per_bar() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let mut config = simple_config();
        config.figure = RhythmicFigure::Triplet;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert_eq!(events.len(), 12);
    }

    /// Build a single-block pattern with an explicit connector, for the connector contour tests.
    fn connector_config(connector: Connector, direction: Direction) -> LineConfig {
        LineConfig {
            pattern: Pattern {
                name: "conn",
                blocks: vec![PatternBlock {
                    count: 3,
                    direction,
                    triad: TriadId::T1,
                    shape: Shape::Monotonic,
                    anchor: Anchor::Nearest,
                    hold_last: 0,
                    lead_rest: 0,
                    connector,
                    contour: None,
                }],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        }
    }

    /// The first note of every block (a triad's grip) is the lowest grip note in ascending mode;
    /// across a held chord those block-starts must climb when the connector is `InvertUp` and
    /// never repeat the previous pitch.
    #[test]
    fn invert_up_connector_climbs_the_inversion_ladder() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cmaj7 | % | % | % |").unwrap();
        let events = generate_line(
            &chart,
            &[],
            &fb,
            &PAIRS[0],
            &connector_config(Connector::InvertUp, Direction::Ascending),
        );
        // No consecutive repeats anywhere.
        for i in 1..events.len() {
            assert_ne!(events[i].midi, events[i - 1].midi, "repeat at {}", i);
        }
        // Each ascending block of 3 starts on its grip's lowest note; those starts should rise
        // over the first few blocks (the staircase) rather than stay pinned.
        let starts: Vec<i32> = events.chunks(3).take(3).map(|c| c[0].midi).collect();
        assert!(
            starts.len() == 3 && starts[2] > starts[0],
            "inversion starts did not climb: {:?}",
            starts
        );
    }

    /// `NearestUp` should travel further than `InvertUp` over the same span — it leaps past the
    /// overlapping inversions to the next non-overlapping grip, so it covers more pitch range.
    #[test]
    fn nearest_up_travels_a_wider_range_than_invert_up() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cmaj7 | % | % | % |").unwrap();
        let span = |conn| {
            let evs = generate_line(
                &chart,
                &[],
                &fb,
                &PAIRS[0],
                &connector_config(conn, Direction::Ascending),
            );
            let lo = evs.iter().map(|e| e.midi).min().unwrap();
            let hi = evs.iter().map(|e| e.midi).max().unwrap();
            hi - lo
        };
        assert!(
            span(Connector::NearestUp) >= span(Connector::InvertUp),
            "nearest should cover at least as much range"
        );
    }

    /// No connector may ever produce a repeated pitch back-to-back (the "connect on a different
    /// note" rule), across all connector types and directions.
    #[test]
    fn no_connector_repeats_a_pitch() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 | Cmaj7 | % |").unwrap();
        for conn in [
            Connector::NearestUp,
            Connector::NearestDown,
            Connector::InvertUp,
            Connector::InvertDown,
            Connector::VoiceLead,
            Connector::Random,
        ] {
            for dir in [Direction::Ascending, Direction::Descending] {
                let events =
                    generate_line(&chart, &[], &fb, &PAIRS[0], &connector_config(conn, dir));
                for i in 1..events.len() {
                    assert_ne!(
                        events[i].midi,
                        events[i - 1].midi,
                        "{:?}/{:?} repeated at {}",
                        conn,
                        dir,
                        i
                    );
                }
            }
        }
    }

    #[test]
    fn every_triad_is_played_as_a_valid_ergonomic_grip() {
        // Free mode (whole neck) is the hardest case: with no grip model the line would grab
        // open strings and stack notes on one string. Each triad's notes must instead form one
        // legal grip — ≤3 notes on adjacent strings, ≤2 per string, no open strings, compact span.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cmaj7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::unrestricted(),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert!(!events.is_empty());
        assert!(
            events.iter().all(|e| e.fret >= 1),
            "no open strings allowed"
        );

        for triad in [TriadId::T1, TriadId::T2] {
            let positions: std::collections::BTreeSet<(u8, u8)> = events
                .iter()
                .filter(|e| e.triad == triad)
                .map(|e| (e.string, e.fret))
                .collect();
            assert!(
                positions.len() <= 3,
                "{:?} used {} distinct positions",
                triad,
                positions.len()
            );
            if positions.is_empty() {
                continue;
            }
            let strings: std::collections::BTreeSet<u8> = positions.iter().map(|p| p.0).collect();
            let span_strings = strings.iter().max().unwrap() - strings.iter().min().unwrap();
            match strings.len() {
                3 => assert_eq!(span_strings, 2, "1+1+1 must use consecutive strings"),
                2 => assert_eq!(
                    span_strings, 1,
                    "a two-string grip must use adjacent strings"
                ),
                1 => {} // a partial triad confined to one string is still inside a grip
                n => panic!("{:?} spread over {} strings", triad, n),
            }
            for s in &strings {
                let count = positions.iter().filter(|p| p.0 == *s).count();
                assert!(count <= 2, "more than two notes on string {}", s);
            }
            let frets: Vec<u8> = positions.iter().map(|p| p.1).collect();
            let span = frets.iter().max().unwrap() - frets.iter().min().unwrap();
            assert!(
                span <= crate::theory::triad_shape::MAX_FRET_SPAN,
                "{:?} span {} too wide",
                triad,
                span,
            );
        }
    }

    #[test]
    fn grips_hold_across_a_progression_in_several_regions() {
        // The no-open-string + valid-grip guarantee must survive a whole progression in every
        // region mode — including low position I (where open strings are reachable) and a
        // split multi-position selection.
        let fb = Fretboard::standard_tuning();
        let chart =
            Chart::parse("Test", "| Dm7 | G7 | Cmaj7 | A7 | Em7b5 | A7 | Dm7 | G7 |").unwrap();
        let regions = [
            ("free", PositionSet::unrestricted()),
            ("V", PositionSet::from_base_frets(&[5])),
            ("I", PositionSet::from_base_frets(&[1])),
            ("III+IX", PositionSet::from_base_frets(&[3, 9])),
        ];
        for (name, positions) in regions {
            let config = LineConfig {
                pattern: Pattern::preset_alternating(),
                figure: RhythmicFigure::Sixteenth,
                positions,
            };
            let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
            assert!(!events.is_empty(), "{name}: no events");
            assert!(
                events.iter().all(|e| e.fret >= 1),
                "{name}: produced an open string"
            );
        }
    }

    #[test]
    fn multi_position_keeps_events_within_the_union_of_windows() {
        // Two disjoint boxes: base 2 -> stretch (1,6), base 9 -> stretch (8,13). Every event
        // must land in one of them and never in the fret-7 gap between.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[2, 9]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert!(!events.is_empty());
        for e in &events {
            assert!(
                (1..=6).contains(&e.fret) || (8..=13).contains(&e.fret),
                "fret {} outside the selected windows",
                e.fret,
            );
        }
    }

    #[test]
    fn straddling_blocks_switch_to_the_new_chords_pair_at_the_boundary() {
        // A block that starts under one chord and crosses the barline must play its remaining
        // notes from the NEW chord's triads. Reproduces the Giant Steps opening leak: with the
        // 3+3 eighth pattern, the T2 block at beats 1.5-2.5 used to carry Bmaj7's D#/A# onto D7.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Bmaj7 D7 | Gmaj7 Bb7 | Ebmaj7 |").unwrap();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &simple_config());

        let mut starts: Vec<f32> = Vec::new();
        let mut legal: Vec<Vec<u8>> = Vec::new();
        let mut cumul = 0.0_f32;
        for c in &chart.changes {
            starts.push(cumul);
            cumul += c.beats;
            let scale = scale_defaults::default_scale(c.quality);
            let (a, b) = gmc::resolve_pair(c.root_pc, scale, &PAIRS[0]);
            legal.push(a.iter().chain(b.iter()).copied().collect());
        }
        for e in &events {
            let idx = starts
                .iter()
                .rposition(|&s| e.beat >= s - 1e-4)
                .unwrap_or(0);
            assert!(
                legal[idx].contains(&e.pitch_class),
                "pc {} at beat {} is not in the sounding chord's pair {:?}",
                e.pitch_class,
                e.beat,
                legal[idx]
            );
        }
    }

    #[test]
    fn events_carry_functional_spelling() {
        let chart = Chart::parse("t", "G7").unwrap();
        let altered = Scale::ALL.iter().position(|s| s.name == "Altered").unwrap();
        let config = LineConfig {
            pattern: Pattern {
                name: "t",
                blocks: vec![
                    PatternBlock::legacy(4, Direction::Ascending, TriadId::T1),
                    PatternBlock::legacy(4, Direction::Ascending, TriadId::T2),
                ],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::default(),
        };
        let events = generate_line(
            &chart,
            &[Some(altered)],
            &Fretboard::standard_tuning(),
            &PAIRS[0],
            &config,
        );

        assert!(!events.is_empty(), "the fixture must produce notes");
        for e in &events {
            // Every spelling must round-trip to the pitch class it came from.
            let natural = [0u8, 2, 4, 5, 7, 9, 11][e.step as usize];
            let spelled_pc = (natural as i16 + e.alter as i16).rem_euclid(12) as u8;
            assert_eq!(
                spelled_pc, e.pitch_class,
                "step {} alter {} does not spell pc {}",
                e.step, e.alter, e.pitch_class
            );
            // G7 + Altered never needs a double accidental.
            assert!(e.alter.abs() <= 1, "unexpected double accidental: {:?}", e);
        }

        // The third of G7 must read as B (step 6, natural), never as Cb.
        let third = events
            .iter()
            .find(|e| e.pitch_class == 11)
            .expect("the fixture must sound the third of G7");
        assert_eq!((third.step, third.alter), (6, 0), "third of G7 must be B");
        // The #11 must read as C# (step 0, sharp), never as Db.
        let sharp11 = events
            .iter()
            .find(|e| e.pitch_class == 1)
            .expect("the fixture must sound the #11 of G7");
        assert_eq!(
            (sharp11.step, sharp11.alter),
            (0, 1),
            "#11 of G7 must be C#"
        );
    }

    #[test]
    fn boundary_glue_prefers_a_chromatic_step_when_available() {
        // Unrestricted region: the new chord's ladder offers candidate notes in every octave,
        // and | Cmaj7 D7 | is built so every old-triad pitch class has a new-triad pitch class
        // a half/whole step away (T2: {E,G,B} -> {F#,A,C}; T1: {E,G,B} -> {D,F,A}). Note the
        // glue targets a specific within-grip VOICE (the block's k-th note), so register floors
        // can rule the chromatic pitch class out — the per-boundary bounds below reflect that.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cmaj7 D7 | Cmaj7 D7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::unrestricted(),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        // Boundary 2.0 (Cmaj7 -> D7) crosses on a MID voice: a 1-2 st target exists in register,
        // so the glue must be chromatic. Boundary 4.0 (D7 -> Cmaj7) crosses on an ascending
        // block's TOP voice, and the incoming B2 has no {D,F,A} top note within 2 st (A2 is
        // never a top voice; the lowest top is D3) — the best available move is 3 st, so assert
        // the glue still takes that minimal step rather than a leap or a repeat.
        for (boundary, max_d) in [(2.0_f32, 2), (4.0_f32, 3)] {
            let cross = events
                .windows(2)
                .find(|w| w[0].beat < boundary && w[1].beat >= boundary)
                .expect("no event pair crossing the boundary");
            let d = (cross[1].midi - cross[0].midi).abs();
            assert!(
                (1..=max_d).contains(&d),
                "glue into beat {} moved {} semitones (want 1-{}): {} -> {}",
                boundary,
                d,
                max_d,
                cross[0].midi,
                cross[1].midi
            );
        }
    }

    /// A one-block pattern carrying a contour.
    ///
    /// `direction` is a real parameter, not a constant: for `Shape::Monotonic` the block's
    /// direction chooses which end of the grip the identity starts from, exactly as it does for
    /// a contour-less block. A helper that hardcoded `Ascending` would make the descending
    /// non-regression test compare two different operations.
    fn contour_config(
        count: u8,
        shape: Shape,
        contour: Vec<u8>,
        direction: Direction,
    ) -> LineConfig {
        LineConfig {
            pattern: Pattern {
                name: "test",
                blocks: vec![PatternBlock {
                    count,
                    direction,
                    triad: TriadId::T1,
                    shape,
                    anchor: Anchor::Nearest,
                    hold_last: 0,
                    lead_rest: 0,
                    connector: Connector::default(),
                    contour: Some(contour),
                }],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        }
    }

    #[test]
    fn ascending_contour_reproduces_the_legacy_ascending_walk() {
        // <1 2 3> must be byte-identical to Monotonic + Ascending. This is the non-regression
        // guarantee the whole design rests on — assert on full event vectors, not spot checks.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();

        let legacy = shaped_config(3, TriadId::T1, Shape::Monotonic, Anchor::Nearest);
        let want = generate_line(&chart, &[], &fb, &PAIRS[0], &legacy);

        let with_contour = contour_config(3, Shape::Monotonic, vec![1, 2, 3], Direction::Ascending);
        let got = generate_line(&chart, &[], &fb, &PAIRS[0], &with_contour);

        let key = |e: &NoteEvent| (e.string, e.fret, e.midi, e.beat.to_bits());
        assert_eq!(
            got.iter().map(key).collect::<Vec<_>>(),
            want.iter().map(key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn descending_contour_reproduces_the_legacy_descending_walk() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();

        let mut legacy = shaped_config(3, TriadId::T1, Shape::Monotonic, Anchor::Nearest);
        legacy.pattern.blocks[0].direction = Direction::Descending;
        let want = generate_line(&chart, &[], &fb, &PAIRS[0], &legacy);

        let with_contour = contour_config(3, Shape::Monotonic, vec![3, 2, 1], Direction::Descending);
        let got = generate_line(&chart, &[], &fb, &PAIRS[0], &with_contour);

        let key = |e: &NoteEvent| (e.string, e.fret, e.midi);
        assert_eq!(
            got.iter().map(key).collect::<Vec<_>>(),
            want.iter().map(key).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_scrambled_contour_shapes_the_emitted_line() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = contour_config(3, Shape::Monotonic, vec![3, 1, 2], Direction::Ascending);
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let midis: Vec<i32> = events.iter().take(3).map(|e| e.midi).collect();
        assert!(midis[0] > midis[2], "first note must be the highest");
        assert!(midis[1] < midis[2], "second note must be the lowest");
    }

    #[test]
    fn a_contour_cycles_over_a_longer_block() {
        // count=6 with a 3-contour is two identical cells — the same cycling rule Shape::Order uses.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = contour_config(6, Shape::Monotonic, vec![2, 3, 1], Direction::Ascending);
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let midis: Vec<i32> = events.iter().take(6).map(|e| e.midi).collect();
        for cell in [&midis[0..3], &midis[3..6]] {
            assert!(cell[1] > cell[0], "rank 3 sits at position 1");
            assert!(cell[2] < cell[0], "rank 1 sits at position 2");
        }
    }

    /// Compute ordinal ranking (1-based) of a list of midis — the inverse permutation.
    fn ranks_of(midis: &[i32]) -> Vec<u8> {
        let mut sorted: Vec<i32> = midis.to_vec();
        sorted.sort_unstable();
        midis
            .iter()
            .map(|m| (sorted.iter().position(|s| *s == *m).unwrap() + 1) as u8)
            .collect()
    }

    #[test]
    fn order_with_contour_produces_the_requested_ordinal_shape() {
        // Shape::Order(0,2,1) + contour(2,3,1) must emit notes with ordinal ranking [2,3,1]
        // i.e. the first note is middle, the second is highest, the third is lowest.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern {
                name: "test",
                blocks: vec![PatternBlock {
                    count: 3,
                    direction: Direction::Ascending,
                    triad: TriadId::T1,
                    shape: Shape::Order(vec![0, 2, 1]),
                    anchor: Anchor::Nearest,
                    hold_last: 0,
                    lead_rest: 0,
                    connector: Connector::default(),
                    contour: Some(vec![2, 3, 1]),
                }],
            },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let midis: Vec<i32> = events.iter().take(3).map(|e| e.midi).collect();
        let ranking = ranks_of(&midis);
        assert_eq!(ranking, vec![2, 3, 1]);
    }

    #[test]
    fn contour_blocks_never_repeat_the_previous_pitch() {
        // The no-repeat probe predicts a rung's first note. Under a contour that prediction has to
        // come from the resolved cell, or a block can open on the pitch that just sounded.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Cm7 | % |").unwrap();
        let blocks = vec![
            PatternBlock {
                count: 3,
                direction: Direction::Ascending,
                triad: TriadId::T1,
                shape: Shape::Monotonic,
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: Some(vec![2, 3, 1]),
            },
            PatternBlock {
                count: 3,
                direction: Direction::Ascending,
                triad: TriadId::T2,
                shape: Shape::Monotonic,
                anchor: Anchor::Nearest,
                hold_last: 0,
                lead_rest: 0,
                connector: Connector::default(),
                contour: Some(vec![3, 1, 2]),
            },
        ];
        let config = LineConfig {
            pattern: Pattern { name: "test", blocks },
            figure: RhythmicFigure::Eighth,
            positions: PositionSet::from_base_frets(&[5]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        for pair in events.windows(2) {
            assert_ne!(pair[0].midi, pair[1].midi, "line repeated a pitch");
        }
    }

    #[test]
    fn order_with_contour_survives_a_mid_block_chord_change() {
        // Shape::Order + contour across a mid-block chord change must not panic and
        // must not emit consecutive duplicate pitches.
        //
        // With sixteenths the Dm7 fills 16 slots, so slot 16 is the first note of G7 and lands
        // at k=4 of a 6-note block — a MID-BLOCK chord change. That path picks its rung through
        // `glue_rung`, and for a contour block the note that actually sounds comes from
        // `resolve_cell`. Task 4 routed `glue_rung` and the no-repeat probe through the resolved
        // cell so the rung is chosen by looking at the note that is actually played.
        //
        // The region is deliberately two positions wide (`from_base_frets(&[5, 9])`), not one.
        // A single narrow position here admits exactly one contour-satisfying assignment on each
        // side of the chord change, so no rung choice — and no cost term — can avoid the repeat;
        // that forced-singleton case is its own test,
        // `a_single_position_region_can_force_a_repeated_pitch`. This test needs a region that
        // actually offers alternatives, so a failure here is meaningful: it would mean rung
        // prediction stopped going through the resolved cell, not that the region ran out of room.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern {
                name: "test",
                blocks: vec![PatternBlock {
                    count: 6,
                    direction: Direction::Ascending,
                    triad: TriadId::T1,
                    shape: Shape::Order(vec![0, 2, 1]),
                    anchor: Anchor::Nearest,
                    hold_last: 0,
                    lead_rest: 0,
                    connector: Connector::default(),
                    contour: Some(vec![2, 3, 1]),
                }],
            },
            figure: RhythmicFigure::Sixteenth,
            positions: PositionSet::from_base_frets(&[5, 9]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert!(!events.is_empty(), "generate_line produced no events");
        let midis: Vec<i32> = events.iter().map(|e| e.midi).collect();
        for window in midis.windows(2) {
            assert_ne!(window[0], window[1], "found consecutive duplicate pitch");
        }
    }

    #[test]
    fn a_single_position_region_can_force_a_repeated_pitch() {
        // A DELIBERATE TRADE-OFF, not an unfixed bug. Read this before "fixing" it.
        //
        // Same setup as `order_with_contour_survives_a_mid_block_chord_change`, but confined to
        // ONE position. In that window, `Shape::Order([0,2,1])` with contour <2 3 1> admits
        // exactly one assignment satisfying the ranking on each side of the chord change. When
        // the feasible set has one element the cost function decides nothing — grip affinity,
        // compactness and the REPEAT penalty are all inert — so if that single assignment opens
        // on the pitch that just sounded, the line repeats it and no rung choice can intervene.
        //
        // Faced with break-the-contour / leave-the-region / repeat-a-pitch, the engine repeats.
        // The other two fail silently: a broken contour makes the exercise lie about its own
        // shape, and leaving the region breaks the position discipline the line exists to
        // practise. A repeated pitch is audible, so the player can see what happened.
        //
        // What this test locks down: the contour is still honoured, every note is still in the
        // region, and the repeat is real rather than incidental.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = LineConfig {
            pattern: Pattern {
                name: "test",
                blocks: vec![PatternBlock {
                    count: 6,
                    direction: Direction::Ascending,
                    triad: TriadId::T1,
                    shape: Shape::Order(vec![0, 2, 1]),
                    anchor: Anchor::Nearest,
                    hold_last: 0,
                    lead_rest: 0,
                    connector: Connector::default(),
                    contour: Some(vec![2, 3, 1]),
                }],
            },
            figure: RhythmicFigure::Sixteenth,
            positions: PositionSet::from_base_frets(&[5]),
        };
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert!(events.len() >= 3, "fixture must produce at least one full cell");

        // The contour survives: the first cell, safely inside the Dm7, ranks 2-3-1.
        let cell: Vec<i32> = events.iter().take(3).map(|e| e.midi).collect();
        let mut sorted = cell.clone();
        sorted.sort_unstable();
        let ranks: Vec<usize> = cell
            .iter()
            .map(|m| sorted.iter().position(|s| s == m).unwrap() + 1)
            .collect();
        assert_eq!(ranks, vec![2, 3, 1], "contour was abandoned, not just repeated");

        // The region survives: every emitted note is playable inside the one position.
        let every_pc: Vec<u8> = (0..12).collect();
        let legal: Vec<i32> = config
            .positions
            .find_notes(&fb, &every_pc)
            .iter()
            .map(|n| n.midi)
            .collect();
        for e in &events {
            assert!(
                legal.contains(&e.midi),
                "midi {} at beat {} escaped the single-position region",
                e.midi,
                e.beat
            );
        }

        // And the repeat really is there — if this ever stops holding, the feasible set widened
        // and the comment above needs revisiting, so failing here is informative either way.
        let midis: Vec<i32> = events.iter().map(|e| e.midi).collect();
        assert!(
            midis.windows(2).any(|w| w[0] == w[1]),
            "expected a forced repeated pitch in a single-position region; \
             if this is gone the trade-off documented above no longer applies"
        );
    }

    #[test]
    fn a_chord_change_truncates_the_current_cell() {
        // Two chords per bar with sixteenths gives each chord two beats; a 3-cell straddles the
        // change. Cells must restart on the new ladder rather than resolve across two harmonies.
        //
        // T1 of Dm7 under the default Dorian scale with PAIRS[0] resolves to the role-ordered
        // pitch classes [E=4, G=7, B=11] — the same fixture the Order tests above rely on. So a
        // note sounding during the Dm7 that carries any other pitch class can only have come from
        // a cell resolved against a different chord.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 G7 |").unwrap();
        let mut config = contour_config(6, Shape::Monotonic, vec![2, 3, 1], Direction::Ascending);
        config.figure = RhythmicFigure::Sixteenth;
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        assert!(!events.is_empty(), "the fixture must produce notes");

        let dm7_beats = chart.changes[0].beats;
        let during_dm7: Vec<&NoteEvent> = events.iter().filter(|e| e.beat < dm7_beats).collect();
        assert!(during_dm7.len() >= 4, "expected several notes inside the first chord");
        for e in &during_dm7 {
            assert!(
                [4, 7, 11].contains(&e.pitch_class),
                "pitch class {} at beat {} does not belong to T1 of Dm7 — a cell leaked across \
                 the chord change",
                e.pitch_class,
                e.beat
            );
        }
    }
}
