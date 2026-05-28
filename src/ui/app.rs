use std::time::Instant;

use eframe::egui;

#[cfg(feature = "native")]
use crate::audio::engine::AudioEngine;

#[cfg(feature = "native")]
type Audio = AudioEngine;
#[cfg(not(feature = "native"))]
type Audio = ();
use crate::theory::chart::PRESETS as TUNE_PRESETS;
use crate::theory::chords::{self, ChordFamily, ChordQuality};
use crate::theory::intervals::Interval;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::Fingering;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::voicings::solver::{SolvedChart, SolverConfig};

pub(crate) const NOTE_COUNTS: [usize; 5] = [2, 3, 4, 5, 6];
pub(crate) const VOICINGS_PER_VOICE_SET: usize = 4;
pub(crate) const MAX_VOICINGS: usize = 256;

#[derive(Clone, Debug)]
pub struct VoicingEntry {
    pub recipe: VoicingRecipe,
    pub tension: &'static str,
    pub fingering: Fingering,
}

#[derive(Clone, Debug)]
pub struct VoicingGroup {
    pub quality: &'static ChordQuality,
    pub recipe: VoicingRecipe,
    pub intervals: Vec<Interval>,
    pub entries: Vec<VoicingEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Browser,
    Tune,
    Gmc,
}

pub(crate) struct GmcState {
    pub(crate) root_index: usize,
    pub(crate) scale_index: usize,
    pub(crate) pair_index: usize,
    pub(crate) show_intervals: bool,
}

impl Default for GmcState {
    fn default() -> Self {
        Self {
            root_index: 0,
            scale_index: 1, // Dorian
            pair_index: 0,
            show_intervals: false,
        }
    }
}

pub(crate) const TUNE_BPM: f32 = 120.0;
pub(crate) const TUNE_RECIPES: [VoicingRecipe; 9] = [
    VoicingRecipe::Shell,
    VoicingRecipe::Closed,
    VoicingRecipe::Drop2,
    VoicingRecipe::Drop3,
    VoicingRecipe::RootlessA,
    VoicingRecipe::RootlessB,
    VoicingRecipe::Quartal,
    VoicingRecipe::UpperStructureTriad,
    VoicingRecipe::TriadPair,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TuneNoteFilter {
    ThreeOrFour,
    Three,
    Four,
    Five,
    ThreeToFive,
}

impl TuneNoteFilter {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ThreeOrFour => "3-4",
            Self::Three => "3",
            Self::Four => "4",
            Self::Five => "5",
            Self::ThreeToFive => "3-5",
        }
    }

    pub(crate) const fn range(self) -> (u8, u8) {
        match self {
            Self::ThreeOrFour => (3, 4),
            Self::Three => (3, 3),
            Self::Four => (4, 4),
            Self::Five => (5, 5),
            Self::ThreeToFive => (3, 5),
        }
    }
}

pub(crate) struct TuneState {
    pub(crate) chart_input: String,
    pub(crate) title_input: String,
    pub(crate) solved: Option<SolvedChart>,
    pub(crate) selected_chord: usize,
    pub(crate) error: Option<String>,
    pub(crate) constraints: TuneConstraints,
    pub(crate) playback_start: Option<Instant>,
    pub(crate) locked: Vec<bool>,
}

pub(crate) struct TuneConstraints {
    pub(crate) tension: f32,
    pub(crate) smoothness: f32,
    pub(crate) variation: u32,
    pub(crate) note_filter: TuneNoteFilter,
    pub(crate) fret_min: u8,
    pub(crate) fret_max: u8,
    pub(crate) max_span: u8,
    pub(crate) allow_open_strings: bool,
    pub(crate) string_filter_on: bool,
    pub(crate) strings: [bool; 6],
    pub(crate) recipe_filter_on: bool,
    pub(crate) recipes: [bool; 9],
}

impl Default for TuneConstraints {
    fn default() -> Self {
        Self {
            tension: 0.3,
            smoothness: 1.0,
            variation: 0,
            note_filter: TuneNoteFilter::ThreeOrFour,
            fret_min: 0,
            fret_max: 15,
            max_span: 5,
            allow_open_strings: true,
            string_filter_on: false,
            strings: [true; 6],
            recipe_filter_on: false,
            recipes: [true; 9],
        }
    }
}

impl TuneConstraints {
    pub(crate) fn to_solver_config(&self) -> SolverConfig {
        let (min_s, max_s) = self.note_filter.range();

        let max_fret = self.fret_max;
        let min_fret = self.fret_min;

        let allowed_strings = if self.string_filter_on {
            Some(self.strings)
        } else {
            None
        };
        let recipes = if self.recipe_filter_on {
            let selected: Vec<VoicingRecipe> = TUNE_RECIPES
                .iter()
                .enumerate()
                .filter_map(|(i, recipe)| self.recipes[i].then_some(*recipe))
                .collect();
            if selected.is_empty() {
                VoicingRecipe::all().to_vec()
            } else {
                selected
            }
        } else {
            VoicingRecipe::all().to_vec()
        };

        SolverConfig {
            rules: VoicingRules {
                min_strings: min_s,
                max_strings: max_s,
                max_fret_span: self.max_span,
                max_fret,
                require_root: false,
            },
            recipes,
            max_candidates: 256,
            min_fret,
            allowed_strings,
            allow_open_strings: self.allow_open_strings,
            tension_target: self.tension,
            tension_weight: 6.0,
            rank_weight: 1,
            smoothness_weight: self.smoothness,
            jitter: self.variation,
        }
    }
}

impl Default for TuneState {
    fn default() -> Self {
        let (title, changes) = TUNE_PRESETS[0];
        Self {
            chart_input: changes.to_string(),
            title_input: title.to_string(),
            solved: None,
            selected_chord: 0,
            error: None,
            constraints: TuneConstraints::default(),
            playback_start: None,
            locked: Vec::new(),
        }
    }
}

pub struct ChordzApp {
    pub(crate) mode: AppMode,
    pub(crate) root_index: usize,
    pub(crate) family_index: usize,
    pub(crate) note_count_index: usize,
    pub(crate) selected_group: usize,
    pub(crate) selected_position: usize,
    pub(crate) groups: Vec<VoicingGroup>,
    pub(crate) fretboard: Fretboard,
    pub(crate) audio: Option<Audio>,
    pub(crate) tune: TuneState,
    pub(crate) gmc: GmcState,
}

impl ChordzApp {
    pub fn new() -> Self {
        #[cfg(feature = "native")]
        let audio = AudioEngine::new().ok();
        #[cfg(not(feature = "native"))]
        let audio: Option<Audio> = None;
        let mut app = Self {
            mode: AppMode::Browser,
            root_index: 0,
            family_index: 0,
            note_count_index: 2,
            selected_group: 0,
            selected_position: 0,
            groups: Vec::new(),
            fretboard: Fretboard::standard_tuning(),
            audio,
            tune: TuneState::default(),
            gmc: GmcState::default(),
        };
        app.refresh_voicings();
        app
    }
}

impl Default for ChordzApp {
    fn default() -> Self {
        Self::new()
    }
}

// --- Shared accessors used by browser, tune, and gmc modules ---

impl ChordzApp {
    pub(crate) fn root(&self) -> &'static str {
        chords::ROOTS[self.root_index]
    }

    pub(crate) fn family(&self) -> ChordFamily {
        ChordFamily::all()[self.family_index]
    }

    pub(crate) fn note_count(&self) -> usize {
        NOTE_COUNTS[self.note_count_index]
    }

    pub(crate) fn selected_quality(&self) -> &'static ChordQuality {
        self.groups
            .get(self.selected_group)
            .map(|g| g.quality)
            .unwrap_or_else(|| find_quality(self.family().quality_names()[0]))
    }

    pub(crate) fn selected_entry(&self) -> Option<(&VoicingGroup, &VoicingEntry)> {
        self.groups.get(self.selected_group).and_then(|g| {
            let pos = self
                .selected_position
                .min(g.entries.len().saturating_sub(1));
            g.entries.get(pos).map(|e| (g, e))
        })
    }

    pub(crate) fn chord_name_for(&self, quality: &ChordQuality) -> String {
        chords::chord_name(self.root(), quality)
    }

    pub(crate) fn current_chord_name(&self) -> String {
        self.chord_name_for(self.selected_quality())
    }
}

// --- eframe::App dispatch ---

impl eframe::App for ChordzApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                if self.tune.playback_start.is_some() {
                    self.tune.playback_start = None;
                    #[cfg(feature = "native")]
                    if let Some(audio) = &mut self.audio {
                        audio.stop_all();
                    }
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });

        egui::TopBottomPanel::top("mode_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.mode, AppMode::Browser, "Browser");
                ui.selectable_value(&mut self.mode, AppMode::Tune, "Tune");
                ui.selectable_value(&mut self.mode, AppMode::Gmc, "GMC");
                ui.separator();
                match self.mode {
                    AppMode::Browser => self.show_selectors(ui),
                    AppMode::Tune => self.show_tune_controls(ui),
                    AppMode::Gmc => self.show_gmc_controls(ui),
                }
            });
        });

        match self.mode {
            AppMode::Browser => self.update_browser(ctx),
            AppMode::Tune => self.update_tune(ctx),
            AppMode::Gmc => self.update_gmc(ctx),
        }
    }
}

// --- Pure logic helpers (no UI dependency) ---

pub(crate) fn find_quality(name: &str) -> &'static ChordQuality {
    ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
}

pub(crate) fn has_unique_pitch_classes(fingering: &Fingering, fretboard: &Fretboard) -> bool {
    let mut seen = [false; 12];
    for note in fingering.notes(fretboard).into_iter().flatten() {
        let index = note.pitch_class as usize;
        if seen[index] {
            return false;
        }
        seen[index] = true;
    }
    true
}

pub(crate) fn recipe_order(recipe: VoicingRecipe) -> usize {
    VoicingRecipe::all()
        .iter()
        .position(|r| *r == recipe)
        .unwrap_or(usize::MAX)
}

pub(crate) fn quality_order(family: ChordFamily, quality: &ChordQuality) -> usize {
    family
        .quality_names()
        .iter()
        .position(|name| *name == quality.name)
        .unwrap_or(usize::MAX)
}

pub(crate) fn tension_score(_quality: &ChordQuality, recipe: VoicingRecipe) -> u32 {
    match recipe {
        VoicingRecipe::Shell => 0,
        VoicingRecipe::Closed | VoicingRecipe::Drop2 | VoicingRecipe::Drop3 => 1,
        VoicingRecipe::RootlessA | VoicingRecipe::RootlessB => 2,
        VoicingRecipe::Quartal => 3,
        VoicingRecipe::UpperStructureTriad | VoicingRecipe::TriadPair => 4,
    }
}

pub(crate) fn tension_label(quality: &ChordQuality, recipe: VoicingRecipe) -> &'static str {
    match tension_score(quality, recipe) {
        0 | 1 => "inside",
        2 | 3 => "color",
        _ => "out",
    }
}
