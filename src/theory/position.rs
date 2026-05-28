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
        (self.base_fret, self.base_fret + 3)
    }

    pub fn stretch_range(&self) -> (u8, u8) {
        (self.base_fret.saturating_sub(1), self.base_fret + 4)
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
    fn fret_note_is_stretch() {
        let pos = NeckPosition::new(5);
        let core = FretNote { string: 0, fret: 5, midi: 40, pitch_class: 4 };
        let stretch = FretNote { string: 0, fret: 4, midi: 39, pitch_class: 3 };
        assert!(!pos.is_stretch(core.fret));
        assert!(pos.is_stretch(stretch.fret));
    }
}
