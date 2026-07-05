// point d'entree du gestionnaire de fichiers graphique

mod application;
mod theme;

use application::ApplicationMonarque;
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result {
    // peripherique preselectionne par le demon de veille
    let preselection = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 680.0])
            .with_min_inner_size([760.0, 520.0])
            .with_app_id("monarquefs"),
        ..Default::default()
    };
    eframe::run_native(
        "MonarqueFS — gestionnaire de fichiers",
        options,
        Box::new(move |_cc| Ok(Box::new(ApplicationMonarque::nouvelle(preselection)))),
    )
}
