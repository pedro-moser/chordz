use wasm_bindgen::prelude::*;

use crate::theory::chart::Chart;
use crate::theory::chords::{self, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::map_voice_set;
use crate::voicings::ranking::rank_fingerings;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::audio::synth;
use crate::theory::notes::Note;
use crate::voicings::solver::{self, SolverConfig};

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

                for fingering in fingerings.iter().take(3) {
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

#[wasm_bindgen]
pub fn solve_chart(chart_text: &str, title: &str) -> JsValue {
    let chart = match Chart::parse(title, chart_text) {
        Ok(c) => c,
        Err(_) => return to_js(&serde_json::json!({"error": "Parse error"})),
    };

    let fb = Fretboard::standard_tuning();
    let config = SolverConfig::default();

    match solver::solve(&chart, &fb, &config) {
        Some(solved) => {
            let changes: Vec<_> = solved.fingerings.iter().map(|c| {
                let positions: Vec<Option<u8>> = c.fingering.positions.to_vec();
                let notes: Vec<_> = c.fingering.notes(&fb).into_iter().map(|n| {
                    n.map(|note| serde_json::json!({"pc": note.pitch_class, "name": PC_NAMES[note.pitch_class as usize]}))
                }).collect();
                let intervals: Vec<_> = c.fingering.played_intervals().iter().map(|iv| iv.name).collect();
                let chord_label = chords::chord_name(&c.root, c.quality);
                serde_json::json!({
                    "chord": chord_label,
                    "recipe": c.recipe.short_label(),
                    "positions": positions,
                    "notes": notes,
                    "intervals": intervals,
                    "beats": c.beats,
                })
            }).collect();
            to_js(&serde_json::json!({"changes": changes}))
        }
        None => to_js(&serde_json::json!({"error": "No solution found"})),
    }
}

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
