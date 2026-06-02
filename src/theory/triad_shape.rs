//! Ergonomic triad fingerings ("grips") and per-chord cell selection.
//!
//! A line that just picks the nearest pitch from every fretboard position of a triad's pitch
//! classes produces unplayable fingerings (open strings, three notes crammed on one string).
//! Real triads are fingered on adjacent strings in one of three distributions — `1+1+1`
//! (one note on each of three strings), `2+1`, or `1+2` (three notes across two strings) — with
//! no open strings and a small fret span. This module enumerates those legal grips and chooses
//! one `(T1, T2)` *cell* per chord by voice-leading, so every triad the line plays is, by
//! construction, one of the three formats.

use crate::theory::position::{FretNote, PositionSet};
use crate::voicings::fretboard::Fretboard;

/// Largest fret span (highest − lowest fret) allowed within one triad grip — a comfortable
/// four-fret hand box.
pub const MAX_FRET_SPAN: u8 = 4;

/// A triad's three pitch classes placed as a single playable grip: on 2 or 3 adjacent strings
/// (distribution `1+1+1`, `2+1`, or `1+2`), no open strings, fret span ≤ [`MAX_FRET_SPAN`].
/// `notes` is sorted by pitch.
#[derive(Clone, Debug)]
pub struct TriadShape {
    pub notes: [FretNote; 3],
}

impl TriadShape {
    /// The grip's hand position as `(mean string, mean fret)` — the anchor used to voice-lead
    /// from one grip to the next.
    pub fn center(&self) -> (f32, f32) {
        let strings: u32 = self.notes.iter().map(|n| n.string as u32).sum();
        let frets: u32 = self.notes.iter().map(|n| n.fret as u32).sum();
        (strings as f32 / 3.0, frets as f32 / 3.0)
    }
}

/// One chord's chosen fingering: a grip for each triad of the pair, kept close together so the
/// pair reads as a single hand position.
#[derive(Clone, Debug)]
pub struct TriadCell {
    pub t1: TriadShape,
    pub t2: TriadShape,
}

impl TriadCell {
    /// The cell's hand position — midpoint of its two grips.
    pub fn center(&self) -> (f32, f32) {
        let (s1, f1) = self.t1.center();
        let (s2, f2) = self.t2.center();
        ((s1 + s2) / 2.0, (f1 + f2) / 2.0)
    }
}

/// All legal grips for a triad (its three pitch classes `pcs`) inside the allowed region.
///
/// Candidate notes come from [`PositionSet::find_notes`] (which already enforces the region),
/// minus open strings; grips are then formed by combining notes on 2 or 3 *adjacent* strings in
/// the valid distributions and keeping only the compact ones.
pub fn enumerate_shapes(
    fretboard: &Fretboard,
    positions: &PositionSet,
    pcs: &[u8; 3],
) -> Vec<TriadShape> {
    let num_strings = fretboard.num_strings();

    // In-region, fretted (non-open) notes of the triad's pitch classes, grouped by string.
    let mut by_string: Vec<Vec<FretNote>> = vec![Vec::new(); num_strings];
    for n in positions.find_notes(fretboard, pcs) {
        if n.fret >= 1 {
            by_string[n.string as usize].push(n);
        }
    }

    let mut shapes = Vec::new();

    // 1+1+1 — three consecutive strings, one note each.
    for s in 0..num_strings.saturating_sub(2) {
        for a in &by_string[s] {
            for b in &by_string[s + 1] {
                for c in &by_string[s + 2] {
                    push_if_valid(&mut shapes, [*a, *b, *c], pcs);
                }
            }
        }
    }

    // 2+1 and 1+2 — two consecutive strings, three notes total.
    for s in 0..num_strings.saturating_sub(1) {
        let (lo, hi) = (&by_string[s], &by_string[s + 1]);
        // two on the lower string, one on the upper
        for i in 0..lo.len() {
            for j in (i + 1)..lo.len() {
                for h in hi {
                    push_if_valid(&mut shapes, [lo[i], lo[j], *h], pcs);
                }
            }
        }
        // one on the lower string, two on the upper
        for l in lo {
            for i in 0..hi.len() {
                for j in (i + 1)..hi.len() {
                    push_if_valid(&mut shapes, [*l, hi[i], hi[j]], pcs);
                }
            }
        }
    }

    shapes
}

/// Accept `notes` as a grip if they cover the triad's three pitch classes exactly once and fit
/// within [`MAX_FRET_SPAN`]; store them sorted by pitch. (String adjacency, the per-string note
/// count, and the no-open-string rule are guaranteed by how the caller builds `notes`.)
fn push_if_valid(out: &mut Vec<TriadShape>, mut notes: [FretNote; 3], pcs: &[u8; 3]) {
    let mut got: Vec<u8> = notes.iter().map(|n| n.pitch_class).collect();
    got.sort_unstable();
    let mut want = pcs.to_vec();
    want.sort_unstable();
    if got != want {
        return;
    }
    let max = notes.iter().map(|n| n.fret).max().unwrap();
    let min = notes.iter().map(|n| n.fret).min().unwrap();
    if max - min > MAX_FRET_SPAN {
        return;
    }
    notes.sort_by_key(|n| n.midi);
    out.push(TriadShape { notes });
}

/// Choose one `(T1, T2)` cell from the available grips. The cost rewards a compact pair (both
/// grips in the same hand area) and, when `prev` is given, minimal movement from the previous
/// chord's hand position. With no previous anchor (the first chord) it seeds toward a compact,
/// low-neck grip. Returns `None` if either triad has no legal grip in the region.
pub fn select_cell(
    t1_shapes: &[TriadShape],
    t2_shapes: &[TriadShape],
    prev: Option<(f32, f32)>,
) -> Option<TriadCell> {
    let mut best: Option<(f32, usize, usize)> = None;
    for (i, t1) in t1_shapes.iter().enumerate() {
        let (s1, f1) = t1.center();
        for (j, t2) in t2_shapes.iter().enumerate() {
            let (s2, f2) = t2.center();
            let pair_cost = (s1 - s2).abs() + (f1 - f2).abs();
            let (cs, cf) = ((s1 + s2) / 2.0, (f1 + f2) / 2.0);
            let move_cost = match prev {
                Some((ps, pf)) => (cs - ps).abs() + (cf - pf).abs(),
                // First chord: no hand to lead from — prefer a low, compact grip deterministically.
                None => cf * 0.5,
            };
            let cost = pair_cost + move_cost;
            if best.is_none_or(|(bc, _, _)| cost < bc - 1e-6) {
                best = Some((cost, i, j));
            }
        }
    }
    best.map(|(_, i, j)| TriadCell {
        t1: t1_shapes[i].clone(),
        t2: t2_shapes[j].clone(),
    })
}

/// The triad's grips arranged as an **inversion ladder**: one grip per distinct pitch-set (the
/// most compact fingering for each), ordered ascending by lowest note. Walking it one rung at a
/// time steps through the inversions (root → 1st → 2nd → root+8va → …) — the substrate an
/// "invert" connector climbs.
pub fn inversion_ladder(
    fretboard: &Fretboard,
    positions: &PositionSet,
    pcs: &[u8; 3],
) -> Vec<TriadShape> {
    // Dedup by pitch-set, keeping the most compact fingering (smallest fret span, then lowest
    // strings). A BTreeMap keyed on the sorted pitch triple yields the rungs already ordered by
    // lowest note.
    let mut best: std::collections::BTreeMap<[i32; 3], TriadShape> = std::collections::BTreeMap::new();
    for g in enumerate_shapes(fretboard, positions, pcs) {
        let key = [g.notes[0].midi, g.notes[1].midi, g.notes[2].midi];
        let rank = grip_rank(&g);
        if best.get(&key).is_none_or(|e| rank < grip_rank(e)) {
            best.insert(key, g);
        }
    }
    best.into_values().collect()
}

/// Lower is more ergonomic: prefer a tighter fret span, then lower (toward the bass) strings.
fn grip_rank(g: &TriadShape) -> (u8, u32) {
    let span =
        g.notes.iter().map(|n| n.fret).max().unwrap() - g.notes.iter().map(|n| n.fret).min().unwrap();
    let string_sum: u32 = g.notes.iter().map(|n| n.string as u32).sum();
    (span, string_sum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voicings::fretboard::Fretboard;

    /// Assert a triad grip obeys all ergonomic rules.
    fn assert_valid_shape(sh: &TriadShape, pcs: &[u8; 3]) {
        // Exactly the three triad pitch classes, one each.
        let mut got: Vec<u8> = sh.notes.iter().map(|n| n.pitch_class).collect();
        got.sort();
        let mut want = pcs.to_vec();
        want.sort();
        assert_eq!(got, want, "grip must cover the triad's pitch classes exactly once");

        // No open strings.
        assert!(sh.notes.iter().all(|n| n.fret >= 1), "grip uses an open string");

        // Adjacent strings + valid distribution (1+1+1, 2+1, 1+2).
        let strings: Vec<u8> = sh.notes.iter().map(|n| n.string).collect();
        let distinct: std::collections::BTreeSet<u8> = strings.iter().copied().collect();
        let span_strings = distinct.iter().max().unwrap() - distinct.iter().min().unwrap();
        match distinct.len() {
            3 => assert_eq!(span_strings, 2, "1+1+1 must be three consecutive strings"),
            2 => assert_eq!(span_strings, 1, "two-string grip must use adjacent strings"),
            n => panic!("a triad grip must use 2 or 3 strings, got {}", n),
        }
        for s in &distinct {
            let count = strings.iter().filter(|x| *x == s).count();
            assert!(count <= 2, "more than two notes on string {}", s);
        }

        // Compact fret span, notes sorted by pitch.
        let frets: Vec<u8> = sh.notes.iter().map(|n| n.fret).collect();
        let span = frets.iter().max().unwrap() - frets.iter().min().unwrap();
        assert!(span <= MAX_FRET_SPAN, "fret span {} exceeds {}", span, MAX_FRET_SPAN);
        for i in 1..sh.notes.len() {
            assert!(sh.notes[i].midi >= sh.notes[i - 1].midi, "grip notes not sorted by pitch");
        }
    }

    #[test]
    fn enumerate_shapes_yields_only_valid_grips_free_mode() {
        let fb = Fretboard::standard_tuning();
        let shapes = enumerate_shapes(&fb, &PositionSet::unrestricted(), &[0, 4, 7]);
        assert!(!shapes.is_empty(), "C major triad must have grips on the neck");
        for sh in &shapes {
            assert_valid_shape(sh, &[0, 4, 7]);
        }
    }

    #[test]
    fn enumerate_shapes_finds_each_distribution() {
        // Across the whole neck a major triad has grips of all three distributions.
        let fb = Fretboard::standard_tuning();
        let shapes = enumerate_shapes(&fb, &PositionSet::unrestricted(), &[0, 4, 7]);
        let has_three = shapes.iter().any(|s| {
            s.notes.iter().map(|n| n.string).collect::<std::collections::BTreeSet<_>>().len() == 3
        });
        let has_two = shapes.iter().any(|s| {
            s.notes.iter().map(|n| n.string).collect::<std::collections::BTreeSet<_>>().len() == 2
        });
        assert!(has_three, "expected at least one 1+1+1 grip");
        assert!(has_two, "expected at least one 2+1 / 1+2 grip");
    }

    #[test]
    fn enumerate_shapes_stays_in_region() {
        // Restricted to position V (stretch 4..9): every grip note lies inside it.
        let fb = Fretboard::standard_tuning();
        let shapes = enumerate_shapes(&fb, &PositionSet::from_base_frets(&[5]), &[0, 4, 7]);
        assert!(!shapes.is_empty());
        for sh in &shapes {
            for n in &sh.notes {
                assert!(n.fret >= 4 && n.fret <= 9, "grip note at fret {} left position V", n.fret);
            }
        }
    }

    #[test]
    fn inversion_ladder_is_sorted_unique_and_all_valid() {
        // The ladder is the climb path: one grip per distinct pitch-set, ascending by lowest
        // note. Walking it rung-by-rung steps through the inversions.
        let fb = Fretboard::standard_tuning();
        let ladder = inversion_ladder(&fb, &PositionSet::from_base_frets(&[5]), &[0, 4, 7]);
        assert!(ladder.len() >= 2, "need several rungs to climb; got {}", ladder.len());
        for i in 1..ladder.len() {
            assert!(
                ladder[i].notes[0].midi >= ladder[i - 1].notes[0].midi,
                "ladder not sorted ascending by lowest note",
            );
        }
        let sets: std::collections::BTreeSet<[i32; 3]> = ladder
            .iter()
            .map(|g| [g.notes[0].midi, g.notes[1].midi, g.notes[2].midi])
            .collect();
        assert_eq!(sets.len(), ladder.len(), "duplicate pitch-sets in ladder");
        for g in &ladder {
            assert_valid_shape(g, &[0, 4, 7]);
        }
    }

    #[test]
    fn inversion_ladder_steps_by_a_single_tone() {
        // Consecutive rungs should differ by one tone (a true inversion step), not a whole
        // octave — that's what makes the staircase overlap and climb smoothly.
        let fb = Fretboard::standard_tuning();
        let ladder = inversion_ladder(&fb, &PositionSet::unrestricted(), &[0, 4, 7]);
        // Somewhere in the ladder, two consecutive rungs share two pitches (the inversion overlap).
        let overlaps = (1..ladder.len()).any(|i| {
            let prev: std::collections::BTreeSet<i32> =
                ladder[i - 1].notes.iter().map(|n| n.midi).collect();
            let shared = ladder[i].notes.iter().filter(|n| prev.contains(&n.midi)).count();
            shared == 2
        });
        assert!(overlaps, "expected an inversion step sharing two pitches");
    }

    #[test]
    fn select_cell_returns_none_when_a_triad_has_no_grip() {
        assert!(select_cell(&[], &[], None).is_none());
    }

    #[test]
    fn select_cell_voice_leads_toward_the_previous_hand_position() {
        // Same triad on two regions; with a previous anchor up the neck, the cell should land
        // up the neck rather than at the nut.
        let fb = Fretboard::standard_tuning();
        let low = enumerate_shapes(&fb, &PositionSet::from_base_frets(&[2]), &[0, 4, 7]);
        let high = enumerate_shapes(&fb, &PositionSet::from_base_frets(&[10]), &[0, 4, 7]);
        let mut all = low.clone();
        all.extend(high.clone());
        // Anchor near fret 11 → chosen T1 grip should be the high-neck one.
        let cell = select_cell(&all, &all, Some((2.5, 11.0))).expect("a cell exists");
        assert!(cell.t1.center().1 >= 8.0, "expected a high-neck grip near the previous hand");
    }
}
