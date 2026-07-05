// application graphique du gestionnaire de fichiers

use eframe::egui;
use gestionnaire_fichiers::{lister_partitions, InfoEntree, InfoPartition, Session, TypeNoeud};
use std::path::Path;

// ecran affiche
enum Ecran {
    Connexion,
    Explorateur,
}

pub struct ApplicationMonarque {
    ecran: Ecran,
    session: Option<Session>,
    // champs de connexion
    chemin_image: String,
    index_partition: String,
    phrase: String,
    partitions: Vec<InfoPartition>,
    // etat de l'explorateur
    chemin_courant: String,
    entrees: Vec<InfoEntree>,
    selection: Option<String>,
    contenu_texte: Option<String>,
    metas: Vec<(String, String)>,
    // champs de saisie
    nom_nouveau_dossier: String,
    chemin_import: String,
    chemin_export: String,
    nouveau_nom: String,
    meta_cle: String,
    meta_valeur: String,
    // message d'etat ou d'erreur
    message: String,
}

impl Default for ApplicationMonarque {
    fn default() -> Self {
        Self {
            ecran: Ecran::Connexion,
            session: None,
            chemin_image: String::new(),
            index_partition: "0".to_string(),
            phrase: String::new(),
            partitions: Vec::new(),
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
        }
    }
}

impl ApplicationMonarque {
    // chemin complet d'une entree du dossier courant
    fn chemin_de(&self, nom: &str) -> String {
        if self.chemin_courant == "/" {
            format!("/{nom}")
        } else {
            format!("{}/{nom}", self.chemin_courant)
        }
    }

    // rafraichissement de la liste du dossier courant
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
            Err(e) => self.message = format!("erreur : {e}"),
        }
    }

    // ouverture de session avec les champs saisis
    fn connecter(&mut self) {
        let Ok(index) = self.index_partition.parse::<usize>() else {
            self.message = "index de partition invalide".to_string();
            return;
        };
        match Session::ouvrir(Path::new(&self.chemin_image), index, &self.phrase) {
            Ok(session) => {
                self.session = Some(session);
                self.ecran = Ecran::Explorateur;
                self.chemin_courant = "/".to_string();
                self.message.clear();
                self.actualiser();
            }
            Err(e) => self.message = format!("erreur : {e}"),
        }
    }

    // fermeture propre de la session
    fn deconnecter(&mut self) {
        if let Some(session) = self.session.take() {
            if let Err(e) = session.fermer() {
                self.message = format!("erreur a la fermeture : {e}");
            }
        }
        self.ecran = Ecran::Connexion;
        self.phrase.clear();
    }

    // selection d'une entree et chargement de ses details
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
            // apercu texte pour les petits fichiers
            self.contenu_texte = match session.lire_fichier(&chemin) {
                Ok(donnees) if donnees.len() <= 64 * 1024 => {
                    Some(String::from_utf8_lossy(&donnees).into_owned())
                }
                Ok(_) => Some("(fichier trop grand pour l'apercu)".to_string()),
                Err(e) => Some(format!("erreur de lecture : {e}")),
            };
        }
    }

    // remontee au dossier parent
    fn remonter(&mut self) {
        if self.chemin_courant == "/" {
            return;
        }
        let position = self.chemin_courant.rfind('/').unwrap();
        self.chemin_courant = if position == 0 {
            "/".to_string()
        } else {
            self.chemin_courant[..position].to_string()
        };
        self.actualiser();
    }

    // ------- panneaux -------

    fn panneau_connexion(&mut self, ui: &mut egui::Ui) {
        ui.heading("MonarqueFS — connexion a un volume");
        ui.add_space(12.0);
        egui::Grid::new("grille_connexion")
            .num_columns(2)
            .spacing([8.0, 8.0])
            .show(ui, |ui| {
                ui.label("image disque");
                ui.add(
                    egui::TextEdit::singleline(&mut self.chemin_image)
                        .hint_text("/chemin/vers/disque.img")
                        .desired_width(360.0),
                );
                ui.end_row();
                ui.label("partition");
                ui.add(
                    egui::TextEdit::singleline(&mut self.index_partition).desired_width(60.0),
                );
                ui.end_row();
                ui.label("phrase secrete");
                ui.add(
                    egui::TextEdit::singleline(&mut self.phrase)
                        .password(true)
                        .desired_width(360.0),
                );
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("lire la table de partition").clicked() {
                match lister_partitions(Path::new(&self.chemin_image)) {
                    Ok(partitions) => {
                        self.partitions = partitions;
                        self.message.clear();
                    }
                    Err(e) => self.message = format!("erreur : {e}"),
                }
            }
            if ui.button("ouvrir le volume").clicked() {
                self.connecter();
            }
        });
        if !self.partitions.is_empty() {
            ui.add_space(8.0);
            ui.label("partitions disponibles :");
            for p in &self.partitions.clone() {
                let texte = format!(
                    "[{}] {} — {:.1} Mo",
                    p.index,
                    p.nom,
                    p.taille_octets as f64 / (1024.0 * 1024.0)
                );
                if ui.selectable_label(false, texte).clicked() {
                    self.index_partition = p.index.to_string();
                }
            }
        }
    }

    fn panneau_actions(&mut self, ui: &mut egui::Ui) {
        ui.heading("actions");
        ui.separator();

        // bouton de creation de dossier
        ui.label("nouveau dossier");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.nom_nouveau_dossier).desired_width(140.0),
            );
            if ui.button("creer").clicked() {
                let chemin = self.chemin_de(&self.nom_nouveau_dossier.clone());
                if let Some(session) = self.session.as_mut() {
                    match session.creer_dossier(&chemin) {
                        Ok(()) => {
                            self.message = format!("dossier cree : {chemin}");
                            self.nom_nouveau_dossier.clear();
                        }
                        Err(e) => self.message = format!("erreur : {e}"),
                    }
                }
                self.actualiser();
            }
        });
        ui.add_space(8.0);

        // bouton d'import depuis l'hote
        ui.label("importer un fichier hote");
        ui.add(
            egui::TextEdit::singleline(&mut self.chemin_import)
                .hint_text("/chemin/hote/fichier")
                .desired_width(200.0),
        );
        if ui.button("importer ici").clicked() {
            let source = self.chemin_import.clone();
            let nom = Path::new(&source)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let destination = self.chemin_de(&nom);
            if let Some(session) = self.session.as_mut() {
                match session.importer(Path::new(&source), &destination) {
                    Ok(()) => self.message = format!("importe : {destination}"),
                    Err(e) => self.message = format!("erreur : {e}"),
                }
            }
            self.actualiser();
        }

        // actions sur la selection
        if let Some(chemin_selection) = self.selection.clone() {
            ui.add_space(12.0);
            ui.separator();
            ui.label(format!("selection : {chemin_selection}"));

            // bouton d'export vers l'hote
            ui.add(
                egui::TextEdit::singleline(&mut self.chemin_export)
                    .hint_text("/chemin/hote/sortie")
                    .desired_width(200.0),
            );
            if ui.button("exporter").clicked() {
                let destination = self.chemin_export.clone();
                if let Some(session) = self.session.as_mut() {
                    match session.exporter(&chemin_selection, Path::new(&destination)) {
                        Ok(()) => self.message = format!("exporte vers {destination}"),
                        Err(e) => self.message = format!("erreur : {e}"),
                    }
                }
            }

            // bouton de renommage
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.nouveau_nom).desired_width(140.0));
                if ui.button("renommer").clicked() {
                    let nom = self.nouveau_nom.clone();
                    if let Some(session) = self.session.as_mut() {
                        match session.renommer(&chemin_selection, &nom) {
                            Ok(()) => self.message = format!("renomme en {nom}"),
                            Err(e) => self.message = format!("erreur : {e}"),
                        }
                    }
                    self.actualiser();
                }
            });

            // bouton de suppression
            if ui.button("supprimer").clicked() {
                if let Some(session) = self.session.as_mut() {
                    match session.supprimer(&chemin_selection) {
                        Ok(()) => self.message = format!("supprime : {chemin_selection}"),
                        Err(e) => self.message = format!("erreur : {e}"),
                    }
                }
                self.actualiser();
            }

            // metadonnees etendues
            ui.add_space(8.0);
            ui.label("metadonnees");
            for (cle, valeur) in self.metas.clone() {
                ui.label(format!("  {cle} = {valeur}"));
            }
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.meta_cle)
                        .hint_text("cle")
                        .desired_width(70.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.meta_valeur)
                        .hint_text("valeur")
                        .desired_width(90.0),
                );
                if ui.button("definir").clicked() {
                    let (cle, valeur) = (self.meta_cle.clone(), self.meta_valeur.clone());
                    if let Some(session) = self.session.as_mut() {
                        match session.definir_meta(&chemin_selection, &cle, &valeur) {
                            Ok(()) => {
                                self.metas = session
                                    .lire_metas(&chemin_selection)
                                    .unwrap_or_default();
                                self.meta_cle.clear();
                                self.meta_valeur.clear();
                            }
                            Err(e) => self.message = format!("erreur : {e}"),
                        }
                    }
                }
            });
        }
    }

    fn panneau_explorateur(&mut self, ui: &mut egui::Ui) {
        // barre de navigation
        ui.horizontal(|ui| {
            if ui.button("⬆ remonter").clicked() {
                self.remonter();
            }
            if ui.button("⟳ actualiser").clicked() {
                self.actualiser();
            }
            ui.label(egui::RichText::new(&self.chemin_courant).monospace().strong());
        });
        ui.separator();

        // liste des entrees du dossier courant
        let entrees = self.entrees.clone();
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                if entrees.is_empty() {
                    ui.label("(dossier vide)");
                }
                for entree in &entrees {
                    let icone = match entree.type_noeud {
                        TypeNoeud::Dossier => "📁",
                        _ => "📄",
                    };
                    let taille = if entree.type_noeud == TypeNoeud::Dossier {
                        String::new()
                    } else {
                        format!("  ({} o)", entree.taille)
                    };
                    let texte = format!("{icone} {}{taille}", entree.nom);
                    let choisi =
                        self.selection.as_deref() == Some(self.chemin_de(&entree.nom).as_str());
                    if ui.selectable_label(choisi, texte).clicked() {
                        self.selectionner(entree);
                    }
                }
            });

        // apercu du fichier selectionne
        if let Some(contenu) = self.contenu_texte.clone() {
            ui.separator();
            ui.label("apercu :");
            egui::ScrollArea::vertical()
                .id_salt("apercu")
                .max_height(160.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut contenu.as_str())
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        }
    }
}

impl eframe::App for ApplicationMonarque {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.ecran {
            Ecran::Connexion => {
                egui::CentralPanel::default().show(ui, |ui| {
                    self.panneau_connexion(ui);
                    if !self.message.is_empty() {
                        ui.add_space(12.0);
                        ui.colored_label(egui::Color32::LIGHT_RED, &self.message);
                    }
                });
            }
            Ecran::Explorateur => {
                egui::Panel::top("barre_haute").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("MonarqueFS");
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button("fermer le volume").clicked() {
                                    self.deconnecter();
                                }
                                if let Some(session) = self.session.as_ref() {
                                    let stats = session.statistiques();
                                    ui.label(format!(
                                        "{} blocs libres / {}",
                                        stats.blocs_libres, stats.nb_blocs_donnees
                                    ));
                                }
                            },
                        );
                    });
                });
                egui::Panel::right("panneau_actions")
                    .min_size(240.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.panneau_actions(ui);
                        });
                    });
                egui::Panel::bottom("barre_message").show(ui, |ui| {
                    ui.label(&self.message);
                });
                egui::CentralPanel::default().show(ui, |ui| {
                    self.panneau_explorateur(ui);
                });
            }
        }
    }
}
