use crate::theory::intervals::Interval;
use crate::theory::notes::Note;

use super::fretboard::Fretboard;
use super::rules::VoicingRules;
use super::voice_set::VoiceSet;

/// A playable mapping of a `VoiceSet` onto the guitar fretboard.
///
/// A `Fingering` is produced from a `VoiceSet` that may have omitted
/// intervals (root, fifth, etc.).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Fingering {
    /// `positions[i]` is `Some(fret)` for string i, or `None` if muted.
    pub positions: [Option<u8>; 6],
    /// `intervals[i]` is the interval played on string i, or `None` if muted.
    pub intervals: [Option<Interval>; 6],
}

impl Fingering {
    /// The fret span of this fingering (highest fret minus lowest fret).
    /// Returns 0 if fewer than 2 strings are played.
    pub fn fret_span(&self) -> u8 {
        let played: Vec<u8> = self.positions.iter().filter_map(|f| *f).collect();
        if played.len() < 2 {
            return 0;
        }
        *played.iter().max().unwrap() - *played.iter().min().unwrap()
    }

    /// How many strings are played (non-muted).
    pub fn played_count(&self) -> usize {
        self.positions.iter().filter(|f| f.is_some()).count()
    }

    /// The played intervals in string order, from low E to high E.
    pub fn played_intervals(&self) -> Vec<Interval> {
        self.intervals.iter().filter_map(|iv| *iv).collect()
    }

    /// Whether the fingering contains the given interval.
    pub fn has_interval(&self, interval: Interval) -> bool {
        self.intervals.iter().flatten().any(|iv| *iv == interval)
    }

    /// The lowest fret used in this fingering.
    /// Returns 0 if no strings are played.
    pub fn lowest_fret(&self) -> u8 {
        self.positions.iter().filter_map(|f| *f).min().unwrap_or(0)
    }

    /// Get the Note played on each string, using the given fretboard.
    /// Returns `None` for muted strings or invalid positions.
    pub fn notes(&self, fretboard: &Fretboard) -> [Option<Note>; 6] {
        let mut result = [None; 6];
        for (i, &fret_opt) in self.positions.iter().enumerate() {
            if let Some(fret) = fret_opt {
                result[i] = fretboard.get_note(i, fret as usize);
            }
        }
        result
    }
}

/// Map a `VoiceSet` to all valid `Fingering`s on the fretboard.
///
/// Uses backtracking to assign each voice to a (string, fret) position
/// that matches the voice's pitch class, respecting `max_fret`, `max_fret_span`,
/// and the number of strings available.
///
/// Returns fingerings sorted by lowest fret, then by positions lexicographically.
pub fn map_voice_set(
    voice_set: &VoiceSet,
    fretboard: &Fretboard,
    rules: &VoicingRules,
) -> Vec<Fingering> {
    if rules.require_root && !voice_set.intervals.contains(&Interval::UNISON) {
        return Vec::new();
    }

    let num_strings = fretboard.num_strings();
    let num_voices = voice_set.len();
    let pcs = voice_set.pitch_classes();

    // Build a lookup: for each pitch class, which (string, fret) positions produce it?
    let mut pc_positions: Vec<Vec<(usize, u8, Note)>> = vec![Vec::new(); 12];
    for s in 0..num_strings {
        for fret in 0..=rules.max_fret as usize {
            if let Some(note) = fretboard.get_note(s, fret) {
                pc_positions[note.pitch_class as usize].push((s, fret as u8, note));
            }
        }
    }

    // Collect candidate (string, fret) for each voice.
    let mut voice_candidates: Vec<Vec<(usize, u8, Note)>> = Vec::with_capacity(num_voices);
    for pc in pcs.iter().take(num_voices) {
        voice_candidates.push(pc_positions[*pc as usize].clone());
    }

    fn fret_stats(positions: &[Option<u8>; 6]) -> (usize, u8) {
        let mut count = 0u8;
        let mut lo = u8::MAX;
        let mut hi = 0u8;
        for fret in positions.iter().flatten() {
            count += 1;
            lo = lo.min(*fret);
            hi = hi.max(*fret);
        }
        let span = if count >= 2 { hi - lo } else { 0 };
        (count as usize, span)
    }

    // Backtrack: assign each voice to a (string, fret) from its candidates.
    let mut used_string = [false; 6];
    let mut positions = [None; 6];
    let mut intervals = [None; 6];
    let mut result = Vec::new();

    struct Mapper<'a> {
        voice_set: &'a VoiceSet,
        voice_candidates: &'a [Vec<(usize, u8, Note)>],
        rules: &'a VoicingRules,
        result: &'a mut Vec<Fingering>,
    }

    impl Mapper<'_> {
        fn backtrack(
            &mut self,
            voice_idx: usize,
            used_string: &mut [bool; 6],
            positions: &mut [Option<u8>; 6],
            intervals: &mut [Option<Interval>; 6],
            last_string: Option<usize>,
            last_midi: Option<i32>,
        ) {
            if voice_idx == self.voice_set.len() {
                let (count, span) = fret_stats(positions);
                if count < self.rules.min_strings as usize
                    || count > self.rules.max_strings as usize
                {
                    return;
                }
                if span > self.rules.max_fret_span {
                    return;
                }
                if self.rules.require_root
                    && !intervals.iter().flatten().any(|iv| *iv == Interval::UNISON)
                {
                    return;
                }

                self.result.push(Fingering {
                    positions: *positions,
                    intervals: *intervals,
                });
                return;
            }

            for &(s, fret, note) in &self.voice_candidates[voice_idx] {
                if used_string[s] {
                    continue;
                }
                if last_string.is_some_and(|prev| s <= prev) {
                    continue;
                }
                if last_midi.is_some_and(|prev| note.midi() < prev) {
                    continue;
                }

                used_string[s] = true;
                positions[s] = Some(fret);
                intervals[s] = Some(self.voice_set.intervals[voice_idx]);

                let (_, current_span) = fret_stats(positions);
                if current_span <= self.rules.max_fret_span {
                    self.backtrack(
                        voice_idx + 1,
                        used_string,
                        positions,
                        intervals,
                        Some(s),
                        Some(note.midi()),
                    );
                }

                used_string[s] = false;
                positions[s] = None;
                intervals[s] = None;
            }
        }
    }

    let mut mapper = Mapper {
        voice_set,
        voice_candidates: &voice_candidates,
        rules,
        result: &mut result,
    };
    mapper.backtrack(
        0,
        &mut used_string,
        &mut positions,
        &mut intervals,
        None,
        None,
    );

    // Deduplicate and sort deterministically.
    result.sort_by(|a, b| {
        a.lowest_fret()
            .cmp(&b.lowest_fret())
            .then_with(|| a.positions.cmp(&b.positions))
    });
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chords::ChordQuality;
    use crate::voicings::recipe::VoicingRecipe;

    #[test]
    fn test_map_voice_set_respects_max_fret() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.max_fret = 5;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let fingerings = map_voice_set(&vs, &fb, &rules);
        for f in &fingerings {
            for fret in f.positions.iter().flatten() {
                assert!(
                    *fret <= rules.max_fret,
                    "Fret {} exceeds max_fret {}",
                    fret,
                    rules.max_fret
                );
            }
        }
    }

    #[test]
    fn test_map_voice_set_respects_max_fret_span() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.max_fret_span = 4;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let fingerings = map_voice_set(&vs, &fb, &rules);
        for f in &fingerings {
            assert!(
                f.fret_span() <= rules.max_fret_span,
                "Fingering {:?} has span {} > max_fret_span {}",
                f.positions,
                f.fret_span(),
                rules.max_fret_span
            );
        }
    }

    #[test]
    fn test_map_voice_set_shell_maps_to_3_strings() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.min_strings = 3;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let shell = VoicingRecipe::Shell.generate_shell(0, quality);
        // The 3-note shell should have root + 3rd + 7th.
        let shell_vs = shell
            .iter()
            .find(|vs| vs.len() == 3)
            .expect("Shell should have a 3-note form");
        let fingerings = map_voice_set(shell_vs, &fb, &rules);
        assert!(
            !fingerings.is_empty(),
            "Shell voicing should map to fingerings"
        );
        for f in &fingerings {
            assert_eq!(
                f.played_count(),
                3,
                "Shell fingering should play exactly 3 strings, got {}",
                f.played_count()
            );
        }
    }

    #[test]
    fn test_map_voice_set_rootless_has_no_root() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.min_strings = 3;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom9").unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(0, quality);
        assert!(!rootless.is_empty(), "Rootless should produce voice sets");
        let rl_vs = &rootless[0];
        // Verify the voice set doesn't contain the root interval.
        assert!(
            !rl_vs.intervals.contains(&Interval::UNISON),
            "Rootless voice set should not contain root"
        );
        rules.require_root = false;
        let fingerings = map_voice_set(rl_vs, &fb, &rules);
        assert!(
            !fingerings.is_empty(),
            "Rootless voicing should map to fingerings"
        );
        // Each fingering's intervals should match the voice set (no root).
        for f in &fingerings {
            assert!(
                !f.has_interval(Interval::UNISON),
                "Rootless fingering should not contain root interval"
            );
        }
    }

    #[test]
    fn test_map_voice_set_honors_require_root() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.min_strings = 3;
        rules.require_root = true;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom9").unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(0, quality);
        let rl_vs = rootless.iter().find(|vs| vs.len() == 3).unwrap();

        let fingerings = map_voice_set(rl_vs, &fb, &rules);

        assert!(
            fingerings.is_empty(),
            "rootless voice sets must not map when require_root is true"
        );
    }

    #[test]
    fn test_map_voice_set_deterministic() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let result_a = map_voice_set(&vs, &fb, &rules);
        let result_b = map_voice_set(&vs, &fb, &rules);
        assert_eq!(result_a, result_b, "map_voice_set should be deterministic");
    }

    #[test]
    fn test_map_voice_set_respects_string_count() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let vs = VoiceSet::from_quality(0, quality, VoicingRecipe::Closed);
        let fingerings = map_voice_set(&vs, &fb, &rules);
        for f in &fingerings {
            // Each fingering should use at most 6 strings.
            assert!(
                f.played_count() <= 6,
                "Fingering should use at most 6 strings"
            );
            // Each fingering should use at least as many strings as voices.
            assert!(
                f.played_count() >= vs.len(),
                "Fingering should use at least {} strings for {} voices",
                vs.len(),
                vs.len()
            );
        }
    }

    #[test]
    fn test_map_voice_set_fingering_intervals_match_voice_set() {
        let fb = Fretboard::standard_tuning();
        let rules = VoicingRules::default_rules();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let vs = VoiceSet::from_quality(3, quality, VoicingRecipe::Closed); // Em7
        let fingerings = map_voice_set(&vs, &fb, &rules);
        for f in &fingerings {
            assert_eq!(
                f.played_intervals(),
                vs.intervals,
                "Fingering intervals should match voice set intervals"
            );
        }
    }

    #[test]
    fn test_map_voice_set_keeps_interval_on_assigned_string() {
        let fb = Fretboard::standard_tuning();
        let mut rules = VoicingRules::default_rules();
        rules.min_strings = 3;
        rules.require_root = false;
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom9").unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(7, quality);
        let vs = rootless.iter().find(|vs| vs.len() == 3).unwrap();

        let fingerings = map_voice_set(vs, &fb, &rules);

        assert!(!fingerings.is_empty());
        for fingering in fingerings.iter().take(20) {
            let played = fingering.played_intervals();
            assert_eq!(played, vs.intervals);
            for (string, fret) in fingering.positions.iter().enumerate() {
                if let Some(fret) = fret {
                    let note = fb.get_note(string, *fret as usize).unwrap();
                    let interval = fingering.intervals[string].unwrap();
                    let expected_pc =
                        (vs.root_pc as i32 + interval.semitones as i32).rem_euclid(12) as u8;
                    assert_eq!(
                        note.pitch_class, expected_pc,
                        "string {} fret {} should match interval {}",
                        string, fret, interval.name
                    );
                }
            }
        }
    }
}
