use crate::theory::chords::ChordQuality;

/// A pitch spelled as notation needs it: letter, accidental, octave.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Spelled {
    /// 0=C, 1=D, 2=E, 3=F, 4=G, 5=A, 6=B.
    pub step: u8,
    /// -2 = double flat, -1 = flat, 0 = natural, 1 = sharp, 2 = double sharp.
    pub alter: i8,
    /// Scientific pitch notation of the SOUNDING pitch; middle C is C4. The
    /// treble-8vb transposition is the renderer's job, not this module's.
    pub octave: i8,
}

/// Pitch class of each unaltered letter, indexed by `step`.
pub(crate) const NATURAL_PC: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];

/// Split a chart root like "Bb" or "F#" into `(step, alter)`.
pub(crate) fn parse_root(root: &str) -> (u8, i8) {
    let mut chars = root.chars();
    let step = match chars.next() {
        Some('C') => 0,
        Some('D') => 1,
        Some('E') => 2,
        Some('F') => 3,
        Some('G') => 4,
        Some('A') => 5,
        Some('B') => 6,
        _ => return (0, 0),
    };
    let alter = match chars.next() {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    (step, alter)
}

/// Which letter above the root a scale tone claims, given its distance in semitones
/// and the chord it sits over.
///
/// Chord tones anchor their own letter; the ambiguous distances (3, 6, 8, 9) are
/// decided by what the chord actually contains. The comparison is plain equality on
/// `semitones` — NOT modulo 12 — because `ChordQuality::intervals` stores tensions as
/// compound intervals (`SHARP9` is 15, `SHARP11` is 18, `m13` is 20). Reducing them
/// would make a #9 masquerade as a b3 and spell G7#9's A# as Bb.
pub(crate) fn letter_offset(semitones: u8, quality: &ChordQuality) -> u8 {
    let is_chord_tone = |s: u8| quality.intervals.iter().any(|i| i.semitones == s);
    match semitones {
        0 => 0,
        1 | 2 => 1, // b9 / 9
        3 => {
            if is_chord_tone(3) {
                2
            } else {
                1
            }
        } // b3 (chord tone) else #9
        // Over a chord that already owns a minor third, semitone 4 is not "the third" —
        // it is the natural fourth of the mode. C dim7 + Locrian bb7 reads Eb then Fb.
        4 => {
            if is_chord_tone(3) {
                3
            } else {
                2
            }
        } // 4 (over a minor third) else 3
        5 => 3, // 11
        6 => {
            if is_chord_tone(6) {
                4
            } else {
                3
            }
        } // b5 (chord tone) else #11
        7 => 4, // 5
        8 => {
            if is_chord_tone(8) {
                4
            } else {
                5
            }
        } // #5 (chord tone) else b13
        9 => {
            if is_chord_tone(9) {
                6
            } else {
                5
            }
        } // bb7 (dim7 chord tone) else 13
        10 | 11 => 6, // b7 / maj7
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn parses_chart_roots() {
        assert_eq!(parse_root("C"), (0, 0));
        assert_eq!(parse_root("Bb"), (6, -1));
        assert_eq!(parse_root("F#"), (3, 1));
        assert_eq!(parse_root("A"), (5, 0));
    }

    #[test]
    fn unparseable_root_falls_back_to_c_natural() {
        assert_eq!(parse_root(""), (0, 0));
        assert_eq!(parse_root("H"), (0, 0));
    }

    #[test]
    fn chord_tones_anchor_their_own_letter() {
        // m7 has a real minor third (3 semitones) -> letter+2.
        assert_eq!(letter_offset(3, quality("m7")), 2);
        // m7b5 has a real tritone -> letter+4, the flat fifth.
        assert_eq!(letter_offset(6, quality("m7b5")), 4);
        // dom7#5 has a real minor sixth -> letter+4, the sharp fifth.
        assert_eq!(letter_offset(8, quality("dom7#5")), 4);
        // dim7 has a real diminished seventh -> letter+6, the bb7.
        assert_eq!(letter_offset(9, quality("dim7")), 6);
    }

    #[test]
    fn tensions_do_not_anchor_letters_despite_compound_semitones() {
        // Interval::SHARP9.semitones == 15, and 15 % 12 == 3. A modulo comparison
        // would make dom7#9 claim a minor third and spell its #9 as Bb. It must not.
        assert_eq!(
            letter_offset(3, quality("dom7#9")),
            1,
            "#9 belongs on the 9th's letter"
        );
        // SHARP11 == 18, 18 % 12 == 6.
        assert_eq!(
            letter_offset(6, quality("dom7#11")),
            3,
            "#11 belongs on the 11th's letter"
        );
        // m13 == 20, 20 % 12 == 8.
        assert_eq!(
            letter_offset(8, quality("dom7b13")),
            5,
            "b13 belongs on the 13th's letter"
        );
        // M13 == 21, 21 % 12 == 9.
        assert_eq!(
            letter_offset(9, quality("dom13")),
            5,
            "13 belongs on the 13th's letter"
        );
    }

    #[test]
    fn semitone_four_moves_off_the_third_when_the_chord_owns_a_minor_third() {
        // Over dim7 the chord's b3 already speaks for the third, so semitone 4 is the
        // mode's natural fourth: C dim7 + Locrian bb7 reads Eb then Fb, never Eb then E.
        assert_eq!(letter_offset(4, quality("dim7")), 3);
        assert_eq!(letter_offset(4, quality("m7")), 3);
        // Over a dominant or major chord it is the third, on the third's letter.
        assert_eq!(letter_offset(4, quality("dom7")), 2);
        assert_eq!(letter_offset(4, quality("maj7")), 2);
    }

    #[test]
    fn plain_dominant_takes_the_altered_tension_readings() {
        let dom7 = quality("dom7");
        assert_eq!(letter_offset(0, dom7), 0);
        assert_eq!(letter_offset(1, dom7), 1); // b9
        assert_eq!(letter_offset(3, dom7), 1); // #9, not b3
        assert_eq!(letter_offset(4, dom7), 2); // 3
        assert_eq!(letter_offset(6, dom7), 3); // #11, not b5
        assert_eq!(letter_offset(8, dom7), 5); // b13, not #5
        assert_eq!(letter_offset(10, dom7), 6); // b7
    }
}
