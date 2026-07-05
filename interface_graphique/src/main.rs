// point d'entree du gestionnaire de fichiers graphique

mod application;

use application::ApplicationMonarque;
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MonarqueFS — gestionnaire de fichiers",
        options,
        Box::new(|_cc| Ok(Box::new(ApplicationMonarque::default()))),
    )
}
