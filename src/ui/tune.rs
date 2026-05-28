use std::time::Instant;

use eframe::egui;

use super::app::{ChordzApp, TUNE_BPM, TUNE_RECIPES};
use super::fretboard::paint_fretboard;
use crate::theory::chart::{Chart, PRESETS as TUNE_PRESETS};
use crate::theory::chords;
use crate::voicings::generate::Fingering;
use crate::voicings::solver::{self, SolvedAlternative};
use crate::voicings::voice_leading;

impl ChordzApp {
    pub(crate) fn show_tune_controls(&mut self, ui: &mut egui::Ui) {
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
            #[cfg(feature = "native")]
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

    pub(crate) fn update_tune(&mut self, ctx: &egui::Context) {
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

        #[cfg(feature = "native")]
        if play_strum {
            self.play_tune_strum();
        }
        #[cfg(feature = "native")]
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
                        super::app::TuneNoteFilter::ThreeOrFour,
                        super::app::TuneNoteFilter::Three,
                        super::app::TuneNoteFilter::Four,
                        super::app::TuneNoteFilter::Five,
                        super::app::TuneNoteFilter::ThreeToFive,
                    ] {
                        ui.selectable_value(&mut c.note_filter, filter, filter.label());
                    }
                });
            ui.checkbox(&mut c.allow_open_strings, "Open");
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
            #[cfg(feature = "native")]
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

    #[cfg(feature = "native")]
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

    #[cfg(feature = "native")]
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
