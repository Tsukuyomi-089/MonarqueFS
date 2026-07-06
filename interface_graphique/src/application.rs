// application graphique du gestionnaire de fichiers monarque

use crate::gestionnaire::Gestionnaire;
use crate::theme;
use eframe::egui;
use gestionnaire_fichiers::mise_a_jour::{self, EtapeMaj};
use gestionnaire_fichiers::{
    autoriser_peripheriques, lister_peripheriques, preparer_support, ArbreVolume,
    InfoPeripherique, Session,
};
use std::path::PathBuf;
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
    Parametres,
    Explorateur,
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
    instant_ecran: Instant,
    // detection des peripheriques
    peripheriques: Vec<InfoPeripherique>,
    derniere_analyse: Option<Instant>,
    // support en cours
    cible: Option<Cible>,
    gestionnaire: Option<Gestionnaire>,
    // saisies de connexion et de formatage
    phrase: String,
    phrase_confirmation: String,
    nom_volume: String,
    accepte_effacement: bool,
    chemin_image: String,
    index_image: String,
    // taches en arriere plan
    formatage: Option<mpsc::Receiver<Result<(), String>>>,
    autorisation: Option<mpsc::Receiver<Result<(), String>>>,
    maj: Option<mpsc::Receiver<EtapeMaj>>,
    journal_maj: Vec<String>,
    // barre de message
    message: String,
    message_erreur: bool,
    instant_message: Instant,
}

impl ApplicationMonarque {
    pub fn nouvelle(preselection: Option<PathBuf>) -> Self {
        let mut application = Self {
            ecran: Ecran::Accueil,
            theme_applique: false,
            instant_ecran: Instant::now(),
            peripheriques: Vec::new(),
            derniere_analyse: None,
            cible: None,
            gestionnaire: None,
            phrase: String::new(),
            phrase_confirmation: String::new(),
            nom_volume: String::new(),
            accepte_effacement: false,
            chemin_image: String::new(),
            index_image: "0".to_string(),
            formatage: None,
            autorisation: None,
            maj: None,
            journal_maj: Vec::new(),
            message: String::new(),
            message_erreur: false,
            instant_message: Instant::now(),
        };
        // lancement par le demon : ouverture directe du deverrouillage
        if let Some(chemin) = preselection {
            let taille = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
            application.cible = Some(Cible {
                etiquette: chemin
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| chemin.display().to_string()),
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
                let arbre = ArbreVolume::nouveau(session, cible.etiquette.clone());
                self.gestionnaire = Some(Gestionnaire::nouveau(arbre));
                self.phrase.clear();
                self.changer_ecran(Ecran::Explorateur);
            }
            Err(e) => self.signaler(format!("{e}"), true),
        }
    }

    fn verrouiller(&mut self) {
        if let Some(gestionnaire) = self.gestionnaire.take() {
            if let Err(e) = gestionnaire.liberer() {
                self.signaler(format!("erreur a la fermeture : {e}"), true);
            }
        }
        self.changer_ecran(Ecran::Accueil);
    }

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

    // demande d'autorisation d'acces via une fenetre graphique
    fn lancer_autorisation(&mut self) {
        let (emetteur, recepteur) = mpsc::channel();
        std::thread::spawn(move || {
            emetteur.send(autoriser_peripheriques()).ok();
        });
        self.autorisation = Some(recepteur);
        self.signaler("fenêtre d'autorisation ouverte…", false);
    }

    // ------- composants visuels -------

    fn appliquer_transition(&self, ui: &mut egui::Ui) -> f32 {
        let t = theme::adoucir(self.instant_ecran.elapsed().as_secs_f32() / DUREE_TRANSITION);
        ui.set_opacity(t);
        ui.add_space((1.0 - t) * 18.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        }
        t
    }

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

    fn carte_peripherique(&mut self, ui: &mut egui::Ui, peripherique: &InfoPeripherique) {
        let temps = ui.input(|i| i.time);
        theme::carte().show(ui, |ui| {
            ui.horizontal(|ui| {
                let icone = if peripherique.est_systeme {
                    "🖥"
                } else if peripherique.amovible {
                    "🔑"
                } else {
                    "💽"
                };
                ui.label(egui::RichText::new(icone).size(30.0));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&peripherique.modele).strong().size(16.0));
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
                    // disque systeme : protege, aucune action destructive
                    if peripherique.est_systeme {
                        ui.label(
                            egui::RichText::new("🔒 disque système protégé")
                                .color(theme::TEXTE_FAIBLE)
                                .small(),
                        );
                        return;
                    }
                    // acces refuse : proposer l'autorisation graphique
                    if !peripherique.accessible {
                        let bouton = egui::Button::new(
                            egui::RichText::new("Autoriser l'accès").color(theme::TEXTE),
                        )
                        .fill(theme::ACCENT);
                        if ui.add_enabled(self.autorisation.is_none(), bouton).clicked() {
                            self.lancer_autorisation();
                        }
                        return;
                    }
                    if peripherique.est_monarque {
                        let pulsation = 0.6 + 0.4 * ((temps * 2.4).sin() * 0.5 + 0.5) as f32;
                        if ui
                            .button(egui::RichText::new("🔓 Déverrouiller").color(theme::TEXTE))
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
        // reception du resultat d'autorisation
        if let Some(recepteur) = &self.autorisation {
            if let Ok(resultat) = recepteur.try_recv() {
                match resultat {
                    Ok(()) => {
                        self.signaler("accès autorisé — analyse des périphériques", false);
                        self.derniere_analyse = None;
                    }
                    Err(e) => self.signaler(e, true),
                }
                self.autorisation = None;
            }
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        self.appliquer_transition(ui);
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
            ui.set_max_width(660.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("PÉRIPHÉRIQUES DÉTECTÉS")
                        .color(theme::TEXTE_FAIBLE)
                        .small()
                        .strong(),
                );
                if ui.small_button("⟳").on_hover_text("analyser").clicked() {
                    self.derniere_analyse = None;
                }
            });

            let peripheriques = self.peripheriques.clone();
            if peripheriques.is_empty() {
                theme::carte().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("🔌").size(26.0));
                        ui.label(
                            egui::RichText::new("branchez une clé usb — détection automatique")
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
                    ui.add(egui::TextEdit::singleline(&mut self.index_image).desired_width(30.0));
                    if ui.button("Ouvrir").clicked() {
                        let index = self.index_image.parse::<usize>().unwrap_or(0);
                        let chemin = PathBuf::from(self.chemin_image.clone());
                        let taille = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
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
            let age_erreur = self.instant_message.elapsed().as_secs_f32();
            if self.message_erreur && age_erreur < 0.45 {
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
                    if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.deverrouiller();
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(55.0);
                        if ui.button("← Retour").clicked() {
                            self.changer_ecran(Ecran::Accueil);
                        }
                        let bouton =
                            egui::Button::new(egui::RichText::new("Déverrouiller").strong())
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
                            self.nom_volume = self.nom_volume.trim().to_string();
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
                    ui.label(egui::RichText::new(mise_a_jour::VERSION).color(theme::OR).strong());
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
                        "télécharge la dernière version, la recompile, l'installe \
                         et relance l'application automatiquement",
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

    fn ecran_explorateur(&mut self, ui: &mut egui::Ui) {
        if self.gestionnaire.is_none() {
            self.changer_ecran(Ecran::Accueil);
            return;
        }
        egui::Panel::bottom("barre_message_gest").show(ui, |ui| {
            if let Some(g) = &self.gestionnaire {
                if !g.message.is_empty() {
                    let couleur = if g.erreur { theme::DANGER } else { theme::SUCCES };
                    let prefixe = if g.erreur { "✖" } else { "✔" };
                    ui.colored_label(couleur, format!("{prefixe} {}", g.message));
                }
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let mut fermer = false;
            if let Some(gestionnaire) = &mut self.gestionnaire {
                gestionnaire.afficher(ui);
                fermer = gestionnaire.fermeture_demandee;
            }
            if fermer {
                self.verrouiller();
            }
        });
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
