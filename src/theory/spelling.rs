use crate::theory::chords::ChordQuality;
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;

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

/// Spell every pitch class of `scale` over a chord, as notation reads it.
///
/// Indexed by pitch class; `None` for pitch classes the scale does not contain.
/// The `octave` field is a placeholder here — `spell_midi` fills it from a real pitch.
pub fn spell_scale(
    root_written: &str,
    quality: &ChordQuality,
    scale: &Scale,
) -> [Option<Spelled>; 12] {
    let (root_step, root_alter) = parse_root(root_written);
    let root_pc = (NATURAL_PC[root_step as usize] as i16 + root_alter as i16).rem_euclid(12) as u8;

    let mut table: [Option<Spelled>; 12] = [None; 12];

    for &semi in &scale.semitones {
        let pc = (root_pc + semi) % 12;
        if table[pc as usize].is_some() {
            continue; // a scale listing the same pitch class twice needs only one spelling
        }
        let step = (root_step + letter_offset(semi, quality)) % 7;
        table[pc as usize] = Some(Spelled {
            step,
            alter: alter_for(step, pc),
            octave: 0,
        });
    }
    table
}

/// The alteration that turns `step`'s natural pitch class into `pc`, on the short side
/// of the octave: pc 9 against letter B is -2 (Bbb), not +10.
fn alter_for(step: u8, pc: u8) -> i8 {
    let mut alter = pc as i16 - NATURAL_PC[step as usize] as i16;
    if alter > 5 {
        alter -= 12;
    } else if alter < -6 {
        alter += 12;
    }
    alter as i8
}

/// Spell one sounding pitch using a table from `spell_scale`.
///
/// A pitch class the scale does not contain falls back to the jazz default names in
/// `PC_NAMES` (flats for Db/Eb/Ab/Bb, sharps for C#/F#). Never panics.
pub fn spell_midi(table: &[Option<Spelled>; 12], midi: i32) -> Spelled {
    let pc = midi.rem_euclid(12) as usize;
    let (step, alter) = match table[pc] {
        Some(s) => (s.step, s.alter),
        None => fallback_spelling(pc),
    };
    // Subtract the alteration first so the octave follows the LETTER: Cb4 sounds at
    // MIDI 59 but is a C, and B#3 sounds at MIDI 60 but is a B.
    let octave = (midi - alter as i32).div_euclid(12) - 1;
    Spelled {
        step,
        alter,
        octave: octave as i8,
    }
}

/// Jazz default spelling for a pitch class the chord-scale does not contain.
fn fallback_spelling(pc: usize) -> (u8, i8) {
    let name = PC_NAMES[pc];
    let mut chars = name.chars();
    let step = match chars.next() {
        Some('C') => 0,
        Some('D') => 1,
        Some('E') => 2,
        Some('F') => 3,
        Some('G') => 4,
        Some('A') => 5,
        _ => 6,
    };
    let alter = match chars.next() {
        Some('#') => 1,
        Some('b') => -1,
        _ => 0,
    };
    (step, alter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    fn scale(name: &str) -> &'static Scale {
        Scale::ALL.iter().find(|s| s.name == name).unwrap()
    }

    /// Render a spelled table as ascending note names from the root, for readable asserts.
    fn spell_names(root: &str, quality_name: &str, scale_name: &str) -> Vec<String> {
        const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
        let q = quality(quality_name);
        let s = scale(scale_name);
        let table = spell_scale(root, q, s);
        let (root_step, root_alter) = parse_root(root);
        let root_pc =
            (NATURAL_PC[root_step as usize] as i16 + root_alter as i16).rem_euclid(12) as u8;
        s.semitones
            .iter()
            .map(|&semi| {
                let pc = (root_pc + semi) % 12;
                let sp = table[pc as usize].expect("scale tone must be spelled");
                let acc = match sp.alter {
                    -2 => "bb",
                    -1 => "b",
                    0 => "",
                    1 => "#",
                    2 => "##",
                    _ => "?",
                };
                format!("{}{}", LETTERS[sp.step as usize], acc)
            })
            .collect()
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

    #[test]
    fn ionian_over_maj7_has_no_accidentals() {
        assert_eq!(
            spell_names("C", "maj7", "Ionian"),
            ["C", "D", "E", "F", "G", "A", "B"]
        );
    }

    #[test]
    fn altered_over_g7_reads_functionally() {
        // The case that killed the scale-degree algorithm: letter A is used twice
        // (Ab and A#) and letter D is not used at all. Third on B, #11 on C#.
        assert_eq!(
            spell_names("G", "dom7", "Altered"),
            ["G", "Ab", "A#", "B", "C#", "Eb", "F"]
        );
    }

    #[test]
    fn altered_reuses_a_letter_rather_than_forcing_a_bijection() {
        // The regression guard for the bug this design exists to avoid: a letter may
        // carry two pitches, and forcing seven notes onto seven distinct letters would
        // push the #9 onto B and spell it Bb.
        let names = spell_names("G", "dom7", "Altered");
        assert_eq!(
            names.iter().filter(|n| n.starts_with('A')).count(),
            2,
            "{:?}",
            names
        );
        assert!(!names.iter().any(|n| n.starts_with('D')), "{:?}", names);
        assert!(
            !names.contains(&"Bb".to_string()),
            "the #9 must be A#, got {:?}",
            names
        );
    }

    #[test]
    fn dim7_keeps_its_double_flat_seventh() {
        // The chord's b3 already speaks for the third, so semitone 4 is the mode's
        // natural fourth: Fb. And the bb7 survives as a genuine double flat.
        assert_eq!(
            spell_names("C", "dim7", "Locrian \u{266D}\u{266D}7"),
            ["C", "Db", "Eb", "Fb", "Gb", "Ab", "Bbb"]
        );
    }

    #[test]
    fn chart_root_spelling_decides_sharps_versus_flats() {
        assert_eq!(spell_names("C#", "dom7", "Mixolydian")[0], "C#");
        assert_eq!(spell_names("Db", "dom7", "Mixolydian")[0], "Db");
    }

    #[test]
    fn m7b5_spells_its_flat_five_on_the_fifth_letter() {
        // Cm7b5 + Locrian: semitone 6 is the chord's b5 -> Gb, never F#.
        let names = spell_names("C", "m7b5", "Locrian");
        assert!(names.contains(&"Gb".to_string()), "got {:?}", names);
        assert!(!names.contains(&"F#".to_string()), "got {:?}", names);
    }

    #[test]
    fn dom7_sharp5_spells_its_raised_fifth_on_the_fifth_letter() {
        // G7#5 + Altered: semitone 8 is the chord's #5 -> D#, not Eb.
        let names = spell_names("G", "dom7#5", "Altered");
        assert!(names.contains(&"D#".to_string()), "got {:?}", names);
    }

    #[test]
    fn every_quality_and_scale_pair_spells_without_panicking() {
        for quality in ChordQuality::ALL {
            for scale in Scale::ALL {
                for root in ["C", "F#", "Bb", "Eb", "A"] {
                    let table = spell_scale(root, quality, scale);
                    let (rs, ra) = parse_root(root);
                    let root_pc = (NATURAL_PC[rs as usize] as i16 + ra as i16).rem_euclid(12) as u8;
                    for &semi in &scale.semitones {
                        let pc = (root_pc + semi) % 12;
                        let sp = table[pc as usize].unwrap_or_else(|| {
                            panic!(
                                "{} {} {} left pc {} unspelled",
                                root, quality.name, scale.name, pc
                            )
                        });
                        assert!(
                            sp.alter.abs() <= 2,
                            "{} {} {} spelled pc {} with alter {}",
                            root,
                            quality.name,
                            scale.name,
                            pc,
                            sp.alter
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn octave_comes_from_middle_c() {
        let table = spell_scale("C", quality("maj7"), scale("Ionian"));
        // MIDI 60 is middle C = C4.
        let c4 = spell_midi(&table, 60);
        assert_eq!((c4.step, c4.alter, c4.octave), (0, 0, 4));
        // MIDI 40 is the guitar's open low E = E2.
        let e2 = spell_midi(&table, 40);
        assert_eq!((e2.step, e2.alter, e2.octave), (2, 0, 2));
    }

    #[test]
    fn octave_follows_the_letter_not_the_midi_division() {
        // Cb4 sounds at MIDI 59, which MIDI-divides into octave 3. As a C it is
        // still octave 4. Build a table that spells pc 11 as Cb.
        let mut table: [Option<Spelled>; 12] = [None; 12];
        table[11] = Some(Spelled {
            step: 0,
            alter: -1,
            octave: 0,
        });
        let cb = spell_midi(&table, 59);
        assert_eq!((cb.step, cb.alter, cb.octave), (0, -1, 4));

        // B#3 sounds at MIDI 60, which MIDI-divides into octave 4. As a B it is octave 3.
        let mut table: [Option<Spelled>; 12] = [None; 12];
        table[0] = Some(Spelled {
            step: 6,
            alter: 1,
            octave: 0,
        });
        let bs = spell_midi(&table, 60);
        assert_eq!((bs.step, bs.alter, bs.octave), (6, 1, 3));
    }

    #[test]
    fn pitch_class_outside_the_scale_falls_back_without_panicking() {
        let table = spell_scale("C", quality("maj7"), scale("Ionian"));
        // Ionian on C has no pc 6; PC_NAMES calls it F#.
        let fs = spell_midi(&table, 66);
        assert_eq!((fs.step, fs.alter, fs.octave), (3, 1, 4));
    }
}
