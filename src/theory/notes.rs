/// A musical note represented by a pitch class (0=C, 1=C#, …, 11=B) and an octave.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Note {
    pub pitch_class: u8,
    pub octave: i8,
}

impl Note {
    /// Create a new note. Pitch class is normalized to 0..12.
    pub fn new(pitch_class: i32, octave: i8) -> Self {
        let pc = ((pitch_class % 12) + 12) % 12;
        let oct = (octave as i32 + (pitch_class - pc as i32) / 12) as i8;
        Note {
            pitch_class: pc as u8,
            octave: oct,
        }
    }

    /// Transpose this note by a given number of semitones.
    pub fn transpose(self, semitones: i32) -> Self {
        Self::new(self.pitch_class as i32 + semitones, self.octave as i32 as i8)
    }

    /// The MIDI note number for this note.
    pub fn midi(self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.pitch_class as i32
    }

    /// Build a Note from a MIDI note number.
    pub fn from_midi(midi: i32) -> Self {
        let octave = midi / 12 - 1;
        let pc = ((midi % 12) + 12) % 12;
        Note {
            pitch_class: pc as u8,
            octave: octave as i8,
        }
    }

    /// Human-readable name, e.g. "C4", "F#3", "Bb5".
    pub fn name(self) -> String {
        let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        format!("{}{}", names[self.pitch_class as usize], self.octave)
    }

    /// Just the pitch class name, e.g. "C", "F#".
    pub fn pc_name(self) -> &'static str {
        const NAMES: [&str; 12] =
            ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        NAMES[self.pitch_class as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_new_normalizes_pitch_class() {
        let n = Note::new(13, 4);
        assert_eq!(n.pitch_class, 1);
        assert_eq!(n.octave, 5);
    }

    #[test]
    fn test_note_new_negative_pitch_class() {
        let n = Note::new(-1, 4);
        assert_eq!(n.pitch_class, 11);
        assert_eq!(n.octave, 3);
    }

    #[test]
    fn test_transpose_up() {
        let n = Note::new(0, 4); // C4
        let t = n.transpose(7);
        assert_eq!(t.pitch_class, 7); // G
        assert_eq!(t.octave, 4);
    }

    #[test]
    fn test_transpose_down_across_octave() {
        let n = Note::new(0, 4); // C4
        let t = n.transpose(-3);
        assert_eq!(t.pitch_class, 9); // A
        assert_eq!(t.octave, 3);
    }

    #[test]
    fn test_midi_roundtrip() {
        let n = Note::new(0, 4);
        assert_eq!(n.midi(), 60);
        assert_eq!(Note::from_midi(60), n);
    }

    #[test]
    fn test_name() {
        let n = Note::new(0, 4);
        assert_eq!(n.name(), "C4");
        let n2 = Note::new(3, 3);
        assert_eq!(n2.name(), "D#3");
    }
}
