use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

use super::recipe::{required_intervals_for_reduction, VoicingRecipe};
use super::stability::{self, get_stability_table, has_duplicate_degree, subset_stability};
use super::voice_set::VoiceSet;

const INTERVAL_B5: Interval = Interval {
    semitones: 6,
    name: "b5",
};

const INTERVAL_DIM7: Interval = Interval {
    semitones: 9,
    name: "dim7",
};

/// Map a semitone (0-11) to the correct Interval for the given quality.
fn semitone_to_interval(semitone: u8, quality: &ChordQuality) -> Interval {
    let s = semitone % 12;

    // Preserve the function declared by the quality instead of collapsing
    // extensions to their simple enharmonic interval (9 -> 2, b13 -> b6).
    if let Some(interval) = quality
        .intervals
        .iter()
        .find(|interval| interval.semitones >= 12 && interval.semitones % 12 == s)
    {
        return *interval;
    }
    if s == 8 && quality.intervals.contains(&Interval::SHARP5) {
        return Interval::SHARP5;
    }

    match s {
        0 => Interval::UNISON,
        1 => Interval::m2,
        2 => Interval::M2,
        3 => {
            let class = stability::degree_class(s, quality);
            if class == 2 { Interval::SHARP9 } else { Interval::m3 }
        }
        4 => {
            if quality.intervals.contains(&Interval::m3)
                && quality.intervals.contains(&Interval::m11)
            {
                Interval::m11
            } else {
                Interval::M3
            }
        }
        5 => Interval::P4,
        6 => {
            let class = stability::degree_class(s, quality);
            if class == 5 { INTERVAL_B5 } else { Interval::tritone }
        }
        7 => Interval::P5,
        8 => Interval::m6,
        9 => {
            let class = stability::degree_class(s, quality);
            if class == 7 { INTERVAL_DIM7 } else { Interval::M6 }
        }
        10 => Interval::m7,
        11 => Interval::M7,
        _ => unreachable!("semitone must be 0-11"),
    }
}

/// Label for a voicing transform.
fn transform_label(transform: Transform) -> &'static str {
    match transform {
        Transform::Close => "closed",
        Transform::Drop2 => "drop2",
        Transform::Drop3 => "drop3",
        Transform::Drop2And3 => "drop2&3",
    }
}

/// Map a transform label to a `VoicingRecipe`.
fn label_to_recipe(label: &str) -> VoicingRecipe {
    match label {
        "closed" => VoicingRecipe::Closed,
        "drop2" => VoicingRecipe::Drop2,
        "drop3" => VoicingRecipe::Drop3,
        // No enum variant for drop2&3 yet; use Closed as fallback.
        "drop2&3" => VoicingRecipe::Closed,
        _ => VoicingRecipe::Closed,
    }
}

#[derive(Clone, Copy)]
enum Transform {
    Close,
    Drop2,
    Drop3,
    Drop2And3,
}

/// Generate all C(n, k) combinations of `items`, calling `f` for each.
fn combinations<T: Copy>(items: &[T], k: usize, f: &mut impl FnMut(&[T])) {
    let mut buf = Vec::with_capacity(k);
    combinations_rec(items, k, 0, &mut buf, f);
}

fn combinations_rec<T: Copy>(
    items: &[T],
    k: usize,
    start: usize,
    buf: &mut Vec<T>,
    f: &mut impl FnMut(&[T]),
) {
    if buf.len() == k {
        f(buf);
        return;
    }
    let remaining = k - buf.len();
    for i in start..=(items.len() - remaining) {
        buf.push(items[i]);
        combinations_rec(items, k, i + 1, buf, f);
        buf.pop();
    }
}

/// Apply close-position inversion `inv` to `sorted_semitones`.
///
/// Returns `(intervals, octave_offsets)` where the first `inv` semitones
/// are rotated to the end and placed in the next octave.
fn close_inversion(
    sorted_semitones: &[u8],
    inv: usize,
    quality: &ChordQuality,
) -> (Vec<Interval>, Vec<i32>) {
    let n = sorted_semitones.len();
    let mut intervals = Vec::with_capacity(n);
    let mut octaves = Vec::with_capacity(n);
    for i in 0..n {
        let idx = (inv + i) % n;
        intervals.push(semitone_to_interval(sorted_semitones[idx], quality));
        // Notes that wrapped around (original index < inv) get octave +1.
        if idx < inv {
            octaves.push(1);
        } else {
            octaves.push(0);
        }
    }
    (intervals, octaves)
}

/// Apply a drop transform to a close-position voicing.
///
/// `drop_indices` are positions (from the top) to drop to the bottom with
/// octave -= 1. For drop2, that's `[1]` (2nd from top); for drop3, `[2]`;
/// for drop2&3, `[1, 2]`.
fn apply_drop(
    intervals: &[Interval],
    octaves: &[i32],
    drop_positions_from_top: &[usize],
) -> (Vec<Interval>, Vec<i32>) {
    let n = intervals.len();

    // Convert "from top" positions to actual indices.
    let mut drop_indices: Vec<usize> = drop_positions_from_top
        .iter()
        .map(|&pos| n - 1 - pos)
        .collect();
    drop_indices.sort();

    // Extract the dropped voices (in order, lowest index first).
    let mut dropped_intervals = Vec::new();
    let mut dropped_octaves = Vec::new();
    for &idx in &drop_indices {
        dropped_intervals.push(intervals[idx]);
        dropped_octaves.push(octaves[idx] - 1);
    }

    // Remaining voices (those not dropped).
    let mut remaining_intervals = Vec::new();
    let mut remaining_octaves = Vec::new();
    for i in 0..n {
        if !drop_indices.contains(&i) {
            remaining_intervals.push(intervals[i]);
            remaining_octaves.push(octaves[i]);
        }
    }

    // Result: dropped voices at front, then remaining.
    let mut result_intervals = dropped_intervals;
    result_intervals.extend(remaining_intervals);
    let mut result_octaves = dropped_octaves;
    result_octaves.extend(remaining_octaves);

    // Normalize octaves so the minimum is 0.
    let min_oct = *result_octaves.iter().min().unwrap();
    if min_oct != 0 {
        for oct in &mut result_octaves {
            *oct -= min_oct;
        }
    }

    (result_intervals, result_octaves)
}

/// Generate all procedural voice sets for a chord quality.
///
/// Enumerates all C(available, note_count) subsets of semitones that pass the
/// stability threshold, then applies close, drop2, drop3, and drop2&3 transforms
/// (with all inversions) to produce a comprehensive vocabulary of `VoiceSet`s.
///
/// Returns `(VoiceSet, stability_score, transform_label)` sorted by stability
/// descending.
pub fn generate_all_voice_sets(
    root_pc: u8,
    quality: &'static ChordQuality,
    note_count: usize,
    next_quality: Option<&'static ChordQuality>,
    min_total_stability: u8,
) -> Vec<(VoiceSet, u16, &'static str)> {
    generate_all_voice_sets_with_abstraction(
        root_pc,
        quality,
        note_count,
        next_quality,
        min_total_stability,
        0.0,
    )
}

/// Generate grounded voice sets for a standalone chord browser.
///
/// Unlike the progression solver, a browser has no harmonic context that can
/// justify adding color tones from outside the displayed quality. Keeping the
/// vocabulary literal prevents two quality labels from publishing the same
/// pitch set while the contextual generator remains free to add tensions.
pub fn generate_literal_voice_sets(
    root_pc: u8,
    quality: &'static ChordQuality,
    note_count: usize,
    min_total_stability: u8,
) -> Vec<(VoiceSet, u16, &'static str)> {
    let declared_pitch_classes: Vec<u8> = quality
        .intervals
        .iter()
        .map(|interval| interval.semitones % 12)
        .collect();

    generate_all_voice_sets(root_pc, quality, note_count, None, min_total_stability)
        .into_iter()
        .filter(|(voice_set, _, _)| {
            voice_set
                .intervals
                .iter()
                .all(|interval| declared_pitch_classes.contains(&(interval.semitones % 12)))
        })
        .collect()
}

pub fn generate_all_voice_sets_with_abstraction(
    root_pc: u8,
    quality: &'static ChordQuality,
    note_count: usize,
    next_quality: Option<&'static ChordQuality>,
    min_total_stability: u8,
    abstraction: f32,
) -> Vec<(VoiceSet, u16, &'static str)> {
    let mut table = get_stability_table(quality, next_quality);
    stability::apply_abstraction(&mut table, quality, abstraction);
    let min_total_stability = stability::adjusted_threshold(min_total_stability, abstraction);

    // Grounded voicings promise that the displayed quality is literally present.
    // Explicit abstraction is the intentional opt-out: it may imply the harmony.
    let required_pitch_classes: Vec<u8> = if abstraction < 0.05 {
        required_intervals_for_reduction(quality)
            .iter()
            .map(|interval| interval.semitones % 12)
            .collect()
    } else {
        Vec::new()
    };

    if required_pitch_classes.len() > note_count {
        return Vec::new();
    }

    // Collect available semitones (stability > 0).
    let available: Vec<u8> = (0u8..12)
        .filter(|&s| table[s as usize] > 0)
        .collect();

    if available.len() < note_count {
        return Vec::new();
    }

    let transforms_for_count: &[Transform] = if note_count <= 3 {
        // 3-note voicings: only close position.
        &[Transform::Close]
    } else {
        &[
            Transform::Close,
            Transform::Drop2,
            Transform::Drop3,
            Transform::Drop2And3,
        ]
    };

    let mut results: Vec<(VoiceSet, u16, &'static str)> = Vec::new();

    combinations(&available, note_count, &mut |subset| {
        if !required_pitch_classes
            .iter()
            .all(|required| subset.contains(required))
        {
            return;
        }
        if has_duplicate_degree(subset, quality) {
            return;
        }
        let stability = subset_stability(&table, subset);
        if stability < min_total_stability as u16 {
            return;
        }

        // subset is already sorted (combinations preserve order).
        let sorted = subset;

        for &transform in transforms_for_count {
            for inv in 0..note_count {
                let (close_intervals, close_octaves) = close_inversion(sorted, inv, quality);

                let (final_intervals, final_octaves) = match transform {
                    Transform::Close => (close_intervals, close_octaves),
                    Transform::Drop2 => {
                        apply_drop(&close_intervals, &close_octaves, &[1])
                    }
                    Transform::Drop3 => {
                        apply_drop(&close_intervals, &close_octaves, &[2])
                    }
                    Transform::Drop2And3 => {
                        apply_drop(&close_intervals, &close_octaves, &[1, 2])
                    }
                };

                let label = transform_label(transform);
                let recipe = label_to_recipe(label);

                let voice_set = VoiceSet::new_procedural(
                    root_pc,
                    final_intervals,
                    final_octaves,
                    recipe,
                    quality,
                );

                results.push((voice_set, stability, label));
            }
        }
    });

    // Sort by stability descending (stable sort preserves subset order for ties).
    results.sort_by_key(|r| std::cmp::Reverse(r.1));

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    #[test]
    fn generates_voice_sets_for_maj7() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 80);
        assert!(!sets.is_empty());
    }

    #[test]
    fn core_tones_have_highest_stability() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 80);
        let top = &sets[0];
        assert_eq!(top.1, 160, "R+3+5+7 = 40+40+40+40 = 160");
    }

    #[test]
    fn four_close_inversions_for_max_stability() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 160);
        let close_count = sets.iter().filter(|s| s.2 == "closed").count();
        assert_eq!(close_count, 4);
    }

    #[test]
    fn drop2_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 80);
        assert!(sets.iter().any(|s| s.2 == "drop2"));
    }

    #[test]
    fn drop3_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 80);
        assert!(sets.iter().any(|s| s.2 == "drop3"));
    }

    #[test]
    fn drop2and3_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 80);
        assert!(sets.iter().any(|s| s.2 == "drop2&3"));
    }

    #[test]
    fn three_note_only_close() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 3, None, 80);
        assert!(sets.iter().all(|s| s.2 == "closed"));
    }

    #[test]
    fn stability_filter_works() {
        let quality = find_quality("maj7");
        let permissive = generate_all_voice_sets(0, quality, 4, None, 80);
        let strict = generate_all_voice_sets(0, quality, 4, None, 140);
        assert!(strict.len() < permissive.len());
    }

    #[test]
    fn em7b5_core_subset_present() {
        let quality = find_quality("m7b5");
        let sets = generate_all_voice_sets(4, quality, 4, None, 80);
        // Core Em7b5 = semitones [0,3,6,10] → PCs [4,7,10,2]
        let has_core = sets.iter().any(|s| {
            let mut pcs: Vec<u8> = s
                .0
                .intervals
                .iter()
                .map(|iv| (4 + iv.semitones) % 12)
                .collect();
            pcs.sort();
            pcs == vec![2, 4, 7, 10]
        });
        assert!(has_core);
    }

    #[test]
    fn explicit_quality_extensions_keep_their_functional_names() {
        let cases = [
            ("dom9", 2, "9"),
            ("dom7b9", 1, "b9"),
            ("dom7#9", 3, "#9"),
            ("m11", 5, "11"),
            ("dom7#11", 6, "#11"),
            ("dom13", 9, "13"),
            ("dom7#5", 8, "#5"),
            ("dom7b13", 8, "b13"),
        ];

        for (quality_name, pitch_class, expected_name) in cases {
            let quality = find_quality(quality_name);
            assert_eq!(
                semitone_to_interval(pitch_class, quality).name,
                expected_name,
                "wrong functional spelling for {quality_name}"
            );
        }
    }

    #[test]
    fn dominant_altered_has_b9() {
        let dom = find_quality("dom7");
        let minor = find_quality("m7");
        let sets = generate_all_voice_sets(0, dom, 4, Some(minor), 80);
        let has_b9 = sets
            .iter()
            .any(|s| s.0.intervals.iter().any(|iv| iv.semitones == 1));
        assert!(has_b9);
    }

    #[test]
    fn literal_browse_sets_are_unique_across_chord_qualities() {
        use std::collections::{HashMap, HashSet};

        for note_count in 2..=6 {
            let mut owners: HashMap<u16, &str> = HashMap::new();
            for quality in ChordQuality::ALL {
                let declared = quality.intervals.iter().fold(0u16, |mask, interval| {
                    mask | (1 << (interval.semitones % 12))
                });
                let mut own_masks = HashSet::new();

                for (voice_set, _, _) in generate_literal_voice_sets(0, quality, note_count, 0) {
                    let mask = voice_set.intervals.iter().fold(0u16, |mask, interval| {
                        mask | (1 << (interval.semitones % 12))
                    });
                    assert_eq!(
                        mask & !declared,
                        0,
                        "{} published a tone outside its literal formula",
                        quality.name
                    );
                    if !own_masks.insert(mask) {
                        continue;
                    }
                    if let Some(previous) = owners.insert(mask, quality.name) {
                        assert_eq!(
                            previous, quality.name,
                            "{previous} and {} published pitch-set mask {mask:#05x} at {note_count} notes",
                            quality.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn grounded_reductions_keep_every_qualitys_defining_pitch_classes() {
        let defining = |quality_name: &str| -> &'static [u8] {
            match quality_name {
                "maj7" => &[4, 11],
                "maj9" => &[2, 4, 11],
                "maj13" => &[4, 9, 11],
                "maj7#11" => &[4, 6, 11],
                "m7" => &[3, 10],
                "m9" => &[2, 3, 10],
                "m11" => &[3, 5, 10],
                "m13" => &[3, 9, 10],
                "m7b5" => &[3, 6, 10],
                "m9b11" => &[1, 3, 4, 10],
                "dom7" => &[4, 10],
                "dom9" => &[2, 4, 10],
                "dom13" => &[4, 9, 10],
                "dom7#5" => &[4, 8, 10],
                "dom7b9" => &[1, 4, 10],
                "dom7#9" => &[3, 4, 10],
                "dom7#11" => &[4, 6, 10],
                // b13 is enharmonic to #5, so the natural fifth distinguishes
                // dom7b13 from dom7#5 as a pitch set.
                "dom7b13" => &[4, 7, 8, 10],
                "dim7" => &[3, 6, 9],
                other => panic!("missing defining-tone contract for {other}"),
            }
        };

        for quality in ChordQuality::ALL {
            let required = defining(quality.name);
            for note_count in 2..=6 {
                let sets = generate_all_voice_sets(0, quality, note_count, None, 0);
                if note_count < required.len() {
                    assert!(
                        sets.is_empty(),
                        "{} should not be representable with {note_count} notes",
                        quality.name
                    );
                    continue;
                }

                assert!(
                    !sets.is_empty(),
                    "{} should be representable with {note_count} notes",
                    quality.name
                );
                for (voice_set, _, _) in sets {
                    let pitch_classes: Vec<u8> = voice_set
                        .intervals
                        .iter()
                        .map(|interval| interval.semitones % 12)
                        .collect();
                    assert!(
                        required.iter().all(|pc| pitch_classes.contains(pc)),
                        "{} at {note_count} notes omitted a defining tone: {:?}",
                        quality.name,
                        voice_set.intervals
                    );
                }
            }
        }
    }

    #[test]
    fn grounded_dominant_reductions_keep_guide_tones_and_named_color() {
        let cases: [(&str, &[Interval]); 6] = [
            ("dom9", &[Interval::M3, Interval::m7, Interval::M9]),
            ("dom13", &[Interval::M3, Interval::m7, Interval::M13]),
            ("dom7b9", &[Interval::M3, Interval::m7, Interval::m9]),
            ("dom7#9", &[Interval::M3, Interval::m7, Interval::SHARP9]),
            ("dom7#11", &[Interval::M3, Interval::m7, Interval::SHARP11]),
            (
                "dom7b13",
                &[Interval::M3, Interval::P5, Interval::m7, Interval::m13],
            ),
        ];

        for (quality_name, required) in cases {
            let quality = find_quality(quality_name);
            for note_count in required.len()..=quality.intervals.len() {
                let sets = generate_all_voice_sets(0, quality, note_count, None, 0);
                assert!(!sets.is_empty(), "{quality_name} at {note_count} notes");
                for (voice_set, _, _) in sets {
                    let pitch_classes: Vec<u8> = voice_set
                        .intervals
                        .iter()
                        .map(|interval| interval.semitones % 12)
                        .collect();
                    assert!(
                        required
                            .iter()
                            .all(|interval| pitch_classes.contains(&(interval.semitones % 12))),
                        "{quality_name} at {note_count} notes omitted a defining tone: {:?}",
                        voice_set.intervals
                    );
                }
            }
        }
    }

    #[test]
    fn m9b11_reduction_keeps_and_spells_its_flat_eleventh() {
        let quality = find_quality("m9b11");
        let sets = generate_all_voice_sets(0, quality, 4, None, 0);
        assert!(!sets.is_empty());

        for (voice_set, _, _) in sets {
            let pitch_classes: Vec<u8> = voice_set
                .intervals
                .iter()
                .map(|interval| interval.semitones % 12)
                .collect();
            assert!([1, 3, 4, 10].iter().all(|pc| pitch_classes.contains(pc)));
            assert!(voice_set
                .intervals
                .iter()
                .any(|interval| interval.name == "b11"));
        }
    }

    #[test]
    fn explicit_abstraction_can_still_omit_defining_tones() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets_with_abstraction(0, quality, 2, None, 0, 1.0);
        assert!(!sets.is_empty());
        assert!(sets.iter().any(|(voice_set, _, _)| {
            !voice_set.intervals.contains(&Interval::M3)
                || !voice_set.intervals.contains(&Interval::M7)
        }));
    }
}
