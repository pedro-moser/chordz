//! Melodic contour (CSEG) realization: turn an ordinal shape into fretboard notes.
//!
//! A contour fixes the *shape* of a cell — which note is lowest, which is highest — while
//! `Shape`/`Shape::Order` fixes its *identity*, i.e. which pitch classes it plays. The two are
//! independent axes. Realizing a contour means choosing, for each wanted pitch class, which of
//! its in-region occurrences to use, so the ordinal ranking comes out as requested.
//!
//! No octave arithmetic is involved: the fretboard already is a quantized register axis, and
//! restricting candidates to the region gives the position constraint for free.

use crate::theory::line_pattern::{Direction, PatternBlock, Shape};
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
/// Returns an empty vector when the block carries no contour, when the contour is malformed
/// (not a permutation of `1..=n`), or when some playing position has no in-region occurrence of
/// its wanted pitch class — in every such case the caller is expected to fall back to its own
/// in-region note selection. Otherwise always returns exactly `contour.len()` notes, degrading to
/// the longest correct rank prefix if the region cannot realize the full shape. Every note this
/// function returns is guaranteed to lie in `positions`; it never panics.
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
    // A malformed contour (rank 0, a rank above n, or a repeated rank) has no meaning as a
    // ranking. Bail out rather than let `by_rank`'s construction below panic on it.
    if !crate::theory::line_pattern::is_valid_contour(contour) {
        return Vec::new();
    }
    let n = contour.len();

    // One candidate list per playing position: every in-region occurrence of its pitch class.
    let candidates: Vec<Vec<FretNote>> = (0..n)
        .map(|j| {
            let pc = cell_pitch_class(grip, role_pcs, block, cell_index, j, n);
            occurrences(positions, fretboard, pc)
        })
        .collect();

    // If any playing position has no in-region occurrence of its wanted pitch class, there is no
    // legal note for it — neither the search nor the degradation fallback can conjure one without
    // escaping the region. Bail out; the caller falls back to its own in-region note selection.
    if candidates.iter().any(|c| c.is_empty()) {
        return Vec::new();
    }

    // Short-circuit before any scoring: if the cursor grip's own notes already realize the
    // requested identity and contour, use them directly, no search.
    //
    // The search below elects a cell by cost, and grips are not guaranteed three distinct
    // strings — they legally reuse one (e.g. a 2+1 distribution). A reused string can make the
    // grip's own notes lose to a displaced alternative on `SAME_STRING` alone, even for the
    // identity contour `<1 2 3>`. Retuning the weights to keep the grip in that case would just
    // be a different bet that some other grip breaks later; testing the grip's own notes
    // directly, before cost ever enters, makes staying in the grip structural instead.
    let grip_cell: Option<Vec<FretNote>> = (0..n)
        .map(|j| {
            let pc = cell_pitch_class(grip, role_pcs, block, cell_index, j, n);
            grip_note_for(grip, pc).filter(|note| candidates[j].contains(note))
        })
        .collect();
    if let Some(cell) = grip_cell {
        if realizes_contour(&cell, contour) {
            return cell;
        }
    }

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
        // lowest in-region occurrence. Deterministic, and it degrades gradually. The guard above
        // guarantees every `candidates[j]` is non-empty, so `first()` always has an in-region note.
        None => {
            let (_, partial) = search.deepest;
            (0..n)
                .map(|j| {
                    partial[j].unwrap_or_else(|| {
                        candidates[j]
                            .first()
                            .copied()
                            .expect("guarded: candidates[j] is never empty here")
                    })
                })
                .collect()
        }
    }
}

/// Which pitch class this cell wants at playing position `j`.
///
/// For `Shape::Order` the identity comes from the triad's roles, cycling over the block exactly
/// as `note_at` does. For `Shape::Monotonic` it is the grip's own pitch classes, walked either
/// ascending or reversed — see [`monotonic_reversed`] for which. This is what makes `<1 2 3>`
/// reproduce today's ascending walk note for note, and `<3 2 1>` reproduce today's descending
/// walk: `Direction::Descending` is not "the contour `<3 2 1>` applied to an ascending identity",
/// it is the same walk with the identity reversed.
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
        Shape::Monotonic => {
            let idx = if monotonic_reversed(block) { 2 - (j % 3) } else { j % 3 };
            grip.notes[idx].pitch_class
        }
    }
}

/// Whether a `Shape::Monotonic` identity should walk the grip high-to-low rather than low-to-high.
///
/// A contour that is itself the canonical ascending (`<1 2 … n>`) or descending (`<n … 2 1>`)
/// permutation names its own walk direction outright: `<3 2 1>` *is* "play the grip top to
/// bottom", independent of whatever `block.direction` happens to carry (a block's `direction`
/// exists for the contour-less legacy walk; once a contour states the shape, the contour is the
/// more specific instruction and wins). Any other — scrambled — contour makes no such ascending
/// or descending claim about itself, so its identity falls back to `block.direction`, matching how
/// `note_at` walks a plain, contour-less block.
fn monotonic_reversed(block: &PatternBlock) -> bool {
    if let Some(contour) = &block.contour {
        let n = contour.len() as u8;
        if *contour == (1..=n).collect::<Vec<u8>>() {
            return false;
        }
        if *contour == (1..=n).rev().collect::<Vec<u8>>() {
            return true;
        }
    }
    block.direction == Direction::Descending
}

/// The exact fretboard occurrence the legacy walk would have played for playing position `j`:
/// the grip note whose pitch class is this position's identity. A grip covers its triad's three
/// pitch classes exactly once (see `triad_shape::push_if_valid`), so this is well-defined
/// whenever `pc` is genuinely one of the grip's three pitch classes; `None` otherwise (a guard,
/// not an assumption — `role_pcs` is caller-controlled).
fn grip_note_for(grip: &TriadShape, pc: u8) -> Option<FretNote> {
    grip.notes.iter().find(|note| note.pitch_class == pc).copied()
}

/// Whether `cell[j]`'s ordinal ranking by midi matches `contour[j]`'s ranks, for every pair of
/// positions. Contours are permutations (`is_valid_contour` guarantees distinct ranks 1..=n), so
/// two positions can never share a contour rank; if their midis tie, that can never satisfy any
/// contour, and the pairwise check below correctly reports a mismatch for it.
fn realizes_contour(cell: &[FretNote], contour: &[u8]) -> bool {
    let n = cell.len();
    (0..n).all(|a| {
        (0..n).all(|b| (contour[a] < contour[b]) == (cell[a].midi < cell[b].midi))
    })
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

    /// A `TriadShape` built from bare pitch classes — string/fret/midi are unused by the paths
    /// under test here (the grip's own notes only matter for `Shape::Monotonic`'s pitch-class
    /// lookup and for grip-affinity scoring, neither of which these tests exercise).
    fn dummy_grip(pcs: [u8; 3]) -> crate::theory::triad_shape::TriadShape {
        crate::theory::triad_shape::TriadShape {
            notes: pcs.map(|pc| FretNote { string: 0, fret: 0, midi: 0, pitch_class: pc }),
        }
    }

    // --- Finding 1: an empty candidate list must short-circuit, never fall through to a
    // fabricated note. ---
    //
    // `PositionSet::from_base_frets(&[24])` clamps to the top of a 24-fret neck: its stretch
    // range is nominally (23, 28) but `Fretboard::get_note` returns `None` past fret 24, so only
    // frets 23-24 actually contribute notes. Measured directly (`occurrences`): pitch classes 0
    // and 5 have zero occurrences there, e.g. pc 1 -> [73], pc 8 -> [68], pc 0 -> [], pc 5 -> [].
    #[test]
    fn empty_candidate_list_returns_empty_vec_not_a_fabricated_note() {
        let fb = Fretboard::standard_tuning();
        let top = PositionSet::from_base_frets(&[24]);
        assert!(
            occurrences(&top, &fb, 0).is_empty(),
            "fixture assumption: pc 0 must be absent from this region"
        );
        // grip pitch classes [0, 4, 7]; Shape::Monotonic reads pc from grip.notes[j % 3], so
        // position 0 wants pc 0 — the one this region cannot supply.
        let grip = dummy_grip([0, 4, 7]);
        let role_pcs = [0u8, 4, 7];
        let block = block_with(vec![1, 2, 3], Shape::Monotonic);
        let cell = resolve_cell(&grip, &role_pcs, &top, &fb, &block, 0, None);
        assert!(cell.is_empty(), "no legal note exists for position 0; must return empty, not guess");
    }

    // --- Finding 2: a malformed contour must never reach `by_rank`'s indexing. ---
    #[test]
    fn contour_with_rank_zero_returns_empty_vec_not_a_panic() {
        let (fb, positions, grip, pcs) = fixture();
        let block = block_with(vec![0, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        assert!(cell.is_empty());
    }

    #[test]
    fn contour_with_rank_above_len_returns_empty_vec_not_a_panic() {
        let (fb, positions, grip, pcs) = fixture();
        // n = 3, but rank 4 has no position — would index out of bounds in `by_rank`.
        let block = block_with(vec![1, 2, 4], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        assert!(cell.is_empty());
    }

    // --- Finding 3: an unsatisfiable-but-populated cell must degrade, not fail closed or
    // escape the region. ---
    //
    // Every position has non-empty candidates (Finding 1's guard does not fire), but no
    // strictly-ascending assignment exists: contour `<3 1 2>` assigns position 1 the *lowest*
    // rank and position 2 the *middle* rank, yet within this near-the-top-of-the-neck region
    // (frets 23-24 only, same clamped window as above) position 1's only candidate (pc 6 -> 78)
    // sits strictly above position 2's only candidate (pc 8 -> 68). The search picks position 1's
    // note first (rank 1 is resolved before rank 2), so no candidate is left for position 2 that
    // is strictly greater — the DFS dead-ends at depth 1 for every branch, `search.best` stays
    // `None`, and the longest-correct-prefix fallback must fill the rest from `candidates[j].first()`.
    #[test]
    fn unsatisfiable_ranking_degrades_to_longest_correct_prefix_in_region() {
        let fb = Fretboard::standard_tuning();
        let top = PositionSet::from_base_frets(&[24]);
        // Confirm the fixture's premise experimentally rather than assuming it.
        assert_eq!(occurrences(&top, &fb, 6).iter().map(|n| n.midi).collect::<Vec<_>>(), vec![78]);
        assert_eq!(occurrences(&top, &fb, 8).iter().map(|n| n.midi).collect::<Vec<_>>(), vec![68]);
        assert_eq!(occurrences(&top, &fb, 9).iter().map(|n| n.midi).collect::<Vec<_>>(), vec![69]);

        let grip = dummy_grip([9, 6, 8]);
        let role_pcs = [9u8, 6, 8];
        // Shape::Order([0,1,2]) is the identity: position j wants role_pcs[j].
        let block = block_with(vec![3, 1, 2], Shape::Order(vec![0, 1, 2]));
        let cell = resolve_cell(&grip, &role_pcs, &top, &fb, &block, 0, None);

        assert_eq!(cell.len(), 3, "must still return a full-length cell");
        let legal: Vec<i32> = top.find_notes(&fb, &[9, 6, 8]).iter().map(|n| n.midi).collect();
        for n in &cell {
            assert!(legal.contains(&n.midi), "degraded cell escaped the region: {n:?}");
        }
        // The ranking is genuinely not what was asked for — this is a degradation, not a lucky
        // full match.
        assert_ne!(ranks_of(&cell), vec![3u8, 1, 2]);
    }

    // --- Defect 1: the Monotonic identity must reverse with `block.direction`, for contours that
    // don't already name their own walk direction. ---
    //
    // `<2 1 3>` is neither the canonical ascending (`<1 2 3>`) nor descending (`<3 2 1>`)
    // permutation, so it makes no claim about its own direction and `cell_pitch_class` falls back
    // to `block.direction` — exactly the branch defect 1 fixes. Every note `resolve_cell` returns
    // is drawn exclusively from occurrences of its position's identity pitch class, so position
    // 0's pitch class in the result is a direct readout of which identity was requested.
    #[test]
    fn descending_direction_reverses_the_monotonic_identity_for_a_scrambled_contour() {
        let (fb, positions, grip, pcs) = fixture();

        let ascending = block_with(vec![2, 1, 3], Shape::Monotonic);
        let cell_asc = resolve_cell(&grip, &pcs, &positions, &fb, &ascending, 0, None);
        assert_eq!(cell_asc.len(), 3);
        assert_eq!(
            cell_asc[0].pitch_class, grip.notes[0].pitch_class,
            "Ascending: position 0 must want the grip's lowest identity"
        );
        assert_eq!(ranks_of(&cell_asc), vec![2, 1, 3]);

        let mut descending = block_with(vec![2, 1, 3], Shape::Monotonic);
        descending.direction = Direction::Descending;
        let cell_desc = resolve_cell(&grip, &pcs, &positions, &fb, &descending, 0, None);
        assert_eq!(cell_desc.len(), 3);
        assert_eq!(
            cell_desc[0].pitch_class, grip.notes[2].pitch_class,
            "Descending: position 0 must want the grip's highest identity, reversed"
        );
        assert_eq!(ranks_of(&cell_desc), vec![2, 1, 3]);
    }

    /// A grip the engine can legitimately produce, reusing one string: two notes on string 5.
    /// `[(string 4, fret 5, midi 64), (string 5, fret 5, midi 69), (string 5, fret 8, midi 72)]`.
    /// Its own arrangement costs `SAME_STRING(24) + span(3) + glue(5) = 32`; a displaced
    /// alternative can cost `span(1) + OFF_GRIP(6*2) + glue(17) = 30` — cheaper. Without a
    /// short-circuit, the cost-driven search would leave this grip even for `<1 2 3>`.
    fn reused_string_grip(fb: &Fretboard) -> (crate::theory::triad_shape::TriadShape, [u8; 3]) {
        let mk = |string: u8, fret: u8| {
            let n = fb.get_note(string as usize, fret as usize).expect("in range");
            FretNote { string, fret, midi: n.midi(), pitch_class: n.pitch_class }
        };
        let notes = [mk(4, 5), mk(5, 5), mk(5, 8)];
        let pcs = [notes[0].pitch_class, notes[1].pitch_class, notes[2].pitch_class];
        (crate::theory::triad_shape::TriadShape { notes }, pcs)
    }

    // --- Defect 2: the short-circuit must fire on identity/contour match alone, not on cost. ---
    #[test]
    fn grip_short_circuit_fires_even_when_the_grip_reuses_a_string() {
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let (grip, pcs) = reused_string_grip(&fb);
        let block = block_with(vec![1, 2, 3], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let got: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        let want: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        assert_eq!(got, want, "short-circuit must keep the reused-string grip for <1 2 3>");
    }

    // The short-circuit must not over-apply: on this same reused-string grip, a genuinely
    // scrambled contour must still displace, paying the cost the grip's own notes cannot avoid.
    #[test]
    fn scrambled_contour_still_displaces_from_a_reused_string_grip() {
        let fb = Fretboard::standard_tuning();
        let positions = PositionSet::from_base_frets(&[5]);
        let (grip, pcs) = reused_string_grip(&fb);
        let block = block_with(vec![3, 1, 2], Shape::Monotonic);
        let cell = resolve_cell(&grip, &pcs, &positions, &fb, &block, 0, None);
        let grip_midis: Vec<i32> = grip.notes.iter().map(|n| n.midi).collect();
        let cell_midis: Vec<i32> = cell.iter().map(|n| n.midi).collect();
        assert_ne!(cell_midis, grip_midis, "a scrambled contour must not short-circuit to the grip");
        assert_eq!(ranks_of(&cell), vec![3, 1, 2]);
    }
}
