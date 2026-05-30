//! Guide-tone "7no5 / 7no3" shell pairs per chord quality (pure core).
//!
//! Distilled from `materiais/meus/Airegin 7no5:7no3 etude.gp`: over each chord the line
//! draws from two 3-note upper-structure shells — a `7no5` (1-3-7 shape) and a `7no3`
//! (1-5-7 shape) — that together spell the chord's extended color. The table is in
//! degrees relative to the chord root, so it is transposition-invariant. Qualities absent
//! from the table fall back to literal shells built from the chord's own intervals.

use crate::theory::chords::ChordQuality;

/// `[7no5, 7no3]` shells as semitone offsets from the chord root.
/// Degree→semitone: 1=0 b9=1 9=2 b3=3 3=4 11=5 #11=6 5=7 #5/b13=8 13=9 b7=10 7=11.
type ShellPair = [[i8; 3]; 2];

const MAJ7: ShellPair = [[11, 6, 9], [4, 7, 2]]; //  7,#11,13 | 3,5,9   (Lydian)
const ALT: ShellPair = [[10, 4, 8], [3, 6, 1]]; //  b7,3,b13 | #9,#11,b9 (altered)
const M7: ShellPair = [[10, 5, 9], [3, 7, 2]]; //   b7,11,13 | b3,5,9  (Dorian)
const M7B5: ShellPair = [[10, 5, 8], [3, 6, 2]]; // b7,11,b13 | b3,b5,9 (Locrian #2)

/// Table lookup by quality family. Order matters: `maj*` and `m7b5`/`m9b11` are matched
/// before the generic minor branch (they also start with 'm'). Dominants all map to the
/// altered shells (the corpus treats every dominant as alt). `None` → literal fallback.
fn table_for(quality: &ChordQuality) -> Option<ShellPair> {
    let n = quality.name;
    if n.starts_with("maj") {
        Some(MAJ7)
    } else if n == "m7b5" || n == "m9b11" {
        Some(M7B5)
    } else if n.starts_with('m') {
        Some(M7)
    } else if n.starts_with("dom") {
        Some(ALT)
    } else {
        None
    }
}

/// Literal shells from the chord's own tones: `7no5 = root-3-7`, `7no3 = root-5-7`.
/// For every `ChordQuality`, `intervals[1..=3]` are the 3rd/5th/7th (all < 12 semitones).
fn literal(quality: &ChordQuality) -> ShellPair {
    debug_assert!(
        quality.intervals.len() >= 4,
        "chord quality must define at least root/3rd/5th/7th"
    );
    let s = |i: usize| quality.intervals.get(i).map(|iv| iv.semitones as i8).unwrap_or(0);
    [[0, s(1), s(3)], [0, s(2), s(3)]]
}

/// Resolve a chord's two guide-tone shells to concrete pitch classes (0..11), in offset
/// order (not sorted) so a `Shape::Order`/`Anchor` can target a specific voice later.
pub fn resolve_shell_pair(root_pc: u8, quality: &ChordQuality) -> ([u8; 3], [u8; 3]) {
    let pair = table_for(quality).unwrap_or_else(|| literal(quality));
    let pc = |off: i8| (root_pc as i16 + off as i16).rem_euclid(12) as u8;
    (
        [pc(pair[0][0]), pc(pair[0][1]), pc(pair[0][2])],
        [pc(pair[1][0]), pc(pair[1][1]), pc(pair[1][2])],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(name: &str) -> &'static ChordQuality {
        ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
    }

    // Roots: C=0, Db=1, D=2, Eb=3, E=4, F=5, F#=6, G=7, Ab=8, A=9, Bb=10, B=11.

    #[test]
    fn m7_shells_match_airegin_labels() {
        // Fm7 (Eb, Bb, D + Ab, C, G) — offsets b7,11,13 | b3,5,9 from F=5.
        let (a, b) = resolve_shell_pair(5, quality("m7"));
        assert_eq!(a, [3, 10, 2]); // Eb, Bb, D
        assert_eq!(b, [8, 0, 7]); // Ab, C, G
    }

    #[test]
    fn maj7_shells_are_lydian_and_match_labels() {
        // C∆7 (B, F#, A + E, G, D) — offsets 7,#11,13 | 3,5,9 from C=0.
        let (a, b) = resolve_shell_pair(0, quality("maj7"));
        assert_eq!(a, [11, 6, 9]); // B, F#, A
        assert_eq!(b, [4, 7, 2]); // E, G, D
    }

    #[test]
    fn dominant_defaults_to_altered_shells() {
        // G7 → 7alt shells (G7alt label: F, B, Eb + Bb, Db, Ab) from G=7.
        let (a, b) = resolve_shell_pair(7, quality("dom7"));
        assert_eq!(a, [5, 11, 3]); // F, B, Eb
        assert_eq!(b, [10, 1, 8]); // Bb, Db, Ab
    }

    #[test]
    fn m7b5_shells_match_labels() {
        // Cm7b5 (Bb, F, Ab + Eb, Gb, D) — offsets b7,11,b13 | b3,b5,9 from C=0.
        let (a, b) = resolve_shell_pair(0, quality("m7b5"));
        assert_eq!(a, [10, 5, 8]); // Bb, F, Ab
        assert_eq!(b, [3, 6, 2]); // Eb, Gb, D
    }

    #[test]
    fn absent_quality_falls_back_to_literal_shells() {
        // dim7 has no table entry → literal: 7no5 = root-3-7, 7no3 = root-5-7.
        // C dim7 intervals = [1, b3, b5, bb7] = [0, 3, 6, 9].
        let (a, b) = resolve_shell_pair(0, quality("dim7"));
        assert_eq!(a, [0, 3, 9]); // root, b3, bb7
        assert_eq!(b, [0, 6, 9]); // root, b5, bb7
    }

    #[test]
    fn shells_are_transposition_invariant() {
        // Same m7 shape one whole step up (F=5 → G=7): every pc shifts by +2 mod 12.
        let (a5, b5) = resolve_shell_pair(5, quality("m7"));
        let (a7, b7) = resolve_shell_pair(7, quality("m7"));
        for i in 0..3 {
            assert_eq!(a7[i], (a5[i] + 2) % 12);
            assert_eq!(b7[i], (b5[i] + 2) % 12);
        }
    }
}
