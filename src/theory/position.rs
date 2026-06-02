use crate::voicings::fretboard::Fretboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FretNote {
    pub string: u8,
    pub fret: u8,
    pub midi: i32,
    pub pitch_class: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rigidity {
    Strict,
    Flexible,
}

#[derive(Clone, Copy, Debug)]
pub struct NeckPosition {
    pub base_fret: u8,
    pub rigidity: Rigidity,
}

impl NeckPosition {
    pub fn new(base_fret: u8) -> Self {
        Self {
            base_fret,
            rigidity: Rigidity::Strict,
        }
    }

    pub fn core_range(&self) -> (u8, u8) {
        (self.base_fret, self.base_fret.saturating_add(3))
    }

    pub fn stretch_range(&self) -> (u8, u8) {
        (self.base_fret.saturating_sub(1), self.base_fret.saturating_add(4))
    }

    pub fn is_stretch(&self, fret: u8) -> bool {
        let (core_lo, core_hi) = self.core_range();
        let (str_lo, str_hi) = self.stretch_range();
        fret >= str_lo && fret <= str_hi && (fret < core_lo || fret > core_hi)
    }

    pub fn find_notes(&self, fretboard: &Fretboard, pitch_classes: &[u8]) -> Vec<FretNote> {
        let (lo, hi) = self.stretch_range();
        let mut notes = Vec::new();
        for s in 0..fretboard.num_strings() {
            for fret in lo..=hi {
                if let Some(note) = fretboard.get_note(s, fret as usize) {
                    if pitch_classes.contains(&note.pitch_class) {
                        notes.push(FretNote {
                            string: s as u8,
                            fret,
                            midi: note.midi(),
                            pitch_class: note.pitch_class,
                        });
                    }
                }
            }
        }
        notes.sort_by_key(|n| n.midi);
        notes
    }

    pub fn shifted(&self, offset: i8) -> Self {
        Self {
            base_fret: (self.base_fret as i8 + offset).max(1) as u8,
            rigidity: self.rigidity,
        }
    }
}

/// The neck region(s) a line may use. An **empty** set means *no restriction* — the line draws
/// from the whole fretboard; otherwise the candidate pool is the union of the selected
/// positions' stretch windows. This is a pure pool filter: the pattern walker is unchanged,
/// it just sees more (or all) candidate notes.
#[derive(Clone, Debug, Default)]
pub struct PositionSet {
    pub positions: Vec<NeckPosition>,
}

impl PositionSet {
    /// No position constraint — the whole neck is available.
    pub fn unrestricted() -> Self {
        Self { positions: Vec::new() }
    }

    /// One `NeckPosition` per base fret (1-12). An empty slice yields an unrestricted set.
    pub fn from_base_frets(base_frets: &[u8]) -> Self {
        Self {
            positions: base_frets.iter().map(|&f| NeckPosition::new(f)).collect(),
        }
    }

    pub fn is_unrestricted(&self) -> bool {
        self.positions.is_empty()
    }

    /// Every fretboard note matching `pitch_classes` that lies in the allowed region, sorted by
    /// pitch. Empty set → scan the whole neck; otherwise union the per-position windows,
    /// deduping notes that overlapping windows would otherwise return twice.
    pub fn find_notes(&self, fretboard: &Fretboard, pitch_classes: &[u8]) -> Vec<FretNote> {
        let mut notes = Vec::new();
        if self.positions.is_empty() {
            for s in 0..fretboard.num_strings() {
                for fret in 0..=fretboard.num_frets {
                    if let Some(note) = fretboard.get_note(s, fret) {
                        if pitch_classes.contains(&note.pitch_class) {
                            notes.push(FretNote {
                                string: s as u8,
                                fret: fret as u8,
                                midi: note.midi(),
                                pitch_class: note.pitch_class,
                            });
                        }
                    }
                }
            }
        } else {
            let mut seen = std::collections::HashSet::new();
            for pos in &self.positions {
                for n in pos.find_notes(fretboard, pitch_classes) {
                    if seen.insert((n.string, n.fret)) {
                        notes.push(n);
                    }
                }
            }
        }
        notes.sort_by_key(|n| n.midi);
        notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voicings::fretboard::Fretboard;

    #[test]
    fn core_range_position_v() {
        let pos = NeckPosition::new(5);
        assert_eq!(pos.core_range(), (5, 8));
    }

    #[test]
    fn stretch_range_position_v() {
        let pos = NeckPosition::new(5);
        assert_eq!(pos.stretch_range(), (4, 9));
    }

    #[test]
    fn core_range_position_i() {
        let pos = NeckPosition::new(1);
        assert_eq!(pos.core_range(), (1, 4));
    }

    #[test]
    fn stretch_range_position_i_clamps_low() {
        let pos = NeckPosition::new(1);
        assert_eq!(pos.stretch_range(), (0, 5));
    }

    #[test]
    fn find_notes_c_major_triad_position_v() {
        let fb = Fretboard::standard_tuning();
        let pos = NeckPosition::new(5);
        let pcs = [0, 4, 7];
        let notes = pos.find_notes(&fb, &pcs);
        assert!(!notes.is_empty());
        for n in &notes {
            assert!(n.fret >= 4 && n.fret <= 9, "fret {} out of stretch range", n.fret);
            assert!(pcs.contains(&n.pitch_class));
        }
    }

    #[test]
    fn find_notes_sorted_ascending_by_pitch() {
        let fb = Fretboard::standard_tuning();
        let pos = NeckPosition::new(5);
        let pcs = [0, 4, 7];
        let notes = pos.find_notes(&fb, &pcs);
        for i in 1..notes.len() {
            assert!(
                notes[i].midi >= notes[i - 1].midi,
                "notes not sorted: {} >= {} failed",
                notes[i].midi,
                notes[i - 1].midi,
            );
        }
    }

    #[test]
    fn high_base_fret_does_not_overflow() {
        // base_fret arrives from the JS boundary as an unvalidated u8; the +3/+4
        // offsets must saturate rather than overflow (panic in debug, wrap in release).
        let pos = NeckPosition::new(254);
        let (c_lo, c_hi) = pos.core_range();
        let (s_lo, s_hi) = pos.stretch_range();
        assert!(c_hi >= c_lo, "core range inverted: {}..{}", c_lo, c_hi);
        assert!(s_hi >= s_lo, "stretch range inverted: {}..{}", s_lo, s_hi);
    }

    #[test]
    fn fret_note_is_stretch() {
        let pos = NeckPosition::new(5);
        let core = FretNote { string: 0, fret: 5, midi: 40, pitch_class: 4 };
        let stretch = FretNote { string: 0, fret: 4, midi: 39, pitch_class: 3 };
        assert!(!pos.is_stretch(core.fret));
        assert!(pos.is_stretch(stretch.fret));
    }

    // --- PositionSet: 0..N neck regions, empty = unrestricted ---

    #[test]
    fn position_set_unrestricted_flag() {
        assert!(PositionSet::unrestricted().is_unrestricted());
        assert!(!PositionSet::from_base_frets(&[5]).is_unrestricted());
    }

    #[test]
    fn position_set_empty_spans_the_whole_neck() {
        // No restriction: the pool reaches both the open frets and the high neck — far wider
        // than any single 6-fret stretch box.
        let fb = Fretboard::standard_tuning();
        let notes = PositionSet::unrestricted().find_notes(&fb, &[0, 4, 7]);
        let min_fret = notes.iter().map(|n| n.fret).min().unwrap();
        let max_fret = notes.iter().map(|n| n.fret).max().unwrap();
        assert!(min_fret <= 1, "expected an open/low note, got min fret {}", min_fret);
        assert!(max_fret >= 12, "expected a high-neck note, got max fret {}", max_fret);
    }

    #[test]
    fn position_set_single_matches_neck_position() {
        // A one-element set must reproduce the legacy single-NeckPosition pool exactly
        // (this is what keeps the native egui app's behavior unchanged).
        let fb = Fretboard::standard_tuning();
        let pcs = [0, 4, 7];
        let single: Vec<(u8, u8)> =
            NeckPosition::new(5).find_notes(&fb, &pcs).iter().map(|n| (n.string, n.fret)).collect();
        let via_set: Vec<(u8, u8)> =
            PositionSet::from_base_frets(&[5]).find_notes(&fb, &pcs).iter().map(|n| (n.string, n.fret)).collect();
        assert_eq!(single, via_set);
    }

    #[test]
    fn position_set_union_covers_both_windows_and_excludes_the_gap() {
        // base 2 -> stretch (1,6); base 9 -> stretch (8,13). Fret 7 sits in neither window.
        let fb = Fretboard::standard_tuning();
        let notes = PositionSet::from_base_frets(&[2, 9]).find_notes(&fb, &[0, 4, 7]);
        assert!(notes.iter().any(|n| (1..=6).contains(&n.fret)), "no note in the low window");
        assert!(notes.iter().any(|n| (8..=13).contains(&n.fret)), "no note in the high window");
        assert!(
            notes.iter().all(|n| (1..=6).contains(&n.fret) || (8..=13).contains(&n.fret)),
            "a note fell outside both selected windows",
        );
        assert!(!notes.iter().any(|n| n.fret == 7), "fret 7 is in the gap and must be excluded");
    }

    #[test]
    fn position_set_dedups_overlapping_windows() {
        // base 5 -> (4,9); base 7 -> (6,11) overlap on frets 6..9 — each (string,fret) once.
        let fb = Fretboard::standard_tuning();
        let notes = PositionSet::from_base_frets(&[5, 7]).find_notes(&fb, &[0, 4, 7]);
        let mut seen = std::collections::HashSet::new();
        for n in &notes {
            assert!(seen.insert((n.string, n.fret)), "duplicate (string {}, fret {})", n.string, n.fret);
        }
    }

    #[test]
    fn position_set_result_sorted_by_midi() {
        let fb = Fretboard::standard_tuning();
        let notes = PositionSet::from_base_frets(&[2, 9]).find_notes(&fb, &[0, 4, 7]);
        for i in 1..notes.len() {
            assert!(notes[i].midi >= notes[i - 1].midi, "pool not sorted by midi");
        }
    }
}
