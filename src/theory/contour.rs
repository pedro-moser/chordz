//! Melodic contour (CSEG) realization: turn an ordinal shape into fretboard notes.
//!
//! A contour fixes the *shape* of a cell — which note is lowest, which is highest — while
//! `Shape`/`Shape::Order` fixes its *identity*, i.e. which pitch classes it plays. The two are
//! independent axes. Realizing a contour means choosing, for each wanted pitch class, which of
//! its in-region occurrences to use, so the ordinal ranking comes out as requested.
//!
//! No octave arithmetic is involved: the fretboard already is a quantized register axis, and
//! restricting candidates to the region gives the position constraint for free.

use crate::theory::line_pattern::{PatternBlock, Shape};
use crate::theory::position::{FretNote, PositionSet};
use crate::theory::triad_shape::TriadShape;
use crate::voicings::fretboard::Fretboard;

/// Cost of two consecutive notes landing on one string — it forces a slide or a legato where
/// the player expects a picked note. Set above any realistic in-window fret span so compactness
/// can never buy a string reuse back.
const SAME_STRING: i32 = 24;

/// Per-note toll for leaving the hand shape the connector chose. Small enough that a contour
/// which genuinely needs displacement can pay it, large enough that nothing drifts for free.
const OFF_GRIP: i32 = 6;

/// Enumeration guard. Real cells settle in a few dozen assignments; this only bounds pathology.
const MAX_ASSIGNMENTS: usize = 512;

/// Resolve one cell into fretboard notes, in playing order.
///
/// Returns an empty vector when the block carries no contour. Otherwise always returns exactly
/// `contour.len()` notes — degrading to the longest correct rank prefix if the region cannot
/// realize the full shape.
pub fn resolve_cell(
    grip: &TriadShape,
    role_pcs: &[u8; 3],
    positions: &PositionSet,
    fretboard: &Fretboard,
    block: &PatternBlock,
    cell_index: usize,
    prev_midi: Option<i32>,
) -> Vec<FretNote> {
    let contour = match &block.contour {
        Some(c) => c,
        None => return Vec::new(),
    };
    let n = contour.len();

    // One candidate list per playing position: every in-region occurrence of its pitch class.
    let candidates: Vec<Vec<FretNote>> = (0..n)
        .map(|j| {
            let pc = cell_pitch_class(grip, role_pcs, block, cell_index, j, n);
            occurrences(positions, fretboard, pc)
        })
        .collect();

    // by_rank[r] is the playing position holding rank r+1, i.e. the r-th lowest note.
    let mut by_rank = vec![0usize; n];
    for (j, &rank) in contour.iter().enumerate() {
        by_rank[(rank - 1) as usize] = j;
    }

    let mut search = Search {
        candidates: &candidates,
        by_rank: &by_rank,
        grip_midis: grip.notes.iter().map(|note| note.midi).collect(),
        prev_midi,
        visited: 0,
        best: None,
        deepest: (0, vec![None; n]),
    };
    search.extend(0, i32::MIN, &mut vec![None; n]);

    match search.best.take() {
        Some((_, cell)) => cell,
        // Unrealizable here: keep the ranks we did satisfy, fill the rest with each position's
        // lowest in-region occurrence. Deterministic, and it degrades gradually.
        None => {
            let (_, partial) = search.deepest;
            (0..n)
                .map(|j| {
                    partial[j]
                        .or_else(|| candidates[j].first().copied())
                        .unwrap_or(grip.notes[j % 3])
                })
                .collect()
        }
    }
}

/// Which pitch class this cell wants at playing position `j`.
///
/// For `Shape::Order` the identity comes from the triad's roles, cycling over the block exactly
/// as `note_at` does. For `Shape::Monotonic` it is the grip's own pitch classes in ascending
/// order — which is what makes `<1 2 3>` reproduce today's ascending walk note for note.
fn cell_pitch_class(
    grip: &TriadShape,
    role_pcs: &[u8; 3],
    block: &PatternBlock,
    cell_index: usize,
    j: usize,
    n: usize,
) -> u8 {
    match &block.shape {
        Shape::Order(order) => {
            let k = cell_index * n + j;
            role_pcs[(order[k % order.len()] % 3) as usize]
        }
        Shape::Monotonic => grip.notes[j % 3].pitch_class,
    }
}

/// Every in-region occurrence of `pc`, ordered by pitch then string so the search is stable.
/// Duplicates at the same pitch on different strings are kept — the cost function decides.
fn occurrences(positions: &PositionSet, fretboard: &Fretboard, pc: u8) -> Vec<FretNote> {
    let mut notes = positions.find_notes(fretboard, &[pc]);
    notes.sort_by_key(|note| (note.midi, note.string));
    notes
}

/// Depth-first walk over rank order, keeping only strictly ascending assignments.
struct Search<'a> {
    candidates: &'a [Vec<FretNote>],
    by_rank: &'a [usize],
    grip_midis: Vec<i32>,
    prev_midi: Option<i32>,
    visited: usize,
    best: Option<(i32, Vec<FretNote>)>,
    deepest: (usize, Vec<Option<FretNote>>),
}

impl Search<'_> {
    fn extend(&mut self, rank: usize, floor: i32, chosen: &mut Vec<Option<FretNote>>) {
        if self.visited >= MAX_ASSIGNMENTS {
            return;
        }
        if rank > self.deepest.0 {
            self.deepest = (rank, chosen.clone());
        }
        if rank == self.by_rank.len() {
            self.visited += 1;
            let cell: Vec<FretNote> = chosen.iter().map(|slot| slot.expect("filled")).collect();
            let cost = self.score(&cell);
            if self.best.as_ref().is_none_or(|(best, _)| cost < *best) {
                self.best = Some((cost, cell));
            }
            return;
        }
        let position = self.by_rank[rank];
        for candidate in &self.candidates[position] {
            // Strictly above the previous rank — equal pitches would make the ranking ambiguous.
            if candidate.midi <= floor {
                continue;
            }
            chosen[position] = Some(*candidate);
            self.extend(rank + 1, candidate.midi, chosen);
            if self.visited >= MAX_ASSIGNMENTS {
                break;
            }
        }
        chosen[position] = None;
    }

    fn score(&self, cell: &[FretNote]) -> i32 {
        let mut cost = 0i32;
        for pair in cell.windows(2) {
            if pair[0].string == pair[1].string {
                cost += SAME_STRING;
            }
        }
        let frets: Vec<i32> = cell.iter().map(|note| note.fret as i32).collect();
        cost += frets.iter().max().unwrap_or(&0) - frets.iter().min().unwrap_or(&0);
        cost += OFF_GRIP
            * cell
                .iter()
                .filter(|note| !self.grip_midis.contains(&note.midi))
                .count() as i32;
        if let Some(prev) = self.prev_midi {
            cost += (cell[0].midi - prev).abs();
        }
        cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::line_pattern::{
        Anchor, Connector, Direction, PatternBlock, Shape, TriadId,
    };
    use crate::theory::position::PositionSet;
    use crate::theory::triad_shape::inversion_ladder;
    use crate::voicings::fretboard::Fretboard;

    /// A block carrying one contour, everything else at its legacy default.
    fn block_with(contour: Vec<u8>, shape: Shape) -> PatternBlock {
        PatternBlock {
            count: contour.len() as u8,
            direction: Direction::Ascending,
            triad: TriadId::T1,
            shape,
            anchor: Anchor::Nearest,
            hold_last: 0,
            lead_rest: 0,
            connector: Connector::default(),
            contour: Some(contour),
        }
    }

    /// C major triad [C=0, E=4, G=7] around the 5th-fret box, lowest rung.
    fn fixture() -> (Fretboard, PositionSet, crate::theory::triad_shape::TriadShape, [u8; 3]) {
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let pcs = [0u8, 4, 7];
        let grips = inversion_ladder(&fb, &positions, &pcs);
        assert!(!grips.is_empty(), "fixture must produce at least one grip");
        (fb, positions, grips[0].clone(), pcs)
    }

    /// The ordinal ranking of a resolved cell, 1-based — the inverse of a contour vector.
    fn ranks_of(cell: &[crate::theory::position::FretNote]) -> Vec<u8> {
        let mut sorted: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        sorted.sort_unstable();
        cell.iter()
            .map(|n| (sorted.iter().position(|m| *m == n.midi).unwrap() + 1) as u8)
            .collect()
    }

    #[test]
    fn every_contour_produces_its_own_ordinal_shape() {
        let (fb, positions, grip, pcs) = fixture();
        for c in [
            vec![1u8, 2, 3],
            vec![1, 3, 2],
            vec![2, 1, 3],
            vec![2, 3, 1],
            vec![3, 1, 2],
            vec![3, 2, 1],
        ] {
            let block = block_with(c.clone(), Shape::Monotonic);
            let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
            assert_eq!(cell.len(), 3, "contour {c:?} must yield 3 notes");
            assert_eq!(ranks_of(&cell), c, "contour {c:?} not realized");
        }
    }

    #[test]
    fn monotonic_contours_reuse_the_cursor_grip_exactly() {
        // <1 2 3> is what the grip already produces, so the grip-affinity term must elect the
        // grip's own notes. This is the property the engine-level non-regression test rests on.
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![1, 2, 3], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let got: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        let want: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        assert_eq!(got, want, "monotonic contour must not leave the grip");
    }

    #[test]
    fn scrambled_contours_must_leave_the_grip() {
        // Inside one grip each pitch class has exactly one midi, hence exactly one rank — so a
        // non-monotonic contour is unrealizable in-grip by construction and must displace.
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![3, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let grip_midis: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        let cell_midis: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        assert_ne!(cell_midis, grip_midis);
        assert_eq!(ranks_of(&cell), vec![3, 1, 2]);
    }

    #[test]
    fn all_notes_stay_inside_the_region() {
        let (fb, positions, grip, pcs) = fixture();
        let legal: Vec<i32> = positions
            .find_notes(&fb, &pcs)
            .iter()
            .map(|n| n.midi)
            .collect();
        for c in [vec![2u8, 3, 1], vec![3, 1, 2], vec![1, 3, 2]] {
            let block = block_with(c.clone(), Shape::Monotonic);
            let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
            for n in &cell {
                assert!(legal.contains(&n.midi), "contour {c:?} escaped the region");
            }
        }
    }

    #[test]
    fn resolution_is_deterministic() {
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![2, 3, 1], Shape::Monotonic);
        let a = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, Some(60));
        let b = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, Some(60));
        let am: Vec<i32> = a.iter().map(|n| n.midi).collect();
        let bm: Vec<i32> = b.iter().map(|n| n.midi).collect();
        assert_eq!(am, bm);
    }

    #[test]
    fn order_plus_contour_is_independent_of_the_starting_rung() {
        // The leak this feature exists to close: today the contour of an Order block drifts with
        // whichever rung the connector landed on. With a contour set, the shape must come out
        // identical from any rung.
        let (fb, positions, _, pcs) = fixture();
        let grips = inversion_ladder(&fb, &positions, &pcs);
        assert!(grips.len() >= 2, "fixture needs at least two rungs");
        let block = block_with(vec![2, 3, 1], Shape::Order(vec![0, 2, 1]));
        let from_first = resolve_cell(&grips[0], &pcs, &positions, &fb, &block, 0, None);
        let from_second = resolve_cell(&grips[1], &pcs, &positions, &fb, &block, 0, None);
        assert_eq!(ranks_of(&from_first), vec![2, 3, 1]);
        assert_eq!(ranks_of(&from_second), vec![2, 3, 1]);
    }

    #[test]
    fn no_contour_yields_no_notes() {
        let (fb, positions, grip, pcs) = fixture();
        let mut block = block_with(vec![1, 2, 3], Shape::Monotonic);
        block.contour = None;
        assert!(resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None).is_empty());
    }

    #[test]
    fn a_region_too_narrow_degrades_instead_of_panicking() {
        // A single-fret window cannot supply three ranked octaves. The resolver must return a
        // full-length cell anyway (longest correct prefix, then lowest available), never panic
        // and never emit an out-of-region note.
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let pcs = [0u8, 4, 7];
        let grips = inversion_ladder(&fb, &positions, &pcs);
        let narrow = PositionSet::from_base_frets(&[1]);
        let block = block_with(vec![3, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grips[0], &pcs, &narrow, &fb, &block, 0, None);
        assert_eq!(cell.len(), 3);
        let legal: Vec<i32> = narrow.find_notes(&fb, &pcs).iter().map(|n| n.midi).collect();
        for n in &cell {
            assert!(legal.contains(&n.midi));
        }
    }
}
