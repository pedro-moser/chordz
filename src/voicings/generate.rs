use std::collections::HashSet;

use crate::theory::chords::ChordQuality;
use crate::theory::notes::Note;

use super::fretboard::Fretboard;
use super::rules::VoicingRules;

/// A single chord voicing on the fretboard.
/// `positions[i]` is `Some(fret)` for string i, or `None` if muted.
/// `intervals[i]` is the index into the chord quality's interval list for that string.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Voicing {
    pub positions: [Option<u8>; 6],
    pub intervals: [Option<usize>; 6],
}

impl Voicing {
    /// The bass note (lowest-pitched played string).
    pub fn bass_note(&self, fretboard: &Fretboard) -> Note {
        self.positions
            .iter()
            .enumerate()
            .filter_map(|(s, f)| f.map(|fret| (s, fret)))
            .map(|(s, f)| fretboard.get_note(s, f as usize))
            .min()
            .unwrap()
    }

    /// How many strings are played (non-muted).
    pub fn played_count(&self) -> usize {
        self.positions.iter().filter(|f| f.is_some()).count()
    }

    /// The (string, fret) of the root note, if present.
    pub fn root_position(&self) -> Option<(usize, u8)> {
        for (s, &fret_opt) in self.positions.iter().enumerate() {
            if let Some(fret) = fret_opt {
                if self.intervals[s] == Some(0) {
                    return Some((s, fret));
                }
            }
        }
        None
    }
}

/// Generate all valid voicings for a given root and chord quality.
pub fn generate(
    root_pc: u8,
    quality: &ChordQuality,
    fretboard: &Fretboard,
    rules: &VoicingRules,
) -> Vec<Voicing> {
    let num_strings = fretboard.num_strings();

    // Compute the pitch class for each interval in the chord.
    let pc: Vec<u8> = quality
        .intervals
        .iter()
        .map(|iv| (root_pc as i32 + iv.semitones as i32) as u8 % 12)
        .collect();

    let mut result = HashSet::new();

    // Iterate over possible base fret positions.
    for base_fret in 0..=rules.max_fret as usize {
        let max_fret = (base_fret + rules.max_fret_span as usize).min(rules.max_fret as usize);

        // For each string, find (fret, interval_index) options within [base_fret, max_fret].
        let mut string_options: Vec<Vec<(u8, usize)>> = Vec::with_capacity(num_strings);
        for s in 0..num_strings {
            let mut opts = Vec::new();
            for fret in base_fret..=max_fret {
                let note = fretboard.get_note(s, fret);
                for (ii, &target_pc) in pc.iter().enumerate() {
                    if note.pitch_class == target_pc {
                        opts.push((fret as u8, ii));
                    }
                }
            }
            string_options.push(opts);
        }

        // Backtrack: assign one interval at a time to an unused string.
        let mut used_string = [false; 6];
        let mut positions = [None; 6];
        let mut intervals = [None; 6];

        fn backtrack(
            interval_idx: usize,
            pc: &[u8],
            string_options: &[Vec<(u8, usize)>],
            used_string: &mut [bool; 6],
            positions: &mut [Option<u8>; 6],
            intervals: &mut [Option<usize>; 6],
            result: &mut HashSet<Voicing>,
        ) {
            if interval_idx == pc.len() {
                result.insert(Voicing {
                    positions: *positions,
                    intervals: *intervals,
                });
                return;
            }

            for s in 0..6 {
                if used_string[s] {
                    continue;
                }
                for &(fret, ii) in &string_options[s] {
                    if ii == interval_idx {
                        used_string[s] = true;
                        positions[s] = Some(fret);
                        intervals[s] = Some(ii);
                        backtrack(
                            interval_idx + 1,
                            pc,
                            string_options,
                            used_string,
                            positions,
                            intervals,
                            result,
                        );
                        used_string[s] = false;
                        positions[s] = None;
                        intervals[s] = None;
                    }
                }
            }
        }

        backtrack(
            0,
            &pc,
            &string_options,
            &mut used_string,
            &mut positions,
            &mut intervals,
            &mut result,
        );
    }

    let mut voicings: Vec<Voicing> = result.into_iter().collect();
    voicings.sort_by(|a, b| a.positions.cmp(&b.positions));
    voicings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::{self, ChordQuality};

    #[test]
    fn test_maj7_has_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let voicings = generate(0, quality, &fb, &rules); // C
        assert!(
            voicings.len() > 0,
            "Cmaj7 should have voicings, got {}",
            voicings.len()
        );
    }

    #[test]
    fn test_m7_has_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let voicings = generate(0, quality, &fb, &rules); // Cm7
        assert!(
            voicings.len() > 0,
            "Cm7 should have voicings, got {}",
            voicings.len()
        );
    }

    #[test]
    fn test_dom7_has_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let voicings = generate(4, quality, &fb, &rules); // E dom7
        assert!(
            voicings.len() > 0,
            "E7 should have voicings, got {}",
            voicings.len()
        );
    }

    #[test]
    fn test_voicing_root_present() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let voicings = generate(0, quality, &fb, &rules); // Cmaj7
        for v in &voicings {
            assert!(
                v.root_position().is_some(),
                "Voicing {:?} missing root",
                v.positions
            );
        }
    }

    #[test]
    fn test_voicing_fret_span() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let voicings = generate(0, quality, &fb, &rules);
        for v in &voicings {
            assert!(
                rules.validate(&v.positions),
                "Voicing {:?} violates rules",
                v.positions
            );
        }
    }

    #[test]
    fn test_voicing_string_count() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let voicings = generate(0, quality, &fb, &rules);
        for v in &voicings {
            let count = v.played_count();
            assert!(
                count >= rules.min_strings as usize && count <= rules.max_strings as usize,
                "Voicing {:?} has {} strings (expected {}-{})",
                v.positions,
                count,
                rules.min_strings,
                rules.max_strings
            );
        }
    }

    #[test]
    fn test_all_qualities_produce_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        for quality in ChordQuality::ALL {
            let voicings = generate(0, quality, &fb, &rules);
            assert!(
                voicings.len() > 0,
                "Quality '{}' produced no voicings",
                quality.name
            );
        }
    }

    #[test]
    fn test_all_roots_produce_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        for root_name in chords::ROOTS {
            let root_pc = chords::root_to_pc(root_name);
            let voicings = generate(root_pc, quality, &fb, &rules);
            assert!(
                voicings.len() > 0,
                "{}maj7 produced no voicings",
                root_name
            );
        }
    }

    #[test]
    fn test_bass_note() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let voicings = generate(0, quality, &fb, &rules);
        for v in &voicings {
            let bass = v.bass_note(&fb);
            // Bass note should be the lowest-pitched played string
            let lowest = v.positions
                .iter()
                .enumerate()
                .filter_map(|(s, f)| f.map(|fret| fb.get_note(s, fret as usize)))
                .min()
                .unwrap();
            assert_eq!(bass, lowest);
        }
    }

    #[test]
    fn test_no_duplicate_voicings() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let voicings = generate(0, quality, &fb, &rules);
        // Check no duplicate position arrays
        let mut seen = HashSet::new();
        for v in &voicings {
            assert!(
                seen.insert(v.positions),
                "Duplicate voicing: {:?}",
                v.positions
            );
        }
    }
}
