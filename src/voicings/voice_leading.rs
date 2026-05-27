use super::fretboard::Fretboard;
use super::generate::Fingering;

/// Measure how much hand movement is required to transition between two fingerings.
///
/// Lower scores mean smoother voice leading. The metric combines:
///
/// - **Fret movement per string**: sum of absolute fret differences on strings
///   that are played in both fingerings.
/// - **String change penalty**: penalty for strings that change from played
///   to muted or vice versa, since that changes the picking pattern.
/// - **Common tone bonus**: negative offset when both fingerings produce the
///   same pitch on the same string (the finger stays put).
///
/// Returns 0 for identical fingerings.
pub fn distance(from: &Fingering, to: &Fingering, fretboard: &Fretboard) -> u32 {
    let from_notes = from.notes(fretboard);
    let to_notes = to.notes(fretboard);

    let mut total: i32 = 0;

    for string in 0..6 {
        match (from.positions[string], to.positions[string]) {
            (Some(f1), Some(f2)) => {
                let fret_delta = (f1 as i32 - f2 as i32).unsigned_abs();
                total += fret_delta as i32;

                // Bonus for common tones (same pitch on the same string).
                if let (Some(n1), Some(n2)) = (from_notes[string], to_notes[string]) {
                    if n1.pitch_class == n2.pitch_class {
                        total -= 2;
                    }
                }
            }
            (Some(_), None) | (None, Some(_)) => {
                total += 3;
            }
            (None, None) => {}
        }
    }

    total.max(0) as u32
}

/// Given a set of candidate fingerings for the next chord, find the one
/// closest to `current` by voice leading distance.
///
/// Returns `None` if `candidates` is empty.
pub fn nearest(
    current: &Fingering,
    candidates: &[Fingering],
    fretboard: &Fretboard,
) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| distance(current, candidate, fretboard))
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::intervals::Interval;
    use crate::voicings::fretboard::Fretboard;

    fn fb() -> Fretboard {
        Fretboard::standard_tuning()
    }

    #[test]
    fn identical_fingerings_have_zero_distance() {
        let f = Fingering {
            positions: [None, Some(3), Some(2), Some(0), Some(0), Some(0)],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                Some(Interval::M7),
                Some(Interval::M3),
            ],
        };
        assert_eq!(distance(&f, &f, &fb()), 0);
    }

    #[test]
    fn small_fret_shift_has_low_distance() {
        // Cmaj7 shape at fret 3 vs fret 5 (same shape, shifted 2 frets).
        let a = Fingering {
            positions: [None, Some(3), Some(2), Some(0), Some(0), None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                Some(Interval::M7),
                None,
            ],
        };
        let b = Fingering {
            positions: [None, Some(5), Some(4), Some(2), Some(2), None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                Some(Interval::M7),
                None,
            ],
        };
        let d = distance(&a, &b, &fb());
        assert_eq!(d, 8); // 4 strings × 2 frets each
    }

    #[test]
    fn common_tones_reduce_distance() {
        let fretboard = fb();
        // Two fingerings that share a common pitch on string 4.
        let a = Fingering {
            positions: [None, None, None, None, Some(5), None],
            intervals: [None, None, None, None, Some(Interval::M3), None],
        };
        // Same string, same fret = same pitch → common tone bonus.
        let b_same = Fingering {
            positions: [None, None, None, None, Some(5), None],
            intervals: [None, None, None, None, Some(Interval::P5), None],
        };
        // Same string, different fret.
        let b_diff = Fingering {
            positions: [None, None, None, None, Some(7), None],
            intervals: [None, None, None, None, Some(Interval::P5), None],
        };

        assert!(distance(&a, &b_same, &fretboard) < distance(&a, &b_diff, &fretboard));
    }

    #[test]
    fn string_change_penalty() {
        let fretboard = fb();
        // Fingering on strings 0-1.
        let a = Fingering {
            positions: [Some(3), Some(5), None, None, None, None],
            intervals: [
                Some(Interval::UNISON),
                Some(Interval::M3),
                None,
                None,
                None,
                None,
            ],
        };
        // Fingering on strings 4-5 (completely different string set).
        let b = Fingering {
            positions: [None, None, None, None, Some(8), Some(7)],
            intervals: [
                None,
                None,
                None,
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
            ],
        };
        let d = distance(&a, &b, &fretboard);
        // Strings 0,1: played→muted = +3 each. Strings 4,5: muted→played = +3 each.
        assert_eq!(d, 12, "4 string changes × 3 penalty each = 12, got {}", d);
    }

    #[test]
    fn nearest_finds_closest() {
        let fretboard = fb();
        let current = Fingering {
            positions: [None, Some(3), Some(2), Some(0), None, None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                None,
                None,
            ],
        };
        let far = Fingering {
            positions: [None, Some(10), Some(9), Some(7), None, None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                None,
                None,
            ],
        };
        let near = Fingering {
            positions: [None, Some(5), Some(4), Some(2), None, None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                None,
                None,
            ],
        };

        let candidates = vec![far, near];
        let idx = nearest(&current, &candidates, &fretboard).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn nearest_returns_none_for_empty() {
        let fretboard = fb();
        let current = Fingering {
            positions: [None, Some(3), None, None, None, None],
            intervals: [None, Some(Interval::UNISON), None, None, None, None],
        };
        assert!(nearest(&current, &[], &fretboard).is_none());
    }

    #[test]
    fn distance_is_symmetric() {
        let fretboard = fb();
        let a = Fingering {
            positions: [None, Some(3), Some(2), Some(0), Some(0), None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::M3),
                Some(Interval::P5),
                Some(Interval::M7),
                None,
            ],
        };
        let b = Fingering {
            positions: [None, Some(5), Some(5), Some(4), Some(5), None],
            intervals: [
                None,
                Some(Interval::UNISON),
                Some(Interval::m3),
                Some(Interval::P5),
                Some(Interval::m7),
                None,
            ],
        };
        assert_eq!(distance(&a, &b, &fretboard), distance(&b, &a, &fretboard));
    }
}
