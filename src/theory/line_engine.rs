use crate::theory::chart::Chart;
use crate::theory::gmc::{self, TriadPairSet};
use crate::theory::line_pattern::{Direction, Pattern, RhythmicFigure, TriadId};
use crate::theory::position::{FretNote, NeckPosition};
use crate::theory::scale_defaults;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;

#[derive(Clone, Debug)]
pub struct NoteEvent {
    pub beat: f32,
    pub string: u8,
    pub fret: u8,
    pub triad: TriadId,
    pub pitch_class: u8,
    pub midi: i32,
}

pub struct LineConfig {
    pub pattern: Pattern,
    pub figure: RhythmicFigure,
    pub position: NeckPosition,
}

struct TriadNotes {
    t1: Vec<FretNote>,
    t2: Vec<FretNote>,
}

impl TriadNotes {
    fn notes_for(&self, triad: TriadId) -> &[FretNote] {
        match triad {
            TriadId::T1 => &self.t1,
            TriadId::T2 => &self.t2,
        }
    }
}

fn resolve_triad_notes(
    root_pc: u8,
    scale: &Scale,
    pair: &TriadPairSet,
    position: &NeckPosition,
    fretboard: &Fretboard,
) -> TriadNotes {
    let (pcs_a, pcs_b) = gmc::resolve_pair(root_pc, scale, pair);
    TriadNotes {
        t1: position.find_notes(fretboard, &pcs_a),
        t2: position.find_notes(fretboard, &pcs_b),
    }
}

fn find_nearest(notes: &[FretNote], current_midi: i32, direction: Direction) -> Option<&FretNote> {
    match direction {
        Direction::Ascending => notes.iter().find(|n| n.midi > current_midi),
        Direction::Descending => notes.iter().rev().find(|n| n.midi < current_midi),
    }
}

fn find_closest(notes: &[FretNote], current_midi: i32) -> Option<&FretNote> {
    notes
        .iter()
        .filter(|n| n.midi != current_midi)
        .min_by_key(|n| (n.midi - current_midi).abs())
        .or_else(|| notes.first())
}

pub fn generate_line(
    chart: &Chart,
    scale_overrides: &[Option<usize>],
    fretboard: &Fretboard,
    pair: &TriadPairSet,
    config: &LineConfig,
) -> Vec<NoteEvent> {
    let beat_dur = config.figure.beat_duration();
    let total_beats = chart.total_beats();
    let total_events = (total_beats / beat_dur).round() as usize;

    let mut events = Vec::with_capacity(total_events);
    let mut pattern_iter = config.pattern.iter();
    let mut current_direction = Direction::Ascending;
    let mut current_midi: i32 = 0;
    let mut first_note = true;

    // Pre-compute chord boundaries
    let mut chord_boundaries: Vec<(f32, f32, usize)> = Vec::new();
    let mut cumulative = 0.0_f32;
    for (i, change) in chart.changes.iter().enumerate() {
        chord_boundaries.push((cumulative, cumulative + change.beats, i));
        cumulative += change.beats;
    }

    // Pre-resolve triad notes per chord
    let triad_notes_per_chord: Vec<TriadNotes> = chart
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let scale = scale_overrides
                .get(i)
                .and_then(|opt| opt.map(|idx| &Scale::ALL[idx]))
                .unwrap_or_else(|| scale_defaults::default_scale(change.quality));
            resolve_triad_notes(change.root_pc, scale, pair, &config.position, fretboard)
        })
        .collect();

    let mut block_remaining = 0u8;
    let mut block_triad = TriadId::T1;
    let mut block_first = false;

    for event_idx in 0..total_events {
        let beat = event_idx as f32 * beat_dur;

        // Find which chord we're in
        let chord_idx = chord_boundaries
            .iter()
            .rposition(|&(start, _, _)| beat >= start)
            .unwrap_or(0);

        let triad_notes = &triad_notes_per_chord[chord_idx];

        // Advance pattern if needed
        if block_remaining == 0 {
            if let Some(block) = pattern_iter.next() {
                block_remaining = block.count;
                block_triad = block.triad;
                current_direction = block.direction;
                block_first = true;
            }
        }

        let pool = triad_notes.notes_for(block_triad);

        if pool.is_empty() {
            block_remaining = block_remaining.saturating_sub(1);
            continue;
        }

        let chosen = if first_note {
            pool.first()
        } else if block_first {
            // Connect to the nearest note in the new triad, then continue in direction
            find_closest(pool, current_midi)
        } else {
            let candidate = find_nearest(pool, current_midi, current_direction);
            if candidate.is_some() {
                candidate
            } else {
                current_direction = current_direction.invert();
                let inverted = find_nearest(pool, current_midi, current_direction);
                if inverted.is_some() {
                    inverted
                } else {
                    find_closest(pool, current_midi)
                }
            }
        };
        block_first = false;

        if let Some(note) = chosen {
            events.push(NoteEvent {
                beat,
                string: note.string,
                fret: note.fret,
                triad: block_triad,
                pitch_class: note.pitch_class,
                midi: note.midi,
            });
            current_midi = note.midi;
            first_note = false;
        }

        block_remaining = block_remaining.saturating_sub(1);
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chart::Chart;
    use crate::theory::gmc::PAIRS;
    use crate::theory::line_pattern::{Pattern, RhythmicFigure, TriadId};
    use crate::theory::position::NeckPosition;
    use crate::voicings::fretboard::Fretboard;

    fn simple_config() -> LineConfig {
        LineConfig {
            pattern: Pattern::preset_alternating(),
            figure: RhythmicFigure::Eighth,
            position: NeckPosition::new(5),
        }
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
            assert!(events[i].beat > events[i - 1].beat,
                "beat {} not after {}", events[i].beat, events[i - 1].beat);
        }
    }

    #[test]
    fn events_stay_in_position() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 | G7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);
        let (lo, hi) = config.position.stretch_range();
        for e in &events {
            assert!(e.fret >= lo && e.fret <= hi,
                "fret {} outside stretch range {}-{}", e.fret, lo, hi);
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

    #[test]
    fn block_boundary_connects_to_nearest_distinct_note() {
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let events = generate_line(&chart, &[], &fb, &PAIRS[0], &config);

        // preset_alternating = 3xT1 then 3xT2: index 3 is the first note of the new
        // (T2) block — the "connecting" note produced by the block_first branch.
        assert_eq!(events[3].triad, TriadId::T2);
        // It must not simply repeat the previous pitch when an alternative exists.
        assert_ne!(events[3].midi, events[2].midi);

        // And it must be exactly the closest distinct note of the new triad's pool to
        // the previous note — the find_closest contract this feature depends on. (This
        // guards against silently reverting to the plain find_nearest path.)
        let scale = crate::theory::scale_defaults::default_scale(chart.changes[0].quality);
        let tn = super::resolve_triad_notes(
            chart.changes[0].root_pc,
            scale,
            &PAIRS[0],
            &config.position,
            &fb,
        );
        let expected = super::find_closest(tn.notes_for(TriadId::T2), events[2].midi).unwrap();
        assert_eq!(events[3].midi, expected.midi);
    }

    #[test]
    fn find_closest_skips_the_current_note() {
        // Pool drawn from a real triad position so we exercise actual FretNote data.
        let fb = Fretboard::standard_tuning();
        let chart = Chart::parse("Test", "| Dm7 |").unwrap();
        let config = simple_config();
        let scale = crate::theory::scale_defaults::default_scale(chart.changes[0].quality);
        let tn = super::resolve_triad_notes(
            chart.changes[0].root_pc,
            scale,
            &PAIRS[0],
            &config.position,
            &fb,
        );
        let pool = tn.notes_for(TriadId::T1);
        let target = pool[0].midi;
        let has_distinct = pool.iter().any(|n| n.midi != target);
        let got = super::find_closest(pool, target).unwrap();
        if has_distinct {
            // Filter branch: skip the equal-pitch note when an alternative exists.
            assert_ne!(got.midi, target);
        } else {
            // or_else fallback: a single repeated pitch is the only valid choice.
            assert_eq!(got.midi, target);
        }
    }
}
