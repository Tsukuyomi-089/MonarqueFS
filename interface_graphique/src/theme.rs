// theme visuel monarque : sombre, arrondi, accents violet et or

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle};

// palette
pub const FOND: Color32 = Color32::from_rgb(0x0f, 0x0f, 0x17);
pub const PANNEAU: Color32 = Color32::from_rgb(0x15, 0x15, 0x20);
pub const CARTE: Color32 = Color32::from_rgb(0x1d, 0x1d, 0x2c);
pub const CARTE_CLAIRE: Color32 = Color32::from_rgb(0x26, 0x26, 0x38);
pub const ACCENT: Color32 = Color32::from_rgb(0x8b, 0x5c, 0xf6);
pub const ACCENT_FONCE: Color32 = Color32::from_rgb(0x6d, 0x3f, 0xd4);
pub const OR: Color32 = Color32::from_rgb(0xf5, 0xc5, 0x42);
pub const TEXTE: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xf2);
pub const TEXTE_FAIBLE: Color32 = Color32::from_rgb(0x8d, 0x8d, 0xa6);
pub const DANGER: Color32 = Color32::from_rgb(0xef, 0x50, 0x50);
pub const SUCCES: Color32 = Color32::from_rgb(0x34, 0xd0, 0x77);

// application du theme global
pub fn appliquer(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();

    // typographie
    style.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
    ]
    .into();

    // espacements genereux
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.spacing.menu_margin = egui::Margin::same(10);

    // animations un peu plus lentes et douces
    style.animation_time = 0.18;

    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.panel_fill = PANNEAU;
    visuals.window_fill = CARTE;
    visuals.extreme_bg_color = FOND;
    visuals.override_text_color = Some(TEXTE);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    // etats des composants
    let arrondi = CornerRadius::same(9);
    for etat in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        etat.corner_radius = arrondi;
    }
    visuals.widgets.inactive.bg_fill = CARTE_CLAIRE;
    visuals.widgets.inactive.weak_bg_fill = CARTE_CLAIRE;
    visuals.widgets.hovered.bg_fill = ACCENT_FONCE;
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x31, 0x31, 0x48);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT_FONCE;
    visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgb(0x2a, 0x2a, 0x3e));

    ctx.set_style_of(egui::Theme::Dark, style);
}

// carte arrondie standard
pub fn carte() -> egui::Frame {
    egui::Frame::new()
        .fill(CARTE)
        .corner_radius(14)
        .inner_margin(16)
}

// adoucissement cubique en sortie
pub fn adoucir(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

// couleurs breeze (kde) pour le gestionnaire de fichiers
pub const BREEZE_FOND: Color32 = Color32::from_rgb(0x1b, 0x1e, 0x20);
pub const BREEZE_PANNEAU: Color32 = Color32::from_rgb(0x2a, 0x2e, 0x32);
pub const BREEZE_CARTE: Color32 = Color32::from_rgb(0x31, 0x36, 0x3b);
pub const BREEZE_BLEU: Color32 = Color32::from_rgb(0x3d, 0xae, 0xe9);
pub const BREEZE_SELECTION: Color32 = Color32::from_rgb(0x2d, 0x5c, 0x76);

// conversion d'un horodatage unix en date lisible
pub fn horodatage_texte(secondes: u64) -> String {
    if secondes == 0 {
        return String::new();
    }
    // algorithme des jours civils (howard hinnant)
    let jours = (secondes / 86_400) as i64;
    let reste = secondes % 86_400;
    let (heure, minute) = ((reste / 3600) as u32, ((reste % 3600) / 60) as u32);
    let z = jours + 719_468;
    let ere = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let jour_ere = (z - ere * 146_097) as u64;
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let annee = annee_ere as i64 + ere * 400;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mp = (5 * jour_annee + 2) / 153;
    let jour = (jour_annee - (153 * mp + 2) / 5 + 1) as u32;
    let mois = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let annee = if mois <= 2 { annee + 1 } else { annee };
    format!("{jour:02}/{mois:02}/{annee} {heure:02}:{minute:02}")
}

// taille affichable
pub fn taille_lisible(octets: u64) -> String {
    const GO: f64 = 1024.0 * 1024.0 * 1024.0;
    const MO: f64 = 1024.0 * 1024.0;
    const KO: f64 = 1024.0;
    let o = octets as f64;
    if o >= GO {
        format!("{:.1} Go", o / GO)
    } else if o >= MO {
        format!("{:.1} Mo", o / MO)
    } else if o >= KO {
        format!("{:.1} Ko", o / KO)
    } else {
        format!("{octets} o")
    }
}
