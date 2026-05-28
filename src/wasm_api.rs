use wasm_bindgen::prelude::*;

use crate::theory::chords::{self, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;

#[wasm_bindgen]
pub fn get_roots() -> JsValue {
    let roots: Vec<&str> = chords::ROOTS.to_vec();
    serde_wasm_bindgen::to_value(&roots).unwrap()
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
    serde_wasm_bindgen::to_value(&scales).unwrap()
}

#[wasm_bindgen]
pub fn get_parent_scale_names() -> JsValue {
    use crate::theory::scales::ParentScale;
    let names: Vec<&str> = ParentScale::ALL.iter().map(|p| p.name()).collect();
    serde_wasm_bindgen::to_value(&names).unwrap()
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
    serde_wasm_bindgen::to_value(&pairs).unwrap()
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
    serde_wasm_bindgen::to_value(&result).unwrap()
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
    serde_wasm_bindgen::to_value(&notes).unwrap()
}

#[wasm_bindgen]
pub fn get_interval_name(semitone: u8) -> String {
    let scale = &Scale::ALL[0];
    scale.interval_name(semitone).to_string()
}
