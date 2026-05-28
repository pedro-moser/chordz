use std::collections::HashMap;
use std::time::Instant;

use eframe::egui;

use super::fretboard::{compact_interval_name, paint_fretboard, paint_panoramic_fretboard};
use crate::audio::engine::AudioEngine;
use crate::theory::chart::Chart;
use crate::theory::chords::{self, ChordQuality};
use crate::theory::gmc::{self, PAIRS};
use crate::theory::scales::Scale;
use crate::theory::intervals::Interval;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::ranking::rank_fingerings;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;
use crate::voicings::solver::{self, SolvedAlternative, SolvedChart, SolverConfig};
use crate::voicings::voice_leading;

const NOTE_COUNTS: [usize; 5] = [2, 3, 4, 5, 6];
const VOICINGS_PER_VOICE_SET: usize = 4;
const MAX_VOICINGS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChordFamily {
    Major,
    Dominant,
    Minor,
    HalfDiminished,
    Diminished,
}

impl ChordFamily {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Major,
            Self::Dominant,
            Self::Minor,
            Self::HalfDiminished,
            Self::Diminished,
        ]
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Major => "Major",
            Self::Dominant => "Dominant",
            Self::Minor => "Minor",
            Self::HalfDiminished => "Half-dim",
            Self::Diminished => "Diminished",
        }
    }

    pub const fn quality_names(self) -> &'static [&'static str] {
        match self {
            Self::Major => &["maj7", "maj9", "maj13", "maj7#11"],
            Self::Dominant => &[
                "dom7", "dom9", "dom13", "dom7b9", "dom7#9", "dom7#5", "dom7#11", "dom7b13",
            ],
            Self::Minor => &["m7", "m9", "m11", "m13"],
            Self::HalfDiminished => &["m7b5", "m9b11"],
            Self::Diminished => &["dim7"],
        }
    }
}

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

struct GmcState {
    root_index: usize,
    scale_index: usize,
    pair_index: usize,
    show_intervals: bool,
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

const TUNE_BPM: f32 = 120.0;
const TUNE_RECIPES: [VoicingRecipe; 9] = [
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
enum TuneNoteFilter {
    ThreeOrFour,
    Three,
    Four,
    Five,
    ThreeToFive,
}

impl TuneNoteFilter {
    const fn label(self) -> &'static str {
        match self {
            Self::ThreeOrFour => "3-4",
            Self::Three => "3",
            Self::Four => "4",
            Self::Five => "5",
            Self::ThreeToFive => "3-5",
        }
    }

    const fn range(self) -> (u8, u8) {
        match self {
            Self::ThreeOrFour => (3, 4),
            Self::Three => (3, 3),
            Self::Four => (4, 4),
            Self::Five => (5, 5),
            Self::ThreeToFive => (3, 5),
        }
    }
}

pub struct TuneState {
    chart_input: String,
    title_input: String,
    solved: Option<SolvedChart>,
    selected_chord: usize,
    error: Option<String>,
    constraints: TuneConstraints,
    playback_start: Option<Instant>,
    locked: Vec<bool>,
}

pub struct TuneConstraints {
    pub tension: f32,
    pub smoothness: f32,
    pub variation: u32,
    note_filter: TuneNoteFilter,
    pub fret_min: u8,
    pub fret_max: u8,
    pub max_span: u8,
    pub allow_open_strings: bool,
    pub expand_basic_chords: bool,
    pub string_filter_on: bool,
    pub strings: [bool; 6],
    pub recipe_filter_on: bool,
    pub recipes: [bool; 9],
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
            expand_basic_chords: true,
            string_filter_on: false,
            strings: [true; 6],
            recipe_filter_on: false,
            recipes: [true; 9],
        }
    }
}

impl TuneConstraints {
    fn to_solver_config(&self) -> SolverConfig {
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
            expand_basic_chords: self.expand_basic_chords,
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
];

pub struct ChordzApp {
    mode: AppMode,
    root_index: usize,
    family_index: usize,
    note_count_index: usize,
    selected_group: usize,
    selected_position: usize,
    groups: Vec<VoicingGroup>,
    fretboard: Fretboard,
    audio: Option<AudioEngine>,
    tune: TuneState,
    gmc: GmcState,
}

impl ChordzApp {
    pub fn new() -> Self {
        let audio = AudioEngine::new().ok();
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

impl ChordzApp {
    fn root(&self) -> &'static str {
        chords::ROOTS[self.root_index]
    }

    fn family(&self) -> ChordFamily {
        ChordFamily::all()[self.family_index]
    }

    fn note_count(&self) -> usize {
        NOTE_COUNTS[self.note_count_index]
    }

    fn selected_quality(&self) -> &'static ChordQuality {
        self.groups
            .get(self.selected_group)
            .map(|g| g.quality)
            .unwrap_or_else(|| find_quality(self.family().quality_names()[0]))
    }

    fn selected_entry(&self) -> Option<(&VoicingGroup, &VoicingEntry)> {
        self.groups.get(self.selected_group).and_then(|g| {
            let pos = self
                .selected_position
                .min(g.entries.len().saturating_sub(1));
            g.entries.get(pos).map(|e| (g, e))
        })
    }

    fn chord_name_for(&self, quality: &ChordQuality) -> String {
        chords::chord_name(self.root(), quality)
    }

    fn current_chord_name(&self) -> String {
        self.chord_name_for(self.selected_quality())
    }

    fn refresh_voicings(&mut self) {
        self.groups = self.generate_groups();
        self.selected_group = self.selected_group.min(self.groups.len().saturating_sub(1));
        self.clamp_position();
    }

    fn clamp_position(&mut self) {
        if let Some(group) = self.groups.get(self.selected_group) {
            self.selected_position = self
                .selected_position
                .min(group.entries.len().saturating_sub(1));
        } else {
            self.selected_position = 0;
        }
    }

    fn generate_groups(&self) -> Vec<VoicingGroup> {
        let note_count = self.note_count();
        let rules = VoicingRules {
            min_strings: note_count as u8,
            max_strings: note_count as u8,
            max_fret_span: 5,
            max_fret: 15,
            require_root: false,
        };
        let root_pc = chords::root_to_pc(self.root()).unwrap();
        let recipes = [
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

        struct FlatEntry {
            quality: &'static ChordQuality,
            recipe: VoicingRecipe,
            tension: &'static str,
            fingering: Fingering,
        }

        let mut flat: Vec<FlatEntry> = Vec::new();

        for quality_name in self.family().quality_names() {
            let quality = find_quality(quality_name);
            for recipe in recipes {
                let voice_sets = recipe.generate_voice_sets(root_pc, quality);
                for voice_set in voice_sets.iter().filter(|vs| vs.len() == note_count) {
                    let mut fingerings = map_voice_set(voice_set, &self.fretboard, &rules);
                    fingerings.retain(|f| has_unique_pitch_classes(f, &self.fretboard));
                    rank_fingerings(&mut fingerings, voice_set, &self.fretboard);
                    flat.extend(fingerings.into_iter().take(VOICINGS_PER_VOICE_SET).map(
                        |fingering| FlatEntry {
                            quality,
                            recipe: voice_set.recipe,
                            tension: tension_label(quality, voice_set.recipe),
                            fingering,
                        },
                    ));
                }
            }
        }

        flat.sort_by(|a, b| {
            tension_score(a.quality, a.recipe)
                .cmp(&tension_score(b.quality, b.recipe))
                .then_with(|| {
                    quality_order(self.family(), a.quality)
                        .cmp(&quality_order(self.family(), b.quality))
                })
                .then_with(|| recipe_order(a.recipe).cmp(&recipe_order(b.recipe)))
                .then_with(|| a.fingering.positions.cmp(&b.fingering.positions))
        });
        flat.dedup_by(|a, b| a.fingering.positions == b.fingering.positions);
        flat.truncate(MAX_VOICINGS);

        let fretboard = &self.fretboard;
        let mut groups: Vec<VoicingGroup> = Vec::new();
        let mut group_index: HashMap<(&str, VoicingRecipe, u16), usize> = HashMap::new();

        for entry in flat {
            let pc_bits: u16 = entry
                .fingering
                .notes(fretboard)
                .into_iter()
                .flatten()
                .fold(0u16, |bits, n| bits | (1 << n.pitch_class));

            let key = (entry.quality.name, entry.recipe, pc_bits);
            if let Some(&idx) = group_index.get(&key) {
                groups[idx].entries.push(VoicingEntry {
                    recipe: entry.recipe,
                    tension: entry.tension,
                    fingering: entry.fingering,
                });
            } else {
                let idx = groups.len();
                let intervals = entry.fingering.played_intervals();
                groups.push(VoicingGroup {
                    quality: entry.quality,
                    recipe: entry.recipe,
                    intervals,
                    entries: vec![VoicingEntry {
                        recipe: entry.recipe,
                        tension: entry.tension,
                        fingering: entry.fingering,
                    }],
                });
                group_index.insert(key, idx);
            }
        }

        groups
    }
}

impl eframe::App for ChordzApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                if self.tune.playback_start.is_some() {
                    self.tune.playback_start = None;
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

// --- Browser mode ---

impl ChordzApp {
    fn update_browser(&mut self, ctx: &egui::Context) {
        let mut play_strum = false;

        ctx.input(|i| {
            if (i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown))
                && self.selected_group + 1 < self.groups.len()
            {
                self.selected_group += 1;
                self.selected_position = 0;
            }
            if (i.key_pressed(egui::Key::K) || i.key_pressed(egui::Key::ArrowUp))
                && self.selected_group > 0
            {
                self.selected_group -= 1;
                self.selected_position = 0;
            }
            if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                if let Some(group) = self.groups.get(self.selected_group) {
                    if self.selected_position + 1 < group.entries.len() {
                        self.selected_position += 1;
                    }
                }
            }
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::ArrowLeft) {
                self.selected_position = self.selected_position.saturating_sub(1);
            }
            if i.key_pressed(egui::Key::Space) {
                play_strum = true;
            }
        });

        if play_strum {
            self.play_current_strum();
        }

        egui::TopBottomPanel::bottom("browser_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let group_count = self.groups.len();
                let group_pos = (self.selected_group + 1).min(group_count);
                let pos_count = self
                    .groups
                    .get(self.selected_group)
                    .map(|g| g.entries.len())
                    .unwrap_or(0);
                let pos_pos = (self.selected_position + 1).min(pos_count);
                ui.label(format!(
                    "{} {} | {} | {}n | group {}/{} | pos {}/{}",
                    self.root(),
                    self.family().name(),
                    self.current_chord_name(),
                    self.note_count(),
                    group_pos,
                    group_count,
                    pos_pos,
                    pos_count,
                ));
            });
        });

        egui::SidePanel::left("voicings")
            .default_width(320.0)
            .show(ctx, |ui| {
                self.show_voicing_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_diagram(ui);
        });
    }
}

// --- Tune mode ---

impl ChordzApp {
    fn show_tune_controls(&mut self, ui: &mut egui::Ui) {
        if ui.button("Solve").clicked() {
            self.solve_chart();
        }
        if self.tune.solved.is_some() {
            ui.separator();
            let locked_count = self.tune.locked.iter().filter(|locked| **locked).count();
            if locked_count > 0 {
                ui.label(format!("{} locked", locked_count));
                if ui.button("Clear locks").clicked() {
                    self.tune.locked.fill(false);
                }
                ui.separator();
            }
            if self.audio.is_some() {
                if self.tune.playback_start.is_some() {
                    if ui.button("Stop (Esc)").clicked() {
                        self.tune.playback_start = None;
                        if let Some(audio) = &mut self.audio {
                            audio.stop_all();
                        }
                    }
                } else if ui.button("Play All (P)").clicked() {
                    self.play_tune_all();
                }
            }
            ui.separator();
            let solved = self.tune.solved.as_ref().unwrap();
            let count = solved.fingerings.len();
            let pos = self.tune.selected_chord + 1;
            ui.label(format!("{}/{}", pos, count));
            let relaxed_count = solved
                .fingerings
                .iter()
                .filter(|change| change.relaxation.is_relaxed())
                .count();
            if relaxed_count > 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 180, 80),
                    format!("{} relaxed", relaxed_count),
                );
            }
        }
        if let Some(err) = &self.tune.error {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
        }
    }

    fn solve_chart(&mut self) {
        self.tune.error = None;
        self.tune.playback_start = None;

        let chart = match Chart::parse(&self.tune.title_input, &self.tune.chart_input) {
            Ok(c) => c,
            Err(e) => {
                self.tune.error = Some(format!("Parse: {:?}", e));
                return;
            }
        };

        let locked = self.locked_alternatives_for(&chart);
        let has_locks = locked.iter().any(Option::is_some);
        let config = self.tune.constraints.to_solver_config();
        let solved = if has_locks {
            solver::solve_with_locks(&chart, &self.fretboard, &config, &locked)
        } else {
            solver::solve(&chart, &self.fretboard, &config)
        };

        match solved {
            Some(solved) => {
                let len = solved.fingerings.len();
                if has_locks {
                    self.tune.locked.resize(len, false);
                    self.tune.selected_chord = self.tune.selected_chord.min(len.saturating_sub(1));
                } else {
                    self.tune.locked = vec![false; len];
                    self.tune.selected_chord = 0;
                }
                self.tune.solved = Some(solved);
            }
            None => self.tune.error = Some("No solution — try relaxing constraints".to_string()),
        }
    }

    fn locked_alternatives_for(&self, chart: &Chart) -> Vec<Option<SolvedAlternative>> {
        let mut locked = vec![None; chart.changes.len()];
        let Some(solved) = &self.tune.solved else {
            return locked;
        };
        if solved.fingerings.len() != chart.changes.len() {
            return locked;
        }

        for (i, change) in chart.changes.iter().enumerate() {
            let Some(true) = self.tune.locked.get(i).copied() else {
                continue;
            };
            let solved_change = &solved.fingerings[i];
            if solved_change.root == change.root
                && solved_change.quality.name == change.quality.name
            {
                locked[i] = Some(SolvedAlternative {
                    fingering: solved_change.fingering.clone(),
                    recipe: solved_change.recipe,
                    tension: solved_change.tension,
                    normalized_tension: solved_change.normalized_tension,
                    rank_score: solved_change.rank_score,
                    relaxation: solved_change.relaxation,
                });
            }
        }

        locked
    }

    fn update_tune(&mut self, ctx: &egui::Context) {
        let mut play_strum = false;
        let mut play_all = false;

        let mut swap_next = false;
        let mut swap_prev = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown) {
                self.tune.playback_start = None;
                if let Some(solved) = &self.tune.solved {
                    if self.tune.selected_chord + 1 < solved.fingerings.len() {
                        self.tune.selected_chord += 1;
                    }
                }
            }
            if i.key_pressed(egui::Key::K) || i.key_pressed(egui::Key::ArrowUp) {
                self.tune.playback_start = None;
                self.tune.selected_chord = self.tune.selected_chord.saturating_sub(1);
            }
            if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                swap_next = true;
            }
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::ArrowLeft) {
                swap_prev = true;
            }
            if i.key_pressed(egui::Key::Space) {
                play_strum = true;
            }
            if i.key_pressed(egui::Key::P) {
                play_all = true;
            }
        });

        if swap_next {
            self.swap_tune_voicing(1);
        } else if swap_prev {
            self.swap_tune_voicing(-1);
        }

        if play_strum {
            self.play_tune_strum();
        }
        if play_all {
            self.play_tune_all();
        }

        if let (Some(start), Some(solved)) = (self.tune.playback_start, self.tune.solved.as_ref()) {
            let elapsed = start.elapsed().as_secs_f32();
            let beat_dur = 60.0 / TUNE_BPM;
            let mut cumulative = 0.0_f32;
            let mut target = 0;
            for (i, change) in solved.fingerings.iter().enumerate() {
                let end = (cumulative + change.beats) * beat_dur;
                if elapsed < end {
                    target = i;
                    break;
                }
                cumulative += change.beats;
                target = i;
            }
            let total_time = solved.fingerings.iter().map(|c| c.beats).sum::<f32>() * beat_dur;
            if elapsed >= total_time {
                self.tune.playback_start = None;
            } else {
                self.tune.selected_chord = target;
                ctx.request_repaint();
            }
        }

        egui::TopBottomPanel::bottom("tune_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(solved) = &self.tune.solved {
                    let idx = self.tune.selected_chord;
                    if let Some(change) = solved.fingerings.get(idx) {
                        let chord_name = chords::chord_name(&change.root, change.quality);
                        let lock = if self.tune.locked.get(idx).copied().unwrap_or(false) {
                            "locked"
                        } else {
                            ""
                        };
                        let relax = if change.relaxation.is_relaxed() {
                            change.relaxation.label()
                        } else {
                            ""
                        };
                        let dist = if idx > 0 {
                            let prev = &solved.fingerings[idx - 1];
                            let d = voice_leading::distance(
                                &prev.fingering,
                                &change.fingering,
                                &self.fretboard,
                            );
                            format!("dist={}", d)
                        } else {
                            String::new()
                        };
                        ui.label(format!(
                            "{} | {} | {}/{}  {} {} {}",
                            self.tune.title_input,
                            chord_name,
                            idx + 1,
                            solved.fingerings.len(),
                            dist,
                            lock,
                            relax,
                        ));
                    }
                } else {
                    ui.label("Enter changes and press Solve");
                }
            });
        });

        egui::SidePanel::left("tune_input")
            .default_width(320.0)
            .show(ctx, |ui| {
                self.show_tune_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_tune_diagram(ui);
        });
    }

    fn show_tune_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Chart");
        ui.horizontal(|ui| {
            ui.label("Tune:");
            egui::ComboBox::from_id_salt("tune_preset")
                .selected_text(&self.tune.title_input)
                .show_ui(ui, |ui| {
                    for &(title, changes) in TUNE_PRESETS {
                        if ui
                            .selectable_label(self.tune.title_input == title, title)
                            .clicked()
                        {
                            self.tune.title_input = title.to_string();
                            self.tune.chart_input = changes.to_string();
                            self.tune.solved = None;
                            self.tune.selected_chord = 0;
                            self.tune.playback_start = None;
                            self.tune.locked.clear();
                        }
                    }
                });
        });
        ui.label("Changes (bars separated by |):");
        ui.add(
            egui::TextEdit::multiline(&mut self.tune.chart_input)
                .desired_width(f32::INFINITY)
                .desired_rows(4)
                .font(egui::FontId::monospace(12.0)),
        );

        ui.add_space(4.0);
        ui.separator();
        self.show_tune_constraints(ui);
        ui.separator();
        ui.add_space(4.0);

        if let Some(solved) = &self.tune.solved {
            ui.heading(format!("Progression ({})", solved.fingerings.len()));
            ui.separator();
            let mono = egui::FontId::monospace(13.0);
            let selected = self.tune.selected_chord;
            egui::ScrollArea::vertical()
                .id_salt("progression")
                .show(ui, |ui| {
                    for (i, change) in solved.fingerings.iter().enumerate() {
                        let chord_name = chords::chord_name(&change.root, change.quality);
                        let fret_lo = change.fingering.lowest_fret();
                        let fret_hi = fret_lo + change.fingering.fret_span();
                        let dist = if i > 0 {
                            let prev = &solved.fingerings[i - 1];
                            let d = voice_leading::distance(
                                &prev.fingering,
                                &change.fingering,
                                &self.fretboard,
                            );
                            format!(" d={:<2}", d)
                        } else {
                            "     ".to_string()
                        };
                        let lock = if self.tune.locked.get(i).copied().unwrap_or(false) {
                            "*"
                        } else {
                            " "
                        };
                        let relax = if change.relaxation.is_relaxed() {
                            "!"
                        } else {
                            " "
                        };
                        let label = format!(
                            "{}{}{:>2}. {:<8} {:>7} f{}-{}{}",
                            lock,
                            relax,
                            i + 1,
                            chord_name,
                            change.recipe.short_label(),
                            fret_lo,
                            fret_hi,
                            dist,
                        );
                        let text = egui::RichText::new(&label).font(mono.clone());
                        if ui.selectable_label(i == selected, text).clicked() {
                            self.tune.selected_chord = i;
                        }
                    }
                });
        }
    }

    fn show_tune_constraints(&mut self, ui: &mut egui::Ui) {
        let c = &mut self.tune.constraints;

        ui.horizontal(|ui| {
            ui.label("Tension:");
            ui.spacing_mut().slider_width = 120.0;
            ui.add(egui::Slider::new(&mut c.tension, 0.0..=1.0).show_value(false));
            let label = if c.tension < 0.2 {
                "grounded"
            } else if c.tension < 0.5 {
                "color"
            } else if c.tension < 0.8 {
                "open"
            } else {
                "abstract"
            };
            ui.label(label);
        });

        ui.horizontal(|ui| {
            ui.label("Smooth:");
            ui.spacing_mut().slider_width = 120.0;
            ui.add(egui::Slider::new(&mut c.smoothness, 0.25..=3.0).show_value(true));
        });

        ui.horizontal(|ui| {
            ui.label("Variation:");
            ui.spacing_mut().slider_width = 120.0;
            ui.add(egui::Slider::new(&mut c.variation, 0..=30).show_value(true));
        });

        ui.horizontal(|ui| {
            ui.label("Notes:");
            egui::ComboBox::from_id_salt("tune_note_filter")
                .selected_text(c.note_filter.label())
                .show_ui(ui, |ui| {
                    for filter in [
                        TuneNoteFilter::ThreeOrFour,
                        TuneNoteFilter::Three,
                        TuneNoteFilter::Four,
                        TuneNoteFilter::Five,
                        TuneNoteFilter::ThreeToFive,
                    ] {
                        ui.selectable_value(&mut c.note_filter, filter, filter.label());
                    }
                });
            ui.checkbox(&mut c.allow_open_strings, "Open");
            ui.checkbox(&mut c.expand_basic_chords, "Extensions");
        });

        ui.label("Fret range");
        ui.add(
            egui::Slider::new(&mut c.fret_min, 0..=15)
                .text("from")
                .show_value(true),
        );
        ui.add(
            egui::Slider::new(&mut c.fret_max, 0..=15)
                .text("to")
                .show_value(true),
        );
        if c.fret_min > c.fret_max {
            c.fret_max = c.fret_min;
        }
        ui.add(
            egui::Slider::new(&mut c.max_span, 2..=7)
                .text("max span")
                .show_value(true),
        );

        ui.horizontal(|ui| {
            ui.checkbox(&mut c.string_filter_on, "Strings");
            if c.string_filter_on {
                let names = ["E", "A", "D", "G", "B", "e"];
                for (i, name) in names.iter().enumerate() {
                    ui.checkbox(&mut c.strings[i], *name);
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut c.recipe_filter_on, "Recipes");
            if c.recipe_filter_on {
                for (i, recipe) in TUNE_RECIPES.iter().enumerate() {
                    ui.checkbox(&mut c.recipes[i], recipe.short_label());
                }
            }
        });
    }

    fn show_tune_diagram(&mut self, ui: &mut egui::Ui) {
        let Some(solved) = self.tune.solved.as_ref() else {
            ui.heading("Enter a chart and press Solve");
            ui.label("Example: Dm7 | G7 | Cmaj7 | Cmaj7");
            return;
        };
        let idx = self.tune.selected_chord;
        let total = solved.fingerings.len();
        let Some(change) = solved.fingerings.get(idx).cloned() else {
            return;
        };

        let chord_name = chords::chord_name(&change.root, change.quality);
        let recipe = change.recipe;
        let fingering = change.fingering.clone();
        let prev_info = if idx > 0 {
            let prev = &solved.fingerings[idx - 1];
            let d = voice_leading::distance(&prev.fingering, &fingering, &self.fretboard);
            let prev_name = chords::chord_name(&prev.root, prev.quality);
            Some((prev_name, d))
        } else {
            None
        };
        let alt_count = solved.alternatives.get(idx).map(|a| a.len()).unwrap_or(0);
        if self.tune.locked.len() < total {
            self.tune.locked.resize(total, false);
        }

        ui.heading(format!(
            "{}  {}  ({}/{})",
            chord_name,
            recipe.short_label(),
            idx + 1,
            total,
        ));
        ui.separator();
        ui.horizontal(|ui| {
            if let Some(locked) = self.tune.locked.get_mut(idx) {
                ui.checkbox(locked, "Lock");
            }
            ui.label(format!(
                "tension {:.0}% (raw {:.2})  score {}  {}",
                change.normalized_tension * 100.0,
                change.tension,
                change.rank_score,
                change.relaxation.label(),
            ));
        });

        let notes = fingering.notes(&self.fretboard);
        let note_names: Vec<String> = notes
            .iter()
            .enumerate()
            .filter_map(|(s, n)| {
                n.map(|note| {
                    let iv = fingering.intervals[s].map(|i| i.name).unwrap_or("?");
                    format!("{} ({})", note.pc_name(), iv)
                })
            })
            .collect();
        ui.label(format!("Notes: {}", note_names.join("  ")));

        if let Some((prev_name, d)) = prev_info {
            ui.label(format!("Voice leading from {}: distance {}", prev_name, d));
        }

        ui.horizontal(|ui| {
            if self.audio.is_some() && ui.button("Strum (Space)").clicked() {
                self.play_tune_strum();
            }
            if alt_count > 1 {
                ui.label(format!("← → swap ({} options)", alt_count));
            }
        });

        ui.add_space(8.0);
        paint_fretboard(ui, &fingering, &self.fretboard);
    }

    fn play_tune_strum(&mut self) {
        self.tune.playback_start = None;
        let fingering = self
            .tune
            .solved
            .as_ref()
            .and_then(|s| s.fingerings.get(self.tune.selected_chord))
            .map(|c| c.fingering.clone());
        if let (Some(audio), Some(f)) = (&mut self.audio, fingering) {
            audio.stop_all();
            let _ = audio.play_voicing(&f, &self.fretboard, 2.0);
        }
    }

    fn swap_tune_voicing(&mut self, direction: i32) {
        let idx = self.tune.selected_chord;
        let Some(solved) = &mut self.tune.solved else {
            return;
        };
        let Some(alts) = solved.alternatives.get(idx) else {
            return;
        };
        if alts.len() <= 1 {
            return;
        }

        let current_pos = solved.fingerings[idx].fingering.positions;
        let prev_pos: Option<[Option<u8>; 6]> = if idx > 0 {
            Some(solved.fingerings[idx - 1].fingering.positions)
        } else {
            None
        };
        let next_pos: Option<[Option<u8>; 6]> = if idx + 1 < solved.fingerings.len() {
            Some(solved.fingerings[idx + 1].fingering.positions)
        } else {
            None
        };

        let prev_f = prev_pos.map(|p| Fingering {
            positions: p,
            intervals: [None; 6],
        });
        let next_f = next_pos.map(|p| Fingering {
            positions: p,
            intervals: [None; 6],
        });

        let mut ranked: Vec<(usize, u32)> = alts
            .iter()
            .enumerate()
            .map(|(i, alt)| {
                let d_prev = prev_f
                    .as_ref()
                    .map(|p| voice_leading::distance(p, &alt.fingering, &self.fretboard))
                    .unwrap_or(0);
                let d_next = next_f
                    .as_ref()
                    .map(|n| voice_leading::distance(&alt.fingering, n, &self.fretboard))
                    .unwrap_or(0);
                (i, d_prev + d_next)
            })
            .collect();
        ranked.sort_by_key(|&(_, d)| d);

        let current_rank = ranked
            .iter()
            .position(|(i, _)| alts[*i].fingering.positions == current_pos)
            .unwrap_or(0);

        let new_rank = if direction > 0 {
            (current_rank + 1) % ranked.len()
        } else {
            (current_rank + ranked.len() - 1) % ranked.len()
        };

        let (alt_idx, _) = ranked[new_rank];
        let alt = alts[alt_idx].clone();

        solved.fingerings[idx].fingering = alt.fingering;
        solved.fingerings[idx].recipe = alt.recipe;
        solved.fingerings[idx].tension = alt.tension;
        solved.fingerings[idx].normalized_tension = alt.normalized_tension;
        solved.fingerings[idx].rank_score = alt.rank_score;
        solved.fingerings[idx].relaxation = alt.relaxation;
    }

    fn play_tune_all(&mut self) {
        let chords: Option<Vec<_>> = self.tune.solved.as_ref().map(|s| {
            s.fingerings
                .iter()
                .map(|c| (c.fingering.clone(), c.beats))
                .collect()
        });
        if let (Some(audio), Some(chords)) = (&mut self.audio, chords) {
            if audio
                .play_progression(&chords, &self.fretboard, TUNE_BPM)
                .is_ok()
            {
                self.tune.selected_chord = 0;
                self.tune.playback_start = Some(Instant::now());
            }
        }
    }
}

impl ChordzApp {
    fn show_selectors(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.label("Root:");
        let prev_root = self.root_index;
        egui::ComboBox::from_id_salt("root")
            .selected_text(self.root())
            .show_ui(ui, |ui| {
                for (i, name) in chords::ROOTS.iter().enumerate() {
                    ui.selectable_value(&mut self.root_index, i, *name);
                }
            });
        changed |= self.root_index != prev_root;

        ui.add_space(12.0);
        ui.label("Family:");
        let prev_family = self.family_index;
        egui::ComboBox::from_id_salt("family")
            .selected_text(self.family().name())
            .show_ui(ui, |ui| {
                for (i, family) in ChordFamily::all().iter().enumerate() {
                    ui.selectable_value(&mut self.family_index, i, family.name());
                }
            });
        changed |= self.family_index != prev_family;

        ui.add_space(12.0);
        ui.label("Notes:");
        let prev_notes = self.note_count_index;
        egui::ComboBox::from_id_salt("notes")
            .selected_text(self.note_count().to_string())
            .show_ui(ui, |ui| {
                for (i, &count) in NOTE_COUNTS.iter().enumerate() {
                    ui.selectable_value(&mut self.note_count_index, i, count.to_string());
                }
            });
        changed |= self.note_count_index != prev_notes;

        if changed {
            self.refresh_voicings();
        }
    }

    fn show_voicing_list(&mut self, ui: &mut egui::Ui) {
        let total: usize = self.groups.iter().map(|g| g.entries.len()).sum();
        let title = format!("Voicings ({} groups, {} total)", self.groups.len(), total);
        ui.heading(&title);
        ui.separator();

        let mono = egui::FontId::monospace(13.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, group) in self.groups.iter().enumerate() {
                let chord_name = self.chord_name_for(group.quality);
                let intervals: Vec<&str> = group
                    .intervals
                    .iter()
                    .map(|iv| compact_interval_name(*iv))
                    .collect();
                let count = group.entries.len();
                let label = format!(
                    "{:<8} {:>7}  {}  ({})",
                    chord_name,
                    group.recipe.short_label(),
                    intervals.join(" "),
                    count,
                );

                let text = egui::RichText::new(&label).font(mono.clone());
                let response = ui.selectable_label(i == self.selected_group, text);
                if response.clicked() {
                    self.selected_group = i;
                    self.selected_position = 0;
                }
            }
        });
    }

    fn show_diagram(&mut self, ui: &mut egui::Ui) {
        if let Some((group, entry)) = self.selected_entry().map(|(g, e)| (g.clone(), e.clone())) {
            let chord_name = self.chord_name_for(group.quality);
            ui.heading(format!(
                "{}  {}  {}",
                chord_name,
                entry.recipe.short_label(),
                entry.tension,
            ));
            ui.separator();

            let notes = entry.fingering.notes(&self.fretboard);
            let note_names: Vec<String> = notes
                .iter()
                .enumerate()
                .filter_map(|(s, n)| {
                    n.map(|note| {
                        let iv = entry.fingering.intervals[s].map(|i| i.name).unwrap_or("?");
                        format!("{} ({})", note.pc_name(), iv)
                    })
                })
                .collect();
            ui.label(format!("Notes: {}", note_names.join("  ")));

            let fret_lo = entry.fingering.lowest_fret();
            let fret_hi = fret_lo + entry.fingering.fret_span();
            let pos_count = group.entries.len();
            let pos = self.selected_position + 1;
            ui.horizontal(|ui| {
                ui.label(format!(
                    "f{}-{}  pos {}/{}",
                    fret_lo, fret_hi, pos, pos_count,
                ));
                if pos_count > 1 {
                    ui.label("  h/l: cycle positions");
                }
            });

            ui.horizontal(|ui| {
                if self.audio.is_some() {
                    if ui.button("Strum (Space)").clicked() {
                        self.play_current_strum();
                    }
                    if ui.button("Arpeggio").clicked() {
                        self.play_current_arpeggio();
                    }
                }
            });

            ui.add_space(8.0);
            paint_fretboard(ui, &entry.fingering, &self.fretboard);
        } else {
            ui.heading("No voicings for this combination");
        }
    }

    fn play_current_strum(&mut self) {
        let fingering = self.selected_entry().map(|(_, e)| e.fingering.clone());
        if let (Some(audio), Some(f)) = (&mut self.audio, fingering) {
            audio.stop_all();
            let _ = audio.play_voicing(&f, &self.fretboard, 2.0);
        }
    }

    fn play_current_arpeggio(&mut self) {
        let fingering = self.selected_entry().map(|(_, e)| e.fingering.clone());
        if let (Some(audio), Some(f)) = (&mut self.audio, fingering) {
            audio.stop_all();
            let _ = audio.play_arpeggio(&f, &self.fretboard, 0.4);
        }
    }
}

// --- GMC mode ---

impl ChordzApp {
    fn show_gmc_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("Root:");
        egui::ComboBox::from_id_salt("gmc_root")
            .selected_text(chords::ROOTS[self.gmc.root_index])
            .show_ui(ui, |ui| {
                for (i, name) in chords::ROOTS.iter().enumerate() {
                    ui.selectable_value(&mut self.gmc.root_index, i, *name);
                }
            });

        ui.add_space(8.0);
        ui.label("Scale:");
        let current_scale = &Scale::ALL[self.gmc.scale_index];
        egui::ComboBox::from_id_salt("gmc_scale")
            .selected_text(current_scale.name)
            .show_ui(ui, |ui| {
                let mut last_parent = None;
                for (i, scale) in Scale::ALL.iter().enumerate() {
                    if last_parent != Some(scale.parent) {
                        if last_parent.is_some() {
                            ui.separator();
                        }
                        ui.label(scale.parent.name());
                        last_parent = Some(scale.parent);
                    }
                    ui.selectable_value(&mut self.gmc.scale_index, i, scale.name);
                }
            });

        ui.add_space(8.0);
        ui.checkbox(&mut self.gmc.show_intervals, "Intervals");
    }

    fn update_gmc(&mut self, ctx: &egui::Context) {
        let scale = &Scale::ALL[self.gmc.scale_index];
        let root_pc = self.gmc.root_index as u8;
        let pair = &PAIRS[self.gmc.pair_index];
        let (triad_a, triad_b) = gmc::resolve_pair(root_pc, scale, pair);

        egui::SidePanel::left("gmc_pairs").default_width(220.0).show(ctx, |ui| {
            ui.heading("Pairs");
            ui.separator();
            for (i, p) in PAIRS.iter().enumerate() {
                let display = gmc::pair_display(root_pc, scale, p);
                let label = format!("{:<10} {}", p.label, display);
                if ui.selectable_label(i == self.gmc.pair_index, label).clicked() {
                    self.gmc.pair_index = i;
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "{} {} — {}",
                chords::ROOTS[self.gmc.root_index],
                scale.name,
                pair.label,
            ));
            ui.separator();
            paint_panoramic_fretboard(
                ui,
                &self.fretboard,
                root_pc,
                &triad_a,
                &triad_b,
                scale,
                self.gmc.show_intervals,
            );
        });
    }
}

// --- Pure logic helpers (no UI dependency) ---

fn find_quality(name: &str) -> &'static ChordQuality {
    ChordQuality::ALL.iter().find(|q| q.name == name).unwrap()
}

fn has_unique_pitch_classes(fingering: &Fingering, fretboard: &Fretboard) -> bool {
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

fn recipe_order(recipe: VoicingRecipe) -> usize {
    VoicingRecipe::all()
        .iter()
        .position(|r| *r == recipe)
        .unwrap_or(usize::MAX)
}

fn quality_order(family: ChordFamily, quality: &ChordQuality) -> usize {
    family
        .quality_names()
        .iter()
        .position(|name| *name == quality.name)
        .unwrap_or(usize::MAX)
}

fn tension_score(quality: &ChordQuality, recipe: VoicingRecipe) -> u32 {
    solver::quality_tension(quality.name) + solver::recipe_tension(recipe)
}

fn tension_label(quality: &ChordQuality, recipe: VoicingRecipe) -> &'static str {
    match tension_score(quality, recipe) {
        0 | 1 => "inside",
        2 | 3 => "color",
        _ => "out",
    }
}
