use crate::theory::notes::Note;

/// A guitar fretboard with configurable tuning and fret count.
#[derive(Clone, Debug)]
pub struct Fretboard {
    /// Open-string notes for each of the 6 strings (index 0 = low E).
    pub open_strings: [Note; 6],
    /// Number of frets (0 = open string, 1..num_frets = fretted).
    pub num_frets: usize,
}

impl Fretboard {
    /// Standard tuning: E2, A2, D3, G3, B3, E4.
    pub fn standard_tuning() -> Self {
        Self {
            open_strings: [
                Note::new(4, 2),  // E2
                Note::new(9, 2),  // A2
                Note::new(2, 3),  // D3
                Note::new(7, 3),  // G3
                Note::new(11, 3), // B3
                Note::new(4, 4),  // E4
            ],
            num_frets: 24,
        }
    }

    /// Get the note at a given string and fret position.
    /// String 0 = low E, string 5 = high E.
    /// Returns `None` if the string index is out of bounds or the fret exceeds `num_frets`.
    pub fn get_note(&self, string: usize, fret: usize) -> Option<Note> {
        if string >= self.open_strings.len() || fret > self.num_frets {
            return None;
        }
        Some(self.open_strings[string].transpose(fret as i32))
    }

    /// The number of strings on this fretboard.
    pub fn num_strings(&self) -> usize {
        self.open_strings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_tuning_open_strings() {
        let fb = Fretboard::standard_tuning();
        assert_eq!(fb.get_note(0, 0).unwrap().pc_name(), "E");
        assert_eq!(fb.get_note(1, 0).unwrap().pc_name(), "A");
        assert_eq!(fb.get_note(2, 0).unwrap().pc_name(), "D");
        assert_eq!(fb.get_note(3, 0).unwrap().pc_name(), "G");
        assert_eq!(fb.get_note(4, 0).unwrap().pc_name(), "B");
        assert_eq!(fb.get_note(5, 0).unwrap().pc_name(), "E");
    }

    #[test]
    fn test_fret_transposition() {
        let fb = Fretboard::standard_tuning();
        // 5th fret on low E = A
        assert_eq!(fb.get_note(0, 5).unwrap().pc_name(), "A");
        // 7th fret on A = E (perfect 5th)
        assert_eq!(fb.get_note(1, 7).unwrap().pc_name(), "E");
        // 12th fret is octave of open string
        assert_eq!(
            fb.get_note(0, 12).unwrap().pitch_class,
            fb.get_note(0, 0).unwrap().pitch_class
        );
    }
}

#[test]
fn test_invalid_string_returns_none() {
    let fb = Fretboard::standard_tuning();
    assert!(fb.get_note(6, 0).is_none());
    assert!(fb.get_note(7, 0).is_none());
}

#[test]
fn test_fret_beyond_num_frets_returns_none() {
    let fb = Fretboard::standard_tuning();
    // num_frets = 24, so fret 25 is out of bounds
    assert!(fb.get_note(0, 24).is_some());
    assert!(fb.get_note(0, 25).is_none());
    assert!(fb.get_note(0, 100).is_none());
}
