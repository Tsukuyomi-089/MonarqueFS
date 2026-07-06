// gestionnaire de fichiers a deux panneaux : hote et volume monarque

use crate::theme;
use eframe::egui;
use gestionnaire_fichiers::arborescence::{copier, Arborescence, EntreeArbre};
use gestionnaire_fichiers::{ArbreHote, ArbreVolume};

// panneau vise par une action
#[derive(Clone, Copy, PartialEq)]
pub enum Cote {
    Hote,
    Volume,
}

// actions differees pour eviter les conflits d'emprunt pendant le rendu
enum Action {
    Entrer(Cote, String),
    Remonter(Cote),
    Racine(Cote),
    Selection(Cote, String),
    Copier(Cote),
    Supprimer(Cote),
    NouveauDossier(Cote),
    Renommer(Cote),
}

pub struct Gestionnaire {
    hote: ArbreHote,
    volume: ArbreVolume,
    liste_hote: Vec<EntreeArbre>,
    liste_volume: Vec<EntreeArbre>,
    selection_hote: Option<String>,
    selection_volume: Option<String>,
    // saisies
    nouveau_dossier_hote: String,
    nouveau_dossier_volume: String,
    renommer_hote: String,
    renommer_volume: String,
    // message d'etat
    pub message: String,
    pub erreur: bool,
    // demande de fermeture du volume
    pub fermeture_demandee: bool,
}

impl Gestionnaire {
    pub fn nouveau(volume: ArbreVolume) -> Self {
        let mut g = Self {
            hote: ArbreHote::nouveau(),
            volume,
            liste_hote: Vec::new(),
            liste_volume: Vec::new(),
            selection_hote: None,
            selection_volume: None,
            nouveau_dossier_hote: String::new(),
            nouveau_dossier_volume: String::new(),
            renommer_hote: String::new(),
            renommer_volume: String::new(),
            message: String::new(),
            erreur: false,
            fermeture_demandee: false,
        };
        g.rafraichir_hote();
        g.rafraichir_volume();
        g
    }

    pub fn liberer(self) -> gestionnaire_fichiers::ResultatFs<()> {
        self.volume.fermer()
    }

    fn signaler(&mut self, texte: impl Into<String>, erreur: bool) {
        self.message = texte.into();
        self.erreur = erreur;
    }

    fn rafraichir_hote(&mut self) {
        match self.hote.lister() {
            Ok(l) => self.liste_hote = l,
            Err(e) => self.signaler(format!("hote : {e}"), true),
        }
        self.selection_hote = None;
    }

    fn rafraichir_volume(&mut self) {
        match self.volume.lister() {
            Ok(l) => self.liste_volume = l,
            Err(e) => self.signaler(format!("volume : {e}"), true),
        }
        self.selection_volume = None;
    }

    // arbre et selection d'un cote
    fn arbre(&mut self, cote: Cote) -> &mut dyn Arborescence {
        match cote {
            Cote::Hote => &mut self.hote,
            Cote::Volume => &mut self.volume,
        }
    }

    fn selection(&self, cote: Cote) -> &Option<String> {
        match cote {
            Cote::Hote => &self.selection_hote,
            Cote::Volume => &self.selection_volume,
        }
    }

    fn rafraichir(&mut self, cote: Cote) {
        match cote {
            Cote::Hote => self.rafraichir_hote(),
            Cote::Volume => self.rafraichir_volume(),
        }
    }

    // application d'une action differee
    fn appliquer(&mut self, action: Action) {
        match action {
            Action::Entrer(cote, nom) => {
                let _ = self.arbre(cote).entrer(&nom);
                self.rafraichir(cote);
            }
            Action::Remonter(cote) => {
                self.arbre(cote).remonter();
                self.rafraichir(cote);
            }
            Action::Racine(cote) => {
                self.arbre(cote).aller_racine();
                self.rafraichir(cote);
            }
            Action::Selection(cote, nom) => match cote {
                Cote::Hote => self.selection_hote = Some(nom),
                Cote::Volume => self.selection_volume = Some(nom),
            },
            Action::Copier(source) => self.copier_vers_autre(source),
            Action::Supprimer(cote) => {
                if let Some(nom) = self.selection(cote).clone() {
                    match self.arbre(cote).supprimer(&nom) {
                        Ok(()) => self.signaler(format!("supprimé : {nom}"), false),
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                    self.rafraichir(cote);
                }
            }
            Action::NouveauDossier(cote) => {
                let nom = match cote {
                    Cote::Hote => self.nouveau_dossier_hote.trim().to_string(),
                    Cote::Volume => self.nouveau_dossier_volume.trim().to_string(),
                };
                if !nom.is_empty() {
                    match self.arbre(cote).creer_dossier(&nom) {
                        Ok(()) => {
                            self.signaler(format!("dossier créé : {nom}"), false);
                            match cote {
                                Cote::Hote => self.nouveau_dossier_hote.clear(),
                                Cote::Volume => self.nouveau_dossier_volume.clear(),
                            }
                        }
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                    self.rafraichir(cote);
                }
            }
            Action::Renommer(cote) => {
                let nouveau = match cote {
                    Cote::Hote => self.renommer_hote.trim().to_string(),
                    Cote::Volume => self.renommer_volume.trim().to_string(),
                };
                if let (Some(nom), false) = (self.selection(cote).clone(), nouveau.is_empty()) {
                    match self.arbre(cote).renommer(&nom, &nouveau) {
                        Ok(()) => self.signaler(format!("renommé : {nom} → {nouveau}"), false),
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                    self.rafraichir(cote);
                }
            }
        }
    }

    // copie de la selection d'un cote vers l'autre
    fn copier_vers_autre(&mut self, source: Cote) {
        let Some(nom) = self.selection(source).clone() else {
            self.signaler("aucune sélection à copier", true);
            return;
        };
        let resultat = match source {
            Cote::Hote => copier(&mut self.hote, &nom, &mut self.volume),
            Cote::Volume => copier(&mut self.volume, &nom, &mut self.hote),
        };
        match resultat {
            Ok(()) => self.signaler(format!("copié : {nom}"), false),
            Err(e) => self.signaler(format!("échec de copie : {e}"), true),
        }
        // rafraichir la destination
        match source {
            Cote::Hote => self.rafraichir_volume(),
            Cote::Volume => self.rafraichir_hote(),
        }
    }

    // rendu d'un panneau, collecte les actions
    fn panneau(
        ui: &mut egui::Ui,
        cote: Cote,
        titre: &str,
        chemin: &str,
        liste: &[EntreeArbre],
        selection: &Option<String>,
        actions: &mut Vec<Action>,
    ) {
        theme::carte().show(ui, |ui| {
            ui.set_min_height(360.0);
            // en-tete du panneau
            ui.horizontal(|ui| {
                let icone = if cote == Cote::Hote { "💻" } else { "🔑" };
                ui.label(egui::RichText::new(icone).size(18.0));
                ui.label(egui::RichText::new(titre).strong().color(theme::OR));
            });
            ui.horizontal(|ui| {
                if ui.small_button("🏠").on_hover_text("racine").clicked() {
                    actions.push(Action::Racine(cote));
                }
                if ui.small_button("⬆").on_hover_text("dossier parent").clicked() {
                    actions.push(Action::Remonter(cote));
                }
                ui.label(
                    egui::RichText::new(chemin)
                        .monospace()
                        .small()
                        .color(theme::TEXTE_FAIBLE),
                );
            });
            ui.separator();

            // liste des entrees
            egui::ScrollArea::vertical()
                .id_salt(titre)
                .max_height(300.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if liste.is_empty() {
                        ui.label(
                            egui::RichText::new("(dossier vide)").color(theme::TEXTE_FAIBLE),
                        );
                    }
                    for entree in liste {
                        let choisi = selection.as_deref() == Some(entree.nom.as_str());
                        let icone = if entree.est_dossier { "📁" } else { "📄" };
                        let etiquette = if entree.est_dossier {
                            format!("{icone} {}", entree.nom)
                        } else {
                            format!("{icone} {}   {}", entree.nom, theme::taille_lisible(entree.taille))
                        };
                        let reponse = ui.selectable_label(choisi, etiquette);
                        if reponse.clicked() {
                            actions.push(Action::Selection(cote, entree.nom.clone()));
                        }
                        // double clic : entrer dans le dossier
                        if reponse.double_clicked() && entree.est_dossier {
                            actions.push(Action::Entrer(cote, entree.nom.clone()));
                        }
                    }
                });
        });
    }

    // barre d'outils d'un panneau
    fn outils_panneau(
        ui: &mut egui::Ui,
        cote: Cote,
        saisie_dossier: &mut String,
        saisie_renommer: &mut String,
        a_selection: bool,
        actions: &mut Vec<Action>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(saisie_dossier)
                    .hint_text("nouveau dossier")
                    .desired_width(120.0),
            );
            if ui.button("📁+").on_hover_text("créer le dossier").clicked() {
                actions.push(Action::NouveauDossier(cote));
            }
            if a_selection {
                ui.add(
                    egui::TextEdit::singleline(saisie_renommer)
                        .hint_text("renommer en")
                        .desired_width(110.0),
                );
                if ui.button("✏").on_hover_text("renommer").clicked() {
                    actions.push(Action::Renommer(cote));
                }
                let poubelle =
                    egui::Button::new(egui::RichText::new("🗑").color(theme::TEXTE))
                        .fill(theme::DANGER.gamma_multiply(0.5));
                if ui.add(poubelle).on_hover_text("supprimer").clicked() {
                    actions.push(Action::Supprimer(cote));
                }
            }
        });
    }

    pub fn afficher(&mut self, ui: &mut egui::Ui) {
        let mut actions: Vec<Action> = Vec::new();

        // barre superieure : titre du volume et fermeture
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("👑").size(20.0));
            ui.label(
                egui::RichText::new(self.volume.etiquette())
                    .strong()
                    .color(theme::OR),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let bouton = egui::Button::new("🔒 Verrouiller").fill(theme::ACCENT_FONCE);
                if ui.add(bouton).clicked() {
                    self.fermeture_demandee = true;
                }
            });
        });
        ui.add_space(4.0);

        // boutons de copie centraux
        ui.horizontal(|ui| {
            ui.with_layout(
                egui::Layout::top_down(egui::Align::Center).with_cross_justify(false),
                |_ui| {},
            );
            let copier_vers_vol = egui::Button::new(
                egui::RichText::new("Copier vers la clé  →").strong(),
            )
            .fill(theme::ACCENT);
            if ui
                .add_enabled(self.selection_hote.is_some(), copier_vers_vol)
                .clicked()
            {
                actions.push(Action::Copier(Cote::Hote));
            }
            let copier_vers_hote =
                egui::Button::new(egui::RichText::new("←  Copier vers le PC").strong())
                    .fill(theme::ACCENT);
            if ui
                .add_enabled(self.selection_volume.is_some(), copier_vers_hote)
                .clicked()
            {
                actions.push(Action::Copier(Cote::Volume));
            }
        });
        ui.add_space(6.0);

        // donnees clonees pour le rendu sans conflit d'emprunt
        let chemin_hote = self.hote.chemin_courant();
        let chemin_volume = self.volume.chemin_courant();
        let liste_hote = self.liste_hote.clone();
        let liste_volume = self.liste_volume.clone();
        let sel_hote = self.selection_hote.clone();
        let sel_volume = self.selection_volume.clone();

        let dispo = ui.available_width();
        ui.horizontal_top(|ui| {
            // panneau hote
            ui.vertical(|ui| {
                ui.set_width(dispo * 0.49);
                Self::panneau(
                    ui,
                    Cote::Hote,
                    "Cet ordinateur",
                    &chemin_hote,
                    &liste_hote,
                    &sel_hote,
                    &mut actions,
                );
                Self::outils_panneau(
                    ui,
                    Cote::Hote,
                    &mut self.nouveau_dossier_hote,
                    &mut self.renommer_hote,
                    sel_hote.is_some(),
                    &mut actions,
                );
            });
            // panneau volume
            ui.vertical(|ui| {
                ui.set_width(dispo * 0.49);
                Self::panneau(
                    ui,
                    Cote::Volume,
                    &self.volume.etiquette(),
                    &chemin_volume,
                    &liste_volume,
                    &sel_volume,
                    &mut actions,
                );
                Self::outils_panneau(
                    ui,
                    Cote::Volume,
                    &mut self.nouveau_dossier_volume,
                    &mut self.renommer_volume,
                    sel_volume.is_some(),
                    &mut actions,
                );
            });
        });

        // application des actions collectees
        for action in actions {
            self.appliquer(action);
        }
    }
}
