use eframe::egui;

use super::app::ChordzApp;

impl ChordzApp {
    pub(crate) fn show_gmc_tune_controls(&mut self, ui: &mut egui::Ui) {
        ui.label("GMC Tune (WIP)");
    }

    pub(crate) fn update_gmc_tune(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("GMC Tune Mode");
            ui.label("Select a chart and press Generate");
        });
    }
}
