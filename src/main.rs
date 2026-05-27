use chordz::ui::app::ChordzApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_title("chordz"),
        ..Default::default()
    };
    eframe::run_native(
        "chordz",
        options,
        Box::new(|_cc| Ok(Box::new(ChordzApp::new()))),
    )
}
