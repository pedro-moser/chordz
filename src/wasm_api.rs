use wasm_bindgen::prelude::*;

use crate::theory::chart::Chart;
use crate::theory::chords::{self, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::ranking::rank_fingerings;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::audio::synth;
use crate::theory::notes::Note;
use crate::voicings::solver::{self, SolvedAlternative, SolvedChart, SolverConfig, RelaxationLevel};

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
    let scale = &Scale::ALL[scale_index];
    let pair = &PAIRS[pair_index];
    let (a, b) = gmc::resolve_pair(root_pc, scale, pair);
    let result = serde_json::json!({
        "triadA": a,
        "triadB": b,
    });
    to_js(&result)
}

#[wasm_bindgen]
pub fn pair_display(root_pc: u8, scale_index: usize, pair_index: usize) -> String {
    let scale = &Scale::ALL[scale_index];
    let pair = &PAIRS[pair_index];
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

fn family_quality_names(family_index: usize) -> &'static [&'static str] {
    match family_index {
        0 => &["maj7", "maj9", "maj13", "maj7#11"],
        1 => &["dom7", "dom9", "dom13", "dom7b9", "dom7#9", "dom7#5", "dom7#11", "dom7b13"],
        2 => &["m7", "m9", "m11", "m13"],
        3 => &["m7b5", "m9b11"],
        4 => &["dim7"],
        _ => &[],
    }
}

#[wasm_bindgen]
pub fn get_families() -> JsValue {
    let families: Vec<_> = ["Major", "Dominant", "Minor", "Half-dim", "Diminished"]
        .iter()
        .enumerate()
        .map(|(i, name)| serde_json::json!({"index": i, "name": name}))
        .collect();
    to_js(&families)
}

#[wasm_bindgen]
pub fn generate_voicings(root_index: usize, family_index: usize, note_count: usize) -> JsValue {
    let root_pc = root_index as u8;
    let fb = Fretboard::standard_tuning();
    let rules = VoicingRules {
        min_strings: note_count as u8,
        max_strings: note_count as u8,
        max_fret_span: 5,
        max_fret: 15,
        require_root: false,
    };

    let mut groups: Vec<serde_json::Value> = Vec::new();

    for quality_name in family_quality_names(family_index) {
        let Some(quality) = ChordQuality::ALL.iter().find(|q| q.name == *quality_name) else {
            continue;
        };
        let chord_label = chords::chord_name(chords::ROOTS[root_index], quality);

        for recipe in VoicingRecipe::all() {
            let voice_sets = recipe.generate_voice_sets(root_pc, quality);
            for voice_set in voice_sets.iter().filter(|vs| vs.len() == note_count) {
                let mut fingerings = map_voice_set(voice_set, &fb, &rules);
                fingerings.retain(|f| {
                    let mut pcs: Vec<u8> = f.notes(&fb).into_iter().flatten().map(|n| n.pitch_class).collect();
                    pcs.sort();
                    pcs.dedup();
                    pcs.len() == f.played_count()
                });
                rank_fingerings(&mut fingerings, voice_set, &fb);

                for fingering in fingerings.iter().take(6) {
                    let positions: Vec<Option<u8>> = fingering.positions.to_vec();
                    let notes: Vec<_> = fingering.notes(&fb).into_iter().map(|n| {
                        n.map(|note| serde_json::json!({"pc": note.pitch_class, "name": PC_NAMES[note.pitch_class as usize]}))
                    }).collect();
                    let intervals: Vec<_> = fingering.played_intervals().iter().map(|iv| iv.name).collect();

                    groups.push(serde_json::json!({
                        "chord": chord_label,
                        "recipe": recipe.short_label(),
                        "positions": positions,
                        "notes": notes,
                        "intervals": intervals,
                    }));
                }
            }
        }
    }

    to_js(&groups)
}

// ---------------------------------------------------------------------------
// Tune mode: presets
// ---------------------------------------------------------------------------

const TUNE_PRESETS: &[(&str, &str)] = &[
    (
        "Stella by Starlight",
        "Em7b5 | A7b9 | Cm7 | F7 | Fm7 | Bb7 | Ebmaj7 | Ab7#11 | \
         Bbmaj7 | Em7b5 A7b9 | Dm7 | Bbm7 Eb7 | Fmaj7 | Em7b5 | Ebmaj7 | D7b9 | \
         G7b13 | % | Cm7 | % | Ab7#11 | % | Bbmaj7 | % | \
         Em7b5 | A7b9 | Dm7b5 | G7b9 | Cm7b5 | F7b9 | Bbmaj7 | %",
    ),
    (
        "Just Friends",
        "Cmaj7 | % | Cm7 | F7 | Gmaj7 | % | Bbm7 | Eb7 | \
         Am7 | D7 | Gmaj7 | Em7 | A7 | % | Am7 | D7 G7 | \
         Cmaj7 | % | Cm7 | F7 | Gmaj7 | % | Bbm7 | Eb7 | \
         Am7 | D7 | F#m7b5 B7b9 | Em7 | A7 | Am7 D7 | Gmaj7 | Dm7 G7",
    ),
    (
        "Moment's Notice",
        "Em7 A7 | Fm7 Bb7 | Ebmaj7 | Abm7 Db7 | \
         Dm7 G7 | Ebm7 Ab7 | Dbmaj7 | Dm7 G7 | \
         Cm7 B7 | Bbm7 Eb7 | Abmaj7 | Abm7 Db7 | \
         Gm7 C7 | Abm7 Db7 | Gbmaj7 | Fm7 Bb7 | \
         Gm7 C7 | Fm7 Bb7 | Ebmaj7 | Fm7 | \
         Gm7 | Fm7 | Ebmaj7 Fm7 | Gm7 Fm7 | \
         Ebmaj7 | %",
    ),
    (
        "Giant Steps",
        "Bmaj7 D7 | Gmaj7 Bb7 | Ebmaj7 | Am7 D7 | \
         Gmaj7 Bb7 | Ebmaj7 F#7 | Bmaj7 | Fm7 Bb7 | \
         Ebmaj7 | Am7 D7 | Gmaj7 | C#m7 F#7 | \
         Bmaj7 | Fm7 Bb7 | Ebmaj7 | C#m7 F#7",
    ),
];

#[wasm_bindgen]
pub fn get_presets() -> JsValue {
    let presets: Vec<_> = TUNE_PRESETS
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
    let tension_target = obj.get("tensionTarget").and_then(|v| v.as_f64()).unwrap_or(0.3) as f32;
    let smoothness_weight = obj.get("smoothnessWeight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let jitter = obj.get("jitter").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let allow_open_strings = obj.get("allowOpenStrings").and_then(|v| v.as_bool()).unwrap_or(true);
    let expand_basic_chords = obj.get("expandBasicChords").and_then(|v| v.as_bool()).unwrap_or(true);

    let recipes = if let Some(arr) = obj.get("recipes").and_then(|v| v.as_array()) {
        let selected: Vec<VoicingRecipe> = arr
            .iter()
            .filter_map(|v| v.as_str())
            .filter_map(|name| {
                RECIPE_NAMES.iter().find(|(n, _)| *n == name).map(|(_, r)| *r)
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
        expand_basic_chords,
        tension_target,
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
        serde_json::json!({
            "chord": chord_label,
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

#[wasm_bindgen]
pub fn synth_chord(positions_js: JsValue, duration: f32) -> Vec<f32> {
    let positions: Vec<Option<u8>> = serde_wasm_bindgen::from_value(positions_js).unwrap_or_default();
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
    let positions: Vec<Option<u8>> = serde_wasm_bindgen::from_value(positions_js).unwrap_or_default();
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
