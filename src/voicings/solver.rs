use std::collections::HashSet;
use std::time::SystemTime;

use super::fretboard::Fretboard;
use super::generate::{map_voice_set, Fingering};
use super::procedural::generate_all_voice_sets_with_abstraction;
use super::ranking::{rank_fingerings_with_options, score as fingering_score};
use super::recipe::VoicingRecipe;
use super::rules::VoicingRules;
use super::voice_leading::distance;
use crate::theory::chart::Chart;
use crate::theory::chords::ChordQuality;

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
    pub bass_pc: Option<u8>,
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
    /// 0.0 = only stable intervals (★3-4), 1.0 = tolerant of dissonance (★1-2).
    pub tension_target: f32,
    /// 0.0 = spell out the chord (R,3,5,7), 1.0 = imply it (4,9,13,omit root).
    pub abstraction: f32,
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
            tension_target: 0.3,
            abstraction: 0.0,
            tension_weight: 6.0,
            rank_weight: 1,
            smoothness_weight: 1.0,
            jitter: 0,
        }
    }
}

const REPEAT_PENALTY: u32 = 50;

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

        let next_quality = chart.changes.get(i + 1).map(|c| c.quality);

        let candidates =
            generate_candidates(change.root_pc, change.quality, next_quality, fretboard, config);
        if !candidates.is_empty() {
            all_candidates.push(candidates);
            continue;
        }
        // Progressively relax: widen fret range, then drop string/note filters.
        let relaxed =
            generate_relaxed(change.root_pc, change.quality, next_quality, fretboard, config);
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
                bass_pc: change.bass_pc,
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

    let fingerings = diversify_repeated_chords(fingerings, &alternatives, fretboard);

    Some(SolvedChart {
        fingerings,
        alternatives,
    })
}

fn diversify_repeated_chords(
    mut fingerings: Vec<SolvedChange>,
    alternatives: &[Vec<SolvedAlternative>],
    fretboard: &Fretboard,
) -> Vec<SolvedChange> {
    let n = fingerings.len();
    let mut used_positions: std::collections::HashMap<String, Vec<[Option<u8>; 6]>> =
        std::collections::HashMap::new();

    for i in 0..n {
        let key = format!("{}_{}", fingerings[i].root, fingerings[i].quality.name);
        let positions = fingerings[i].fingering.positions;

        if let Some(prev) = used_positions.get(&key) {
            if prev.contains(&positions) {
                if let Some(alts) = alternatives.get(i) {
                    let prev_pos = if i > 0 {
                        Some(&fingerings[i - 1].fingering)
                    } else {
                        None
                    };
                    let mut best: Option<&SolvedAlternative> = None;
                    let mut best_dist = u32::MAX;
                    for alt in alts {
                        if prev.contains(&alt.fingering.positions) {
                            continue;
                        }
                        let d = prev_pos
                            .map(|p| distance(p, &alt.fingering, fretboard))
                            .unwrap_or(0);
                        if d < best_dist {
                            best_dist = d;
                            best = Some(alt);
                        }
                    }
                    if let Some(alt) = best {
                        fingerings[i].fingering = alt.fingering.clone();
                        fingerings[i].recipe = alt.recipe;
                        fingerings[i].tension = alt.tension;
                        fingerings[i].normalized_tension = alt.normalized_tension;
                        fingerings[i].rank_score = alt.rank_score;
                        fingerings[i].relaxation = alt.relaxation;
                    }
                }
            }
        }

        used_positions
            .entry(key)
            .or_default()
            .push(fingerings[i].fingering.positions);
    }

    fingerings
}

fn generate_candidates(
    root_pc: u8,
    quality: &'static ChordQuality,
    next_quality: Option<&'static ChordQuality>,
    fretboard: &Fretboard,
    config: &SolverConfig,
) -> Vec<SolvedAlternative> {
    generate_candidates_with_relaxation(
        root_pc,
        quality,
        next_quality,
        fretboard,
        config,
        RelaxationLevel::Exact,
    )
}

fn generate_candidates_with_relaxation(
    root_pc: u8,
    quality: &'static ChordQuality,
    next_quality: Option<&'static ChordQuality>,
    fretboard: &Fretboard,
    config: &SolverConfig,
    relaxation: RelaxationLevel,
) -> Vec<SolvedAlternative> {
    let rules = &config.rules;
    let mut all: Vec<SolvedAlternative> = Vec::new();

    // Map tension_target to min_total_stability.
    let min_stability: u8 = if config.tension_target < 0.15 {
        140
    } else if config.tension_target < 0.45 {
        120
    } else if config.tension_target < 0.75 {
        100
    } else {
        80
    };

    // Generate voice sets for each valid note count.
    let mut all_voice_sets = Vec::new();
    for nc in rules.min_strings..=rules.max_strings {
        all_voice_sets.extend(generate_all_voice_sets_with_abstraction(
            root_pc,
            quality,
            nc as usize,
            next_quality,
            min_stability,
            config.abstraction,
        ));
    }

    for (voice_set, stability, label) in &all_voice_sets {
        // Recipe/label filter: match procedural labels against recipe short labels.
        if !config.recipes.is_empty()
            && !config.recipes.iter().any(|r| r.short_label() == *label)
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

        rank_fingerings_with_options(&mut fingerings, voice_set, fretboard, false, config.abstraction);
        // Use stability as tension: higher stability = lower tension.
        let max_possible = rules.max_strings as f32 * 40.0;
        let tension = 1.0 - (*stability as f32 / max_possible).min(1.0);
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
    next_quality: Option<&'static ChordQuality>,
    fretboard: &Fretboard,
    config: &SolverConfig,
) -> Vec<SolvedAlternative> {
    let mut relaxed = config.clone();

    // Step 1: allow fewer strings (stay in position)
    relaxed.rules.min_strings = 2;
    let cands = generate_candidates_with_relaxation(
        root_pc,
        quality,
        next_quality,
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
        next_quality,
        fretboard,
        &relaxed,
        RelaxationLevel::IgnoreStringFilter,
    );
    if !cands.is_empty() {
        return cands;
    }

    // Step 3: widen fret range by +/-2 as last resort
    relaxed.min_fret = relaxed.min_fret.saturating_sub(2);
    relaxed.rules.max_fret = (relaxed.rules.max_fret + 2).min(15);
    generate_candidates_with_relaxation(
        root_pc,
        quality,
        next_quality,
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
    use crate::theory::intervals::Interval;

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
            .enumerate()
            .map(|(i, change)| {
                let next_q = chart.changes.get(i + 1).map(|c| c.quality);
                let candidates =
                    generate_candidates(change.root_pc, change.quality, next_q, &fb, &config);
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

        let cands = generate_candidates(0, quality, None, &fb, &config);

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
    fn procedural_generates_extensions_for_basic_qualities() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let cands = generate_candidates(0, quality, None, &fb, &SolverConfig::default());

        // The procedural generator includes 9ths via the stability table.
        // Semitone 2 (the 9th) appears as Interval::M2 in pitch-class space.
        assert!(
            cands
                .iter()
                .any(|candidate| candidate.fingering.has_interval(Interval::M2)),
            "maj7 should include voicings with 9th (M2 in pitch-class space)"
        );
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
            let cands = generate_candidates(root_pc, quality, None, &fb, &config);
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
                max_t - min_t >= 0.1,
                "{} tension range too narrow: {:.2}..{:.2}",
                name,
                min_t,
                max_t
            );
        }
    }

    #[test]
    fn bbmaj7_produces_voicing_with_ninth() {
        let fb = Fretboard::standard_tuning();
        let quality = ChordQuality::ALL.iter().find(|q| q.name == "maj7").unwrap();
        let config = SolverConfig::default();
        let cands = generate_candidates(10, quality, None, &fb, &config); // Bb = pc 10

        // The procedural generator should produce some voicing containing
        // the 9th (C = pc 0, which is semitone 2 from Bb).
        // In pitch-class space this shows up as Interval::M2.
        assert!(
            cands
                .iter()
                .any(|candidate| candidate.fingering.has_interval(Interval::M2)),
            "Bbmaj7 should include a voicing with the 9th among {} candidates",
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
        let cands = generate_candidates(9, quality, None, &fb, &config); // A7b9

        let tensions: Vec<f32> = cands.iter().map(|candidate| candidate.tension).collect();
        let min_t = tensions.iter().cloned().fold(f32::MAX, f32::min);
        let max_t = tensions.iter().cloned().fold(0.0f32, f32::max);

        assert!(
            cands.len() >= 30,
            "A7b9 should have at least 30 candidates, got {}",
            cands.len()
        );
        assert!(
            max_t - min_t >= 0.1,
            "A7b9 tension range too narrow: {:.2}..{:.2}",
            min_t,
            max_t
        );

        let recipes: std::collections::HashSet<VoicingRecipe> =
            cands.iter().map(|candidate| candidate.recipe).collect();
        assert!(
            recipes.len() >= 2,
            "A7b9 should use at least 2 recipe types, got {}",
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
