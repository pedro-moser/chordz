use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

use super::recipe::VoicingRecipe;

/// An abstract voicing before guitar mapping.
///
/// A `VoiceSet` represents a selected subset of intervals from a chord formula,
/// arranged in a specific order. It may contain fewer intervals than the source
/// chord (e.g., omitting the root or fifth), which is the essence of jazz guitar
/// voicing: selective note choice over full chord stacks.
///
/// The `intervals` list defines the vertical structure from bass to treble.
/// `octave_offsets` control the register of each voice relative to the root's octave.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceSet {
    /// The root pitch class (0=C, 1=C#, ..., 11=B).
    pub root_pc: u8,
    /// Selected intervals in voice order (bass to treble).
    pub intervals: Vec<Interval>,
    /// Octave offset for each interval relative to the root's octave.
    /// A value of 0 means the interval sits in the same octave as the root;
    /// 1 means one octave higher, -1 means one octave lower.
    pub octave_offsets: Vec<i32>,
    /// The recipe that produced this voice set.
    pub recipe: VoicingRecipe,
    /// The source chord quality this voice set was derived from.
    pub source_quality: &'static ChordQuality,
}

impl VoiceSet {
    /// Create a new `VoiceSet`, validating that all intervals belong to the
    /// source chord quality.
    ///
    /// # Panics
    ///
    /// Panics if `intervals` is empty or contains an interval not present
    /// in `source_quality`.
    pub fn new(
        root_pc: u8,
        intervals: Vec<Interval>,
        octave_offsets: Vec<i32>,
        recipe: VoicingRecipe,
        source_quality: &'static ChordQuality,
    ) -> Self {
        assert!(
            !intervals.is_empty(),
            "VoiceSet must have at least one interval"
        );
        assert_eq!(
            intervals.len(),
            octave_offsets.len(),
            "intervals and octave_offsets must have the same length"
        );
        for iv in &intervals {
            assert!(
                source_quality.intervals.contains(iv),
                "interval {:?} is not in source quality '{}'",
                iv.name,
                source_quality.name
            );
        }
        Self {
            root_pc,
            intervals,
            octave_offsets,
            recipe,
            source_quality,
        }
    }

    /// Compute the pitch class for each interval, accounting for octave offsets.
    pub fn pitch_classes(&self) -> Vec<u8> {
        self.intervals
            .iter()
            .zip(self.octave_offsets.iter())
            .map(|(iv, &oct)| {
                let total_semitones = iv.semitones as i32 + oct * 12;
                ((self.root_pc as i32 + total_semitones).rem_euclid(12)) as u8
            })
            .collect()
    }

    /// The number of voices in this set.
    pub fn len(&self) -> usize {
        self.intervals.len()
    }

    /// Whether this voice set has no intervals.
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    /// Returns the intervals from `source_quality` that are not present
    /// in this voice set.
    pub fn omissions(&self) -> Vec<Interval> {
        self.source_quality
            .intervals
            .iter()
            .filter(|iv| !self.intervals.contains(iv))
            .copied()
            .collect()
    }

    /// Create a VoiceSet containing all intervals from the quality
    /// in close position (all octave offsets 0).
    pub fn from_quality(
        root_pc: u8,
        quality: &'static ChordQuality,
        recipe: VoicingRecipe,
    ) -> Self {
        let intervals = quality.intervals.to_vec();
        let octave_offsets = vec![0; intervals.len()];
        Self::new(root_pc, intervals, octave_offsets, recipe, quality)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::intervals::Interval;

    #[test]
    fn test_rootless_major_voice_set() {
        // Cmaj13 rootless: intervals 3 7 9 13 (omitting root and fifth).
        let quality = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "maj13")
            .unwrap();
        let vs = VoiceSet::new(
            0, // C
            vec![Interval::M3, Interval::M7, Interval::M9, Interval::M13],
            vec![1, 1, 2, 2],
            VoicingRecipe::RootlessA,
            quality,
        );

        assert_eq!(vs.len(), 4);
        assert_eq!(vs.recipe, VoicingRecipe::RootlessA);
        assert_eq!(vs.root_pc, 0);

        // Verify pitch classes: C + M3(4) = E(4), C + M7(11) = B(11),
        // C + M9(14) = D(2), C + M13(21) = A(9)
        assert_eq!(vs.pitch_classes(), vec![4, 11, 2, 9]);

        // Verify omissions: root (1) and fifth (5) are omitted.
        let omissions = vs.omissions();
        assert_eq!(omissions.len(), 2);
        assert!(omissions.contains(&Interval::UNISON));
        assert!(omissions.contains(&Interval::P5));
    }

    #[test]
    fn test_voice_set_fewer_intervals_than_source() {
        // A maj7 quality has 4 intervals; our voice set has 3.
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        assert_eq!(quality.intervals.len(), 4);

        let vs = VoiceSet::new(
            0,
            vec![Interval::M3, Interval::M7, Interval::P5],
            vec![1, 1, 1],
            VoicingRecipe::Shell,
            quality,
        );

        assert_eq!(vs.len(), 3);
        assert!(vs.len() < quality.intervals.len());
    }

    #[test]
    fn test_from_quality() {
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let vs = VoiceSet::from_quality(7, quality, VoicingRecipe::Closed); // G

        assert_eq!(vs.len(), quality.intervals.len());
        assert_eq!(vs.intervals, quality.intervals.to_vec());
        assert_eq!(vs.recipe, VoicingRecipe::Closed);
        assert!(vs.omissions().is_empty());
    }

    #[test]
    fn test_rejects_invalid_interval() {
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        // m3 is not in maj7, so this should panic.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            VoiceSet::new(
                0,
                vec![Interval::M3, Interval::m3],
                vec![1, 1],
                VoicingRecipe::Closed,
                quality,
            )
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_empty_intervals() {
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            VoiceSet::new(0, vec![], vec![], VoicingRecipe::Closed, quality)
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_pitch_classes_with_octave_offsets() {
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        // Cmaj7 with offsets: 3rd in octave 1, 7th in octave 0.
        let vs = VoiceSet::new(
            0,
            vec![Interval::M3, Interval::M7],
            vec![1, 0],
            VoicingRecipe::Drop2,
            quality,
        );

        // M3 at octave 1: (0 + 4 + 12) % 12 = 4 (E)
        // M7 at octave 0: (0 + 11 + 0) % 12 = 11 (B)
        assert_eq!(vs.pitch_classes(), vec![4, 11]);
    }
}
