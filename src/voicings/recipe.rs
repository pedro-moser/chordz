use super::voice_set::VoiceSet;
use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

/// A musical strategy for selecting and arranging chord tones
/// before mapping them to the fretboard.
///
/// Each recipe defines a different voicing style used in jazz guitar:
/// from close-position chords to shell voicings, rootless forms,
/// drop voicings, quartal stacks, and upper-structure triads.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VoicingRecipe {
    /// Close-position voicing using consecutive chord tones.
    Closed,
    /// Three-note shell: 3rd and 7th (guide tones), optionally with root.
    Shell,
    /// Rootless voicing family A: common forms built on 3rd and 7th.
    RootlessA,
    /// Rootless voicing family B: alternate inversion/family of rootless forms.
    RootlessB,
    /// Four-note close structure with the second voice from the top dropped an octave.
    Drop2,
    /// Four-note close structure with the third voice from the top dropped an octave.
    Drop3,
    /// Stacks of perfect fourths derived from chord/scale material.
    Quartal,
    /// Triad stacked over guide tones, common for altered dominants.
    UpperStructureTriad,
    /// Two triads from a parent scale, alternating notes.
    TriadPair,
}

impl VoicingRecipe {
    /// Human-readable name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Shell => "shell",
            Self::RootlessA => "rootless-a",
            Self::RootlessB => "rootless-b",
            Self::Drop2 => "drop2",
            Self::Drop3 => "drop3",
            Self::Quartal => "quartal",
            Self::UpperStructureTriad => "upper-structure-triad",
            Self::TriadPair => "triad-pair",
        }
    }

    /// Whether this recipe typically omits the root note.
    pub fn typically_rootless(&self) -> bool {
        matches!(
            self,
            Self::RootlessA | Self::RootlessB | Self::UpperStructureTriad
        )
    }

    /// All defined recipes.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Closed,
            Self::Shell,
            Self::RootlessA,
            Self::RootlessB,
            Self::Drop2,
            Self::Drop3,
            Self::Quartal,
            Self::UpperStructureTriad,
            Self::TriadPair,
        ]
    }

    /// Generate shell voice sets for the given chord.
    ///
    /// Shell voicings use the guide tones (3rd and 7th) as the core,
    /// optionally adding the root. The fifth is intentionally omitted.
    ///
    /// Returns voice sets in order: 2-note guide-tone pair first,
    /// then 3-note version with root (if the quality contains a root).
    /// Returns an empty vec for chord qualities that cannot form a shell
    /// (e.g., diminished chords without a recognizable 3rd/7th pair).
    pub fn generate_shell(&self, root_pc: u8, quality: &'static ChordQuality) -> Vec<VoiceSet> {
        // Determine chord family from the quality's intervals.
        let has_m3 = quality.intervals.contains(&Interval::M3);
        let has_m7 = quality.intervals.contains(&Interval::M7);
        let has_m3b = quality.intervals.contains(&Interval::m3);
        let has_m7b = quality.intervals.contains(&Interval::m7);

        let (third, seventh): (Interval, Interval) = if has_m3 && has_m7 {
            // Major: M3 + M7
            (Interval::M3, Interval::M7)
        } else if has_m3 && has_m7b {
            // Dominant: M3 + m7
            (Interval::M3, Interval::m7)
        } else if has_m3b && has_m7b {
            // Minor: m3 + m7
            (Interval::m3, Interval::m7)
        } else {
            // Unsupported chord quality for shell voicings.
            return Vec::new();
        };

        let mut result = Vec::new();

        // 2-note shell: guide tones only, both in octave 1.
        result.push(VoiceSet::new(
            root_pc,
            vec![third, seventh],
            vec![1, 1],
            *self,
            quality,
        ));

        // 3-note shell: root + guide tones (if quality contains root).
        if quality.intervals.contains(&Interval::UNISON) {
            result.push(VoiceSet::new(
                root_pc,
                vec![Interval::UNISON, third, seventh],
                vec![0, 1, 1],
                *self,
                quality,
            ));
        }

        result
    }

    /// Generate rootless voice sets for the given chord.
    ///
    /// Rootless voicings omit the root entirely, using the guide tones
    /// (3rd and 7th) as the foundation and adding color tones from the
    /// chord quality. This is the standard approach for comping in a
    /// band context where the bassist covers the root.
    ///
    /// Returns voice sets in order: 4-note form first (if color tones
    /// are available), then 3-note forms with one color tone each,
    /// then the 2-note guide-tone pair as a fallback.
    /// Returns an empty vec for chord qualities that cannot form a
    /// rootless voicing (no recognizable 3rd/7th pair).
    pub fn generate_rootless(&self, root_pc: u8, quality: &'static ChordQuality) -> Vec<VoiceSet> {
        // Determine chord family from the quality's intervals.
        let has_m3 = quality.intervals.contains(&Interval::M3);
        let has_m7 = quality.intervals.contains(&Interval::M7);
        let has_m3b = quality.intervals.contains(&Interval::m3);
        let has_m7b = quality.intervals.contains(&Interval::m7);

        let (third, seventh): (Interval, Interval) = if has_m3 && has_m7 {
            // Major: M3 + M7
            (Interval::M3, Interval::M7)
        } else if has_m3 && has_m7b {
            // Dominant: M3 + m7
            (Interval::M3, Interval::m7)
        } else if has_m3b && has_m7b {
            // Minor: m3 + m7
            (Interval::m3, Interval::m7)
        } else {
            // Unsupported chord quality for rootless voicings.
            return Vec::new();
        };

        let mut result = Vec::new();

        // Choose color tones based on chord family and available intervals.
        let color_tones: &[Interval] = if has_m3 && has_m7 {
            // Major: 9 and 13 are the standard color tones.
            if quality.intervals.contains(&Interval::M9)
                && quality.intervals.contains(&Interval::M13)
            {
                &[Interval::M9, Interval::M13]
            } else if quality.intervals.contains(&Interval::M9) {
                &[Interval::M9]
            } else if quality.intervals.contains(&Interval::M13) {
                &[Interval::M13]
            } else {
                &[]
            }
        } else if has_m3 && has_m7b {
            // Dominant: 9 and 13 are the standard color tones.
            if quality.intervals.contains(&Interval::M9)
                && quality.intervals.contains(&Interval::M13)
            {
                &[Interval::M9, Interval::M13]
            } else if quality.intervals.contains(&Interval::M9) {
                &[Interval::M9]
            } else if quality.intervals.contains(&Interval::M13) {
                &[Interval::M13]
            } else {
                &[]
            }
        } else {
            // Minor: 9 and 11 are the standard color tones.
            if quality.intervals.contains(&Interval::M9)
                && quality.intervals.contains(&Interval::M11)
            {
                &[Interval::M9, Interval::M11]
            } else if quality.intervals.contains(&Interval::M9) {
                &[Interval::M9]
            } else if quality.intervals.contains(&Interval::M11) {
                &[Interval::M11]
            } else {
                &[]
            }
        };

        // 4-note rootless: 3rd, 7th, and two color tones.
        if color_tones.len() >= 2 {
            result.push(VoiceSet::new(
                root_pc,
                vec![third, seventh, color_tones[0], color_tones[1]],
                vec![1, 1, 2, 2],
                *self,
                quality,
            ));
        }

        // 3-note rootless: 3rd, 7th, and one color tone each.
        for &color in color_tones {
            result.push(VoiceSet::new(
                root_pc,
                vec![third, seventh, color],
                vec![1, 1, 2],
                *self,
                quality,
            ));
        }

        // 2-note rootless: guide tones only (always included).
        result.push(VoiceSet::new(
            root_pc,
            vec![third, seventh],
            vec![1, 1],
            *self,
            quality,
        ));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_names() {
        assert_eq!(VoicingRecipe::Closed.name(), "closed");
        assert_eq!(VoicingRecipe::Shell.name(), "shell");
        assert_eq!(VoicingRecipe::RootlessA.name(), "rootless-a");
        assert_eq!(VoicingRecipe::RootlessB.name(), "rootless-b");
        assert_eq!(VoicingRecipe::Drop2.name(), "drop2");
        assert_eq!(VoicingRecipe::Drop3.name(), "drop3");
        assert_eq!(VoicingRecipe::Quartal.name(), "quartal");
        assert_eq!(
            VoicingRecipe::UpperStructureTriad.name(),
            "upper-structure-triad"
        );
        assert_eq!(VoicingRecipe::TriadPair.name(), "triad-pair");
    }

    #[test]
    fn test_recipe_all_contains_all_variants() {
        let all = VoicingRecipe::all();
        assert_eq!(all.len(), 9);
        assert!(all.contains(&VoicingRecipe::Closed));
        assert!(all.contains(&VoicingRecipe::Shell));
        assert!(all.contains(&VoicingRecipe::RootlessA));
        assert!(all.contains(&VoicingRecipe::RootlessB));
        assert!(all.contains(&VoicingRecipe::Drop2));
        assert!(all.contains(&VoicingRecipe::Drop3));
        assert!(all.contains(&VoicingRecipe::Quartal));
        assert!(all.contains(&VoicingRecipe::UpperStructureTriad));
        assert!(all.contains(&VoicingRecipe::TriadPair));
    }

    #[test]
    fn test_typically_rootless() {
        assert!(!VoicingRecipe::Closed.typically_rootless());
        assert!(!VoicingRecipe::Shell.typically_rootless());
        assert!(VoicingRecipe::RootlessA.typically_rootless());
        assert!(VoicingRecipe::RootlessB.typically_rootless());
        assert!(!VoicingRecipe::Drop2.typically_rootless());
        assert!(!VoicingRecipe::Drop3.typically_rootless());
        assert!(!VoicingRecipe::Quartal.typically_rootless());
        assert!(VoicingRecipe::UpperStructureTriad.typically_rootless());
        assert!(!VoicingRecipe::TriadPair.typically_rootless());
    }

    #[test]
    fn test_recipe_derives() {
        // Verify Clone, Copy, Debug, Eq, Hash, PartialEq work.
        let a = VoicingRecipe::Shell;
        let b = a; // Copy
        assert_eq!(a, b); // PartialEq + Eq
        assert!(format!("{:?}", a).contains("Shell")); // Debug
        use std::collections::HashSet;
        let mut set = HashSet::new();
        assert!(set.insert(a)); // Hash
        assert!(!set.insert(b)); // Hash + Eq
    }

    #[test]
    fn test_exhaustive_match() {
        // Compile-time proof that all variants are covered.
        for recipe in VoicingRecipe::all() {
            match recipe {
                VoicingRecipe::Closed
                | VoicingRecipe::Shell
                | VoicingRecipe::RootlessA
                | VoicingRecipe::RootlessB
                | VoicingRecipe::Drop2
                | VoicingRecipe::Drop3
                | VoicingRecipe::Quartal
                | VoicingRecipe::UpperStructureTriad
                | VoicingRecipe::TriadPair => {}
            }
        }
    }

    #[test]
    fn test_dominant_shell_includes_guide_tones() {
        // G7 shell must include 3 (M3) and b7 (m7).
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let shells = VoicingRecipe::Shell.generate_shell(7, quality); // G

        assert!(!shells.is_empty());
        for shell in &shells {
            assert!(
                shell.intervals.contains(&Interval::M3),
                "dominant shell must contain 3rd"
            );
            assert!(
                shell.intervals.contains(&Interval::m7),
                "dominant shell must contain b7"
            );
        }
    }

    #[test]
    fn test_minor_shell_includes_guide_tones() {
        // Dm7 shell must include b3 (m3) and b7 (m7).
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m7").unwrap();
        let shells = VoicingRecipe::Shell.generate_shell(2, quality); // D

        assert!(!shells.is_empty());
        for shell in &shells {
            assert!(
                shell.intervals.contains(&Interval::m3),
                "minor shell must contain b3"
            );
            assert!(
                shell.intervals.contains(&Interval::m7),
                "minor shell must contain b7"
            );
        }
    }

    #[test]
    fn test_major_shell_includes_guide_tones() {
        // Cmaj7 shell must include 3 (M3) and 7 (M7).
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let shells = VoicingRecipe::Shell.generate_shell(0, quality); // C

        assert!(!shells.is_empty());
        for shell in &shells {
            assert!(
                shell.intervals.contains(&Interval::M3),
                "major shell must contain 3rd"
            );
            assert!(
                shell.intervals.contains(&Interval::M7),
                "major shell must contain 7th"
            );
        }
    }

    #[test]
    fn test_shell_produces_three_note_voice_sets() {
        // Shell generation produces both 2-note and 3-note voice sets.
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "dom7").unwrap();
        let shells = VoicingRecipe::Shell.generate_shell(7, quality);

        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0].len(), 2); // guide tones only
        assert_eq!(shells[1].len(), 3); // root + guide tones
    }

    #[test]
    fn test_shell_omits_fifth() {
        // Shell voicings intentionally omit the fifth.
        for quality_name in &["dom7", "maj7", "m7"] {
            let quality = ChordQuality::ALL
                .iter()
                .find(|q| q.name == *quality_name)
                .unwrap();
            // Verify the quality actually has a fifth.
            assert!(
                quality.intervals.contains(&Interval::P5),
                "quality {} should have a fifth",
                quality_name
            );

            let shells = VoicingRecipe::Shell.generate_shell(0, quality);
            for shell in &shells {
                assert!(
                    !shell.intervals.contains(&Interval::P5),
                    "shell for {} must omit the fifth",
                    quality_name
                );
            }
        }
    }

    #[test]
    fn test_shell_extended_chord_omits_fifth() {
        // Shell for extended chords (dom13, maj13, m13) also omits the fifth.
        for quality_name in &["dom13", "maj13", "m13"] {
            let quality = ChordQuality::ALL
                .iter()
                .find(|q| q.name == *quality_name)
                .unwrap();
            let shells = VoicingRecipe::Shell.generate_shell(0, quality);

            assert!(!shells.is_empty());
            for shell in &shells {
                assert!(
                    !shell.intervals.contains(&Interval::P5),
                    "shell for {} must omit the fifth",
                    quality_name
                );
            }
        }
    }

    #[test]
    fn test_dominant_rootless_g13_no_root() {
        // G13 rootless must produce intervals without 1 (root).
        let quality = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom13")
            .unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(7, quality); // G

        assert!(!rootless.is_empty());
        for vs in &rootless {
            assert!(
                !vs.intervals.contains(&Interval::UNISON),
                "G13 rootless must not contain root (1)"
            );
        }

        // The 4-note form should be 3 b7 9 13.
        let four_note = rootless.iter().find(|vs| vs.len() == 4).unwrap();
        assert!(four_note.intervals.contains(&Interval::M3));
        assert!(four_note.intervals.contains(&Interval::m7));
        assert!(four_note.intervals.contains(&Interval::M9));
        assert!(four_note.intervals.contains(&Interval::M13));
    }

    #[test]
    fn test_major_rootless_cmaj13_produces_3_7_9_13() {
        // Cmaj13 rootless can produce 3 7 9 13.
        let quality = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "maj13")
            .unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(0, quality); // C

        assert!(!rootless.is_empty());

        // Find the 4-note form and verify it is 3 7 9 13.
        let four_note = rootless.iter().find(|vs| vs.len() == 4).unwrap();
        assert_eq!(
            four_note.intervals,
            vec![Interval::M3, Interval::M7, Interval::M9, Interval::M13]
        );

        // Verify no root in any voice set.
        for vs in &rootless {
            assert!(
                !vs.intervals.contains(&Interval::UNISON),
                "Cmaj13 rootless must not contain root (1)"
            );
        }
    }

    #[test]
    fn test_minor_rootless_dm11_produces_b3_b7_9_11() {
        // Dm11 rootless can produce b3 b7 9 11.
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "m11").unwrap();
        let rootless = VoicingRecipe::RootlessA.generate_rootless(2, quality); // D

        assert!(!rootless.is_empty());

        // Find the 4-note form and verify it is b3 b7 9 11.
        let four_note = rootless.iter().find(|vs| vs.len() == 4).unwrap();
        assert_eq!(
            four_note.intervals,
            vec![Interval::m3, Interval::m7, Interval::M9, Interval::M11]
        );

        // Verify no root in any voice set.
        for vs in &rootless {
            assert!(
                !vs.intervals.contains(&Interval::UNISON),
                "Dm11 rootless must not contain root (1)"
            );
        }
    }

    #[test]
    fn test_rootless_omission_is_intentional() {
        // Root omission in rootless voicings is intentional, not accidental.
        // Every voice set produced by generate_rootless must omit the root.
        // This is verified across all three chord families.
        for (quality_name, root_pc) in [
            ("dom13", 7u8), // G
            ("dom9", 7u8),  // G
            ("dom7", 7u8),  // G
            ("maj13", 0u8), // C
            ("maj9", 0u8),  // C
            ("maj7", 0u8),  // C
            ("m11", 2u8),   // D
            ("m9", 2u8),    // D
            ("m7", 2u8),    // D
        ] {
            let quality = ChordQuality::ALL
                .iter()
                .find(|q| q.name == quality_name)
                .unwrap();
            let rootless = VoicingRecipe::RootlessA.generate_rootless(root_pc, quality);

            assert!(
                !rootless.is_empty(),
                "rootless generation for {} should not be empty",
                quality_name
            );

            for vs in &rootless {
                assert!(
                    !vs.intervals.contains(&Interval::UNISON),
                    "{} rootless must intentionally omit root (1), found in {:?} intervals",
                    quality_name,
                    vs.intervals.iter().map(|i| i.name).collect::<Vec<_>>()
                );

                // Verify root appears in omissions (proves intentional omission).
                let omissions = vs.omissions();
                assert!(
                    omissions.contains(&Interval::UNISON),
                    "{} rootless must list root (1) in omissions",
                    quality_name
                );
            }
        }
    }

    #[test]
    fn test_rootless_includes_guide_tones() {
        // Every rootless voice set must contain the guide tones (3rd and 7th).
        for (quality_name, root_pc, third, seventh) in [
            ("dom13", 7u8, Interval::M3, Interval::m7),
            ("maj13", 0u8, Interval::M3, Interval::M7),
            ("m11", 2u8, Interval::m3, Interval::m7),
        ] {
            let quality = ChordQuality::ALL
                .iter()
                .find(|q| q.name == quality_name)
                .unwrap();
            let rootless = VoicingRecipe::RootlessA.generate_rootless(root_pc, quality);

            for vs in &rootless {
                assert!(
                    vs.intervals.contains(&third),
                    "{} rootless must contain 3rd",
                    quality_name
                );
                assert!(
                    vs.intervals.contains(&seventh),
                    "{} rootless must contain 7th",
                    quality_name
                );
            }
        }
    }

    #[test]
    fn test_rootless_basic_chords_fallback() {
        // Basic chords (dom7, maj7, m7) without extensions still produce
        // rootless voicings (guide-tone pair), since they lack color tones.
        for (quality_name, root_pc) in [("dom7", 7u8), ("maj7", 0u8), ("m7", 2u8)] {
            let quality = ChordQuality::ALL
                .iter()
                .find(|q| q.name == quality_name)
                .unwrap();
            let rootless = VoicingRecipe::RootlessA.generate_rootless(root_pc, quality);

            // Should produce exactly 1 voice set: the 2-note guide-tone pair.
            assert_eq!(rootless.len(), 1);
            assert_eq!(rootless[0].len(), 2);
            assert!(!rootless[0].intervals.contains(&Interval::UNISON));
        }
    }
}
