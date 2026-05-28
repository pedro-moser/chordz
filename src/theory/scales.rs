#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentScale {
    Major,
    HarmonicMinor,
    MelodicMinor,
    HarmonicMajor,
}

impl ParentScale {
    pub const ALL: [Self; 4] = [
        Self::Major,
        Self::HarmonicMinor,
        Self::MelodicMinor,
        Self::HarmonicMajor,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Major => "Major",
            Self::HarmonicMinor => "Harm. Minor",
            Self::MelodicMinor => "Mel. Minor",
            Self::HarmonicMajor => "Harm. Major",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scale {
    pub name: &'static str,
    pub parent: ParentScale,
    pub degree: u8,
    pub semitones: [u8; 7],
}

impl Scale {
    pub const ALL: &[Scale] = &[
        // Major modes
        Scale { name: "Ionian", parent: ParentScale::Major, degree: 1, semitones: [0, 2, 4, 5, 7, 9, 11] },
        Scale { name: "Dorian", parent: ParentScale::Major, degree: 2, semitones: [0, 2, 3, 5, 7, 9, 10] },
        Scale { name: "Phrygian", parent: ParentScale::Major, degree: 3, semitones: [0, 1, 3, 5, 7, 8, 10] },
        Scale { name: "Lydian", parent: ParentScale::Major, degree: 4, semitones: [0, 2, 4, 6, 7, 9, 11] },
        Scale { name: "Mixolydian", parent: ParentScale::Major, degree: 5, semitones: [0, 2, 4, 5, 7, 9, 10] },
        Scale { name: "Aeolian", parent: ParentScale::Major, degree: 6, semitones: [0, 2, 3, 5, 7, 8, 10] },
        Scale { name: "Locrian", parent: ParentScale::Major, degree: 7, semitones: [0, 1, 3, 5, 6, 8, 10] },
        // Harmonic Minor modes
        Scale { name: "Harmonic Minor", parent: ParentScale::HarmonicMinor, degree: 1, semitones: [0, 2, 3, 5, 7, 8, 11] },
        Scale { name: "Locrian \u{266E}6", parent: ParentScale::HarmonicMinor, degree: 2, semitones: [0, 1, 3, 5, 6, 9, 10] },
        Scale { name: "Ionian #5", parent: ParentScale::HarmonicMinor, degree: 3, semitones: [0, 2, 4, 5, 8, 9, 11] },
        Scale { name: "Dorian #4", parent: ParentScale::HarmonicMinor, degree: 4, semitones: [0, 2, 3, 6, 7, 9, 10] },
        Scale { name: "Phrygian Dominant", parent: ParentScale::HarmonicMinor, degree: 5, semitones: [0, 1, 4, 5, 7, 8, 10] },
        Scale { name: "Lydian #2", parent: ParentScale::HarmonicMinor, degree: 6, semitones: [0, 3, 4, 6, 7, 9, 11] },
        Scale { name: "Superlocrian \u{266D}\u{266D}7", parent: ParentScale::HarmonicMinor, degree: 7, semitones: [0, 1, 3, 4, 6, 8, 9] },
        // Melodic Minor modes
        Scale { name: "Melodic Minor", parent: ParentScale::MelodicMinor, degree: 1, semitones: [0, 2, 3, 5, 7, 9, 11] },
        Scale { name: "Dorian \u{266D}2", parent: ParentScale::MelodicMinor, degree: 2, semitones: [0, 1, 3, 5, 7, 9, 10] },
        Scale { name: "Lydian Augmented", parent: ParentScale::MelodicMinor, degree: 3, semitones: [0, 2, 4, 6, 8, 9, 11] },
        Scale { name: "Lydian Dominant", parent: ParentScale::MelodicMinor, degree: 4, semitones: [0, 2, 4, 6, 7, 9, 10] },
        Scale { name: "Mixolydian \u{266D}6", parent: ParentScale::MelodicMinor, degree: 5, semitones: [0, 2, 4, 5, 7, 8, 10] },
        Scale { name: "Aeolian \u{266D}5", parent: ParentScale::MelodicMinor, degree: 6, semitones: [0, 2, 3, 5, 6, 8, 10] },
        Scale { name: "Altered", parent: ParentScale::MelodicMinor, degree: 7, semitones: [0, 1, 3, 4, 6, 8, 10] },
        // Harmonic Major modes
        Scale { name: "Harmonic Major", parent: ParentScale::HarmonicMajor, degree: 1, semitones: [0, 2, 4, 5, 7, 8, 11] },
        Scale { name: "Dorian \u{266D}5", parent: ParentScale::HarmonicMajor, degree: 2, semitones: [0, 2, 3, 5, 6, 9, 10] },
        Scale { name: "Phrygian \u{266D}4", parent: ParentScale::HarmonicMajor, degree: 3, semitones: [0, 1, 3, 4, 7, 8, 10] },
        Scale { name: "Melodic Minor #4", parent: ParentScale::HarmonicMajor, degree: 4, semitones: [0, 2, 3, 6, 7, 9, 11] },
        Scale { name: "Mixolydian \u{266D}2", parent: ParentScale::HarmonicMajor, degree: 5, semitones: [0, 1, 4, 5, 7, 9, 10] },
        Scale { name: "Lydian Augmented #2", parent: ParentScale::HarmonicMajor, degree: 6, semitones: [0, 3, 4, 6, 8, 9, 11] },
        Scale { name: "Locrian \u{266D}\u{266D}7", parent: ParentScale::HarmonicMajor, degree: 7, semitones: [0, 1, 3, 4, 6, 8, 9] },
    ];

    pub fn non_root_semitones(&self) -> [u8; 6] {
        [
            self.semitones[1],
            self.semitones[2],
            self.semitones[3],
            self.semitones[4],
            self.semitones[5],
            self.semitones[6],
        ]
    }

    pub fn interval_name(&self, semitone: u8) -> &'static str {
        match semitone {
            0 => "1",
            1 => "b2",
            2 => "2",
            3 => "b3",
            4 => "3",
            5 => "4",
            6 => "#4",
            7 => "5",
            8 => "b6",
            9 => "6",
            10 => "b7",
            11 => "7",
            _ => "?",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scales_have_28_entries() {
        assert_eq!(Scale::ALL.len(), 28);
    }

    #[test]
    fn all_scales_start_with_zero() {
        for scale in Scale::ALL {
            assert_eq!(scale.semitones[0], 0, "{} does not start with 0", scale.name);
        }
    }

    #[test]
    fn all_scales_are_ascending() {
        for scale in Scale::ALL {
            for i in 1..7 {
                assert!(
                    scale.semitones[i] > scale.semitones[i - 1],
                    "{} is not ascending at index {}: {:?}",
                    scale.name, i, scale.semitones
                );
            }
        }
    }

    #[test]
    fn all_scales_within_octave() {
        for scale in Scale::ALL {
            for &s in &scale.semitones {
                assert!(s < 12, "{} has semitone >= 12", scale.name);
            }
        }
    }

    #[test]
    fn seven_modes_per_parent() {
        for parent in ParentScale::ALL {
            let count = Scale::ALL.iter().filter(|s| s.parent == parent).count();
            assert_eq!(count, 7, "{:?} has {} modes, expected 7", parent, count);
        }
    }

    #[test]
    fn dorian_semitones_correct() {
        let dorian = Scale::ALL.iter().find(|s| s.name == "Dorian").unwrap();
        assert_eq!(dorian.semitones, [0, 2, 3, 5, 7, 9, 10]);
    }

    #[test]
    fn non_root_semitones_excludes_zero() {
        let ionian = &Scale::ALL[0];
        assert_eq!(ionian.non_root_semitones(), [2, 4, 5, 7, 9, 11]);
    }
}
