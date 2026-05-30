use std::time::Instant;

use eframe::egui;

use super::app::{ChordzApp, TUNE_BPM};
use crate::theory::chart::{Chart, PRESETS as TUNE_PRESETS};
use crate::theory::chords;
use crate::theory::gmc::PAIRS;
use crate::theory::line_engine::{self, LineConfig, NoteEvent};
use crate::theory::line_pattern::{Direction, Pattern, PatternBlock, RhythmicFigure, TriadId};
use crate::theory::position::Rigidity;
use crate::theory::scale_defaults;
use crate::theory::scales::Scale;

// ── Colour palette ───────────────────────────────────────────────────────────
const T1_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 160, 255);
const T2_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 140, 50);
const GOLD: egui::Color32 = egui::Color32::from_rgb(255, 200, 60);

fn triad_color(triad: TriadId) -> egui::Color32 {
    match triad {
        TriadId::T1 => T1_COLOR,
        TriadId::T2 => T2_COLOR,
    }
}

fn roman_numeral(fret: u8) -> &'static str {
    match fret {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        11 => "XI",
        12 => "XII",
        _ => "?",
    }
}

// ── Playback constant ────────────────────────────────────────────────────────
const GMC_BPM: f32 = TUNE_BPM;

// ═════════════════════════════════════════════════════════════════════════════
impl ChordzApp {
    // ── Top-bar controls ─────────────────────────────────────────────────────
    pub(crate) fn show_gmc_tune_controls(&mut self, ui: &mut egui::Ui) {
        if ui.button("Generate").clicked() {
            self.generate_gmc_line();
        }
        ui.separator();
        if self.gmc_tune.generated.is_some() {
            if self.gmc_tune.playback_start.is_some() {
                ui.colored_label(egui::Color32::from_rgb(100, 220, 100), "▶ Playing");
                if ui.button("Pause (Space)").clicked() {
                    self.gmc_tune_pause();
                }
            } else {
                let n = self.gmc_tune.selected_measure + 1;
                ui.label(format!("Measure {}", n));
                if ui.button("Play (Space)").clicked() {
                    self.gmc_tune_play();
                }
            }
        }
        if let Some(err) = &self.gmc_tune.error.clone() {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
        }
    }

    // ── Main update ───────────────────────────────────────────────────────────
    pub(crate) fn update_gmc_tune(&mut self, ctx: &egui::Context) {
        // Keyboard handling
        let mut nav: i32 = 0;
        let mut toggle_play = false;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::H) {
                nav = -1;
            }
            if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::L) {
                nav = 1;
            }
            if i.key_pressed(egui::Key::Space) {
                toggle_play = true;
            }
        });

        if toggle_play {
            if self.gmc_tune.playback_start.is_some() {
                self.gmc_tune_pause();
            } else {
                self.gmc_tune_play();
            }
        }

        // Advance playback
        if let Some(start) = self.gmc_tune.playback_start {
            if let Some(chart) = self.try_parse_chart() {
                let elapsed = start.elapsed().as_secs_f32();
                let beat_dur = 60.0 / GMC_BPM;
                let mut cumulative = 0.0_f32;
                let mut target = self.gmc_tune.selected_measure;
                for (i, change) in chart.changes.iter().enumerate() {
                    let end = (cumulative + change.beats) * beat_dur;
                    if elapsed < end {
                        target = i;
                        break;
                    }
                    cumulative += change.beats;
                    target = i;
                }
                let total_time: f32 = chart
                    .changes
                    .iter()
                    .map(|c| c.beats * 60.0 / GMC_BPM)
                    .sum();
                if elapsed >= total_time {
                    self.gmc_tune.playback_start = None;
                    self.gmc_tune.selected_measure =
                        self.gmc_tune.last_clicked_measure.unwrap_or(0);
                } else {
                    self.gmc_tune.selected_measure = target;
                    ctx.request_repaint();
                }
            }
        }

        // Navigate when paused
        if nav != 0 && self.gmc_tune.playback_start.is_none() {
            if let Some(chart) = self.try_parse_chart() {
                let n = chart.changes.len();
                if n > 0 {
                    let cur = self.gmc_tune.selected_measure as i32;
                    self.gmc_tune.selected_measure =
                        ((cur + nav).rem_euclid(n as i32)) as usize;
                }
            }
        }

        egui::SidePanel::left("gmc_tune_sidebar")
            .default_width(300.0)
            .show(ctx, |ui| {
                self.show_gmc_tune_sidebar(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_gmc_tune_central(ui);
        });
    }

    // ── Sidebar ───────────────────────────────────────────────────────────────
    fn show_gmc_tune_sidebar(&mut self, ui: &mut egui::Ui) {
        // ── Chart input ──────────────────────────────────────────────────────
        ui.heading("Chart");
        ui.horizontal(|ui| {
            ui.label("Tune:");
            egui::ComboBox::from_id_salt("gmc_tune_preset")
                .selected_text(&self.gmc_tune.title_input)
                .show_ui(ui, |ui| {
                    for &(title, changes) in TUNE_PRESETS {
                        if ui
                            .selectable_label(self.gmc_tune.title_input == title, title)
                            .clicked()
                        {
                            self.gmc_tune.title_input = title.to_string();
                            self.gmc_tune.chart_input = changes.to_string();
                            self.gmc_tune.generated = None;
                            self.gmc_tune.selected_measure = 0;
                            self.gmc_tune.playback_start = None;
                            self.gmc_tune.scale_overrides.clear();
                        }
                    }
                });
        });
        ui.label("Changes (bars separated by |):");
        ui.add(
            egui::TextEdit::multiline(&mut self.gmc_tune.chart_input)
                .desired_width(f32::INFINITY)
                .desired_rows(4)
                .font(egui::FontId::monospace(12.0)),
        );

        ui.add_space(6.0);
        ui.separator();

        // ── Pair selector ────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Pair:");
            let current_label = PAIRS[self.gmc_tune.pair_index].label;
            egui::ComboBox::from_id_salt("gmc_pair")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    for (i, pair) in PAIRS.iter().enumerate() {
                        if ui
                            .selectable_label(self.gmc_tune.pair_index == i, pair.label)
                            .clicked()
                        {
                            self.gmc_tune.pair_index = i;
                        }
                    }
                });
        });

        // ── Figure selector ──────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Figure:");
            let current_label = self.gmc_tune.figure.label();
            egui::ComboBox::from_id_salt("gmc_figure")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    for fig in RhythmicFigure::ALL {
                        if ui
                            .selectable_label(self.gmc_tune.figure == fig, fig.label())
                            .clicked()
                        {
                            self.gmc_tune.figure = fig;
                        }
                    }
                });
        });

        // ── Position selector ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Position:");
            let fret = self.gmc_tune.position.base_fret;
            egui::ComboBox::from_id_salt("gmc_position")
                .selected_text(roman_numeral(fret))
                .show_ui(ui, |ui| {
                    for f in 1u8..=12 {
                        if ui
                            .selectable_label(fret == f, roman_numeral(f))
                            .clicked()
                        {
                            self.gmc_tune.position.base_fret = f;
                        }
                    }
                });
            let is_flexible = self.gmc_tune.position.rigidity == Rigidity::Flexible;
            let mut flex = is_flexible;
            if ui.checkbox(&mut flex, "Flexible").changed() {
                self.gmc_tune.position.rigidity = if flex {
                    Rigidity::Flexible
                } else {
                    Rigidity::Strict
                };
            }
        });

        ui.add_space(4.0);
        ui.separator();

        // ── Pattern section ──────────────────────────────────────────────────
        ui.heading("Pattern");
        ui.horizontal(|ui| {
            ui.label("Preset:");
            let name = self.gmc_tune.pattern.name;
            egui::ComboBox::from_id_salt("gmc_pattern_preset")
                .selected_text(name)
                .show_ui(ui, |ui| {
                    for preset in Pattern::all_presets() {
                        let selected = name == preset.name;
                        if ui.selectable_label(selected, preset.name).clicked() {
                            self.gmc_tune.pattern = preset;
                        }
                    }
                });
        });

        // Pattern block editor (Task 7)
        let mut changed = false;
        let block_count = self.gmc_tune.pattern.blocks.len();
        let mut remove_idx: Option<usize> = None;
        let mut add_block = false;

        egui::Grid::new("pattern_blocks")
            .num_columns(5)
            .spacing([4.0, 2.0])
            .show(ui, |ui| {
                for (i, block) in self.gmc_tune.pattern.blocks.iter_mut().enumerate() {
                    // Count DragValue
                    let mut count = block.count as i32;
                    if ui
                        .add(
                            egui::DragValue::new(&mut count)
                                .range(1..=6)
                                .speed(0.1),
                        )
                        .changed()
                    {
                        block.count = count as u8;
                        changed = true;
                    }

                    // Direction toggle
                    let dir_label = block.direction.label();
                    if ui.button(dir_label).clicked() {
                        block.direction = block.direction.invert();
                        changed = true;
                    }

                    // Triad toggle
                    let triad_label = block.triad.label();
                    let triad_color = match block.triad {
                        TriadId::T1 => T1_COLOR,
                        TriadId::T2 => T2_COLOR,
                    };
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new(triad_label).color(triad_color),
                        ))
                        .clicked()
                    {
                        block.triad = match block.triad {
                            TriadId::T1 => TriadId::T2,
                            TriadId::T2 => TriadId::T1,
                        };
                        changed = true;
                    }

                    // Remove button (only if more than 1 block)
                    if block_count > 1 {
                        if ui
                            .add(egui::Button::new(
                                egui::RichText::new("✕").color(egui::Color32::LIGHT_RED),
                            ))
                            .clicked()
                        {
                            remove_idx = Some(i);
                        }
                    } else {
                        ui.label("  ");
                    }

                    ui.end_row();
                }
            });

        if ui
            .add_enabled(
                block_count < 6,
                egui::Button::new("+ Add block"),
            )
            .clicked()
        {
            add_block = true;
        }

        if let Some(idx) = remove_idx {
            self.gmc_tune.pattern.blocks.remove(idx);
            self.gmc_tune.pattern.name = "Custom";
            changed = true;
        }
        if add_block {
            let last = self.gmc_tune.pattern.blocks.last().cloned();
            let new_block = PatternBlock::legacy(
                3,
                last
                    .as_ref()
                    .map(|b| b.direction.invert())
                    .unwrap_or(Direction::Ascending),
                last
                    .as_ref()
                    .map(|b| match b.triad {
                        TriadId::T1 => TriadId::T2,
                        TriadId::T2 => TriadId::T1,
                    })
                    .unwrap_or(TriadId::T1),
            );
            self.gmc_tune.pattern.blocks.push(new_block);
            self.gmc_tune.pattern.name = "Custom";
            changed = true;
        }
        if changed {
            self.gmc_tune.pattern.name = "Custom";
        }

        ui.add_space(4.0);
        ui.separator();

        // ── Scale overrides ──────────────────────────────────────────────────
        ui.heading("Scale overrides");
        let chart_opt = Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input).ok();
        if let Some(chart) = &chart_opt {
            let n = chart.changes.len();
            // Sync override vec length
            if self.gmc_tune.scale_overrides.len() != n {
                self.gmc_tune.scale_overrides.resize(n, None);
            }

            egui::ScrollArea::vertical()
                .id_salt("scale_overrides_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("scale_overrides_grid")
                        .num_columns(2)
                        .spacing([4.0, 2.0])
                        .show(ui, |ui| {
                            for (i, change) in chart.changes.iter().enumerate() {
                                let chord_name =
                                    chords::chord_name(&change.root, change.quality);
                                let default_scale = scale_defaults::default_scale(change.quality);
                                let override_idx = self.gmc_tune.scale_overrides[i];
                                let display = override_idx
                                    .map(|idx| Scale::ALL[idx].name)
                                    .unwrap_or(default_scale.name);
                                let color = if override_idx.is_some() {
                                    GOLD
                                } else {
                                    egui::Color32::GRAY
                                };
                                ui.colored_label(color, format!("{}: {}", chord_name, display));
                                egui::ComboBox::from_id_salt(format!("scale_ovr_{}", i))
                                    .selected_text(display)
                                    .show_ui(ui, |ui| {
                                        // Default option
                                        if ui
                                            .selectable_label(
                                                override_idx.is_none(),
                                                format!("Default ({})", default_scale.name),
                                            )
                                            .clicked()
                                        {
                                            self.gmc_tune.scale_overrides[i] = None;
                                        }
                                        for (si, scale) in Scale::ALL.iter().enumerate() {
                                            if ui
                                                .selectable_label(
                                                    override_idx == Some(si),
                                                    scale.name,
                                                )
                                                .clicked()
                                            {
                                                self.gmc_tune.scale_overrides[i] = Some(si);
                                            }
                                        }
                                    });
                                ui.end_row();
                            }
                        });
                });
        } else {
            ui.label("Enter a valid chart above");
        }
    }

    // ── Central panel ─────────────────────────────────────────────────────────
    fn show_gmc_tune_central(&mut self, ui: &mut egui::Ui) {
        let generated = self.gmc_tune.generated.clone();
        let chart_opt = self.try_parse_chart();
        let Some(chart) = chart_opt else {
            ui.heading("Enter a valid chart and press Generate");
            return;
        };
        let Some(events) = generated else {
            ui.heading("Press Generate to build a line");
            return;
        };

        // ── Tab renderer (Task 8) ────────────────────────────────────────────
        let num_measures = chart.changes.len();
        let selected_measure = self.gmc_tune.selected_measure.min(num_measures.saturating_sub(1));

        // Beat-to-event index: build per-measure event list
        // Map each event to its measure index
        let mut measure_events: Vec<Vec<&NoteEvent>> = vec![Vec::new(); num_measures];
        {
            let mut boundaries: Vec<(f32, f32, usize)> = Vec::new();
            let mut cumul = 0.0_f32;
            for (i, change) in chart.changes.iter().enumerate() {
                boundaries.push((cumul, cumul + change.beats, i));
                cumul += change.beats;
            }
            for event in &events {
                let midx = boundaries
                    .iter()
                    .rposition(|&(start, _, _)| event.beat >= start)
                    .unwrap_or(0);
                if midx < num_measures {
                    measure_events[midx].push(event);
                }
            }
        }

        // Tab dimensions
        let measure_width = 120.0_f32;
        let string_spacing = 14.0_f32;
        let num_strings = 6usize;
        let tab_height = string_spacing * (num_strings as f32 - 1.0) + 60.0; // room for labels

        let tab_area_width = measure_width * num_measures as f32 + 20.0;

        let mut clicked_measure: Option<usize> = None;

        ui.heading("Tab");
        egui::ScrollArea::horizontal()
            .id_salt("gmc_tab_scroll")
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(tab_area_width, tab_height),
                    egui::Sense::click(),
                );
                let origin = response.rect.left_top();

                // If the whole area was clicked, find which measure
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let rel_x = pos.x - origin.x;
                        let midx = (rel_x / measure_width) as usize;
                        if midx < num_measures {
                            clicked_measure = Some(midx);
                        }
                    }
                }

                let top_label_y = origin.y + 10.0;
                let scale_label_y = origin.y + 22.0;
                let strings_top = origin.y + 36.0;

                // Draw 6 string lines
                let total_tab_width = measure_width * num_measures as f32;
                for s in 0..num_strings {
                    let y = strings_top + s as f32 * string_spacing;
                    painter.line_segment(
                        [
                            egui::pos2(origin.x, y),
                            egui::pos2(origin.x + total_tab_width, y),
                        ],
                        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(80)),
                    );
                }

                // Draw measures
                for (midx, change) in chart.changes.iter().enumerate() {
                    let mx = origin.x + midx as f32 * measure_width;

                    // Selection background
                    if midx == selected_measure {
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(mx, origin.y),
                                egui::vec2(measure_width, tab_height),
                            ),
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(60, 80, 120, 80),
                        );
                    }

                    // Bar line
                    let bar_x = mx;
                    painter.line_segment(
                        [
                            egui::pos2(bar_x, strings_top),
                            egui::pos2(
                                bar_x,
                                strings_top + (num_strings - 1) as f32 * string_spacing,
                            ),
                        ],
                        egui::Stroke::new(1.5_f32, egui::Color32::from_gray(140)),
                    );

                    // Chord name above strings
                    let chord_name = chords::chord_name(&change.root, change.quality);
                    painter.text(
                        egui::pos2(mx + 4.0, top_label_y),
                        egui::Align2::LEFT_CENTER,
                        &chord_name,
                        egui::FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );

                    // Scale name below chord (gray = default, gold = override)
                    let default_scale = scale_defaults::default_scale(change.quality);
                    let (scale_name, scale_color) =
                        if let Some(Some(si)) = self.gmc_tune.scale_overrides.get(midx) {
                            (Scale::ALL[*si].name, GOLD)
                        } else {
                            (default_scale.name, egui::Color32::GRAY)
                        };
                    painter.text(
                        egui::pos2(mx + 4.0, scale_label_y),
                        egui::Align2::LEFT_CENTER,
                        scale_name,
                        egui::FontId::proportional(9.0),
                        scale_color,
                    );

                    // Draw notes for this measure
                    let mevents = &measure_events[midx];
                    let n_events = mevents.len().max(1);
                    let note_x_step = (measure_width - 8.0) / n_events as f32;

                    for (ei, event) in mevents.iter().enumerate() {
                        // Tab string: string 0 = low E, but in tab top line = high e (string 5)
                        let tab_str = 5 - event.string as usize;
                        let ey = strings_top + tab_str as f32 * string_spacing;
                        let ex = mx + 4.0 + (ei as f32 + 0.5) * note_x_step;

                        let color = triad_color(event.triad);
                        let label = event.fret.to_string();

                        // Background rect to cover string line
                        let text_w = 10.0_f32.max(label.len() as f32 * 6.5);
                        painter.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(ex, ey),
                                egui::vec2(text_w + 2.0, string_spacing * 0.85),
                            ),
                            2.0,
                            egui::Color32::from_gray(28),
                        );

                        painter.text(
                            egui::pos2(ex, ey),
                            egui::Align2::CENTER_CENTER,
                            &label,
                            egui::FontId::monospace(10.0),
                            color,
                        );
                    }
                }

                // Final bar line
                let end_x = origin.x + total_tab_width;
                painter.line_segment(
                    [
                        egui::pos2(end_x, strings_top),
                        egui::pos2(
                            end_x,
                            strings_top + (num_strings - 1) as f32 * string_spacing,
                        ),
                    ],
                    egui::Stroke::new(1.5_f32, egui::Color32::from_gray(140)),
                );
            });

        // Apply measure click
        if let Some(midx) = clicked_measure {
            self.gmc_tune.selected_measure = midx;
            self.gmc_tune.last_clicked_measure = Some(midx);
            // Stop playback if playing
            if self.gmc_tune.playback_start.is_some() {
                self.gmc_tune.playback_start = None;
            }
        }

        ui.add_space(8.0);
        ui.separator();

        // ── Per-measure fretboard (Task 9) ────────────────────────────────────
        let sel = self.gmc_tune.selected_measure.min(num_measures.saturating_sub(1));
        let change = &chart.changes[sel];
        let chord_name = chords::chord_name(&change.root, change.quality);
        ui.label(format!("Measure {} — {}", sel + 1, chord_name));

        let sel_events: Vec<NoteEvent> = measure_events[sel]
            .iter()
            .map(|e| (*e).clone())
            .collect();

        self.paint_gmc_fretboard(ui, &sel_events);
    }

    // ── Panoramic fretboard for selected measure ───────────────────────────────
    fn paint_gmc_fretboard(&self, ui: &mut egui::Ui, events: &[NoteEvent]) {
        let position = &self.gmc_tune.position;
        let (core_lo, core_hi) = position.core_range();
        let (stretch_lo, stretch_hi) = position.stretch_range();

        // Show frets from stretch range, at least 5 wide
        let fret_lo = stretch_lo;
        let fret_hi = stretch_hi.max(fret_lo + 4);
        let num_frets = (fret_hi - fret_lo + 1) as usize;
        let num_strings = 6usize;

        let string_spacing = 22.0_f32;
        let fret_spacing = 46.0_f32;
        let left_margin = 40.0_f32;
        let top_margin = 20.0_f32;
        let dot_radius = 9.0_f32;

        let total_width = left_margin + fret_spacing * num_frets as f32 + 20.0;
        let total_height = top_margin + string_spacing * (num_strings as f32 - 1.0) + 30.0;

        let (response, painter) =
            ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::hover());
        let origin = response.rect.left_top();

        let stroke_thin = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(70));

        // Core-position background highlight
        let core_start_fret = (core_lo.saturating_sub(fret_lo)) as f32;
        let core_end_fret = (core_hi.saturating_sub(fret_lo) + 1) as f32;
        let bg_x0 = origin.x + left_margin + core_start_fret * fret_spacing;
        let bg_x1 = origin.x + left_margin + core_end_fret * fret_spacing;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(bg_x0, origin.y + top_margin),
                egui::pos2(
                    bg_x1,
                    origin.y + top_margin + (num_strings as f32 - 1.0) * string_spacing,
                ),
            ),
            0.0,
            egui::Color32::from_rgba_unmultiplied(80, 80, 80, 60),
        );

        // Strings (horizontal)
        for s in 0..num_strings {
            let y = origin.y + top_margin + s as f32 * string_spacing;
            let x0 = origin.x + left_margin;
            let x1 = origin.x + left_margin + num_frets as f32 * fret_spacing;
            painter.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], stroke_thin);
        }

        // Fret lines (vertical) + fret numbers
        for f in 0..=num_frets {
            let x = origin.x + left_margin + f as f32 * fret_spacing;
            let y0 = origin.y + top_margin;
            let y1 = origin.y + top_margin + (num_strings as f32 - 1.0) * string_spacing;
            painter.line_segment([egui::pos2(x, y0), egui::pos2(x, y1)], stroke_thin);

            if f < num_frets {
                let fret_num = fret_lo as usize + f;
                let label_x = origin.x + left_margin + f as f32 * fret_spacing + fret_spacing * 0.5;
                painter.text(
                    egui::pos2(label_x, origin.y + 8.0),
                    egui::Align2::CENTER_CENTER,
                    fret_num.to_string(),
                    egui::FontId::monospace(10.0),
                    egui::Color32::DARK_GRAY,
                );
            }
        }

        // String labels on left
        let string_labels = ["E", "A", "D", "G", "B", "e"];
        for s in 0..num_strings {
            // s=0 is low E in model; displayed top-to-bottom as E,A,D,G,B,e
            let y = origin.y + top_margin + s as f32 * string_spacing;
            painter.text(
                egui::pos2(origin.x + 18.0, y),
                egui::Align2::CENTER_CENTER,
                string_labels[s],
                egui::FontId::monospace(11.0),
                egui::Color32::LIGHT_GRAY,
            );
        }

        // Draw notes
        for (order, event) in events.iter().enumerate() {
            // String 0 = low E, displayed as top row in panoramic view
            // In panoramic: s=0 is low E at top
            let fy = if event.fret == 0 {
                origin.x + left_margin
            } else {
                origin.x
                    + left_margin
                    + (event.fret as f32 - fret_lo as f32 - 0.5) * fret_spacing
                    + fret_spacing
            };

            let s = event.string as usize;
            let sy = origin.y + top_margin + s as f32 * string_spacing;

            // Only draw if fret is in our displayed range
            if event.fret < fret_lo || event.fret > fret_hi {
                continue;
            }

            let dot_x = origin.x
                + left_margin
                + (event.fret as f32 - fret_lo as f32 - 0.5) * fret_spacing
                + fret_spacing;
            let _ = fy;

            let color = triad_color(event.triad);
            painter.circle_filled(egui::pos2(dot_x, sy), dot_radius, color);

            // Order number
            painter.text(
                egui::pos2(dot_x, sy),
                egui::Align2::CENTER_CENTER,
                (order + 1).to_string(),
                egui::FontId::monospace(8.0),
                egui::Color32::BLACK,
            );
        }
    }

    // ── Generate ──────────────────────────────────────────────────────────────
    fn generate_gmc_line(&mut self) {
        self.gmc_tune.error = None;
        let chart = match Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input) {
            Ok(c) => c,
            Err(e) => {
                self.gmc_tune.error = Some(format!("Parse error: {:?}", e));
                return;
            }
        };

        let n = chart.changes.len();
        if self.gmc_tune.scale_overrides.len() != n {
            self.gmc_tune.scale_overrides.resize(n, None);
        }

        let pair = &PAIRS[self.gmc_tune.pair_index];
        let config = LineConfig {
            pattern: self.gmc_tune.pattern.clone(),
            figure: self.gmc_tune.figure,
            position: self.gmc_tune.position,
        };

        let events = line_engine::generate_line(
            &chart,
            &self.gmc_tune.scale_overrides,
            &self.fretboard,
            pair,
            &config,
        );

        self.gmc_tune.generated = Some(events);
        self.gmc_tune.selected_measure = 0;
        self.gmc_tune.playback_start = None;
    }

    // ── Playback helpers ──────────────────────────────────────────────────────
    fn gmc_tune_play(&mut self) {
        if self.gmc_tune.generated.is_none() {
            return;
        }
        self.gmc_tune.playback_start = Some(Instant::now());
    }

    fn gmc_tune_pause(&mut self) {
        self.gmc_tune.playback_start = None;
        if let Some(last) = self.gmc_tune.last_clicked_measure {
            self.gmc_tune.selected_measure = last;
        } else {
            self.gmc_tune.selected_measure = 0;
        }
    }

    // ── Parse chart helper ────────────────────────────────────────────────────
    fn try_parse_chart(&self) -> Option<Chart> {
        Chart::parse(&self.gmc_tune.title_input, &self.gmc_tune.chart_input).ok()
    }
}
