use std::collections::HashSet;
use std::time::SystemTime;

use super::fretboard::Fretboard;
use super::generate::{map_voice_set, Fingering};
use super::ranking::{rank_fingerings, score as fingering_score};
use super::recipe::VoicingRecipe;
use super::rules::VoicingRules;
use super::voice_leading::distance;
use super::voice_set::VoiceSet;
use crate::theory::chart::Chart;
use crate::theory::chords::ChordQuality;
use crate::theory::intervals::Interval;

/// A solved voice-leading path through a chord chart.
#[derive(Clone, Debug)]
pub struct SolvedChart {
    pub fingerings: Vec<SolvedChange>,
    /// All candidate voicings per chord position, for manual swapping.
    pub alternatives: Vec<Vec<SolvedAlternative>>,
}

/// A single chord in the solved path.
#[derive(Clone, Debug)]
pub struct SolvedChange {
    pub root: String,
    pub quality: &'static ChordQuality,
    pub beats: f32,
    pub fingering: Fingering,
    pub recipe: VoicingRecipe,
    pub tension: f32,
    pub normalized_tension: f32,
    pub rank_score: i32,
    pub relaxation: RelaxationLevel,
}

/// A candidate voicing retained for solving and manual swapping.
#[derive(Clone, Debug)]
pub struct SolvedAlternative {
    pub fingering: Fingering,
    pub recipe: VoicingRecipe,
    pub tension: f32,
    pub normalized_tension: f32,
    pub rank_score: i32,
    pub relaxation: RelaxationLevel,
}

/// How far the solver had to relax the user's filters to produce a candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelaxationLevel {
    Exact,
    FewerNotes,
    IgnoreStringFilter,
    WiderFretRange,
}

impl RelaxationLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::FewerNotes => "2-note fallback",
            Self::IgnoreStringFilter => "strings relaxed",
            Self::WiderFretRange => "range widened",
        }
    }

    pub const fn is_relaxed(self) -> bool {
        !matches!(self, Self::Exact)
    }
}

/// Configuration for the voice-leading solver.
#[derive(Clone, Debug)]
pub struct SolverConfig {
    pub rules: VoicingRules,
    pub recipes: Vec<VoicingRecipe>,
    pub max_candidates: usize,
    pub min_fret: u8,
    pub allowed_strings: Option<[bool; 6]>,
    pub allow_open_strings: bool,
    pub expand_basic_chords: bool,
    /// 0.0 = prefer grounded voicings (shells, drops), 1.0 = prefer abstract
    /// (quartal, upper structures). The solver adds a penalty proportional to
    /// how far each candidate's tension is from this target.
    pub tension_target: f32,
    /// How much tension mismatch costs relative to voice-leading distance.
    /// 0 = tension ignored, higher = stronger preference.
    pub tension_weight: f32,
    /// How strongly solved paths should prefer high-ranked ergonomic/musical
    /// candidates before considering movement.
    pub rank_weight: u32,
    /// Multiplier for voice-leading distance. Higher values prefer less hand
    /// movement even when another candidate has a better standalone score.
    pub smoothness_weight: f32,
    /// Random jitter added to DP transition costs so each Solve produces a
    /// different arrangement. 0 = fully deterministic.
    pub jitter: u32,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            rules: VoicingRules {
                min_strings: 3,
                max_strings: 4,
                max_fret_span: 5,
                max_fret: 15,
                require_root: false,
            },
            recipes: VoicingRecipe::all().to_vec(),
            max_candidates: 256,
            min_fret: 0,
            allowed_strings: None,
            allow_open_strings: true,
            expand_basic_chords: true,
            tension_target: 0.3,
            tension_weight: 6.0,
            rank_weight: 1,
            smoothness_weight: 1.0,
            jitter: 0,
        }
    }
}

const MAX_TENSION: f32 = 10.0;
const REPEAT_PENALTY: u32 = 50;

static DOM7_ALTERED_VOICING: ChordQuality = ChordQuality {
    name: "dom7alt_v",
    intervals: &[
        Interval::UNISON,
        Interval::M3,
        Interval::P5,
        Interval::m7,
        Interval::m9,
        Interval::SHARP9,
        Interval::m13,
    ],
};

static DOM7_LYDIAN_VOICING: ChordQuality = ChordQuality {
    name: "dom7lyd_v",
    intervals: &[
        Interval::UNISON,
        Interval::M3,
        Interval::P5,
        Interval::m7,
        Interval::M9,
        Interval::SHARP11,
        Interval::M13,
    ],
};

pub fn recipe_tension(recipe: VoicingRecipe) -> u32 {
    match recipe {
        VoicingRecipe::Shell => 0,
        VoicingRecipe::Closed => 1,
        VoicingRecipe::Drop2 => 1,
        VoicingRecipe::Drop3 => 1,
        VoicingRecipe::RootlessA => 2,
        VoicingRecipe::RootlessB => 2,
        VoicingRecipe::Quartal => 3,
        VoicingRecipe::UpperStructureTriad => 4,
        VoicingRecipe::TriadPair => 4,
    }
}

pub fn quality_tension(name: &str) -> u32 {
    match name {
        "maj7" | "m7" | "dom7" => 0,
        "maj9" | "m9" | "dom9" | "m7b5" | "dim7" => 1,
        "maj13" | "m11" | "m13" | "dom13" | "maj7#11" => 2,
        "dom7b9" | "dom7#9" | "dom7#5" | "dom7#11" | "dom7b13" | "m9b11" => 3,
        _ => 2,
    }
}

pub fn voice_set_tension(quality_name: &str, voice_set: &VoiceSet) -> f32 {
    let recipe_base = recipe_tension(voice_set.recipe) as f32;
    let quality_base = quality_tension(quality_name) as f32;
    let rootless = if !voice_set.intervals.contains(&Interval::UNISON) {
        1.5
    } else {
        0.0
    };
    let extensions = voice_set
        .intervals
        .iter()
        .filter(|i| i.semitones > 11)
        .count() as f32;
    (recipe_base + quality_base + rootless + extensions).min(MAX_TENSION) / MAX_TENSION
}

/// Solve optimal voice leading for a chord chart using dynamic programming.
///
/// For each chord in the chart, generates candidate fingerings, then finds
/// the sequence that minimizes total voice-leading distance (Viterbi-style).
///
/// Returns `None` if any chord produces zero candidates.
pub fn solve(chart: &Chart, fretboard: &Fretboard, config: &SolverConfig) -> Option<SolvedChart> {
    solve_with_locks(chart, fretboard, config, &[])
}

/// Solve a chart while forcing selected chord positions to keep a chosen
/// alternative. Locks are indexed by chart change position.
pub fn solve_with_locks(
    chart: &Chart,
    fretboard: &Fretboard,
    config: &SolverConfig,
    locks: &[Option<SolvedAlternative>],
) -> Option<SolvedChart> {
    let n = chart.changes.len();
    if n == 0 {
        return None;
    }

    // Generate candidates for each chord, auto-relaxing constraints if needed.
    let mut all_candidates: Vec<Vec<SolvedAlternative>> = Vec::with_capacity(n);
    for (i, change) in chart.changes.iter().enumerate() {
        if let Some(Some(locked)) = locks.get(i) {
            all_candidates.push(vec![locked.clone()]);
            continue;
        }

        let candidates = generate_candidates(change.root_pc, change.quality, fretboard, config);
        if !candidates.is_empty() {
            all_candidates.push(candidates);
            continue;
        }
        // Progressively relax: widen fret range, then drop string/note filters.
        let relaxed = generate_relaxed(change.root_pc, change.quality, fretboard, config);
        if relaxed.is_empty() {
            return None;
        }
        all_candidates.push(relaxed);
    }

    normalize_candidate_tensions(&mut all_candidates);

    // Tension weight scales with slider extremity: stronger at 0.0/1.0.
    let slider_extreme = (2.0 * config.tension_target - 1.0).abs();
    let effective_weight = config.tension_weight * (1.0 + 2.0 * slider_extreme);

    // Precompute tension penalty per candidate.
    let tension_penalties: Vec<Vec<u32>> = all_candidates
        .iter()
        .map(|cands| {
            cands
                .iter()
                .map(|candidate| {
                    let deviation = (candidate.normalized_tension - config.tension_target).abs();
                    (deviation * effective_weight * 10.0) as u32
                })
                .collect()
        })
        .collect();

    let rank_penalties: Vec<Vec<u32>> = all_candidates
        .iter()
        .map(|cands| {
            let best = cands.iter().map(|c| c.rank_score).max().unwrap_or(0);
            cands
                .iter()
                .map(|candidate| {
                    let delta = best.saturating_sub(candidate.rank_score).max(0) as u32;
                    delta.saturating_mul(config.rank_weight)
                })
                .collect()
        })
        .collect();

    let seed = if config.jitter > 0 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(42)
    } else {
        0
    };

    // DP: cost[i][j] = minimum total cost to reach candidate j of chord i.
    let mut cost: Vec<Vec<u32>> = Vec::with_capacity(n);
    let mut parent: Vec<Vec<usize>> = Vec::with_capacity(n);

    // Base case: first chord, tension penalty + jitter.
    let base_costs: Vec<u32> = tension_penalties[0]
        .iter()
        .enumerate()
        .map(|(j, &tp)| {
            tp.saturating_add(rank_penalties[0][j])
                .saturating_add(jitter_noise(seed, 0, j, 0, config.jitter))
        })
        .collect();
    cost.push(base_costs);
    parent.push(vec![0; all_candidates[0].len()]);

    // Fill forward.
    for i in 1..n {
        let m_prev = all_candidates[i - 1].len();
        let m_curr = all_candidates[i].len();
        let mut layer_cost = vec![u32::MAX; m_curr];
        let mut layer_parent = vec![0usize; m_curr];

        for j in 0..m_curr {
            let t_penalty = tension_penalties[i][j];
            for k in 0..m_prev {
                let d = weighted_distance(
                    distance(
                        &all_candidates[i - 1][k].fingering,
                        &all_candidates[i][j].fingering,
                        fretboard,
                    ),
                    config.smoothness_weight,
                );
                let repeat_penalty = if all_candidates[i][j].fingering.positions
                    == all_candidates[i - 1][k].fingering.positions
                {
                    REPEAT_PENALTY
                } else {
                    0
                };
                let noise = jitter_noise(seed, i, j, k, config.jitter);
                let total = cost[i - 1][k]
                    .saturating_add(d)
                    .saturating_add(t_penalty)
                    .saturating_add(rank_penalties[i][j])
                    .saturating_add(repeat_penalty)
                    .saturating_add(noise);
                if total < layer_cost[j] {
                    layer_cost[j] = total;
                    layer_parent[j] = k;
                }
            }
        }

        cost.push(layer_cost);
        parent.push(layer_parent);
    }

    // Backtrace: find the best endpoint, then walk parents.
    let last_layer = &cost[n - 1];
    let mut best_idx = 0;
    for (i, &c) in last_layer.iter().enumerate() {
        if c < last_layer[best_idx] {
            best_idx = i;
        }
    }

    let mut path = vec![0usize; n];
    path[n - 1] = best_idx;
    for i in (1..n).rev() {
        path[i - 1] = parent[i][path[i]];
    }

    let fingerings = path
        .iter()
        .enumerate()
        .map(|(i, &j)| {
            let change = &chart.changes[i];
            let candidate = &all_candidates[i][j];
            SolvedChange {
                root: change.root.clone(),
                quality: change.quality,
                beats: change.beats,
                fingering: candidate.fingering.clone(),
                recipe: candidate.recipe,
                tension: candidate.tension,
                normalized_tension: candidate.normalized_tension,
                rank_score: candidate.rank_score,
                relaxation: candidate.relaxation,
            }
        })
        .collect();

    let alternatives = all_candidates;

    Some(SolvedChart {
        fingerings,
        alternatives,
    })
}

/// Map a basic quality to its most extended family member for voicing generation.
///
/// Jazz convention: "Cmaj7" on a chart means "use any available diatonic
/// extension." This gives the voicing engine access to 9ths, 11ths, 13ths
/// that a guitarist would naturally add.
fn extended_for_voicing(quality: &'static ChordQuality) -> &'static ChordQuality {
    match quality.name {
        "dom7b9" | "dom7#9" | "dom7b13" => return &DOM7_ALTERED_VOICING,
        "dom7#11" => return &DOM7_LYDIAN_VOICING,
        _ => {}
    }
    let target = match quality.name {
        "maj7" | "maj9" => "maj13",
        "m7" | "m9" | "m11" => "m13",
        "dom7" | "dom9" => "dom13",
        "m7b5" => "m9b11",
        _ => return quality,
    };
    ChordQuality::ALL
        .iter()
        .find(|q| q.name == target)
        .unwrap_or(quality)
}

fn generate_candidates(
    root_pc: u8,
    quality: &'static ChordQuality,
    fretboard: &Fretboard,
    config: &SolverConfig,
) -> Vec<SolvedAlternative> {
    generate_candidates_with_relaxation(root_pc, quality, fretboard, config, RelaxationLevel::Exact)
}

fn generate_candidates_with_relaxation(
    root_pc: u8,
    quality: &'static ChordQuality,
    fretboard: &Fretboard,
    config: &SolverConfig,
    relaxation: RelaxationLevel,
) -> Vec<SolvedAlternative> {
    let rules = &config.rules;
    let mut all: Vec<SolvedAlternative> = Vec::new();
    let voicing_quality = if config.expand_basic_chords {
        extended_for_voicing(quality)
    } else {
        quality
    };

    for &recipe in &config.recipes {
        let voice_sets = recipe.generate_voice_sets(root_pc, voicing_quality);
        for voice_set in &voice_sets {
            if voice_set.len() < rules.min_strings as usize
                || voice_set.len() > rules.max_strings as usize
            {
                continue;
            }
            let mut fingerings = map_voice_set(voice_set, fretboard, rules);

            if config.min_fret > 0 {
                fingerings.retain(|f| respects_min_fret(f, config.min_fret));
            }
            if !config.allow_open_strings {
                fingerings.retain(|f| !uses_open_strings(f));
            }
            if let Some(strings) = &config.allowed_strings {
                fingerings.retain(|f| {
                    f.positions
                        .iter()
                        .enumerate()
                        .all(|(s, pos)| pos.is_none() || strings[s])
                });
            }

            rank_fingerings(&mut fingerings, voice_set, fretboard);
            let tension = voice_set_tension(quality.name, voice_set);
            all.extend(fingerings.into_iter().take(3).map(|fingering| {
                let rank_score = fingering_score(&fingering, voice_set, fretboard);
                SolvedAlternative {
                    fingering,
                    recipe: voice_set.recipe,
                    tension,
                    normalized_tension: tension,
                    rank_score,
                    relaxation,
                }
            }));
        }
    }

    dedup_candidates_preserving_order(all, config.max_candidates)
}

fn dedup_candidates_preserving_order(
    candidates: Vec<SolvedAlternative>,
    max_candidates: usize,
) -> Vec<SolvedAlternative> {
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(candidates.len().min(max_candidates));

    for candidate in candidates {
        if seen.insert(candidate.fingering.positions) {
            result.push(candidate);
            if result.len() == max_candidates {
                break;
            }
        }
    }

    result
}

fn generate_relaxed(
    root_pc: u8,
    quality: &'static ChordQuality,
    fretboard: &Fretboard,
    config: &SolverConfig,
) -> Vec<SolvedAlternative> {
    let mut relaxed = config.clone();

    // Step 1: allow fewer strings (stay in position)
    relaxed.rules.min_strings = 2;
    let cands = generate_candidates_with_relaxation(
        root_pc,
        quality,
        fretboard,
        &relaxed,
        RelaxationLevel::FewerNotes,
    );
    if !cands.is_empty() {
        return cands;
    }

    // Step 2: also drop string filter (stay in fret range)
    relaxed.allowed_strings = None;
    let cands = generate_candidates_with_relaxation(
        root_pc,
        quality,
        fretboard,
        &relaxed,
        RelaxationLevel::IgnoreStringFilter,
    );
    if !cands.is_empty() {
        return cands;
    }

    // Step 3: widen fret range by ±2 as last resort
    relaxed.min_fret = relaxed.min_fret.saturating_sub(2);
    relaxed.rules.max_fret = (relaxed.rules.max_fret + 2).min(15);
    generate_candidates_with_relaxation(
        root_pc,
        quality,
        fretboard,
        &relaxed,
        RelaxationLevel::WiderFretRange,
    )
}

fn normalize_candidate_tensions(all_candidates: &mut [Vec<SolvedAlternative>]) {
    for candidates in all_candidates {
        let min_tension = candidates
            .iter()
            .map(|c| c.tension)
            .fold(f32::INFINITY, f32::min);
        let max_tension = candidates
            .iter()
            .map(|c| c.tension)
            .fold(f32::NEG_INFINITY, f32::max);
        let range = max_tension - min_tension;
        for candidate in candidates {
            candidate.normalized_tension = if range > f32::EPSILON {
                (candidate.tension - min_tension) / range
            } else {
                0.0
            };
        }
    }
}

fn weighted_distance(distance: u32, weight: f32) -> u32 {
    (distance as f32 * weight.max(0.0)).round() as u32
}

fn respects_min_fret(fingering: &Fingering, min_fret: u8) -> bool {
    if min_fret == 0 {
        return true;
    }
    fingering
        .positions
        .iter()
        .filter_map(|f| *f)
        .filter(|f| *f > 0)
        .min()
        .is_some_and(|lowest_fretted| lowest_fretted >= min_fret)
}

fn uses_open_strings(fingering: &Fingering) -> bool {
    fingering.positions.contains(&Some(0))
}

fn jitter_noise(seed: u64, i: usize, j: usize, k: usize, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let mut h = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(i as u64);
    h = h.wrapping_mul(6364136223846793005).wrapping_add(j as u64);
    h = h.wrapping_mul(6364136223846793005).wrapping_add(k as u64);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h as u32) % (max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::chart::Chart;

    #[test]
    fn solve_simple_ii_v_i() {
        let chart = Chart::parse("ii-V-I", "| Dm7 | G7 | Cmaj7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig::default();

        let solved = solve(&chart, &fb, &config).unwrap();
        assert_eq!(solved.fingerings.len(), 3);
        assert_eq!(solved.fingerings[0].root, "D");
        assert_eq!(solved.fingerings[1].root, "G");
        assert_eq!(solved.fingerings[2].root, "C");
    }

    #[test]
    fn solve_minimizes_total_distance() {
        let chart = Chart::parse("ii-V-I", "| Dm7 | G7 | Cmaj7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig {
            jitter: 0,
            ..Default::default()
        };

        let solved = solve(&chart, &fb, &config).unwrap();

        let total_distance: u32 = solved
            .fingerings
            .windows(2)
            .map(|w| distance(&w[0].fingering, &w[1].fingering, &fb))
            .sum();

        let naive_candidates: Vec<Fingering> = chart
            .changes
            .iter()
            .map(|change| {
                let candidates = generate_candidates(change.root_pc, change.quality, &fb, &config);
                candidates.into_iter().next().unwrap().fingering
            })
            .collect();

        let naive_distance: u32 = naive_candidates
            .windows(2)
            .map(|w| distance(&w[0], &w[1], &fb))
            .sum();

        assert!(
            total_distance <= naive_distance,
            "solved ({}) should be <= naive ({})",
            total_distance,
            naive_distance
        );
    }

    #[test]
    fn solve_stella_first_8_bars() {
        let input = "\
            | Em7b5 | A7b9  | Cm7   | F7    |\
            | Fm7   | Bb7   | Ebmaj7| Ab7   |";
        let chart = Chart::parse("Stella", input).unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig::default();

        let solved = solve(&chart, &fb, &config).unwrap();
        assert_eq!(solved.fingerings.len(), 8);

        // Voice leading should keep hand relatively stable.
        let total_distance: u32 = solved
            .fingerings
            .windows(2)
            .map(|w| distance(&w[0].fingering, &w[1].fingering, &fb))
            .sum();

        // With 7 transitions, average distance per transition should be reasonable.
        let avg = total_distance as f64 / 7.0;
        assert!(
            avg < 20.0,
            "average voice leading distance should be reasonable, got {:.1}",
            avg
        );
    }

    #[test]
    fn solve_deterministic_without_jitter() {
        let chart = Chart::parse("Test", "| Dm7 | G7 | Cmaj7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig {
            jitter: 0,
            ..Default::default()
        };

        let a = solve(&chart, &fb, &config).unwrap();
        let b = solve(&chart, &fb, &config).unwrap();

        for (fa, fb) in a.fingerings.iter().zip(b.fingerings.iter()) {
            assert_eq!(fa.fingering.positions, fb.fingering.positions);
        }
    }

    #[test]
    fn solve_stella_full_32_bars() {
        let input = "Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11 | \
                     Bbmaj7 | Em7b5 A7b9 | Dm7 | Bbm7 Eb7 | Fmaj7 | Em7b5 | Ebmaj7 | D7b9 | \
                     G7b13 | % | Cm7 | % | Ab7#11 | % | Bbmaj7 | % | \
                     Em7b5 | A7b9 | Dm7b5 | G7b9 | Cm7b5 | F7b9 | Bbmaj7 | %";
        let chart = Chart::parse("Stella by Starlight", input).unwrap();
        assert_eq!(chart.changes.len(), 34);

        let fb = Fretboard::standard_tuning();
        let solved = solve(&chart, &fb, &SolverConfig::default()).unwrap();
        assert_eq!(solved.fingerings.len(), 34);

        let total_distance: u32 = solved
            .fingerings
            .windows(2)
            .map(|w| distance(&w[0].fingering, &w[1].fingering, &fb))
            .sum();
        let avg = total_distance as f64 / 33.0;
        assert!(
            avg < 20.0,
            "avg voice leading distance {:.1} should be < 20",
            avg,
        );
    }

    #[test]
    fn solve_respects_note_count_config() {
        let chart = Chart::parse("Test", "| Cmaj7 | G7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let mut config = SolverConfig::default();
        config.rules.min_strings = 3;
        config.rules.max_strings = 3;

        let solved = solve(&chart, &fb, &config).unwrap();
        for change in &solved.fingerings {
            assert_eq!(
                change.fingering.played_count(),
                3,
                "all voicings should use exactly 3 strings"
            );
        }
    }

    #[test]
    fn solve_with_locks_preserves_locked_alternative() {
        let chart = Chart::parse("Test", "| Dm7 | G7 | Cmaj7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig::default();

        let solved = solve(&chart, &fb, &config).unwrap();
        let locked_alt = solved.alternatives[1]
            .iter()
            .find(|alt| alt.fingering.positions != solved.fingerings[1].fingering.positions)
            .cloned()
            .unwrap_or_else(|| solved.alternatives[1][0].clone());
        let locks = vec![None, Some(locked_alt.clone()), None];

        let solved_with_lock = solve_with_locks(&chart, &fb, &config, &locks).unwrap();

        assert_eq!(
            solved_with_lock.fingerings[1].fingering.positions,
            locked_alt.fingering.positions
        );
        assert_eq!(solved_with_lock.alternatives[1].len(), 1);
    }

    #[test]
    fn disallow_open_strings_filters_candidates() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let config = SolverConfig {
            allow_open_strings: false,
            ..Default::default()
        };

        let cands = generate_candidates(0, quality, &fb, &config);

        assert!(!cands.is_empty());
        assert!(cands
            .iter()
            .all(|candidate| !uses_open_strings(&candidate.fingering)));
    }

    #[test]
    fn min_fret_filter_allows_open_strings_when_enabled() {
        let fingering = Fingering {
            positions: [Some(0), Some(5), Some(7), None, None, None],
            intervals: [None; 6],
        };

        assert!(respects_min_fret(&fingering, 5));
        assert!(uses_open_strings(&fingering));
    }

    #[test]
    fn expand_basic_chords_controls_added_extensions() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let expanded = generate_candidates(0, quality, &fb, &SolverConfig::default());
        let literal = generate_candidates(
            0,
            quality,
            &fb,
            &SolverConfig {
                expand_basic_chords: false,
                ..Default::default()
            },
        );

        assert!(expanded
            .iter()
            .any(|candidate| candidate.fingering.has_interval(Interval::M9)));
        assert!(literal.iter().all(|candidate| {
            !candidate.fingering.has_interval(Interval::M9)
                && !candidate.fingering.has_interval(Interval::M13)
        }));
    }

    #[test]
    fn solve_normalizes_candidate_tension_per_chord() {
        let chart = Chart::parse("Test", "| Em7b5 | A7b9 | Cm7 | F7 |").unwrap();
        let fb = Fretboard::standard_tuning();
        let solved = solve(&chart, &fb, &SolverConfig::default()).unwrap();
        let tensions: Vec<f32> = solved.alternatives[1]
            .iter()
            .map(|candidate| candidate.normalized_tension)
            .collect();

        assert!(tensions.iter().all(|t| (0.0..=1.0).contains(t)));
        assert!(tensions.iter().any(|t| *t >= 0.9));
    }

    #[test]
    fn candidate_dedup_preserves_rank_order_before_truncation() {
        let first_ranked = Fingering {
            positions: [None, Some(5), None, None, None, None],
            intervals: [None; 6],
        };
        let lexicographically_earlier = Fingering {
            positions: [None, Some(1), None, None, None, None],
            intervals: [None; 6],
        };

        let candidates = vec![
            test_candidate(first_ranked.clone(), VoicingRecipe::Shell, 10),
            test_candidate(lexicographically_earlier.clone(), VoicingRecipe::Drop2, 8),
            test_candidate(first_ranked.clone(), VoicingRecipe::RootlessA, 12),
        ];

        let deduped = dedup_candidates_preserving_order(candidates, 2);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].fingering.positions, first_ranked.positions);
        assert_eq!(
            deduped[1].fingering.positions,
            lexicographically_earlier.positions
        );
    }

    fn test_candidate(
        fingering: Fingering,
        recipe: VoicingRecipe,
        rank_score: i32,
    ) -> SolvedAlternative {
        SolvedAlternative {
            fingering,
            recipe,
            tension: 0.0,
            normalized_tension: 0.0,
            rank_score,
            relaxation: RelaxationLevel::Exact,
        }
    }

    #[test]
    fn tension_distribution_is_wide() {
        let fb = Fretboard::standard_tuning();
        let config = SolverConfig::default();

        for (name, root_pc) in [("maj7", 0u8), ("dom7", 7), ("m7", 2)] {
            let quality = ChordQuality::ALL.iter().find(|q| q.name == name).unwrap();
            let cands = generate_candidates(root_pc, quality, &fb, &config);
            let tensions: Vec<f32> = cands.iter().map(|candidate| candidate.tension).collect();
            let min_t = tensions.iter().cloned().fold(f32::MAX, f32::min);
            let max_t = tensions.iter().cloned().fold(0.0f32, f32::max);
            eprintln!(
                "{}: {} cands, tension {:.2}..{:.2}",
                name,
                cands.len(),
                min_t,
                max_t
            );

            let mut buckets = [0u32; 10];
            for &t in &tensions {
                let i = ((t * 10.0) as usize).min(9);
                buckets[i] += 1;
            }
            for (i, &count) in buckets.iter().enumerate() {
                if count > 0 {
                    eprintln!(
                        "  {:.1}-{:.1}: {} cands",
                        i as f32 / 10.0,
                        (i + 1) as f32 / 10.0,
                        count
                    );
                }
            }

            assert!(
                max_t - min_t >= 0.2,
                "{} tension range too narrow: {:.2}..{:.2}",
                name,
                min_t,
                max_t
            );
        }
    }

    #[test]
    fn bbmaj7_produces_rootless_with_ninth() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let config = SolverConfig::default();
        let cands = generate_candidates(10, quality, &fb, &config); // Bb = pc 10

        // x 5 7 5 6 x = [None, Some(5), Some(7), Some(5), Some(6), None]
        // = D(3rd), A(7th), C(9th), F(5th) — rootless with 9th
        let target = [None, Some(5), Some(7), Some(5), Some(6), None];
        assert!(
            cands
                .iter()
                .any(|candidate| candidate.fingering.positions == target),
            "Bbmaj7 should include rootless voicing x-5-7-5-6-x among {} candidates",
            cands.len()
        );
    }

    #[test]
    fn solve_stella_tight_constraints_relaxes_gracefully() {
        let input = "Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11 | \
                     Bbmaj7 | Em7b5 A7b9 | Dm7 | Bbm7 Eb7 | Fmaj7 | Em7b5 | Ebmaj7 | D7b9 | \
                     G7b13 | % | Cm7 | % | Ab7#11 | % | Bbmaj7 | % | \
                     Em7b5 | A7b9 | Dm7b5 | G7b9 | Cm7b5 | F7b9 | Bbmaj7 | %";
        let chart = Chart::parse("Stella", input).unwrap();
        let fb = Fretboard::standard_tuning();
        let mut config = SolverConfig::default();
        config.rules.min_strings = 4;
        config.rules.max_strings = 4;
        config.rules.max_fret = 9;
        config.min_fret = 5;
        config.allowed_strings = Some([false, true, true, true, true, false]);

        let solved = solve(&chart, &fb, &config);
        assert!(
            solved.is_some(),
            "solver should auto-relax and find a solution"
        );
        assert_eq!(solved.unwrap().fingerings.len(), 34);
    }

    #[test]
    fn dom7b9_extended_produces_varied_candidates() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL
            .iter()
            .find(|q| q.name == "dom7b9")
            .unwrap();
        let config = SolverConfig::default();
        let cands = generate_candidates(9, quality, &fb, &config); // A7b9

        let tensions: Vec<f32> = cands.iter().map(|candidate| candidate.tension).collect();
        let min_t = tensions.iter().cloned().fold(f32::MAX, f32::min);
        let max_t = tensions.iter().cloned().fold(0.0f32, f32::max);

        assert!(
            cands.len() >= 30,
            "A7b9 should have at least 30 candidates, got {}",
            cands.len()
        );
        assert!(
            max_t - min_t >= 0.3,
            "A7b9 tension range too narrow: {:.2}..{:.2}",
            min_t,
            max_t
        );

        let recipes: std::collections::HashSet<VoicingRecipe> =
            cands.iter().map(|candidate| candidate.recipe).collect();
        assert!(
            recipes.len() >= 4,
            "A7b9 should use at least 4 recipe types, got {}",
            recipes.len()
        );
    }

    #[test]
    fn tension_slider_changes_a7b9_voicing() {
        let input = "Em7b5 | A7b9 | Cm7 | F7";
        let chart = Chart::parse("Test", input).unwrap();
        let fb = Fretboard::standard_tuning();

        let grounded = SolverConfig {
            tension_target: 0.0,
            ..Default::default()
        };
        let abstract_ = SolverConfig {
            tension_target: 1.0,
            ..Default::default()
        };

        let solved_g = solve(&chart, &fb, &grounded).unwrap();
        let solved_a = solve(&chart, &fb, &abstract_).unwrap();

        let g_recipe = solved_g.fingerings[1].recipe;
        let a_recipe = solved_a.fingerings[1].recipe;
        let g_pos = &solved_g.fingerings[1].fingering.positions;
        let a_pos = &solved_a.fingerings[1].fingering.positions;

        assert!(
            g_recipe != a_recipe || g_pos != a_pos,
            "A7b9 should differ between grounded ({:?}) and abstract ({:?})",
            g_recipe,
            a_recipe
        );
    }
}
