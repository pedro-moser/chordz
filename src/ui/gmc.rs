use eframe::egui;

use super::app::{ChordzApp, GmcSubMode};
use super::fretboard::paint_panoramic_fretboard;
use crate::theory::chords;
use crate::theory::gmc::{self, PAIRS};
use crate::theory::scales::Scale;

impl ChordzApp {
    pub(crate) fn show_gmc_controls(&mut self, ui: &mut egui::Ui) {
        ui.selectable_value(&mut self.gmc.sub_mode, GmcSubMode::Explorer, "Explorer");
        ui.selectable_value(&mut self.gmc.sub_mode, GmcSubMode::Tune, "Tune");
        ui.separator();
        match self.gmc.sub_mode {
            GmcSubMode::Explorer => self.show_gmc_explorer_controls(ui),
            GmcSubMode::Tune => self.show_gmc_tune_controls(ui),
        }
    }

    fn show_gmc_explorer_controls(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn update_gmc(&mut self, ctx: &egui::Context) {
        match self.gmc.sub_mode {
            GmcSubMode::Explorer => self.update_gmc_explorer(ctx),
            GmcSubMode::Tune => self.update_gmc_tune(ctx),
        }
    }

    fn update_gmc_explorer(&mut self, ctx: &egui::Context) {
        let scale = &Scale::ALL[self.gmc.scale_index];
        let root_pc = self.gmc.root_index as u8;
        let pair = &PAIRS[self.gmc.pair_index];
        let (triad_a, triad_b) = gmc::resolve_pair(root_pc, scale, pair);

        egui::SidePanel::left("gmc_pairs")
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Pairs");
                ui.separator();
                for (i, p) in PAIRS.iter().enumerate() {
                    let display = gmc::pair_display(root_pc, scale, p);
                    let label = format!("{:<10} {}", p.label, display);
                    if ui
                        .selectable_label(i == self.gmc.pair_index, label)
                        .clicked()
                    {
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
