use eframe::egui;

use crate::theory::intervals::Interval;
use crate::theory::notes::PC_NAMES;
use crate::theory::scales::Scale;
use crate::voicings::fretboard::Fretboard;
use crate::voicings::generate::Fingering;

/// Draw the selected guitar fingering as an egui fretboard widget.
///
/// The widget is intentionally stateless: callers own selection, audio, and
/// candidate generation. This keeps it reusable across browser and chart modes.
pub(crate) fn paint_fretboard(ui: &mut egui::Ui, fingering: &Fingering, fretboard: &Fretboard) {
    let played: Vec<u8> = fingering.positions.iter().filter_map(|f| *f).collect();
    if played.is_empty() {
        return;
    }

    let min_fret = *played.iter().filter(|f| **f > 0).min().unwrap_or(&0);
    let max_fret = *played.iter().max().unwrap();
    let has_open = played.contains(&0);
    let start_fret = if has_open { 0 } else { min_fret };
    let end_fret = max_fret.max(start_fret + 3);
    let num_frets = (end_fret - start_fret + 1) as usize;

    let string_spacing = 28.0_f32;
    let fret_spacing = 40.0_f32;
    let left_margin = 40.0_f32;
    let top_margin = 30.0_f32;
    let dot_radius = 10.0_f32;

    let total_width = left_margin + string_spacing * 5.0 + 40.0;
    let total_height = top_margin + fret_spacing * num_frets as f32 + 20.0;

    let (response, painter) =
        ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::hover());
    let origin = response.rect.left_top();

    let string_labels = ["E", "A", "D", "G", "B", "e"];
    let stroke_thin = egui::Stroke::new(1.0_f32, egui::Color32::GRAY);
    let stroke_thick = egui::Stroke::new(2.0_f32, egui::Color32::WHITE);

    for (s, label) in string_labels.iter().enumerate() {
        let x = origin.x + left_margin + s as f32 * string_spacing;
        painter.text(
            egui::pos2(x, origin.y + 8.0),
            egui::Align2::CENTER_CENTER,
            *label,
            egui::FontId::monospace(13.0),
            egui::Color32::LIGHT_GRAY,
        );
    }

    for f in 0..=num_frets {
        let y = origin.y + top_margin + f as f32 * fret_spacing;
        let fret_num = start_fret as usize + f;

        let x_start = origin.x + left_margin - string_spacing * 0.4;
        let x_end = origin.x + left_margin + 5.0 * string_spacing + string_spacing * 0.4;
        painter.line_segment(
            [egui::pos2(x_start, y), egui::pos2(x_end, y)],
            if f == 0 && start_fret == 0 {
                stroke_thick
            } else {
                stroke_thin
            },
        );

        if f < num_frets {
            let label_y = y + fret_spacing * 0.5;
            painter.text(
                egui::pos2(origin.x + 16.0, label_y),
                egui::Align2::CENTER_CENTER,
                fret_num.to_string(),
                egui::FontId::monospace(12.0),
                egui::Color32::DARK_GRAY,
            );
        }
    }

    for s in 0..6 {
        let x = origin.x + left_margin + s as f32 * string_spacing;
        let y_top = origin.y + top_margin;
        let y_bot = origin.y + top_margin + num_frets as f32 * fret_spacing;
        painter.line_segment([egui::pos2(x, y_top), egui::pos2(x, y_bot)], stroke_thin);
    }

    for s in 0..6 {
        let x = origin.x + left_margin + s as f32 * string_spacing;

        match fingering.positions[s] {
            None => {
                painter.text(
                    egui::pos2(x, origin.y + top_margin - 12.0),
                    egui::Align2::CENTER_CENTER,
                    "×",
                    egui::FontId::monospace(14.0),
                    egui::Color32::from_rgb(120, 120, 120),
                );
            }
            Some(0) => {
                painter.circle_stroke(
                    egui::pos2(x, origin.y + top_margin - 12.0),
                    6.0,
                    egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
                );
            }
            Some(fret) => {
                let fret_offset = (fret - start_fret) as f32;
                let y = origin.y + top_margin + fret_offset * fret_spacing + fret_spacing * 0.5;

                let is_root = fingering.intervals[s] == Some(Interval::UNISON);
                let color = if is_root {
                    egui::Color32::from_rgb(255, 140, 50)
                } else {
                    egui::Color32::from_rgb(100, 180, 255)
                };
                painter.circle_filled(egui::pos2(x, y), dot_radius, color);

                if let Some(iv) = fingering.intervals[s] {
                    painter.text(
                        egui::pos2(x, y),
                        egui::Align2::CENTER_CENTER,
                        compact_interval_name(iv),
                        egui::FontId::monospace(9.0),
                        egui::Color32::BLACK,
                    );
                }

                if let Some(note) = fretboard.get_note(s, fret as usize) {
                    painter.text(
                        egui::pos2(x, y + dot_radius + 8.0),
                        egui::Align2::CENTER_CENTER,
                        note.pc_name(),
                        egui::FontId::monospace(9.0),
                        egui::Color32::from_rgb(160, 160, 160),
                    );
                }
            }
        }
    }
}

pub(crate) fn compact_interval_name(interval: Interval) -> &'static str {
    if interval == Interval::UNISON {
        "R"
    } else if interval == Interval::M7 {
        "M7"
    } else if interval == Interval::dim7 {
        "d7"
    } else {
        interval.name
    }
}

pub(crate) fn paint_panoramic_fretboard(
    ui: &mut egui::Ui,
    fretboard: &Fretboard,
    root_pc: u8,
    triad_a: &[u8; 3],
    triad_b: &[u8; 3],
    scale: &Scale,
    show_intervals: bool,
) {
    let num_frets: usize = 15;
    let num_strings: usize = 6;

    let string_spacing = 22.0_f32;
    let fret_spacing = 50.0_f32;
    let left_margin = 30.0_f32;
    let top_margin = 24.0_f32;
    let dot_radius = 8.0_f32;

    let total_width = left_margin + fret_spacing * num_frets as f32 + 20.0;
    let total_height = top_margin + string_spacing * (num_strings - 1) as f32 + 30.0;

    let (response, painter) =
        ui.allocate_painter(egui::vec2(total_width, total_height), egui::Sense::hover());
    let origin = response.rect.left_top();

    let color_a = egui::Color32::from_rgb(100, 160, 255);
    let color_b = egui::Color32::from_rgb(255, 140, 50);
    let stroke_thin = egui::Stroke::new(1.0_f32, egui::Color32::GRAY);
    let stroke_nut = egui::Stroke::new(2.5_f32, egui::Color32::WHITE);

    // Draw fret lines (vertical)
    for f in 0..=num_frets {
        let x = origin.x + left_margin + f as f32 * fret_spacing;
        let y_top = origin.y + top_margin;
        let y_bot = origin.y + top_margin + (num_strings - 1) as f32 * string_spacing;
        let stroke = if f == 0 { stroke_nut } else { stroke_thin };
        painter.line_segment([egui::pos2(x, y_top), egui::pos2(x, y_bot)], stroke);
    }

    // Draw strings (horizontal)
    for s in 0..num_strings {
        let y = origin.y + top_margin + s as f32 * string_spacing;
        let x_start = origin.x + left_margin;
        let x_end = origin.x + left_margin + num_frets as f32 * fret_spacing;
        painter.line_segment([egui::pos2(x_start, y), egui::pos2(x_end, y)], stroke_thin);
    }

    // Draw fret numbers
    for f in 0..num_frets {
        let x = origin.x + left_margin + f as f32 * fret_spacing + fret_spacing * 0.5;
        painter.text(
            egui::pos2(x, origin.y + 8.0),
            egui::Align2::CENTER_CENTER,
            f.to_string(),
            egui::FontId::monospace(10.0),
            egui::Color32::DARK_GRAY,
        );
    }

    // Draw dots
    for s in 0..num_strings {
        let y = origin.y + top_margin + s as f32 * string_spacing;
        for f in 0..=num_frets {
            let Some(note) = fretboard.get_note(s, f) else {
                continue;
            };
            let pc = note.pitch_class;
            let (color, in_pair) = if triad_a.contains(&pc) {
                (color_a, true)
            } else if triad_b.contains(&pc) {
                (color_b, true)
            } else {
                (egui::Color32::TRANSPARENT, false)
            };

            if !in_pair {
                continue;
            }

            let x = if f == 0 {
                origin.x + left_margin
            } else {
                origin.x + left_margin + (f - 1) as f32 * fret_spacing + fret_spacing * 0.5
            };

            painter.circle_filled(egui::pos2(x, y), dot_radius, color);

            let label = if show_intervals {
                let semitone = (pc + 12 - root_pc) % 12;
                scale.interval_name(semitone)
            } else {
                PC_NAMES[pc as usize]
            };
            painter.text(
                egui::pos2(x, y),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::monospace(8.0),
                egui::Color32::BLACK,
            );
        }
    }
}
