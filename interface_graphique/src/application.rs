// application graphique du gestionnaire de fichiers monarque

use crate::theme;
use eframe::egui;
use gestionnaire_fichiers::mise_a_jour::{self, EtapeMaj};
use gestionnaire_fichiers::{
    lister_peripheriques, preparer_support, InfoEntree, InfoPeripherique, Session, TypeNoeud,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// intervalle d'analyse des peripheriques
const INTERVALLE_ANALYSE: Duration = Duration::from_secs(2);
// duree des transitions d'ecran
const DUREE_TRANSITION: f32 = 0.35;

// ecran affiche
#[derive(PartialEq)]
enum Ecran {
    Accueil,
    Deverrouillage,
    Formatage,
    Explorateur,
    Parametres,
}

// support cible : peripherique reel ou image disque
#[derive(Clone)]
struct Cible {
    chemin: PathBuf,
    etiquette: String,
    taille_octets: u64,
    index_partition: usize,
}

pub struct ApplicationMonarque {
    ecran: Ecran,
    theme_applique: bool,
    // animation de transition d'ecran
    instant_ecran: Instant,
    // detection des peripheriques
    peripheriques: Vec<InfoPeripherique>,
    derniere_analyse: Option<Instant>,
    // support en cours
    cible: Option<Cible>,
    session: Option<Session>,
    // saisies de connexion et de formatage
    phrase: String,
    phrase_confirmation: String,
    nom_volume: String,
    accepte_effacement: bool,
    chemin_image: String,
    index_image: String,
    // formatage en arriere plan
    formatage: Option<mpsc::Receiver<Result<(), String>>>,
    // mise a jour en arriere plan
    maj: Option<mpsc::Receiver<EtapeMaj>>,
    journal_maj: Vec<String>,
    // etat de l'explorateur
    chemin_courant: String,
    entrees: Vec<InfoEntree>,
    selection: Option<String>,
    contenu_texte: Option<String>,
    metas: Vec<(String, String)>,
    nom_nouveau_dossier: String,
    chemin_import: String,
    chemin_export: String,
    nouveau_nom: String,
    meta_cle: String,
    meta_valeur: String,
    // barre de message
    message: String,
    message_erreur: bool,
    instant_message: Instant,
}

impl ApplicationMonarque {
    // creation, avec peripherique preselectionne par le demon de veille
    pub fn nouvelle(preselection: Option<PathBuf>) -> Self {
        let mut application = Self {
            ecran: Ecran::Accueil,
            theme_applique: false,
            instant_ecran: Instant::now(),
            peripheriques: Vec::new(),
            derniere_analyse: None,
            cible: None,
            session: None,
            phrase: String::new(),
            phrase_confirmation: String::new(),
            nom_volume: String::new(),
            accepte_effacement: false,
            chemin_image: String::new(),
            index_image: "0".to_string(),
            formatage: None,
            maj: None,
            journal_maj: Vec::new(),
            chemin_courant: "/".to_string(),
            entrees: Vec::new(),
            selection: None,
            contenu_texte: None,
            metas: Vec::new(),
            nom_nouveau_dossier: String::new(),
            chemin_import: String::new(),
            chemin_export: String::new(),
            nouveau_nom: String::new(),
            meta_cle: String::new(),
            meta_valeur: String::new(),
            message: String::new(),
            message_erreur: false,
            instant_message: Instant::now(),
        };
        // lancement par le demon : ouverture directe du deverrouillage
        if let Some(chemin) = preselection {
            let taille = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
            application.cible = Some(Cible {
                etiquette: chemin.display().to_string(),
                chemin,
                taille_octets: taille,
                index_partition: 0,
            });
            application.changer_ecran(Ecran::Deverrouillage);
        }
        application
    }

    fn changer_ecran(&mut self, ecran: Ecran) {
        self.ecran = ecran;
        self.instant_ecran = Instant::now();
        self.message.clear();
    }

    fn signaler(&mut self, texte: impl Into<String>, erreur: bool) {
        self.message = texte.into();
        self.message_erreur = erreur;
        self.instant_message = Instant::now();
    }

    // analyse periodique des peripheriques branches
    fn analyser_peripheriques(&mut self) {
        let a_jour = self
            .derniere_analyse
            .is_some_and(|i| i.elapsed() < INTERVALLE_ANALYSE);
        if a_jour {
            return;
        }
        self.peripheriques = lister_peripheriques();
        self.derniere_analyse = Some(Instant::now());
    }

    // ------- volume -------

    fn deverrouiller(&mut self) {
        let Some(cible) = self.cible.clone() else {
            return;
        };
        match Session::ouvrir(&cible.chemin, cible.index_partition, &self.phrase) {
            Ok(session) => {
                self.session = Some(session);
                self.phrase.clear();
                self.chemin_courant = "/".to_string();
                self.changer_ecran(Ecran::Explorateur);
                self.actualiser();
            }
            Err(e) => self.signaler(format!("{e}"), true),
        }
    }

    fn verrouiller(&mut self) {
        if let Some(session) = self.session.take() {
            if let Err(e) = session.fermer() {
                self.signaler(format!("erreur a la fermeture : {e}"), true);
            }
        }
        self.changer_ecran(Ecran::Accueil);
    }

    // lancement du formatage en arriere plan
    fn lancer_formatage(&mut self) {
        let Some(cible) = self.cible.clone() else {
            return;
        };
        let (emetteur, recepteur) = mpsc::channel();
        let nom = self.nom_volume.clone();
        let phrase = self.phrase.clone();
        std::thread::spawn(move || {
            let resultat =
                preparer_support(&cible.chemin, &nom, &phrase).map_err(|e| format!("{e}"));
            emetteur.send(resultat).ok();
        });
        self.formatage = Some(recepteur);
    }

    // ------- explorateur -------

    fn chemin_de(&self, nom: &str) -> String {
        if self.chemin_courant == "/" {
            format!("/{nom}")
        } else {
            format!("{}/{nom}", self.chemin_courant)
        }
    }

    fn actualiser(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        match session.lister(&self.chemin_courant) {
            Ok(entrees) => {
                self.entrees = entrees;
                self.selection = None;
                self.contenu_texte = None;
                self.metas.clear();
            }
            Err(e) => self.signaler(format!("{e}"), true),
        }
    }

    fn selectionner(&mut self, entree: &InfoEntree) {
        let chemin = self.chemin_de(&entree.nom);
        if entree.type_noeud == TypeNoeud::Dossier {
            self.chemin_courant = chemin;
            self.actualiser();
            return;
        }
        self.selection = Some(chemin.clone());
        self.nouveau_nom = entree.nom.clone();
        if let Some(session) = self.session.as_mut() {
            self.metas = session.lire_metas(&chemin).unwrap_or_default();
            self.contenu_texte = match session.lire_fichier(&chemin) {
                Ok(donnees) if donnees.len() <= 64 * 1024 => {
                    Some(String::from_utf8_lossy(&donnees).into_owned())
                }
                Ok(_) => Some("(fichier trop grand pour l'apercu)".to_string()),
                Err(e) => Some(format!("erreur de lecture : {e}")),
            };
        }
    }

    // ------- composants visuels -------

    // transition d'entree de l'ecran : fondu et glissement
    fn appliquer_transition(&self, ui: &mut egui::Ui) -> f32 {
        let t = theme::adoucir(self.instant_ecran.elapsed().as_secs_f32() / DUREE_TRANSITION);
        ui.set_opacity(t);
        ui.add_space((1.0 - t) * 18.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        }
        t
    }

    // en-tete anime avec la couronne flottante
    fn entete(&self, ui: &mut egui::Ui) {
        let temps = ui.input(|i| i.time);
        ui.vertical_centered(|ui| {
            ui.add_space(6.0 + (temps * 1.6).sin() as f32 * 3.0);
            ui.label(egui::RichText::new("👑").size(44.0));
            ui.label(
                egui::RichText::new("MonarqueFS")
                    .size(30.0)
                    .strong()
                    .color(theme::OR),
            );
            ui.label(
                egui::RichText::new("vos fichiers, chiffrés par défaut")
                    .color(theme::TEXTE_FAIBLE),
            );
        });
        ui.add_space(14.0);
    }

    // barre de message avec fondu de sortie
    fn barre_message(&self, ui: &mut egui::Ui) {
        if self.message.is_empty() {
            return;
        }
        let age = self.instant_message.elapsed().as_secs_f32();
        let opacite = if self.message_erreur {
            1.0
        } else {
            (1.0 - (age - 4.0) / 1.5).clamp(0.0, 1.0)
        };
        if opacite <= 0.0 {
            return;
        }
        ui.set_opacity(opacite);
        let couleur = if self.message_erreur {
            theme::DANGER
        } else {
            theme::SUCCES
        };
        let prefixe = if self.message_erreur { "✖" } else { "✔" };
        ui.colored_label(couleur, format!("{prefixe} {}", self.message));
        if opacite < 1.0 {
            ui.ctx().request_repaint();
        }
    }

    // carte d'un peripherique detecte
    fn carte_peripherique(&mut self, ui: &mut egui::Ui, peripherique: &InfoPeripherique) {
        let temps = ui.input(|i| i.time);
        theme::carte().show(ui, |ui| {
            ui.horizontal(|ui| {
                let icone = if peripherique.amovible { "🔑" } else { "💽" };
                ui.label(egui::RichText::new(icone).size(30.0));
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&peripherique.modele)
                            .strong()
                            .size(16.0),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}",
                            peripherique.chemin.display(),
                            theme::taille_lisible(peripherique.taille_octets)
                        ))
                        .color(theme::TEXTE_FAIBLE)
                        .small(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !peripherique.accessible {
                        ui.label(
                            egui::RichText::new("accès refusé")
                                .color(theme::DANGER)
                                .small(),
                        )
                        .on_hover_text("executer : sudo monarque installer_udev");
                    } else if peripherique.est_monarque {
                        // badge dore pulsant
                        let pulsation = 0.6 + 0.4 * ((temps * 2.4).sin() * 0.5 + 0.5) as f32;
                        if ui
                            .button(
                                egui::RichText::new("🔓 Déverrouiller").color(theme::TEXTE),
                            )
                            .clicked()
                        {
                            self.cible = Some(Cible {
                                chemin: peripherique.chemin.clone(),
                                etiquette: peripherique.modele.clone(),
                                taille_octets: peripherique.taille_octets,
                                index_partition: 0,
                            });
                            self.changer_ecran(Ecran::Deverrouillage);
                        }
                        ui.label(
                            egui::RichText::new("● MonarqueFS")
                                .color(theme::OR.gamma_multiply(pulsation))
                                .strong(),
                        );
                        ui.ctx().request_repaint();
                    } else {
                        if ui.button("Formater…").clicked() {
                            self.cible = Some(Cible {
                                chemin: peripherique.chemin.clone(),
                                etiquette: peripherique.modele.clone(),
                                taille_octets: peripherique.taille_octets,
                                index_partition: 0,
                            });
                            self.nom_volume.clear();
                            self.phrase.clear();
                            self.phrase_confirmation.clear();
                            self.accepte_effacement = false;
                            self.changer_ecran(Ecran::Formatage);
                        }
                        ui.label(
                            egui::RichText::new("non formaté")
                                .color(theme::TEXTE_FAIBLE)
                                .small(),
                        );
                    }
                });
            });
        });
    }

    // ------- ecrans -------

    fn ecran_accueil(&mut self, ui: &mut egui::Ui) {
        self.analyser_peripheriques();
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        self.appliquer_transition(ui);
        // acces aux parametres en haut a droite
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui
                .button(egui::RichText::new("⚙").size(18.0))
                .on_hover_text("paramètres et mise à jour")
                .clicked()
            {
                self.changer_ecran(Ecran::Parametres);
            }
        });
        self.entete(ui);

        ui.vertical_centered(|ui| {
            ui.set_max_width(640.0);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("PÉRIPHÉRIQUES DÉTECTÉS")
                        .color(theme::TEXTE_FAIBLE)
                        .small()
                        .strong(),
                );
                if ui.small_button("⟳").on_hover_text("analyser maintenant").clicked() {
                    self.derniere_analyse = None;
                }
            });

            let peripheriques = self.peripheriques.clone();
            if peripheriques.is_empty() {
                theme::carte().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("🔌").size(26.0));
                        ui.label(
                            egui::RichText::new(
                                "branchez une clé usb — la détection est automatique",
                            )
                            .color(theme::TEXTE_FAIBLE),
                        );
                    });
                });
            }
            for peripherique in &peripheriques {
                self.carte_peripherique(ui, peripherique);
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("IMAGE DISQUE")
                    .color(theme::TEXTE_FAIBLE)
                    .small()
                    .strong(),
            );
            theme::carte().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🗄");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.chemin_image)
                            .hint_text("/chemin/vers/disque.img")
                            .desired_width(280.0),
                    );
                    ui.label("partition");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.index_image).desired_width(30.0),
                    );
                    if ui.button("Ouvrir").clicked() {
                        let index = self.index_image.parse::<usize>().unwrap_or(0);
                        let chemin = PathBuf::from(self.chemin_image.clone());
                        let taille =
                            std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
                        if taille == 0 {
                            self.signaler("image introuvable", true);
                        } else {
                            self.cible = Some(Cible {
                                etiquette: self.chemin_image.clone(),
                                chemin,
                                taille_octets: taille,
                                index_partition: index,
                            });
                            self.changer_ecran(Ecran::Deverrouillage);
                        }
                    }
                });
            });

            ui.add_space(8.0);
            self.barre_message(ui);
        });
    }

    fn ecran_deverrouillage(&mut self, ui: &mut egui::Ui) {
        self.appliquer_transition(ui);
        let Some(cible) = self.cible.clone() else {
            self.changer_ecran(Ecran::Accueil);
            return;
        };
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(430.0);
            // secousse horizontale en cas d'erreur
            let age_erreur = self.instant_message.elapsed().as_secs_f32();
            if self.message_erreur && age_erreur < 0.45 {
                ui.add_space(0.0);
                let secousse = ((0.45 - age_erreur) * 22.0) * (age_erreur * 55.0).sin();
                ui.horizontal(|ui| ui.add_space(10.0 + secousse));
                ui.ctx().request_repaint();
            }
            theme::carte().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("🔐").size(42.0));
                    ui.label(egui::RichText::new(&cible.etiquette).strong().size(17.0));
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}",
                            cible.chemin.display(),
                            theme::taille_lisible(cible.taille_octets)
                        ))
                        .color(theme::TEXTE_FAIBLE)
                        .small(),
                    );
                    ui.add_space(12.0);
                    let champ = ui.add(
                        egui::TextEdit::singleline(&mut self.phrase)
                            .password(true)
                            .hint_text("phrase secrète")
                            .desired_width(300.0),
                    );
                    // validation directe par la touche entree
                    if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.deverrouiller();
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(55.0);
                        if ui.button("← Retour").clicked() {
                            self.changer_ecran(Ecran::Accueil);
                        }
                        let bouton = egui::Button::new(
                            egui::RichText::new("Déverrouiller").strong(),
                        )
                        .fill(theme::ACCENT);
                        if ui.add(bouton).clicked() {
                            self.deverrouiller();
                        }
                    });
                    ui.add_space(6.0);
                    self.barre_message(ui);
                });
            });
        });
    }

    // solidite approximative de la phrase
    fn solidite_phrase(&self) -> (f32, &'static str, egui::Color32) {
        let longueur = self.phrase.chars().count();
        let varietes = [
            self.phrase.chars().any(|c| c.is_lowercase()),
            self.phrase.chars().any(|c| c.is_uppercase()),
            self.phrase.chars().any(|c| c.is_ascii_digit()),
            self.phrase.chars().any(|c| !c.is_alphanumeric()),
        ]
        .iter()
        .filter(|&&v| v)
        .count();
        let score = ((longueur as f32 / 16.0) * 0.7 + (varietes as f32 / 4.0) * 0.3).min(1.0);
        if score < 0.4 {
            (score, "faible", theme::DANGER)
        } else if score < 0.7 {
            (score, "moyenne", theme::OR)
        } else {
            (score, "solide", theme::SUCCES)
        }
    }

    fn ecran_formatage(&mut self, ui: &mut egui::Ui) {
        self.appliquer_transition(ui);
        let Some(cible) = self.cible.clone() else {
            self.changer_ecran(Ecran::Accueil);
            return;
        };

        // formatage en cours : attente du fil d'arriere plan
        if let Some(recepteur) = &self.formatage {
            match recepteur.try_recv() {
                Ok(Ok(())) => {
                    self.formatage = None;
                    self.phrase.clear();
                    self.phrase_confirmation.clear();
                    self.changer_ecran(Ecran::Deverrouillage);
                    self.signaler("volume créé — entrez votre phrase pour l'ouvrir", false);
                    return;
                }
                Ok(Err(e)) => {
                    self.formatage = None;
                    self.signaler(e, true);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ui.add_space(80.0);
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Spinner::new().size(42.0).color(theme::ACCENT));
                        ui.add_space(10.0);
                        ui.label("chiffrement du volume en cours…");
                        ui.label(
                            egui::RichText::new("cela peut prendre du temps sur un grand support")
                                .color(theme::TEXTE_FAIBLE)
                                .small(),
                        );
                    });
                    ui.ctx().request_repaint_after(Duration::from_millis(80));
                    return;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.formatage = None;
                    self.signaler("le formatage a été interrompu", true);
                }
            }
        }

        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(460.0);
            theme::carte().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("⚠").size(36.0).color(theme::OR));
                    ui.label(
                        egui::RichText::new(format!("Formater {}", cible.etiquette))
                            .strong()
                            .size(18.0),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} — tout le contenu actuel sera effacé",
                            cible.chemin.display(),
                            theme::taille_lisible(cible.taille_octets)
                        ))
                        .color(theme::TEXTE_FAIBLE)
                        .small(),
                    );
                });
                ui.add_space(12.0);
                ui.label("nom du volume");
                ui.add(
                    egui::TextEdit::singleline(&mut self.nom_volume)
                        .hint_text("ma_cle")
                        .desired_width(f32::INFINITY),
                );
                ui.label("phrase secrète");
                ui.add(
                    egui::TextEdit::singleline(&mut self.phrase)
                        .password(true)
                        .hint_text("au moins 8 caractères")
                        .desired_width(f32::INFINITY),
                );
                // jauge animee de solidite
                if !self.phrase.is_empty() {
                    let (score, texte, couleur) = self.solidite_phrase();
                    let progression = ui
                        .ctx()
                        .animate_value_with_time(egui::Id::new("solidite"), score, 0.25);
                    ui.add(
                        egui::ProgressBar::new(progression)
                            .desired_height(6.0)
                            .fill(couleur),
                    );
                    ui.label(
                        egui::RichText::new(format!("solidité : {texte}"))
                            .color(couleur)
                            .small(),
                    );
                }
                ui.label("confirmation");
                ui.add(
                    egui::TextEdit::singleline(&mut self.phrase_confirmation)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                ui.checkbox(
                    &mut self.accepte_effacement,
                    "je comprends que toutes les données seront effacées",
                );
                ui.add_space(10.0);

                let valide = !self.nom_volume.trim().is_empty()
                    && self.phrase.chars().count() >= 8
                    && self.phrase == self.phrase_confirmation
                    && self.accepte_effacement;
                ui.horizontal(|ui| {
                    if ui.button("← Annuler").clicked() {
                        self.changer_ecran(Ecran::Accueil);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let bouton =
                            egui::Button::new(egui::RichText::new("Formater et chiffrer").strong())
                                .fill(if valide { theme::DANGER } else { theme::CARTE_CLAIRE });
                        if ui.add_enabled(valide, bouton).clicked() {
                            let nom = self.nom_volume.trim().to_string();
                            self.nom_volume = nom;
                            self.lancer_formatage();
                        }
                    });
                });
                if !self.phrase.is_empty()
                    && !self.phrase_confirmation.is_empty()
                    && self.phrase != self.phrase_confirmation
                {
                    ui.label(
                        egui::RichText::new("les deux phrases ne correspondent pas")
                            .color(theme::DANGER)
                            .small(),
                    );
                }
                self.barre_message(ui);
            });
        });
    }

    fn ecran_parametres(&mut self, ui: &mut egui::Ui) {
        self.appliquer_transition(ui);

        // reception de la progression de mise a jour
        if let Some(recepteur) = &self.maj {
            loop {
                match recepteur.try_recv() {
                    Ok(EtapeMaj::Info(texte)) => self.journal_maj.push(texte),
                    Ok(EtapeMaj::Terminee(Ok(chemin_gui))) => {
                        self.journal_maj.push("redémarrage de l'application…".into());
                        mise_a_jour::relancer(&chemin_gui);
                    }
                    Ok(EtapeMaj::Terminee(Err(e))) => {
                        self.maj = None;
                        self.journal_maj.push(format!("✖ {e}"));
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.maj = None;
                        break;
                    }
                }
            }
            ui.ctx().request_repaint_after(Duration::from_millis(150));
        }

        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(540.0);
            ui.label(egui::RichText::new("⚙").size(38.0));
            ui.label(egui::RichText::new("Paramètres").size(24.0).strong());
            ui.add_space(10.0);

            theme::carte().show(ui, |ui| {
                ui.label(
                    egui::RichText::new("APPLICATION")
                        .color(theme::TEXTE_FAIBLE)
                        .small()
                        .strong(),
                );
                ui.horizontal(|ui| {
                    ui.label("version installée :");
                    ui.label(
                        egui::RichText::new(mise_a_jour::VERSION)
                            .color(theme::OR)
                            .strong(),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("dépôt officiel :");
                    ui.hyperlink(mise_a_jour::DEPOT);
                });
            });

            theme::carte().show(ui, |ui| {
                ui.label(
                    egui::RichText::new("MISE À JOUR")
                        .color(theme::TEXTE_FAIBLE)
                        .small()
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(
                        "télécharge la dernière version du dépôt, la recompile, \
                         l'installe et relance l'application automatiquement",
                    )
                    .color(theme::TEXTE_FAIBLE)
                    .small(),
                );
                ui.add_space(6.0);

                let en_cours = self.maj.is_some();
                ui.horizontal(|ui| {
                    let bouton =
                        egui::Button::new(egui::RichText::new("🔄 Mettre à jour").strong())
                            .fill(theme::ACCENT);
                    if ui.add_enabled(!en_cours, bouton).clicked() {
                        self.journal_maj.clear();
                        self.journal_maj.push("démarrage de la mise à jour…".into());
                        let (emetteur, recepteur) = mpsc::channel();
                        std::thread::spawn(move || mise_a_jour::mettre_a_jour(emetteur));
                        self.maj = Some(recepteur);
                    }
                    if en_cours {
                        ui.add(egui::Spinner::new().size(20.0).color(theme::ACCENT));
                    }
                });

                // journal de progression
                if !self.journal_maj.is_empty() {
                    ui.add_space(6.0);
                    egui::Frame::new()
                        .fill(theme::FOND)
                        .corner_radius(8)
                        .inner_margin(10)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("journal_maj")
                                .max_height(160.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for ligne in &self.journal_maj {
                                        let couleur = if ligne.starts_with('✖') {
                                            theme::DANGER
                                        } else {
                                            theme::TEXTE_FAIBLE
                                        };
                                        ui.label(
                                            egui::RichText::new(ligne)
                                                .color(couleur)
                                                .monospace()
                                                .small(),
                                        );
                                    }
                                });
                        });
                }
            });

            ui.add_space(6.0);
            if ui.button("← Retour").clicked() {
                self.changer_ecran(Ecran::Accueil);
            }
        });
    }

    fn fil_ariane(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("🏠").size(16.0))
                .on_hover_text("racine")
                .clicked()
            {
                self.chemin_courant = "/".to_string();
                self.actualiser();
            }
            let segments: Vec<String> = self
                .chemin_courant
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            let mut cumul = String::new();
            for segment in &segments {
                ui.label(egui::RichText::new("›").color(theme::TEXTE_FAIBLE));
                cumul.push('/');
                cumul.push_str(segment);
                if ui.button(segment).clicked() {
                    self.chemin_courant = cumul.clone();
                    self.actualiser();
                }
            }
        });
    }

    fn ecran_explorateur(&mut self, ui: &mut egui::Ui) {
        let Some(cible) = self.cible.clone() else {
            self.changer_ecran(Ecran::Accueil);
            return;
        };

        egui::Panel::top("barre_haute").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("👑").size(22.0));
                ui.label(
                    egui::RichText::new(&cible.etiquette)
                        .strong()
                        .color(theme::OR),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let bouton = egui::Button::new("🔒 Verrouiller").fill(theme::ACCENT_FONCE);
                    if ui.add(bouton).clicked() {
                        self.verrouiller();
                        return;
                    }
                    if let Some(session) = self.session.as_ref() {
                        let stats = session.statistiques();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} libres",
                                theme::taille_lisible(
                                    stats.blocs_libres * stats.taille_bloc as u64
                                )
                            ))
                            .color(theme::TEXTE_FAIBLE)
                            .small(),
                        );
                    }
                });
            });
            ui.add_space(4.0);
        });
        if self.session.is_none() {
            return;
        }

        egui::Panel::right("panneau_actions")
            .min_size(250.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.panneau_actions(ui);
                });
            });

        egui::Panel::bottom("barre_message").show(ui, |ui| {
            self.barre_message(ui);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            self.appliquer_transition(ui);
            self.fil_ariane(ui);
            ui.add_space(4.0);

            let entrees = self.entrees.clone();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if entrees.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("🗀").size(34.0));
                        ui.label(
                            egui::RichText::new("dossier vide").color(theme::TEXTE_FAIBLE),
                        );
                    });
                }
                for entree in &entrees {
                    let choisi = self.selection.as_deref()
                        == Some(self.chemin_de(&entree.nom).as_str());
                    let reponse = theme::carte()
                        .fill(if choisi {
                            theme::CARTE_CLAIRE
                        } else {
                            theme::CARTE
                        })
                        .inner_margin(10)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let icone = match entree.type_noeud {
                                    TypeNoeud::Dossier => "📁",
                                    _ => "📄",
                                };
                                ui.label(egui::RichText::new(icone).size(20.0));
                                ui.label(egui::RichText::new(&entree.nom).size(15.0));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if entree.type_noeud != TypeNoeud::Dossier {
                                            ui.label(
                                                egui::RichText::new(theme::taille_lisible(
                                                    entree.taille,
                                                ))
                                                .color(theme::TEXTE_FAIBLE)
                                                .small(),
                                            );
                                        }
                                    },
                                );
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    // survol : voile lumineux anime
                    let survol = ui.ctx().animate_bool_with_time(
                        reponse.id.with("survol"),
                        reponse.hovered(),
                        0.12,
                    );
                    if survol > 0.0 {
                        ui.painter().rect_filled(
                            reponse.rect,
                            14.0,
                            egui::Color32::from_white_alpha((survol * 7.0) as u8),
                        );
                    }
                    if reponse.clicked() {
                        self.selectionner(entree);
                    }
                }
            });

            if let Some(contenu) = self.contenu_texte.clone() {
                ui.separator();
                ui.label(
                    egui::RichText::new("APERÇU")
                        .color(theme::TEXTE_FAIBLE)
                        .small()
                        .strong(),
                );
                egui::ScrollArea::vertical()
                    .id_salt("apercu")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut contenu.as_str())
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
            }
        });
    }

    fn panneau_actions(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("CRÉER")
                .color(theme::TEXTE_FAIBLE)
                .small()
                .strong(),
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.nom_nouveau_dossier)
                    .hint_text("nom du dossier")
                    .desired_width(140.0),
            );
            if ui.button("📁+").on_hover_text("créer le dossier").clicked() {
                let chemin = self.chemin_de(&self.nom_nouveau_dossier.clone());
                if let Some(session) = self.session.as_mut() {
                    match session.creer_dossier(&chemin) {
                        Ok(()) => {
                            self.nom_nouveau_dossier.clear();
                            self.signaler(format!("dossier créé : {chemin}"), false);
                        }
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                }
                self.actualiser();
            }
        });

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new("IMPORTER")
                .color(theme::TEXTE_FAIBLE)
                .small()
                .strong(),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.chemin_import)
                .hint_text("/chemin/hote/fichier")
                .desired_width(f32::INFINITY),
        );
        if ui.button("⤵ Importer ici").clicked() {
            let source = self.chemin_import.clone();
            let nom = Path::new(&source)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let destination = self.chemin_de(&nom);
            if let Some(session) = self.session.as_mut() {
                match session.importer(Path::new(&source), &destination) {
                    Ok(()) => self.signaler(format!("importé : {destination}"), false),
                    Err(e) => self.signaler(format!("{e}"), true),
                }
            }
            self.actualiser();
        }

        if let Some(chemin_selection) = self.selection.clone() {
            ui.add_space(12.0);
            ui.separator();
            ui.label(
                egui::RichText::new("SÉLECTION")
                    .color(theme::TEXTE_FAIBLE)
                    .small()
                    .strong(),
            );
            ui.label(egui::RichText::new(&chemin_selection).small());

            ui.add(
                egui::TextEdit::singleline(&mut self.chemin_export)
                    .hint_text("/chemin/hote/sortie")
                    .desired_width(f32::INFINITY),
            );
            if ui.button("⤴ Exporter").clicked() {
                let destination = self.chemin_export.clone();
                if let Some(session) = self.session.as_mut() {
                    match session.exporter(&chemin_selection, Path::new(&destination)) {
                        Ok(()) => self.signaler(format!("exporté vers {destination}"), false),
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                }
            }

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.nouveau_nom).desired_width(130.0),
                );
                if ui.button("✏").on_hover_text("renommer").clicked() {
                    let nom = self.nouveau_nom.clone();
                    if let Some(session) = self.session.as_mut() {
                        match session.renommer(&chemin_selection, &nom) {
                            Ok(()) => self.signaler(format!("renommé en {nom}"), false),
                            Err(e) => self.signaler(format!("{e}"), true),
                        }
                    }
                    self.actualiser();
                }
            });

            let bouton_supprimer =
                egui::Button::new(egui::RichText::new("🗑 Supprimer").color(theme::TEXTE))
                    .fill(theme::DANGER.gamma_multiply(0.55));
            if ui.add(bouton_supprimer).clicked() {
                if let Some(session) = self.session.as_mut() {
                    match session.supprimer(&chemin_selection) {
                        Ok(()) => self.signaler(format!("supprimé : {chemin_selection}"), false),
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                }
                self.actualiser();
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("MÉTADONNÉES")
                    .color(theme::TEXTE_FAIBLE)
                    .small()
                    .strong(),
            );
            for (cle, valeur) in self.metas.clone() {
                ui.label(
                    egui::RichText::new(format!("{cle} = {valeur}"))
                        .color(theme::OR)
                        .small(),
                );
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.meta_cle)
                        .hint_text("clé")
                        .desired_width(70.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.meta_valeur)
                        .hint_text("valeur")
                        .desired_width(90.0),
                );
                if ui.button("+").clicked() {
                    let (cle, valeur) = (self.meta_cle.clone(), self.meta_valeur.clone());
                    if let Some(session) = self.session.as_mut() {
                        match session.definir_meta(&chemin_selection, &cle, &valeur) {
                            Ok(()) => {
                                self.metas =
                                    session.lire_metas(&chemin_selection).unwrap_or_default();
                                self.meta_cle.clear();
                                self.meta_valeur.clear();
                            }
                            Err(e) => self.signaler(format!("{e}"), true),
                        }
                    }
                }
            });
        }
    }
}

impl eframe::App for ApplicationMonarque {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.theme_applique {
            theme::appliquer(ui.ctx());
            self.theme_applique = true;
        }
        match self.ecran {
            Ecran::Accueil => {
                egui::CentralPanel::default().show(ui, |ui| self.ecran_accueil(ui));
            }
            Ecran::Deverrouillage => {
                egui::CentralPanel::default().show(ui, |ui| self.ecran_deverrouillage(ui));
            }
            Ecran::Formatage => {
                egui::CentralPanel::default().show(ui, |ui| self.ecran_formatage(ui));
            }
            Ecran::Parametres => {
                egui::CentralPanel::default().show(ui, |ui| self.ecran_parametres(ui));
            }
            Ecran::Explorateur => self.ecran_explorateur(ui),
        }
    }
}
