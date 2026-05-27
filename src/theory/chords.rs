use crate::theory::intervals::Interval;

/// A chord quality defined by a name and a set of intervals from the root.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChordQuality {
    pub name: &'static str,
    pub intervals: &'static [Interval],
}

impl ChordQuality {
    /// All supported jazz chord qualities.
    pub const ALL: &'static [Self] = &[
        // Major family
        Self {
            name: "maj7",
            intervals: &[Interval::UNISON, Interval::M3, Interval::P5, Interval::M7],
        },
        Self {
            name: "maj9",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::M7,
                Interval::M9,
            ],
        },
        Self {
            name: "maj13",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::M7,
                Interval::M9,
                Interval::M13,
            ],
        },
        Self {
            name: "maj7#11",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::M7,
                Interval::SHARP11,
            ],
        },
        // Minor family
        Self {
            name: "m7",
            intervals: &[Interval::UNISON, Interval::m3, Interval::P5, Interval::m7],
        },
        Self {
            name: "m9",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::P5,
                Interval::m7,
                Interval::M9,
            ],
        },
        Self {
            name: "m11",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::P5,
                Interval::m7,
                Interval::M9,
                Interval::M11,
            ],
        },
        Self {
            name: "m13",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::P5,
                Interval::m7,
                Interval::M9,
                Interval::M13,
            ],
        },
        Self {
            name: "m7b5",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::tritone,
                Interval::m7,
            ],
        },
        Self {
            name: "m9b11",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::tritone,
                Interval::m7,
                Interval::m9,
                Interval::m11,
            ],
        },
        // Dominant family
        Self {
            name: "dom7",
            intervals: &[Interval::UNISON, Interval::M3, Interval::P5, Interval::m7],
        },
        Self {
            name: "dom9",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::M9,
            ],
        },
        Self {
            name: "dom13",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::M9,
                Interval::M13,
            ],
        },
        Self {
            name: "dom7#5",
            intervals: &[Interval::UNISON, Interval::M3, Interval::m6, Interval::m7],
        },
        Self {
            name: "dom7b9",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::m9,
            ],
        },
        Self {
            name: "dom7#9",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::SHARP9,
            ],
        },
        Self {
            name: "dom7#11",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::SHARP11,
            ],
        },
        Self {
            name: "dom7b13",
            intervals: &[
                Interval::UNISON,
                Interval::M3,
                Interval::P5,
                Interval::m7,
                Interval::m13,
            ],
        },
        // Diminished
        Self {
            name: "dim7",
            intervals: &[
                Interval::UNISON,
                Interval::m3,
                Interval::tritone,
                Interval::dim7,
            ],
        },
        // Augmented
        Self {
            name: "aug7",
            intervals: &[Interval::UNISON, Interval::M3, Interval::m6, Interval::m7],
        },
    ];
}

/// All 12 pitch class root names, following jazz conventions (flats preferred).
pub const ROOTS: [&str; 12] = [
    "C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];

/// Get the pitch class (0=C, 1=C#, …, 11=B) for a root name.
pub fn root_to_pc(root: &str) -> Option<u8> {
    match root {
        "C" => Some(0),
        "C#" | "Db" => Some(1),
        "D" => Some(2),
        "D#" | "Eb" => Some(3),
        "E" => Some(4),
        "F" => Some(5),
        "F#" | "Gb" => Some(6),
        "G" => Some(7),
        "G#" | "Ab" => Some(8),
        "A" => Some(9),
        "A#" | "Bb" => Some(10),
        "B" => Some(11),
        _ => None,
    }
}

/// Format a full chord name from root and quality, e.g. "Cmaj7", "Dm9".
pub fn chord_name(root: &str, quality: &ChordQuality) -> String {
    match quality.name {
        "maj7" | "maj9" | "maj13" | "maj7#11" => format!("{}{}", root, quality.name),
        "m7" | "m9" | "m11" | "m13" | "m7b5" | "m9b11" => {
            format!("{}m{}", root, &quality.name[1..])
        }
        "dom7" | "dom9" | "dom13" | "dom7#5" | "dom7b9" | "dom7#9" | "dom7#11" | "dom7b13" => {
            format!(
                "{}{}",
                root,
                quality.name.strip_prefix("dom").unwrap_or(quality.name)
            )
        }
        "dim7" => format!("{}dim7", root),
        "aug7" => format!("{}aug7", root),
        _ => format!("{}{}", root, quality.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_to_pc() {
        assert_eq!(root_to_pc("C"), Some(0));
        assert_eq!(root_to_pc("C#"), Some(1));
        assert_eq!(root_to_pc("Db"), Some(1));
        assert_eq!(root_to_pc("Eb"), Some(3));
        assert_eq!(root_to_pc("D#"), Some(3));
        assert_eq!(root_to_pc("F"), Some(5));
        assert_eq!(root_to_pc("Bb"), Some(10));
        assert_eq!(root_to_pc("A#"), Some(10));
        assert_eq!(root_to_pc("B"), Some(11));
        assert_eq!(root_to_pc("H"), None);
        assert_eq!(root_to_pc("X"), None);
    }

    #[test]
    fn test_roots_use_jazz_conventions() {
        assert_eq!(ROOTS[1], "Db");
        assert_eq!(ROOTS[3], "Eb");
        assert_eq!(ROOTS[6], "F#");
        assert_eq!(ROOTS[8], "Ab");
        assert_eq!(ROOTS[10], "Bb");
    }

    #[test]
    fn test_chord_name_with_flats() {
        let dom7 = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        assert_eq!(chord_name("Bb", dom7), "Bb7");
        let m7 = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        assert_eq!(chord_name("Eb", m7), "Ebm7");
        let maj7 = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        assert_eq!(chord_name("Ab", maj7), "Abmaj7");
    }

    #[test]
    fn test_chord_name_major() {
        let maj7 = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        assert_eq!(chord_name("C", maj7), "Cmaj7");
    }

    #[test]
    fn test_chord_name_minor() {
        let m7 = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        assert_eq!(chord_name("D", m7), "Dm7");
    }

    #[test]
    fn test_chord_name_dominant() {
        let dom7 = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        assert_eq!(chord_name("G", dom7), "G7");
        let dom9 = ChordQuality::ALL.iter().find(|q| q.name == "dom9").unwrap();
        assert_eq!(chord_name("G", dom9), "G9");
        let dom13 = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom13")
            .unwrap();
        assert_eq!(chord_name("G", dom13), "G13");
    }

    #[test]
    fn test_chord_name_dominant_sharp5() {
        let dom7s5 = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom7#5")
            .unwrap();
        assert_eq!(chord_name("G", dom7s5), "G7#5");
    }

    #[test]
    fn test_chord_name_new_qualities() {
        let maj7s11 = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "maj7#11")
            .unwrap();
        assert_eq!(chord_name("C", maj7s11), "Cmaj7#11");
        let dom7s11 = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom7#11")
            .unwrap();
        assert_eq!(chord_name("G", dom7s11), "G7#11");
        let dom7b13 = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom7b13")
            .unwrap();
        assert_eq!(chord_name("Ab", dom7b13), "Ab7b13");
    }

    #[test]
    fn test_all_qualities_have_intervals() {
        for q in ChordQuality::ALL {
            assert!(
                !q.intervals.is_empty(),
                "Quality {} has no intervals",
                q.name
            );
            assert!(
                q.intervals[0].semitones == 0,
                "Quality {} does not start with unison",
                q.name
            );
        }
    }
}
