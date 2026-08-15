use std::collections::HashMap;

use eframe::egui;

use super::app::{
    find_quality, has_unique_pitch_classes, quality_order, recipe_order, tension_label,
    tension_score, ChordzApp, VoicingEntry, VoicingGroup, MAX_VOICINGS, NOTE_COUNTS,
    VOICINGS_PER_VOICE_SET,
};
use super::fretboard::{compact_interval_name, paint_fretboard};
use crate::theory::chords;
use crate::theory::chords::ChordFamily;
use crate::voicings::generate::{map_voice_set, Fingering};
use crate::voicings::procedural::generate_literal_voice_sets;
use crate::voicings::ranking::rank_fingerings_with_options;
use crate::voicings::recipe::VoicingRecipe;
use crate::voicings::rules::VoicingRules;

impl ChordzApp {
    pub(crate) fn refresh_voicings(&mut self) {
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
        let min_stability = 100u8;

        struct FlatEntry {
            quality: &'static chords::ChordQuality,
            recipe: VoicingRecipe,
            tension: &'static str,
            fingering: Fingering,
        }

        let mut flat: Vec<FlatEntry> = Vec::new();

        for quality_name in self.family().quality_names() {
            let quality = find_quality(quality_name);
            let voice_sets =
                generate_literal_voice_sets(root_pc, quality, note_count, min_stability);

            for (voice_set, _stability, _label) in &voice_sets {
                let mut fingerings = map_voice_set(voice_set, &self.fretboard, &rules);
                fingerings.retain(|f| has_unique_pitch_classes(f, &self.fretboard));
                rank_fingerings_with_options(&mut fingerings, voice_set, &self.fretboard, false, 0.0);
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

// --- Browser mode UI ---

impl ChordzApp {
    pub(crate) fn update_browser(&mut self, ctx: &egui::Context) {
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

        #[cfg(feature = "native")]
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

    pub(crate) fn show_selectors(&mut self, ui: &mut egui::Ui) {
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

            #[cfg(feature = "native")]
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

    #[cfg(feature = "native")]
    fn play_current_strum(&mut self) {
        let fingering = self.selected_entry().map(|(_, e)| e.fingering.clone());
        if let (Some(audio), Some(f)) = (&mut self.audio, fingering) {
            audio.stop_all();
            let _ = audio.play_voicing(&f, &self.fretboard, 2.0);
        }
    }

    #[cfg(feature = "native")]
    fn play_current_arpeggio(&mut self) {
        let fingering = self.selected_entry().map(|(_, e)| e.fingering.clone());
        if let (Some(audio), Some(f)) = (&mut self.audio, fingering) {
            audio.stop_all();
            let _ = audio.play_arpeggio(&f, &self.fretboard, 0.4);
        }
    }
}
