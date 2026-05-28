use std::cmp::Reverse;

use crate::theory::intervals::Interval;

use super::fretboard::Fretboard;
use super::generate::Fingering;
use super::recipe::guide_tones;
use super::voice_set::VoiceSet;

/// Score a single `Fingering` for musical and ergonomic usefulness.
///
/// Higher scores indicate better fingerings. The score combines:
///
/// - **Fret span penalty**: wider spans score lower (`-span`).
/// - **Guide tone bonus**: `+20` if both guide tones (3rd and 7th family)
///   are present in the fingering's intervals.
/// - **Muddy cluster penalty**: `-10` for each pair of notes on the low
///   strings (E, A, D) that are within a minor third (≤ 3 semitones) of
///   each other, including unisons and octaves.
pub fn score(fingering: &Fingering, voice_set: &VoiceSet, fretboard: &Fretboard) -> i32 {
    score_with_crunch(fingering, voice_set, fretboard, false)
}

pub fn score_with_crunch(
    fingering: &Fingering,
    voice_set: &VoiceSet,
    fretboard: &Fretboard,
    prefer_crunch: bool,
) -> i32 {
    let span_score = -(fingering.fret_span() as i32);
    let guide_tone_score = guide_tone_score(fingering, voice_set);
    let muddy_score = muddy_cluster_penalty(fingering, fretboard);
    let consecutive = consecutive_strings_bonus(fingering);
    let root_bass = root_in_bass_bonus(fingering, fretboard);
    let spacing = register_spacing_bonus(fingering, fretboard);
    let crunch = if prefer_crunch {
        crunch_bonus(fingering, fretboard)
    } else {
        0
    };

    span_score + guide_tone_score + muddy_score + consecutive + root_bass + spacing + crunch
}

/// Sort `fingerings` in place by descending score (best first).
///
/// Uses `sort_by` (stable sort), so fingerings with equal scores
/// retain their relative order. The result is deterministic for a
/// given input.
pub fn rank_fingerings(fingerings: &mut [Fingering], voice_set: &VoiceSet, fretboard: &Fretboard) {
    fingerings.sort_by_key(|fingering| Reverse(score(fingering, voice_set, fretboard)));
}

pub fn rank_fingerings_with_crunch(
    fingerings: &mut [Fingering],
    voice_set: &VoiceSet,
    fretboard: &Fretboard,
) {
    fingerings
        .sort_by_key(|fingering| Reverse(score_with_crunch(fingering, voice_set, fretboard, true)));
}

fn guide_tone_score(fingering: &Fingering, voice_set: &VoiceSet) -> i32 {
    let Some((gt1, gt2)) = guide_tones(voice_set.source_quality) else {
        return 0;
    };
    if fingering.has_interval(gt1) && fingering.has_interval(gt2) {
        20
    } else {
        0
    }
}

/// Penalize fingerings with muddy low-register clusters.
///
/// Checks pairs of notes on the low strings (0=E2, 1=A2, 2=D3).
/// If any pair is within a minor third (≤ 3 semitones in MIDI distance),
/// a penalty of `-10` is applied per offending pair.
fn muddy_cluster_penalty(fingering: &Fingering, fretboard: &Fretboard) -> i32 {
    let notes = fingering.notes(fretboard);

    // Collect (string, midi) for played notes on low strings.
    let low_notes: Vec<(usize, i32)> = notes
        .iter()
        .enumerate()
        .take(3) // strings 0, 1, 2 only
        .filter_map(|(s, n)| n.map(|note| (s, note.midi())))
        .collect();

    let mut penalty = 0;
    for i in 0..low_notes.len() {
        for j in (i + 1)..low_notes.len() {
            let dist = (low_notes[i].1 - low_notes[j].1).abs();
            if dist <= 3 {
                penalty -= 10;
            }
        }
    }
    penalty
}

fn consecutive_strings_bonus(fingering: &Fingering) -> i32 {
    let played: Vec<usize> = fingering
        .positions
        .iter()
        .enumerate()
        .filter_map(|(s, p)| p.map(|_| s))
        .collect();
    if played.len() < 2 {
        return 0;
    }
    let mut gaps = 0;
    for i in 1..played.len() {
        if played[i] - played[i - 1] > 1 {
            gaps += 1;
        }
    }
    match gaps {
        0 => 10,
        1 => 3,
        _ => -5,
    }
}

fn root_in_bass_bonus(fingering: &Fingering, fretboard: &Fretboard) -> i32 {
    let notes = fingering.notes(fretboard);
    let lowest = notes.iter().flatten().min();
    let root_iv = fingering
        .intervals
        .iter()
        .enumerate()
        .find(|(_, iv)| iv.is_some_and(|i| i == Interval::UNISON));

    if let (Some(lowest_note), Some((root_s, _))) = (lowest, root_iv) {
        if let Some(root_note) = &notes[root_s] {
            if root_note == lowest_note {
                return 8;
            }
        }
    }
    0
}

fn register_spacing_bonus(fingering: &Fingering, fretboard: &Fretboard) -> i32 {
    let mut midis: Vec<i32> = fingering
        .notes(fretboard)
        .into_iter()
        .flatten()
        .map(|n| n.midi())
        .collect();
    midis.sort();
    if midis.len() < 3 {
        return 0;
    }
    let bottom_gap = midis[1] - midis[0];
    let top_gap = midis[midis.len() - 1] - midis[midis.len() - 2];
    if bottom_gap >= top_gap {
        5
    } else if bottom_gap < top_gap.saturating_sub(5) {
        -3
    } else {
        0
    }
}

fn crunch_bonus(fingering: &Fingering, fretboard: &Fretboard) -> i32 {
    let mut midis: Vec<i32> = fingering
        .notes(fretboard)
        .into_iter()
        .flatten()
        .map(|n| n.midi())
        .collect();
    midis.sort();
    if midis.len() < 2 {
        return 0;
    }
    let mut small_intervals = 0;
    for i in 1..midis.len() {
        let gap = midis[i] - midis[i - 1];
        if (1..=2).contains(&gap) {
            small_intervals += 1;
        }
    }
    match small_intervals {
        1 => 15,
        2 => 8,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::ChordQuality;
    use crate::theory::intervals::Interval;
    use crate::voicings::fretboard::Fretboard;
    use crate::voicings::generate::{map_voice_set, Fingering};
    use crate::voicings::recipe::VoicingRecipe;
    use crate::voicings::rules::VoicingRules;
    use crate::voicings::voice_set::VoiceSet;

    fn setup() -> (Fretboard, VoicingRules, &'static ChordQuality) {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        (fb, rules, quality)
    }

    #[test]
    fn test_smaller_span_ranks_better() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);

        // Compact: frets 8 and 11 on strings 3 and 4 (span = 3)
        let compact = Fingering {
            positions: [None, None, None, Some(8), Some(11), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::M7),
                None,
            ],
        };
        // Wide: frets 1 and 8 on strings 4 and 5 (span = 7)
        let wide = Fingering {
            positions: [None, None, None, None, Some(1), Some(8)],
            intervals: [
                None,
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::M7),
            ],
        };

        let compact_score = score(&compact, &vs, &fb);
        let wide_score = score(&wide, &vs, &fb);

        assert!(
            compact_score > wide_score,
            "compact ({}) should beat wide ({}), span {} vs {}",
            compact_score,
            wide_score,
            compact.fret_span(),
            wide.fret_span()
        );
    }

    #[test]
    fn test_guide_tone_complete_outscores_incomplete() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);

        // Complete: has both M3 and M7 (guide tones for maj7)
        let complete = Fingering {
            positions: [None, None, None, Some(4), Some(8), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::M7),
                None,
            ],
        };
        // Incomplete: has M3 but not M7
        let incomplete = Fingering {
            positions: [None, None, None, Some(4), Some(7), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::P5),
                None,
            ],
        };

        let complete_score = score(&complete, &vs, &fb);
        let incomplete_score = score(&incomplete, &vs, &fb);

        assert!(
            complete_score > incomplete_score,
            "complete ({}) should beat incomplete ({})",
            complete_score,
            incomplete_score
        );
    }

    #[test]
    fn test_muddy_low_cluster_penalized() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);

        // Muddy: A2 on low E string fret 5 and open A string (unison).
        let muddy = Fingering {
            positions: [Some(5), Some(0), None, None, None, None],
            intervals: [
                Some(Interval::M2),
                Some(Interval::M2),
                None,
                None,
                None,
                None,
            ],
        };
        // Clean: A2 on low E string fret 5 and open high E string (far apart).
        let clean = Fingering {
            positions: [Some(5), None, None, None, None, Some(0)],
            intervals: [
                Some(Interval::M2),
                None,
                None,
                None,
                None,
                Some(Interval::M2),
            ],
        };

        let muddy_score = score(&muddy, &vs, &fb);
        let clean_score = score(&clean, &vs, &fb);

        assert!(
            clean_score > muddy_score,
            "clean ({}) should beat muddy ({})",
            clean_score,
            muddy_score
        );
    }

    #[test]
    fn test_ranking_deterministic() {
        let (fb, rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let fingerings = map_voice_set(&vs, &fb, &rules);

        let mut a = fingerings.clone();
        let mut b = fingerings.clone();

        rank_fingerings(&mut a, &vs, &fb);
        rank_fingerings(&mut b, &vs, &fb);

        assert_eq!(a, b, "ranking must be deterministic");
    }

    #[test]
    fn test_rank_fingerings_orders_by_score_descending() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);

        let mut fingerings = vec![
            // Low score: wide span, no guide tones
            Fingering {
                positions: [None, None, None, None, Some(0), Some(5)],
                intervals: [
                    None,
                    None,
                    None,
                    None,
                    Some(Interval::UNISON),
                    Some(Interval::P4),
                ],
            },
            // High score: compact span, both guide tones
            Fingering {
                positions: [None, None, None, Some(4), Some(8), None],
                intervals: [
                    None,
                    None,
                    None,
                    Some(Interval::M3),
                    Some(Interval::M7),
                    None,
                ],
            },
            // Medium score: moderate span, one guide tone
            Fingering {
                positions: [None, None, Some(5), None, Some(9), None],
                intervals: [
                    None,
                    None,
                    Some(Interval::M3),
                    None,
                    Some(Interval::P5),
                    None,
                ],
            },
        ];

        rank_fingerings(&mut fingerings, &vs, &fb);

        // Verify scores are in descending order
        let scores: Vec<i32> = fingerings.iter().map(|f| score(f, &vs, &fb)).collect();
        for i in 0..scores.len() - 1 {
            assert!(
                scores[i] >= scores[i + 1],
                "scores should be descending: {} at {} >= {} at {}",
                scores[i],
                i,
                scores[i + 1],
                i + 1
            );
        }
    }

    #[test]
    fn test_empty_fingerings() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let mut fingerings: Vec<Fingering> = vec![];
        rank_fingerings(&mut fingerings, &vs, &fb);
        assert!(fingerings.is_empty());
    }

    #[test]
    fn test_single_fingering() {
        let (fb, _rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let original = Fingering {
            positions: [None, None, None, Some(4), Some(8), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::M7),
                None,
            ],
        };
        let mut fingerings = vec![original.clone()];
        rank_fingerings(&mut fingerings, &vs, &fb);
        assert_eq!(fingerings.len(), 1);
        assert_eq!(fingerings[0], original);
    }

    #[test]
    fn test_minor_guide_tones() {
        let (fb, _rules, _quality) = setup();
        let m7_quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let vs = VoiceSet::from_quality(0, m7_quality, VoicingRecipe::Closed);

        // Complete: has both m3 and m7 (guide tones for m7)
        let complete = Fingering {
            positions: [None, None, None, Some(3), Some(8), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::m3),
                Some(Interval::m7),
                None,
            ],
        };
        // Incomplete: has m3 but not m7
        let incomplete = Fingering {
            positions: [None, None, None, Some(3), Some(7), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::m3),
                Some(Interval::P5),
                None,
            ],
        };

        let complete_score = score(&complete, &vs, &fb);
        let incomplete_score = score(&incomplete, &vs, &fb);

        assert!(
            complete_score > incomplete_score,
            "complete minor ({}) should beat incomplete minor ({})",
            complete_score,
            incomplete_score
        );
    }

    #[test]
    fn test_dominant_guide_tones() {
        let (fb, _rules, _quality) = setup();
        let dom7_quality = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let vs = VoiceSet::from_quality(0, dom7_quality, VoicingRecipe::Closed);

        // Complete: has both M3 and m7 (guide tones for dom7)
        let complete = Fingering {
            positions: [None, None, None, Some(4), Some(10), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::m7),
                None,
            ],
        };
        // Incomplete: has M3 but not m7
        let incomplete = Fingering {
            positions: [None, None, None, Some(4), Some(7), None],
            intervals: [
                None,
                None,
                None,
                Some(Interval::M3),
                Some(Interval::P5),
                None,
            ],
        };

        let complete_score = score(&complete, &vs, &fb);
        let incomplete_score = score(&incomplete, &vs, &fb);

        assert!(
            complete_score > incomplete_score,
            "complete dominant ({}) should beat incomplete dominant ({})",
            complete_score,
            incomplete_score
        );
    }

    #[test]
    fn test_ranking_integration_with_map_voice_set() {
        let (fb, rules, quality) = setup();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let mut fingerings = map_voice_set(&vs, &fb, &rules);

        let pre_scores: Vec<i32> = fingerings.iter().map(|f| score(f, &vs, &fb)).collect();
        rank_fingerings(&mut fingerings, &vs, &fb);
        let post_scores: Vec<i32> = fingerings.iter().map(|f| score(f, &vs, &fb)).collect();

        // Same scores, just reordered
        assert_eq!(pre_scores.len(), post_scores.len());

        // Post-ranking scores are in descending order
        for i in 0..post_scores.len().saturating_sub(1) {
            assert!(
                post_scores[i] >= post_scores[i + 1],
                "post-ranking scores must be descending"
            );
        }

        // A sorted version of pre_scores equals post_scores
        let mut sorted_pre = pre_scores.clone();
        sorted_pre.sort_by(|a, b| b.cmp(a));
        assert_eq!(
            sorted_pre, post_scores,
            "ranking should produce a score-sorted permutation"
        );
    }
}
