use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

use super::recipe::VoicingRecipe;
use super::stability::{self, get_stability_table, subset_stability};
use super::voice_set::VoiceSet;

/// Map a semitone (0-11) to the corresponding basic Interval constant.
fn semitone_to_interval(semitone: u8) -> Interval {
    match semitone {
        0 => Interval::UNISON,
        1 => Interval::m2,
        2 => Interval::M2,
        3 => Interval::m3,
        4 => Interval::M3,
        5 => Interval::P4,
        6 => Interval::tritone,
        7 => Interval::P5,
        8 => Interval::m6,
        9 => Interval::M6,
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
fn close_inversion(sorted_semitones: &[u8], inv: usize) -> (Vec<Interval>, Vec<i32>) {
    let n = sorted_semitones.len();
    let mut intervals = Vec::with_capacity(n);
    let mut octaves = Vec::with_capacity(n);
    for i in 0..n {
        let idx = (inv + i) % n;
        intervals.push(semitone_to_interval(sorted_semitones[idx]));
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
        let stability = subset_stability(&table, subset);
        if stability < min_total_stability as u16 {
            return;
        }

        // subset is already sorted (combinations preserve order).
        let sorted = subset;

        for &transform in transforms_for_count {
            for inv in 0..note_count {
                let (close_intervals, close_octaves) = close_inversion(sorted, inv);

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
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        assert!(!sets.is_empty());
    }

    #[test]
    fn core_tones_have_highest_stability() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        let top = &sets[0];
        assert_eq!(top.1, 16, "R+3+5+7 = 4+4+4+4 = 16");
    }

    #[test]
    fn four_close_inversions_for_max_stability() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 16);
        let close_count = sets.iter().filter(|s| s.2 == "closed").count();
        assert_eq!(close_count, 4);
    }

    #[test]
    fn drop2_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        assert!(sets.iter().any(|s| s.2 == "drop2"));
    }

    #[test]
    fn drop3_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        assert!(sets.iter().any(|s| s.2 == "drop3"));
    }

    #[test]
    fn drop2and3_generated() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 4, None, 8);
        assert!(sets.iter().any(|s| s.2 == "drop2&3"));
    }

    #[test]
    fn three_note_only_close() {
        let quality = find_quality("maj7");
        let sets = generate_all_voice_sets(0, quality, 3, None, 8);
        assert!(sets.iter().all(|s| s.2 == "closed"));
    }

    #[test]
    fn stability_filter_works() {
        let quality = find_quality("maj7");
        let permissive = generate_all_voice_sets(0, quality, 4, None, 8);
        let strict = generate_all_voice_sets(0, quality, 4, None, 14);
        assert!(strict.len() < permissive.len());
    }

    #[test]
    fn em7b5_core_subset_present() {
        let quality = find_quality("m7b5");
        let sets = generate_all_voice_sets(4, quality, 4, None, 8);
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
    fn dominant_altered_has_b9() {
        let dom = find_quality("dom7");
        let minor = find_quality("m7");
        let sets = generate_all_voice_sets(0, dom, 4, Some(minor), 8);
        let has_b9 = sets
            .iter()
            .any(|s| s.0.intervals.iter().any(|iv| iv.semitones == 1));
        assert!(has_b9);
    }
}
