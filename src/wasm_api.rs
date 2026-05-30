use wasm_bindgen::prelude::*;

use crate::audio::synth;
use crate::theory::chart::{Chart, PRESETS};
use crate::theory::chords::{self, ChordFamily, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::notes::Note;
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::procedural::generate_all_voice_sets;
use crate::voicings::ranking::rank_fingerings_with_options;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::voicings::solver::{
    self, RelaxationLevel, SolvedAlternative, SolvedChart, SolverConfig,
};

#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

fn to_js(value: &impl serde::Serialize) -> JsValue {
    let json = serde_json::to_string(value).unwrap();
    js_sys::JSON::parse(&json).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn get_roots() -> JsValue {
    let roots: Vec<&str> = chords::ROOTS.to_vec();
    to_js(&roots)
}

#[wasm_bindgen]
pub fn get_all_scales() -> JsValue {
    let scales: Vec<_> = Scale::ALL
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "parent": s.parent.name(),
                "degree": s.degree,
                "semitones": s.semitones,
            })
        })
        .collect();
    to_js(&scales)
}

#[wasm_bindgen]
pub fn get_parent_scale_names() -> JsValue {
    use crate::theory::scales::ParentScale;
    let names: Vec<&str> = ParentScale::ALL.iter().map(|p| p.name()).collect();
    to_js(&names)
}

#[wasm_bindgen]
pub fn get_pairs() -> JsValue {
    let pairs: Vec<_> = PAIRS
        .iter()
        .map(|p| {
            serde_json::json!({
                "label": p.label,
                "indicesA": p.indices.0,
                "indicesB": p.indices.1,
            })
        })
        .collect();
    to_js(&pairs)
}

#[wasm_bindgen]
pub fn resolve_pair(root_pc: u8, scale_index: usize, pair_index: usize) -> JsValue {
    let Some(scale) = Scale::ALL.get(scale_index) else {
        return to_js(&serde_json::json!({"error": "invalid scale_index"}));
    };
    let Some(pair) = PAIRS.get(pair_index) else {
        return to_js(&serde_json::json!({"error": "invalid pair_index"}));
    };
    let (a, b) = gmc::resolve_pair(root_pc, scale, pair);
    let result = serde_json::json!({
        "triadA": a,
        "triadB": b,
    });
    to_js(&result)
}

#[wasm_bindgen]
pub fn pair_display(root_pc: u8, scale_index: usize, pair_index: usize) -> String {
    let Some(scale) = Scale::ALL.get(scale_index) else {
        return String::new();
    };
    let Some(pair) = PAIRS.get(pair_index) else {
        return String::new();
    };
    gmc::pair_display(root_pc, scale, pair)
}

#[wasm_bindgen]
pub fn get_fretboard_notes() -> JsValue {
    let fb = Fretboard::standard_tuning();
    let mut notes: Vec<Vec<_>> = Vec::new();
    for s in 0..6 {
        let mut string_notes = Vec::new();
        for f in 0..=15 {
            if let Some(note) = fb.get_note(s, f) {
                string_notes.push(serde_json::json!({
                    "pc": note.pitch_class,
                    "name": PC_NAMES[note.pitch_class as usize],
                }));
            }
        }
        notes.push(string_notes);
    }
    to_js(&notes)
}

#[wasm_bindgen]
pub fn get_interval_name(semitone: u8) -> String {
    let scale = &Scale::ALL[0];
    scale.interval_name(semitone).to_string()
}

#[wasm_bindgen]
pub fn get_families() -> JsValue {
    let families: Vec<_> = ChordFamily::all()
        .iter()
        .enumerate()
        .map(|(i, f)| serde_json::json!({"index": i, "name": f.name()}))
        .collect();
    to_js(&families)
}

#[wasm_bindgen]
pub fn generate_voicings(
    root_index: usize,
    family_index: usize,
    note_count: usize,
    prefer_crunch: bool,
) -> JsValue {
    let Some(root_name) = chords::ROOTS.get(root_index) else {
        return to_js(&serde_json::json!({"error": "invalid root_index"}));
    };
    let root_pc = root_index as u8;
    let fb = Fretboard::standard_tuning();
    let rules = VoicingRules {
        min_strings: note_count as u8,
        max_strings: note_count as u8,
        max_fret_span: 5,
        max_fret: 15,
        require_root: false,
    };

    let Some(family) = ChordFamily::all().get(family_index) else {
        return to_js(&serde_json::json!({"error": "invalid family_index"}));
    };
    let mut groups: Vec<serde_json::Value> = Vec::new();
    let min_stability = 100u8;

    for quality_name in family.quality_names() {
        let Some(quality) = ChordQuality::ALL.iter().find(|q| q.name == *quality_name) else {
            continue;
        };
        let chord_label = chords::chord_name(root_name, quality);

        let voice_sets = generate_all_voice_sets(root_pc, quality, note_count, None, min_stability);

        for (voice_set, _stability, label) in &voice_sets {
            let mut fingerings = map_voice_set(voice_set, &fb, &rules);
            fingerings.retain(|f| {
                let mut pcs: Vec<u8> = f
                    .notes(&fb)
                    .into_iter()
                    .flatten()
                    .map(|n| n.pitch_class)
                    .collect();
                pcs.sort();
                pcs.dedup();
                pcs.len() == f.played_count()
            });
            rank_fingerings_with_options(&mut fingerings, voice_set, &fb, prefer_crunch, 0.0);

            for fingering in fingerings.iter().take(6) {
                let positions: Vec<Option<u8>> = fingering.positions.to_vec();
                let notes: Vec<_> = fingering.notes(&fb).into_iter().map(|n| {
                    n.map(|note| serde_json::json!({"pc": note.pitch_class, "name": PC_NAMES[note.pitch_class as usize]}))
                }).collect();
                let intervals: Vec<_> = fingering
                    .played_intervals()
                    .iter()
                    .map(|iv| iv.name)
                    .collect();

                groups.push(serde_json::json!({
                    "chord": chord_label,
                    "recipe": label,
                    "positions": positions,
                    "notes": notes,
                    "intervals": intervals,
                }));
            }
        }
    }

    to_js(&groups)
}

// ---------------------------------------------------------------------------
// Tune mode: presets
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn get_presets() -> JsValue {
    let presets: Vec<_> = PRESETS
        .iter()
        .map(|(title, chart)| serde_json::json!({"title": title, "chart": chart}))
        .collect();
    to_js(&presets)
}

// ---------------------------------------------------------------------------
// Tune mode: solver config parsing
// ---------------------------------------------------------------------------

const RECIPE_NAMES: &[(&str, VoicingRecipe)] = &[
    ("shell", VoicingRecipe::Shell),
    ("closed", VoicingRecipe::Closed),
    ("drop2", VoicingRecipe::Drop2),
    ("drop3", VoicingRecipe::Drop3),
    ("rless-a", VoicingRecipe::RootlessA),
    ("rless-b", VoicingRecipe::RootlessB),
    ("quartal", VoicingRecipe::Quartal),
    ("upper", VoicingRecipe::UpperStructureTriad),
    ("triads", VoicingRecipe::TriadPair),
];

fn parse_solver_config(config_js: JsValue) -> SolverConfig {
    let obj: serde_json::Value = match serde_wasm_bindgen::from_value(config_js) {
        Ok(v) => v,
        Err(_) => return SolverConfig::default(),
    };

    let min_strings = obj.get("minStrings").and_then(|v| v.as_u64()).unwrap_or(3) as u8;
    let max_strings = obj.get("maxStrings").and_then(|v| v.as_u64()).unwrap_or(4) as u8;
    let max_fret_span = obj.get("maxFretSpan").and_then(|v| v.as_u64()).unwrap_or(5) as u8;
    let max_fret = obj.get("maxFret").and_then(|v| v.as_u64()).unwrap_or(15) as u8;
    let min_fret = obj.get("minFret").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let tension_target = obj
        .get("tensionTarget")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3) as f32;
    let smoothness_weight = obj
        .get("smoothnessWeight")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as f32;
    let jitter = obj.get("jitter").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let allow_open_strings = obj
        .get("allowOpenStrings")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let recipes = if let Some(arr) = obj.get("recipes").and_then(|v| v.as_array()) {
        let selected: Vec<VoicingRecipe> = arr
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|name| {
                RECIPE_NAMES
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, r)| *r)
            })
            .collect();
        if selected.is_empty() {
            VoicingRecipe::all().to_vec()
        } else {
            selected
        }
    } else {
        VoicingRecipe::all().to_vec()
    };

    let allowed_strings = if let Some(arr) = obj.get("allowedStrings").and_then(|v| v.as_array()) {
        if arr.len() == 6 {
            let mut strings = [true; 6];
            for (i, v) in arr.iter().enumerate() {
                strings[i] = v.as_bool().unwrap_or(true);
            }
            Some(strings)
        } else {
            None
        }
    } else {
        None
    };

    SolverConfig {
        rules: VoicingRules {
            min_strings,
            max_strings,
            max_fret_span,
            max_fret,
            require_root: false,
        },
        recipes,
        max_candidates: 256,
        min_fret,
        allowed_strings,
        allow_open_strings,
        tension_target,
        abstraction: obj.get("abstraction").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        tension_weight: 6.0,
        rank_weight: 1,
        smoothness_weight,
        jitter,
    }
}

// ---------------------------------------------------------------------------
// Tune mode: serialization helpers
// ---------------------------------------------------------------------------

fn serialize_fingering(f: &Fingering, recipe: VoicingRecipe, fb: &Fretboard) -> serde_json::Value {
    let positions: Vec<Option<u8>> = f.positions.to_vec();
    let notes: Vec<_> = f.notes(fb).into_iter().map(|n| {
        n.map(|note| serde_json::json!({"pc": note.pitch_class, "name": PC_NAMES[note.pitch_class as usize]}))
    }).collect();
    let intervals: Vec<_> = f.played_intervals().iter().map(|iv| iv.name).collect();
    serde_json::json!({
        "positions": positions,
        "notes": notes,
        "intervals": intervals,
        "recipe": recipe.short_label(),
    })
}

fn serialize_solved(solved: &SolvedChart, fb: &Fretboard) -> JsValue {
    let changes: Vec<_> = solved.fingerings.iter().enumerate().map(|(i, c)| {
        let alts: Vec<serde_json::Value> = solved.alternatives.get(i).map(|a| {
            a.iter().take(10).map(|alt| {
                let mut obj = serialize_fingering(&alt.fingering, alt.recipe, fb);
                obj.as_object_mut().unwrap().insert(
                    "tension".to_string(),
                    serde_json::json!(alt.normalized_tension),
                );
                obj.as_object_mut().unwrap().insert(
                    "relaxation".to_string(),
                    serde_json::json!(alt.relaxation.label()),
                );
                obj
            }).collect()
        }).unwrap_or_default();

        let chord_label = chords::chord_name(&c.root, c.quality);
        let root_pc = chords::root_to_pc(&c.root).unwrap_or(0);
        let bass_pc = c.bass_pc.unwrap_or(root_pc);
        let chord_display = if c.bass_pc.is_some() {
            format!("{}/{}", chord_label, PC_NAMES[bass_pc as usize])
        } else {
            chord_label.clone()
        };
        serde_json::json!({
            "chord": chord_display,
            "rootPc": root_pc,
            "bassPc": bass_pc,
            "recipe": c.recipe.short_label(),
            "positions": c.fingering.positions.to_vec() as Vec<Option<u8>>,
            "notes": c.fingering.notes(fb).into_iter().map(|n| {
                n.map(|note| serde_json::json!({"pc": note.pitch_class, "name": PC_NAMES[note.pitch_class as usize]}))
            }).collect::<Vec<_>>(),
            "intervals": c.fingering.played_intervals().iter().map(|iv| iv.name).collect::<Vec<_>>(),
            "beats": c.beats,
            "alternatives": alts,
            "relaxation": c.relaxation.label(),
            "tension": c.normalized_tension,
        })
    }).collect();
    to_js(&serde_json::json!({"changes": changes}))
}

// ---------------------------------------------------------------------------
// Tune mode: solve_chart (with config + alternatives)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn solve_chart(chart_text: &str, title: &str, config_js: JsValue) -> JsValue {
    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let fb = Fretboard::standard_tuning();
    let config = parse_solver_config(config_js);

    match solver::solve(&chart, &fb, &config) {
        Some(solved) => serialize_solved(&solved, &fb),
        None => to_js(&serde_json::json!({"error": "No solution found"})),
    }
}

// ---------------------------------------------------------------------------
// Tune mode: solve_chart_with_locks
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn solve_chart_with_locks(
    chart_text: &str,
    title: &str,
    config_js: JsValue,
    locks_js: JsValue,
) -> JsValue {
    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let fb = Fretboard::standard_tuning();
    let config = parse_solver_config(config_js);

    // Parse locks: array of {positions: [...], recipe: "..."} | null
    let locks_raw: Vec<Option<serde_json::Value>> =
        serde_wasm_bindgen::from_value(locks_js).unwrap_or_default();

    let locks: Vec<Option<SolvedAlternative>> = locks_raw
        .into_iter()
        .map(|lock_opt| {
            let obj = lock_opt?;
            let positions_arr = obj.get("positions")?.as_array()?;
            if positions_arr.len() != 6 {
                return None;
            }
            let mut positions = [None; 6];
            for (i, v) in positions_arr.iter().enumerate() {
                positions[i] = if v.is_null() {
                    None
                } else {
                    Some(v.as_u64()? as u8)
                };
            }

            let recipe_name = obj.get("recipe")?.as_str()?;
            let recipe = RECIPE_NAMES
                .iter()
                .find(|(n, _)| *n == recipe_name)
                .map(|(_, r)| *r)?;

            // Reconstruct a minimal Fingering (intervals will be blank but
            // the solver only compares positions for locked alternatives).
            let fingering = Fingering {
                positions,
                intervals: [None; 6],
            };

            Some(SolvedAlternative {
                fingering,
                recipe,
                tension: 0.0,
                normalized_tension: 0.0,
                rank_score: 0,
                relaxation: RelaxationLevel::Exact,
            })
        })
        .collect();

    match solver::solve_with_locks(&chart, &fb, &config, &locks) {
        Some(solved) => serialize_solved(&solved, &fb),
        None => to_js(&serde_json::json!({"error": "No solution found"})),
    }
}

// ---------------------------------------------------------------------------
// Audio
// ---------------------------------------------------------------------------

/// Duplicate a mono buffer into interleaved L/R stereo for the Web Audio bridge.
fn interleave_mono(mono: &[f32]) -> Vec<f32> {
    let mut interleaved = Vec::with_capacity(mono.len() * 2);
    for &s in mono {
        interleaved.push(s);
        interleaved.push(s);
    }
    interleaved
}

#[wasm_bindgen]
pub fn synth_single_note(midi: i32, duration: f32) -> Vec<f32> {
    let note = Note::from_midi(midi);
    interleave_mono(&synth::generate_pluck(note, duration))
}

#[wasm_bindgen]
pub fn synth_bass_note(root_pc: u8, duration: f32) -> Vec<f32> {
    let note = Note::new(root_pc as i32, 2); // octave 2 = bass register
    interleave_mono(&synth::generate_pluck(note, duration))
}

#[wasm_bindgen]
pub fn synth_chord(positions_js: JsValue, duration: f32) -> Vec<f32> {
    let positions: Vec<Option<u8>> =
        serde_wasm_bindgen::from_value(positions_js).unwrap_or_default();
    let fb = Fretboard::standard_tuning();
    let notes: Vec<Note> = positions
        .iter()
        .enumerate()
        .filter_map(|(s, fret)| fret.and_then(|f| fb.get_note(s, f as usize)))
        .collect();
    if notes.is_empty() {
        return Vec::new();
    }
    let stereo = synth::generate_chord(&notes, duration);
    let mut interleaved = Vec::with_capacity(stereo.left.len() * 2);
    for i in 0..stereo.left.len() {
        interleaved.push(stereo.left[i]);
        interleaved.push(stereo.right[i]);
    }
    interleaved
}

#[wasm_bindgen]
pub fn synth_arpeggio(positions_js: JsValue, note_duration: f32) -> Vec<f32> {
    let positions: Vec<Option<u8>> =
        serde_wasm_bindgen::from_value(positions_js).unwrap_or_default();
    let fb = Fretboard::standard_tuning();
    let notes: Vec<Note> = positions
        .iter()
        .enumerate()
        .filter_map(|(s, fret)| fret.and_then(|f| fb.get_note(s, f as usize)))
        .collect();
    if notes.is_empty() {
        return Vec::new();
    }
    let total_duration = note_duration * notes.len() as f32 + 0.5;
    let sample_rate = 44100u32;
    let total_samples = (sample_rate as f32 * total_duration) as usize;
    let mut left = vec![0.0f32; total_samples];
    let mut right = vec![0.0f32; total_samples];
    let gain = 1.0 / notes.len() as f32;
    for (i, &note) in notes.iter().enumerate() {
        let offset = (sample_rate as f32 * note_duration * i as f32) as usize;
        let pluck = synth::generate_pluck(note, note_duration + 0.3);
        let pan = i as f32 / (notes.len() - 1).max(1) as f32;
        let lg = gain * (1.0 - pan * 0.3);
        let rg = gain * (0.7 + pan * 0.3);
        for (j, &s) in pluck.iter().enumerate() {
            let idx = offset + j;
            if idx < total_samples {
                left[idx] += s * lg;
                right[idx] += s * rg;
            }
        }
    }
    let mut interleaved = Vec::with_capacity(total_samples * 2);
    for i in 0..total_samples {
        interleaved.push(left[i]);
        interleaved.push(right[i]);
    }
    interleaved
}

// ---------------------------------------------------------------------------
// GMC Tune mode: line generation
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn generate_gmc_line(
    chart_text: &str,
    title: &str,
    pair_index: usize,
    scale_overrides_js: JsValue,
    figure_index: usize,     // 0=Eighth, 1=Sixteenth, 2=Triplet
    position_fret: u8,       // base fret 1-12
    pattern_js: JsValue,     // array of {count, direction, triad}
) -> JsValue {
    use crate::theory::line_engine::{self, LineConfig};
    use crate::theory::line_pattern::{Anchor, Direction, Pattern, PatternBlock, RhythmicFigure, Shape, TriadId};
    use crate::theory::position::NeckPosition;
    use crate::theory::scale_defaults;

    // Parse chart
    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    // Parse scale overrides: array of (number | null)
    let overrides_raw: Vec<Option<usize>> = serde_wasm_bindgen::from_value(scale_overrides_js)
        .unwrap_or_default();

    // Parse pattern blocks
    let blocks_raw: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(pattern_js)
        .unwrap_or_default();
    let blocks: Vec<PatternBlock> = blocks_raw.iter().map(|b| {
        // Clamp before the u8 cast: the JS UI bounds count to 1..6, but the core is the
        // trust boundary — an unclamped u64 would wrap on cast (260 -> 4) and count=0
        // yields a degenerate zero-length block.
        let count = b["count"].as_u64().unwrap_or(3).clamp(1, 6) as u8;
        let direction = if b["direction"].as_str() == Some("desc") { Direction::Descending } else { Direction::Ascending };
        let triad = if b["triad"].as_str() == Some("T2") { TriadId::T2 } else { TriadId::T1 };
        // Optional shape/anchor (absent in legacy payloads -> the original walk). `shape`
        // is an array of voice roles [0,2,1] = 1-5-3; `anchor` is "root"|"third"|"fifth".
        let shape = match b["shape"].as_array() {
            Some(arr) if !arr.is_empty() => {
                let order: Vec<u8> = arr.iter().filter_map(|v| v.as_u64()).map(|r| (r % 3) as u8).collect();
                if order.is_empty() { Shape::Monotonic } else { Shape::Order(order) }
            }
            _ => Shape::Monotonic,
        };
        let anchor = match b["anchor"].as_str() {
            Some("root") => Anchor::Root,
            Some("third") => Anchor::Third,
            Some("fifth") => Anchor::Fifth,
            _ => Anchor::Nearest,
        };
        PatternBlock { count, direction, triad, shape, anchor }
    }).collect();

    if blocks.is_empty() {
        return to_js(&serde_json::json!({"error": "empty pattern"}));
    }

    let pattern = Pattern { name: "custom", blocks };
    let figure = match figure_index {
        1 => RhythmicFigure::Sixteenth,
        2 => RhythmicFigure::Triplet,
        _ => RhythmicFigure::Eighth,
    };

    let pair = match PAIRS.get(pair_index) {
        Some(p) => p,
        None => return to_js(&serde_json::json!({"error": "invalid pair_index"})),
    };

    let config = LineConfig {
        pattern,
        figure,
        position: NeckPosition::new(position_fret),
    };

    let fretboard = Fretboard::standard_tuning();
    let events = line_engine::generate_line(&chart, &overrides_raw, &fretboard, pair, &config);

    // Build response with events and chord info
    let changes_info: Vec<_> = chart.changes.iter().enumerate().map(|(i, c)| {
        let default_scale = scale_defaults::default_scale(c.quality);
        // Use checked indexing: the override index comes from JS unvalidated, so an
        // out-of-range value must degrade to the default scale rather than trap wasm.
        // is_override reflects whether the override actually resolved (matches the line
        // engine's own fallback), so a bad index doesn't falsely flag the chord as edited.
        let override_scale = overrides_raw
            .get(i)
            .and_then(|o| o.and_then(|idx| Scale::ALL.get(idx)));
        let actual_scale = override_scale.unwrap_or(default_scale);
        let is_override = override_scale.is_some();
        serde_json::json!({
            "chord": chords::chord_name(&c.root, c.quality),
            "rootPc": c.root_pc,
            "bassPc": c.bass_pc,
            "beats": c.beats,
            "defaultScale": default_scale.name,
            "defaultScaleIndex": Scale::ALL.iter().position(|s| s.name == default_scale.name),
            "activeScale": actual_scale.name,
            "isOverride": is_override,
        })
    }).collect();

    let events_json: Vec<_> = events.iter().map(|e| {
        serde_json::json!({
            "beat": e.beat,
            "string": e.string,
            "fret": e.fret,
            "triad": if e.triad == TriadId::T1 { "T1" } else { "T2" },
            "pitchClass": e.pitch_class,
            "midi": e.midi,
        })
    }).collect();

    to_js(&serde_json::json!({
        "events": events_json,
        "changes": changes_info,
        "totalBeats": chart.total_beats(),
    }))
}

/// Generate a guide-tone "shell étude" line (Motor E) over a chart. Same controls as the
/// GMC line minus the triad pair and scale overrides — the two 3-note shells per chord are
/// derived from the chord quality (see `theory::shells`). Response shape matches
/// `generate_gmc_line` so the web renderer is shared.
#[wasm_bindgen]
pub fn generate_shell_etude(
    chart_text: &str,
    title: &str,
    figure_index: usize, // 0=Eighth, 1=Sixteenth, 2=Triplet
    position_fret: u8,   // base fret 1-12
    pattern_js: JsValue, // array of {count, direction, triad, shape?, anchor?}
) -> JsValue {
    use crate::theory::line_engine::{self, LineConfig};
    use crate::theory::line_pattern::{Anchor, Direction, Pattern, PatternBlock, RhythmicFigure, Shape, TriadId};
    use crate::theory::position::NeckPosition;
    use crate::theory::scale_defaults;

    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(e) => return to_js(&serde_json::json!({"error": format!("{}", e)})),
    };

    let blocks_raw: Vec<serde_json::Value> =
        serde_wasm_bindgen::from_value(pattern_js).unwrap_or_default();
    let blocks: Vec<PatternBlock> = blocks_raw
        .iter()
        .map(|b| {
            let count = b["count"].as_u64().unwrap_or(3).clamp(1, 6) as u8;
            let direction = if b["direction"].as_str() == Some("desc") {
                Direction::Descending
            } else {
                Direction::Ascending
            };
            let triad = if b["triad"].as_str() == Some("T2") {
                TriadId::T2
            } else {
                TriadId::T1
            };
            let shape = match b["shape"].as_array() {
                Some(arr) if !arr.is_empty() => {
                    let order: Vec<u8> =
                        arr.iter().filter_map(|v| v.as_u64()).map(|r| (r % 3) as u8).collect();
                    if order.is_empty() { Shape::Monotonic } else { Shape::Order(order) }
                }
                _ => Shape::Monotonic,
            };
            let anchor = match b["anchor"].as_str() {
                Some("root") => Anchor::Root,
                Some("third") => Anchor::Third,
                Some("fifth") => Anchor::Fifth,
                _ => Anchor::Nearest,
            };
            PatternBlock { count, direction, triad, shape, anchor }
        })
        .collect();

    if blocks.is_empty() {
        return to_js(&serde_json::json!({"error": "empty pattern"}));
    }

    let pattern = Pattern { name: "shell", blocks };
    let figure = match figure_index {
        1 => RhythmicFigure::Sixteenth,
        2 => RhythmicFigure::Triplet,
        _ => RhythmicFigure::Eighth,
    };

    let config = LineConfig { pattern, figure, position: NeckPosition::new(position_fret) };
    let fretboard = Fretboard::standard_tuning();
    let events = line_engine::generate_shell_line(&chart, &fretboard, &config);

    let changes_info: Vec<_> = chart
        .changes
        .iter()
        .map(|c| {
            let default_scale = scale_defaults::default_scale(c.quality);
            serde_json::json!({
                "chord": chords::chord_name(&c.root, c.quality),
                "rootPc": c.root_pc,
                "bassPc": c.bass_pc,
                "beats": c.beats,
                "defaultScale": default_scale.name,
                "defaultScaleIndex": Scale::ALL.iter().position(|s| s.name == default_scale.name),
                "activeScale": default_scale.name,
                "isOverride": false,
            })
        })
        .collect();

    let events_json: Vec<_> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "beat": e.beat,
                "string": e.string,
                "fret": e.fret,
                "triad": if e.triad == TriadId::T1 { "T1" } else { "T2" },
                "pitchClass": e.pitch_class,
                "midi": e.midi,
            })
        })
        .collect();

    to_js(&serde_json::json!({
        "events": events_json,
        "changes": changes_info,
        "totalBeats": chart.total_beats(),
    }))
}

/// Generate a quarter-note walking bass line for a chord sequence. Each input segment is
/// `{ rootPc, bassPc (nullable), symbol, beats }`; the symbol is parsed only for its
/// authoritative quality intervals (3rd/5th/7th), while rootPc/bassPc come from the caller.
#[wasm_bindgen]
pub fn generate_walking_bass(segments_js: JsValue) -> JsValue {
    use crate::theory::chart::quality_from_symbol;
    use crate::theory::walking_bass::{self, BassSegment};

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SegIn {
        root_pc: u8,
        #[serde(default)]
        bass_pc: Option<u8>,
        symbol: String,
        beats: f32,
    }

    let raw: Vec<SegIn> = serde_wasm_bindgen::from_value(segments_js).unwrap_or_default();
    // Fall back to a plain dominant 7 if a symbol fails to parse, so a stray chord never
    // drops the whole line.
    let default_quality = ChordQuality::ALL
        .iter()
        .find(|q| q.name == "dom7")
        .expect("dom7 quality exists");

    let segments: Vec<BassSegment> = raw
        .iter()
        .map(|s| BassSegment {
            root_pc: s.root_pc,
            bass_pc: s.bass_pc,
            quality: quality_from_symbol(&s.symbol).unwrap_or(default_quality),
            beats: s.beats,
        })
        .collect();

    let line = walking_bass::walking_bass_line(&segments);
    let notes_json: Vec<_> = line
        .iter()
        .map(|n| {
            serde_json::json!({
                "midi": n.midi,
                "beat": n.beat,
                "beats": n.beats,
                "chord": n.chord,
            })
        })
        .collect();
    to_js(&notes_json)
}

#[wasm_bindgen]
pub fn get_default_scale_index(quality_name: &str) -> JsValue {
    use crate::theory::scale_defaults;
    let quality = match ChordQuality::ALL.iter().find(|q| q.name == quality_name) {
        Some(q) => q,
        None => return JsValue::NULL,
    };
    let scale = scale_defaults::default_scale(quality);
    let idx = Scale::ALL.iter().position(|s| s.name == scale.name);
    to_js(&idx)
}
