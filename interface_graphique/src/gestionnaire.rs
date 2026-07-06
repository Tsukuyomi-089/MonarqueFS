// gestionnaire de fichiers dans le style dolphin (kde)
// panneau lateral, navigation historique, vues details/icones, presse-papiers

use crate::theme;
use eframe::egui;
use gestionnaire_fichiers::arborescence::{
    copier, emplacements_hote, Arborescence, EntreeArbre,
};
use gestionnaire_fichiers::{ArbreHote, ArbreVolume, ResultatFs, Session};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// source affichee par un panneau
#[derive(Clone, Copy, PartialEq)]
pub enum Cote {
    Hote,
    Volume,
}

// mode d'affichage des entrees
#[derive(Clone, Copy, PartialEq)]
enum ModeVue {
    Details,
    Icones,
    Compact,
}

// colonne de tri
#[derive(Clone, Copy, PartialEq)]
enum Tri {
    Nom,
    Taille,
    Date,
}

// position de navigation
#[derive(Clone, PartialEq)]
struct Position {
    cote: Cote,
    chemin: String,
}

// etat d'un panneau de navigation
struct Vue {
    position: Position,
    historique: Vec<Position>,
    index_hist: usize,
    entrees: Vec<EntreeArbre>,
    selection: HashSet<String>,
    mode: ModeVue,
    tri: Tri,
    tri_inverse: bool,
}

impl Vue {
    fn nouvelle(position: Position) -> Self {
        Self {
            historique: vec![position.clone()],
            index_hist: 0,
            position,
            entrees: Vec::new(),
            selection: HashSet::new(),
            mode: ModeVue::Details,
            tri: Tri::Nom,
            tri_inverse: false,
        }
    }
}

// contenu du presse-papiers
struct PressePapier {
    position: Position,
    noms: Vec<String>,
    couper: bool,
}

// renommage en ligne
struct Renommage {
    vue: usize,
    ancien: String,
    saisie: String,
    focalise: bool,
}

// fiche de proprietes
struct Proprietes {
    nom: String,
    emplacement: String,
    est_dossier: bool,
    taille: u64,
    modifie: u64,
    metas: Vec<(String, String)>,
}

// actions differees pour eviter les conflits d'emprunt
enum Action {
    Activer(usize),
    Naviguer(usize, Position),
    Retour(usize),
    Avancer(usize),
    Parent(usize),
    Ouvrir(usize, String),
    SelectionSimple(usize, String),
    SelectionCtrl(usize, String),
    Mode(usize, ModeVue),
    Trier(usize, Tri),
    Copier(usize),
    Couper(usize),
    Coller(usize),
    DemanderSuppression(usize),
    DemanderRenommage(usize),
    NouveauDossier(usize),
    Proprietes(usize, String),
    Actualiser(usize),
    BasculerScission,
}

pub struct Gestionnaire {
    hote: ArbreHote,
    volume: ArbreVolume,
    vues: Vec<Vue>,
    active: usize,
    scission: bool,
    emplacements: Vec<(String, String, PathBuf)>,
    presse_papier: Option<PressePapier>,
    filtre: String,
    renommage: Option<Renommage>,
    proprietes: Option<Proprietes>,
    apercu: Option<(String, String)>,
    suppression: Option<Vec<String>>,
    saisie_dossier: String,
    creation_dossier: bool,
    pub message: String,
    pub erreur: bool,
    pub fermeture_demandee: bool,
}

impl Gestionnaire {
    pub fn nouveau(volume: ArbreVolume) -> Self {
        let depart_hote = Position {
            cote: Cote::Hote,
            chemin: std::env::var("HOME").unwrap_or_else(|_| "/".into()),
        };
        let depart_volume = Position {
            cote: Cote::Volume,
            chemin: "/".into(),
        };
        let mut g = Self {
            hote: ArbreHote::nouveau(),
            volume,
            vues: vec![Vue::nouvelle(depart_hote), Vue::nouvelle(depart_volume)],
            active: 1,
            scission: true,
            emplacements: emplacements_hote(),
            presse_papier: None,
            filtre: String::new(),
            renommage: None,
            proprietes: None,
            apercu: None,
            suppression: None,
            saisie_dossier: String::new(),
            creation_dossier: false,
            message: String::new(),
            erreur: false,
            fermeture_demandee: false,
        };
        g.rafraichir(0);
        g.rafraichir(1);
        g
    }

    pub fn liberer(self) -> ResultatFs<()> {
        self.volume.fermer()
    }

    fn signaler(&mut self, texte: impl Into<String>, erreur: bool) {
        self.message = texte.into();
        self.erreur = erreur;
    }

    // arbre positionne sur la vue demandee
    fn arbre_positionne(&mut self, i: usize) -> &mut dyn Arborescence {
        let position = self.vues[i].position.clone();
        let arbre: &mut dyn Arborescence = match position.cote {
            Cote::Hote => &mut self.hote,
            Cote::Volume => &mut self.volume,
        };
        arbre.positionner(&position.chemin).ok();
        arbre
    }

    // rechargement des entrees d'une vue avec tri
    fn rafraichir(&mut self, i: usize) {
        let resultat = self.arbre_positionne(i).lister();
        let vue = &mut self.vues[i];
        match resultat {
            Ok(mut entrees) => {
                let (tri, inverse) = (vue.tri, vue.tri_inverse);
                entrees.sort_by(|a, b| {
                    // dossiers toujours en tete
                    let ordre = b.est_dossier.cmp(&a.est_dossier).then_with(|| match tri {
                        Tri::Nom => a.nom.to_lowercase().cmp(&b.nom.to_lowercase()),
                        Tri::Taille => a.taille.cmp(&b.taille),
                        Tri::Date => a.modifie.cmp(&b.modifie),
                    });
                    if inverse && !ordre.is_eq() && a.est_dossier == b.est_dossier {
                        ordre.reverse()
                    } else {
                        ordre
                    }
                });
                vue.entrees = entrees;
            }
            Err(e) => {
                vue.entrees.clear();
                self.signaler(format!("{e}"), true);
            }
        }
        self.vues[i].selection.clear();
    }

    // navigation avec historique
    fn naviguer(&mut self, i: usize, position: Position) {
        let vue = &mut self.vues[i];
        if vue.position == position {
            return;
        }
        vue.historique.truncate(vue.index_hist + 1);
        vue.historique.push(position.clone());
        vue.index_hist = vue.historique.len() - 1;
        vue.position = position;
        self.rafraichir(i);
    }

    fn appliquer(&mut self, action: Action) {
        match action {
            Action::Activer(i) => self.active = i,
            Action::Naviguer(i, position) => self.naviguer(i, position),
            Action::Retour(i) => {
                let vue = &mut self.vues[i];
                if vue.index_hist > 0 {
                    vue.index_hist -= 1;
                    vue.position = vue.historique[vue.index_hist].clone();
                    self.rafraichir(i);
                }
            }
            Action::Avancer(i) => {
                let vue = &mut self.vues[i];
                if vue.index_hist + 1 < vue.historique.len() {
                    vue.index_hist += 1;
                    vue.position = vue.historique[vue.index_hist].clone();
                    self.rafraichir(i);
                }
            }
            Action::Parent(i) => {
                let position = self.vues[i].position.clone();
                let parent = match position.cote {
                    Cote::Hote => Path::new(&position.chemin)
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "/".into()),
                    Cote::Volume => {
                        let pos = position.chemin.rfind('/').unwrap_or(0);
                        if pos == 0 {
                            "/".into()
                        } else {
                            position.chemin[..pos].to_string()
                        }
                    }
                };
                self.naviguer(
                    i,
                    Position {
                        cote: position.cote,
                        chemin: parent,
                    },
                );
            }
            Action::Ouvrir(i, nom) => self.ouvrir(i, &nom),
            Action::SelectionSimple(i, nom) => {
                self.active = i;
                let vue = &mut self.vues[i];
                vue.selection.clear();
                vue.selection.insert(nom);
            }
            Action::SelectionCtrl(i, nom) => {
                self.active = i;
                let vue = &mut self.vues[i];
                if !vue.selection.remove(&nom) {
                    vue.selection.insert(nom);
                }
            }
            Action::Mode(i, mode) => self.vues[i].mode = mode,
            Action::Trier(i, tri) => {
                let vue = &mut self.vues[i];
                if vue.tri == tri {
                    vue.tri_inverse = !vue.tri_inverse;
                } else {
                    vue.tri = tri;
                    vue.tri_inverse = false;
                }
                self.rafraichir(i);
            }
            Action::Copier(i) => self.mettre_presse_papier(i, false),
            Action::Couper(i) => self.mettre_presse_papier(i, true),
            Action::Coller(i) => self.coller(i),
            Action::DemanderSuppression(i) => {
                self.active = i;
                let noms: Vec<String> = self.vues[i].selection.iter().cloned().collect();
                if !noms.is_empty() {
                    self.suppression = Some(noms);
                }
            }
            Action::DemanderRenommage(i) => {
                if let Some(nom) = self.vues[i].selection.iter().next().cloned() {
                    self.renommage = Some(Renommage {
                        vue: i,
                        saisie: nom.clone(),
                        ancien: nom,
                        focalise: false,
                    });
                }
            }
            Action::NouveauDossier(i) => {
                let nom = self.saisie_dossier.trim().to_string();
                if !nom.is_empty() {
                    match self.arbre_positionne(i).creer_dossier(&nom) {
                        Ok(()) => self.signaler(format!("dossier créé : {nom}"), false),
                        Err(e) => self.signaler(format!("{e}"), true),
                    }
                    self.rafraichir(i);
                }
                self.saisie_dossier.clear();
                self.creation_dossier = false;
            }
            Action::Proprietes(i, nom) => self.charger_proprietes(i, &nom),
            Action::Actualiser(i) => self.rafraichir(i),
            Action::BasculerScission => {
                self.scission = !self.scission;
                if !self.scission {
                    self.active = 0;
                }
            }
        }
    }

    // ouverture : dossier -> navigation ; fichier hote -> application systeme ;
    // fichier du volume -> apercu dechiffre en memoire uniquement
    fn ouvrir(&mut self, i: usize, nom: &str) {
        let position = self.vues[i].position.clone();
        let est_dossier = self.arbre_positionne(i).est_dossier(nom);
        if est_dossier {
            let chemin = match position.cote {
                Cote::Hote => Path::new(&position.chemin).join(nom).display().to_string(),
                Cote::Volume => {
                    if position.chemin == "/" {
                        format!("/{nom}")
                    } else {
                        format!("{}/{nom}", position.chemin)
                    }
                }
            };
            self.naviguer(
                i,
                Position {
                    cote: position.cote,
                    chemin,
                },
            );
            return;
        }
        match position.cote {
            Cote::Hote => {
                let chemin = Path::new(&position.chemin).join(nom);
                if std::process::Command::new("xdg-open").arg(&chemin).spawn().is_err() {
                    self.signaler("impossible d'ouvrir le fichier", true);
                }
            }
            Cote::Volume => {
                // dechiffrement en memoire, jamais de copie en clair sur disque
                let mut contenu = Vec::new();
                let resultat = self.arbre_positionne(i).lire_flux(nom, &mut contenu);
                match resultat {
                    Ok(()) if contenu.len() <= 512 * 1024 => {
                        self.apercu = Some((
                            nom.to_string(),
                            String::from_utf8_lossy(&contenu).into_owned(),
                        ));
                    }
                    Ok(()) => self.signaler(
                        "fichier trop grand pour l'aperçu — copiez-le vers le PC pour l'ouvrir",
                        true,
                    ),
                    Err(e) => self.signaler(format!("{e}"), true),
                }
            }
        }
    }

    fn mettre_presse_papier(&mut self, i: usize, couper: bool) {
        let noms: Vec<String> = self.vues[i].selection.iter().cloned().collect();
        if noms.is_empty() {
            return;
        }
        let verbe = if couper { "coupé" } else { "copié" };
        self.signaler(format!("{} élément(s) {verbe}", noms.len()), false);
        self.presse_papier = Some(PressePapier {
            position: self.vues[i].position.clone(),
            noms,
            couper,
        });
    }

    // collage du presse-papiers dans la vue cible
    fn coller(&mut self, i: usize) {
        let Some(pp) = self.presse_papier.take() else {
            return;
        };
        let cible = self.vues[i].position.clone();
        if pp.position == cible && !pp.couper {
            self.signaler("source et destination identiques", true);
            self.presse_papier = Some(pp);
            return;
        }
        let mut erreurs = 0;
        for nom in &pp.noms {
            let resultat = match (pp.position.cote, cible.cote) {
                // hote vers hote : copie systeme directe
                (Cote::Hote, Cote::Hote) => copier_hote_recursif(
                    &Path::new(&pp.position.chemin).join(nom),
                    &Path::new(&cible.chemin).join(nom),
                ),
                // volume vers volume : recursif au sein de la meme session
                (Cote::Volume, Cote::Volume) => copier_volume_recursif(
                    self.volume.session(),
                    &chemin_volume(&pp.position.chemin, nom),
                    &chemin_volume(&cible.chemin, nom),
                ),
                // croise : copie en flux entre les deux arbres
                (Cote::Hote, Cote::Volume) => {
                    self.hote.positionner(&pp.position.chemin).ok();
                    self.volume.positionner(&cible.chemin).ok();
                    copier(&mut self.hote, nom, &mut self.volume)
                }
                (Cote::Volume, Cote::Hote) => {
                    self.volume.positionner(&pp.position.chemin).ok();
                    self.hote.positionner(&cible.chemin).ok();
                    copier(&mut self.volume, nom, &mut self.hote)
                }
            };
            if let Err(e) = resultat {
                erreurs += 1;
                self.signaler(format!("échec sur {nom} : {e}"), true);
            }
        }
        // deplacement : suppression de la source apres copie reussie
        if pp.couper && erreurs == 0 {
            for nom in &pp.noms {
                let arbre: &mut dyn Arborescence = match pp.position.cote {
                    Cote::Hote => &mut self.hote,
                    Cote::Volume => &mut self.volume,
                };
                arbre.positionner(&pp.position.chemin).ok();
                arbre.supprimer(nom).ok();
            }
        }
        if erreurs == 0 {
            let verbe = if pp.couper { "déplacé" } else { "collé" };
            self.signaler(format!("{} élément(s) {verbe}", pp.noms.len()), false);
        }
        self.rafraichir(0);
        if self.vues.len() > 1 {
            self.rafraichir(1);
        }
    }

    fn charger_proprietes(&mut self, i: usize, nom: &str) {
        let position = self.vues[i].position.clone();
        let entree = self.vues[i].entrees.iter().find(|e| e.nom == nom).cloned();
        let Some(entree) = entree else { return };
        // metadonnees etendues pour les fichiers du volume
        let metas = if position.cote == Cote::Volume {
            let chemin = chemin_volume(&position.chemin, nom);
            self.volume.session().lire_metas(&chemin).unwrap_or_default()
        } else {
            Vec::new()
        };
        self.proprietes = Some(Proprietes {
            nom: entree.nom,
            emplacement: match position.cote {
                Cote::Hote => position.chemin.clone(),
                Cote::Volume => format!("{}:{}", self.volume.etiquette(), position.chemin),
            },
            est_dossier: entree.est_dossier,
            taille: entree.taille,
            modifie: entree.modifie,
            metas,
        });
    }

    fn valider_renommage(&mut self) {
        let Some(renommage) = self.renommage.take() else {
            return;
        };
        let nouveau = renommage.saisie.trim().to_string();
        if nouveau.is_empty() || nouveau == renommage.ancien {
            return;
        }
        let i = renommage.vue;
        match self.arbre_positionne(i).renommer(&renommage.ancien, &nouveau) {
            Ok(()) => self.signaler(format!("renommé : {} → {nouveau}", renommage.ancien), false),
            Err(e) => self.signaler(format!("{e}"), true),
        }
        self.rafraichir(i);
    }

    // ------- rendu -------

    // raccourcis clavier du gestionnaire
    fn raccourcis(&self, ui: &egui::Ui, actions: &mut Vec<Action>) {
        // pas de raccourcis pendant un renommage ou une saisie
        if self.renommage.is_some() || self.creation_dossier || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let i = self.active;
        ui.input(|entree| {
            if entree.key_pressed(egui::Key::F2) {
                actions.push(Action::DemanderRenommage(i));
            }
            if entree.key_pressed(egui::Key::Delete) {
                actions.push(Action::DemanderSuppression(i));
            }
            if entree.key_pressed(egui::Key::F3) {
                actions.push(Action::BasculerScission);
            }
            if entree.key_pressed(egui::Key::F5) {
                actions.push(Action::Actualiser(i));
            }
            if entree.modifiers.ctrl {
                if entree.key_pressed(egui::Key::C) {
                    actions.push(Action::Copier(i));
                }
                if entree.key_pressed(egui::Key::X) {
                    actions.push(Action::Couper(i));
                }
                if entree.key_pressed(egui::Key::V) {
                    actions.push(Action::Coller(i));
                }
            }
            if entree.modifiers.alt {
                if entree.key_pressed(egui::Key::ArrowLeft) {
                    actions.push(Action::Retour(i));
                }
                if entree.key_pressed(egui::Key::ArrowRight) {
                    actions.push(Action::Avancer(i));
                }
                if entree.key_pressed(egui::Key::ArrowUp) {
                    actions.push(Action::Parent(i));
                }
            }
        });
    }

    // panneau lateral des emplacements, comme dolphin
    fn panneau_lateral(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("EMPLACEMENTS")
                .color(theme::TEXTE_FAIBLE)
                .small()
                .strong(),
        );
        let emplacements = self.emplacements.clone();
        for (icone, nom, chemin) in &emplacements {
            let actif = self.vues[self.active].position.cote == Cote::Hote
                && self.vues[self.active].position.chemin == chemin.display().to_string();
            if ui
                .selectable_label(actif, format!("{icone} {nom}"))
                .clicked()
            {
                actions.push(Action::Naviguer(
                    self.active,
                    Position {
                        cote: Cote::Hote,
                        chemin: chemin.display().to_string(),
                    },
                ));
            }
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new("PÉRIPHÉRIQUES")
                .color(theme::TEXTE_FAIBLE)
                .small()
                .strong(),
        );
        let etiquette = self.volume.etiquette();
        let actif = self.vues[self.active].position.cote == Cote::Volume;
        if ui
            .selectable_label(
                actif,
                egui::RichText::new(format!("🔑 {etiquette}")).color(theme::OR),
            )
            .clicked()
        {
            actions.push(Action::Naviguer(
                self.active,
                Position {
                    cote: Cote::Volume,
                    chemin: "/".into(),
                },
            ));
        }
        // espace libre du volume
        let stats = self.volume.session().statistiques();
        let libre = stats.blocs_libres * stats.taille_bloc as u64;
        let total = stats.nb_blocs_donnees * stats.taille_bloc as u64;
        ui.add(
            egui::ProgressBar::new(1.0 - libre as f32 / total.max(1) as f32)
                .desired_height(6.0)
                .fill(theme::BREEZE_BLEU),
        );
        ui.label(
            egui::RichText::new(format!("{} libres", theme::taille_lisible(libre)))
                .color(theme::TEXTE_FAIBLE)
                .small(),
        );
        ui.add_space(10.0);
        let bouton = egui::Button::new("🔒 Verrouiller").fill(theme::ACCENT_FONCE);
        if ui.add(bouton).clicked() {
            self.fermeture_demandee = true;
        }
    }

    // barre d'outils : navigation, fil d'ariane, modes de vue
    fn barre_outils(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let i = self.active;
        let vue = &self.vues[i];
        ui.horizontal(|ui| {
            let retour_possible = vue.index_hist > 0;
            let avance_possible = vue.index_hist + 1 < vue.historique.len();
            if ui
                .add_enabled(retour_possible, egui::Button::new("←"))
                .on_hover_text("précédent (alt+gauche)")
                .clicked()
            {
                actions.push(Action::Retour(i));
            }
            if ui
                .add_enabled(avance_possible, egui::Button::new("→"))
                .on_hover_text("suivant (alt+droite)")
                .clicked()
            {
                actions.push(Action::Avancer(i));
            }
            if ui.button("↑").on_hover_text("dossier parent (alt+haut)").clicked() {
                actions.push(Action::Parent(i));
            }
            ui.separator();

            // fil d'ariane cliquable
            let position = vue.position.clone();
            let racine_texte = match position.cote {
                Cote::Hote => "💻".to_string(),
                Cote::Volume => format!("🔑 {}", self.volume.etiquette()),
            };
            if ui.button(racine_texte).clicked() {
                actions.push(Action::Naviguer(
                    i,
                    Position {
                        cote: position.cote,
                        chemin: "/".into(),
                    },
                ));
            }
            let segments: Vec<String> = position
                .chemin
                .split('/')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            let mut cumul = String::new();
            for segment in &segments {
                ui.label(egui::RichText::new("›").color(theme::TEXTE_FAIBLE));
                cumul.push('/');
                cumul.push_str(segment);
                if ui.button(segment).clicked() {
                    actions.push(Action::Naviguer(
                        i,
                        Position {
                            cote: position.cote,
                            chemin: cumul.clone(),
                        },
                    ));
                }
            }

            // filtre et modes a droite
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let scission_texte = if self.scission { "◫" } else { "◻" };
                if ui
                    .button(scission_texte)
                    .on_hover_text("scinder la vue (f3)")
                    .clicked()
                {
                    actions.push(Action::BasculerScission);
                }
                ui.separator();
                for (mode, icone, nom) in [
                    (ModeVue::Details, "☰", "détails"),
                    (ModeVue::Compact, "≡", "compact"),
                    (ModeVue::Icones, "▦", "icônes"),
                ] {
                    if ui
                        .selectable_label(vue.mode == mode, icone)
                        .on_hover_text(nom)
                        .clicked()
                    {
                        actions.push(Action::Mode(i, mode));
                    }
                }
                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.filtre)
                        .hint_text("🔍 filtrer")
                        .desired_width(130.0),
                );
            });
        });
    }

    // menu contextuel d'une entree
    fn menu_entree(
        &self,
        ui: &mut egui::Ui,
        i: usize,
        nom: &str,
        actions: &mut Vec<Action>,
    ) {
        ui.set_min_width(190.0);
        if ui.button("↗ Ouvrir").clicked() {
            actions.push(Action::Ouvrir(i, nom.to_string()));
            ui.close();
        }
        ui.separator();
        if ui.button("✂ Couper          Ctrl+X").clicked() {
            actions.push(Action::SelectionSimple(i, nom.to_string()));
            actions.push(Action::Couper(i));
            ui.close();
        }
        if ui.button("⧉ Copier          Ctrl+C").clicked() {
            actions.push(Action::SelectionSimple(i, nom.to_string()));
            actions.push(Action::Copier(i));
            ui.close();
        }
        if self.presse_papier.is_some() && ui.button("📋 Coller          Ctrl+V").clicked() {
            actions.push(Action::Coller(i));
            ui.close();
        }
        ui.separator();
        if ui.button("✏ Renommer        F2").clicked() {
            actions.push(Action::SelectionSimple(i, nom.to_string()));
            actions.push(Action::DemanderRenommage(i));
            ui.close();
        }
        let supprimer =
            egui::Button::new(egui::RichText::new("🗑 Supprimer       Suppr").color(theme::DANGER));
        if ui.add(supprimer).clicked() {
            actions.push(Action::SelectionSimple(i, nom.to_string()));
            actions.push(Action::DemanderSuppression(i));
            ui.close();
        }
        ui.separator();
        if ui.button("ℹ Propriétés").clicked() {
            actions.push(Action::Proprietes(i, nom.to_string()));
            ui.close();
        }
    }

    // menu contextuel du fond du panneau
    fn menu_fond(&mut self, ui: &mut egui::Ui, i: usize, actions: &mut Vec<Action>) {
        ui.set_min_width(190.0);
        if ui.button("📁 Nouveau dossier…").clicked() {
            self.creation_dossier = true;
            self.active = i;
            ui.close();
        }
        if self.presse_papier.is_some() && ui.button("📋 Coller          Ctrl+V").clicked() {
            actions.push(Action::Coller(i));
            ui.close();
        }
        if ui.button("⟳ Actualiser       F5").clicked() {
            actions.push(Action::Actualiser(i));
            ui.close();
        }
    }

    // rendu d'une entree selon le mode, renvoie la reponse du clic
    fn afficher_entree(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        entree: &EntreeArbre,
        actions: &mut Vec<Action>,
    ) {
        let choisi = self.vues[i].selection.contains(&entree.nom);
        let coupe = self
            .presse_papier
            .as_ref()
            .is_some_and(|pp| pp.couper && pp.noms.contains(&entree.nom));
        let icone = if entree.est_dossier { "📁" } else { "📄" };

        // renommage en ligne a la place du nom
        if let Some(renommage) = &mut self.renommage {
            if renommage.vue == i && renommage.ancien == entree.nom {
                let champ = ui.add(
                    egui::TextEdit::singleline(&mut renommage.saisie).desired_width(180.0),
                );
                if !renommage.focalise {
                    champ.request_focus();
                    renommage.focalise = true;
                }
                if ui.input(|e| e.key_pressed(egui::Key::Escape)) {
                    self.renommage = None;
                } else if champ.lost_focus() {
                    if ui.input(|e| e.key_pressed(egui::Key::Enter)) {
                        self.valider_renommage();
                    } else {
                        self.renommage = None;
                    }
                }
                return;
            }
        }

        let mut texte = egui::RichText::new(format!("{icone} {}", entree.nom));
        if coupe {
            texte = texte.color(theme::TEXTE_FAIBLE).italics();
        }
        let reponse = ui.selectable_label(choisi, texte);
        if reponse.clicked() {
            if ui.input(|e| e.modifiers.ctrl) {
                actions.push(Action::SelectionCtrl(i, entree.nom.clone()));
            } else {
                actions.push(Action::SelectionSimple(i, entree.nom.clone()));
            }
        }
        if reponse.double_clicked() {
            actions.push(Action::Ouvrir(i, entree.nom.clone()));
        }
        reponse.context_menu(|ui| self.menu_entree(ui, i, &entree.nom, actions));
    }

    // rendu du contenu d'un panneau
    fn panneau_fichiers(&mut self, ui: &mut egui::Ui, i: usize, actions: &mut Vec<Action>) {
        let est_active = self.active == i;
        let vue_mode = self.vues[i].mode;
        let filtre = self.filtre.to_lowercase();
        let entrees: Vec<EntreeArbre> = self.vues[i]
            .entrees
            .iter()
            .filter(|e| filtre.is_empty() || e.nom.to_lowercase().contains(&filtre))
            .cloned()
            .collect();

        let bord = if est_active && self.scission {
            egui::Stroke::new(1.0, theme::BREEZE_BLEU)
        } else {
            egui::Stroke::new(1.0, theme::BREEZE_CARTE)
        };
        let fond = egui::Frame::new()
            .fill(theme::BREEZE_FOND)
            .stroke(bord)
            .corner_radius(6)
            .inner_margin(8);
        let reponse_fond = fond.show(ui, |ui| {
            ui.set_min_height(ui.available_height() - 4.0);
            // en-tetes de colonnes en mode details
            if vue_mode == ModeVue::Details {
                let vue = &self.vues[i];
                let fleche = |actif: bool, inverse: bool| {
                    if !actif {
                        ""
                    } else if inverse {
                        " ▼"
                    } else {
                        " ▲"
                    }
                };
                ui.horizontal(|ui| {
                    if ui
                        .button(format!(
                            "Nom{}",
                            fleche(vue.tri == Tri::Nom, vue.tri_inverse)
                        ))
                        .clicked()
                    {
                        actions.push(Action::Trier(i, Tri::Nom));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(format!(
                                "Modifié{}",
                                fleche(vue.tri == Tri::Date, vue.tri_inverse)
                            ))
                            .clicked()
                        {
                            actions.push(Action::Trier(i, Tri::Date));
                        }
                        if ui
                            .button(format!(
                                "Taille{}",
                                fleche(vue.tri == Tri::Taille, vue.tri_inverse)
                            ))
                            .clicked()
                        {
                            actions.push(Action::Trier(i, Tri::Taille));
                        }
                    });
                });
                ui.separator();
            }

            egui::ScrollArea::vertical()
                .id_salt(format!("panneau_{i}"))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if entrees.is_empty() {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("(dossier vide)").color(theme::TEXTE_FAIBLE),
                            );
                        });
                    }
                    match vue_mode {
                        ModeVue::Details => {
                            egui::Grid::new(format!("grille_{i}"))
                                .striped(true)
                                .num_columns(3)
                                .min_col_width(60.0)
                                .show(ui, |ui| {
                                    for entree in &entrees {
                                        ui.horizontal(|ui| {
                                            ui.set_min_width(ui.available_width() - 220.0);
                                            self.afficher_entree(ui, i, entree, actions);
                                        });
                                        let taille = if entree.est_dossier {
                                            String::new()
                                        } else {
                                            theme::taille_lisible(entree.taille)
                                        };
                                        ui.label(
                                            egui::RichText::new(taille)
                                                .color(theme::TEXTE_FAIBLE)
                                                .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(theme::horodatage_texte(
                                                entree.modifie,
                                            ))
                                            .color(theme::TEXTE_FAIBLE)
                                            .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                        ModeVue::Compact => {
                            ui.horizontal_wrapped(|ui| {
                                for entree in &entrees {
                                    self.afficher_entree(ui, i, entree, actions);
                                }
                            });
                        }
                        ModeVue::Icones => {
                            ui.horizontal_wrapped(|ui| {
                                for entree in &entrees {
                                    ui.allocate_ui(egui::vec2(96.0, 86.0), |ui| {
                                        ui.vertical_centered(|ui| {
                                            let icone =
                                                if entree.est_dossier { "📁" } else { "📄" };
                                            ui.label(egui::RichText::new(icone).size(34.0));
                                            self.afficher_entree_icone(ui, i, entree, actions);
                                        });
                                    });
                                }
                            });
                        }
                    }
                    // zone residuelle cliquable pour le menu du fond
                    ui.allocate_space(egui::vec2(ui.available_width(), 40.0));
                });
        });
        // clic dans le panneau : activation ; clic droit sur le fond : menu
        let reponse = reponse_fond.response.interact(egui::Sense::click());
        if reponse.clicked() {
            actions.push(Action::Activer(i));
        }
        reponse.context_menu(|ui| self.menu_fond(ui, i, actions));
    }

    // libelle sous l'icone en mode icones
    fn afficher_entree_icone(
        &mut self,
        ui: &mut egui::Ui,
        i: usize,
        entree: &EntreeArbre,
        actions: &mut Vec<Action>,
    ) {
        let choisi = self.vues[i].selection.contains(&entree.nom);
        let nom_court = if entree.nom.chars().count() > 12 {
            let tronque: String = entree.nom.chars().take(11).collect();
            format!("{tronque}…")
        } else {
            entree.nom.clone()
        };
        let reponse = ui.selectable_label(choisi, egui::RichText::new(nom_court).small());
        if reponse.clicked() {
            if ui.input(|e| e.modifiers.ctrl) {
                actions.push(Action::SelectionCtrl(i, entree.nom.clone()));
            } else {
                actions.push(Action::SelectionSimple(i, entree.nom.clone()));
            }
        }
        if reponse.double_clicked() {
            actions.push(Action::Ouvrir(i, entree.nom.clone()));
        }
        reponse.context_menu(|ui| self.menu_entree(ui, i, &entree.nom, actions));
    }

    // barre d'etat : compte des entrees, comme dolphin
    fn barre_etat(&mut self, ui: &mut egui::Ui) {
        let vue = &self.vues[self.active];
        let dossiers = vue.entrees.iter().filter(|e| e.est_dossier).count();
        let fichiers = vue.entrees.len() - dossiers;
        let octets: u64 = vue
            .entrees
            .iter()
            .filter(|e| !e.est_dossier)
            .map(|e| e.taille)
            .sum();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{dossiers} dossier(s), {fichiers} fichier(s) ({})",
                    theme::taille_lisible(octets)
                ))
                .color(theme::TEXTE_FAIBLE)
                .small(),
            );
            let selection = vue.selection.len();
            if selection > 0 {
                ui.label(
                    egui::RichText::new(format!("— {selection} sélectionné(s)"))
                        .color(theme::BREEZE_BLEU)
                        .small(),
                );
            }
            if !self.message.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let couleur = if self.erreur { theme::DANGER } else { theme::SUCCES };
                    ui.label(egui::RichText::new(&self.message).color(couleur).small());
                });
            }
        });
    }

    // fenetres modales : suppression, proprietes, apercu, nouveau dossier
    fn modales(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        // confirmation de suppression
        if let Some(noms) = self.suppression.clone() {
            egui::Modal::new(egui::Id::new("suppression_entrees")).show(ctx, |ui| {
                ui.set_max_width(380.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("🗑").size(30.0).color(theme::DANGER));
                    ui.label(
                        egui::RichText::new(format!("Supprimer {} élément(s) ?", noms.len()))
                            .strong(),
                    );
                    for nom in noms.iter().take(6) {
                        ui.label(egui::RichText::new(nom).color(theme::TEXTE_FAIBLE).small());
                    }
                    if noms.len() > 6 {
                        ui.label(
                            egui::RichText::new(format!("… et {} autres", noms.len() - 6))
                                .color(theme::TEXTE_FAIBLE)
                                .small(),
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(70.0);
                        if ui.button("Annuler").clicked() {
                            self.suppression = None;
                        }
                        let bouton = egui::Button::new(
                            egui::RichText::new("Supprimer").strong(),
                        )
                        .fill(theme::DANGER);
                        if ui.add(bouton).clicked() {
                            let i = self.active;
                            for nom in &noms {
                                if let Err(e) = self.arbre_positionne(i).supprimer(nom) {
                                    self.signaler(format!("{e}"), true);
                                }
                            }
                            self.suppression = None;
                            actions.push(Action::Actualiser(i));
                        }
                    });
                });
            });
        }

        // fiche de proprietes
        let mut fermer_proprietes = false;
        if let Some(proprietes) = &self.proprietes {
            egui::Modal::new(egui::Id::new("proprietes_entree")).show(ctx, |ui| {
                ui.set_max_width(400.0);
                ui.vertical_centered(|ui| {
                    let icone = if proprietes.est_dossier { "📁" } else { "📄" };
                    ui.label(egui::RichText::new(icone).size(34.0));
                    ui.label(egui::RichText::new(&proprietes.nom).strong().size(17.0));
                });
                ui.add_space(8.0);
                egui::Grid::new("grille_proprietes")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("type").color(theme::TEXTE_FAIBLE));
                        ui.label(if proprietes.est_dossier { "dossier" } else { "fichier" });
                        ui.end_row();
                        ui.label(egui::RichText::new("emplacement").color(theme::TEXTE_FAIBLE));
                        ui.label(&proprietes.emplacement);
                        ui.end_row();
                        if !proprietes.est_dossier {
                            ui.label(egui::RichText::new("taille").color(theme::TEXTE_FAIBLE));
                            ui.label(format!(
                                "{} ({} octets)",
                                theme::taille_lisible(proprietes.taille),
                                proprietes.taille
                            ));
                            ui.end_row();
                        }
                        ui.label(egui::RichText::new("modifié").color(theme::TEXTE_FAIBLE));
                        ui.label(theme::horodatage_texte(proprietes.modifie));
                        ui.end_row();
                        for (cle, valeur) in &proprietes.metas {
                            ui.label(egui::RichText::new(cle).color(theme::OR));
                            ui.label(valeur);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    if ui.button("Fermer").clicked() {
                        fermer_proprietes = true;
                    }
                });
            });
        }
        if fermer_proprietes {
            self.proprietes = None;
        }

        // apercu d'un fichier du volume
        let mut fermer_apercu = false;
        if let Some((nom, contenu)) = &self.apercu {
            egui::Modal::new(egui::Id::new("apercu_fichier")).show(ctx, |ui| {
                ui.set_max_width(560.0);
                ui.label(egui::RichText::new(nom).strong());
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut contenu.as_str())
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );
                    });
                ui.vertical_centered(|ui| {
                    if ui.button("Fermer").clicked() {
                        fermer_apercu = true;
                    }
                });
            });
        }
        if fermer_apercu {
            self.apercu = None;
        }

        // creation de dossier
        if self.creation_dossier {
            egui::Modal::new(egui::Id::new("nouveau_dossier")).show(ctx, |ui| {
                ui.set_max_width(320.0);
                ui.label(egui::RichText::new("📁 Nouveau dossier").strong());
                let champ = ui.add(
                    egui::TextEdit::singleline(&mut self.saisie_dossier)
                        .hint_text("nom du dossier")
                        .desired_width(f32::INFINITY),
                );
                champ.request_focus();
                let valider = champ.lost_focus() && ui.input(|e| e.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if ui.button("Annuler").clicked()
                        || ui.input(|e| e.key_pressed(egui::Key::Escape))
                    {
                        self.creation_dossier = false;
                        self.saisie_dossier.clear();
                    }
                    let bouton = egui::Button::new("Créer").fill(theme::ACCENT);
                    if ui.add(bouton).clicked() || valider {
                        actions.push(Action::NouveauDossier(self.active));
                    }
                });
            });
        }
    }

    pub fn afficher(&mut self, ui: &mut egui::Ui) {
        // ambiance breeze locale au gestionnaire
        let visuels = &mut ui.style_mut().visuals;
        visuels.panel_fill = theme::BREEZE_PANNEAU;
        visuels.selection.bg_fill = theme::BREEZE_SELECTION;
        visuels.selection.stroke = egui::Stroke::new(1.0, theme::BREEZE_BLEU);

        let mut actions: Vec<Action> = Vec::new();
        self.raccourcis(ui, &mut actions);

        // panneau lateral des emplacements
        egui::Panel::left("panneau_lateral")
            .default_size(190.0)
            .show(ui, |ui| {
                self.panneau_lateral(ui, &mut actions);
            });

        // barre d'outils
        egui::Panel::top("barre_outils").show(ui, |ui| {
            ui.add_space(4.0);
            self.barre_outils(ui, &mut actions);
            ui.add_space(4.0);
        });

        // barre d'etat
        egui::Panel::bottom("barre_etat").show(ui, |ui| {
            self.barre_etat(ui);
        });

        // zone centrale : un ou deux panneaux
        egui::CentralPanel::default().show(ui, |ui| {
            let largeur = ui.available_width();
            if self.scission {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(largeur * 0.495);
                        self.panneau_fichiers(ui, 0, &mut actions);
                    });
                    ui.vertical(|ui| {
                        ui.set_width(largeur * 0.495);
                        self.panneau_fichiers(ui, 1, &mut actions);
                    });
                });
            } else {
                self.panneau_fichiers(ui, self.active.min(self.vues.len() - 1), &mut actions);
            }
        });

        let ctx = ui.ctx().clone();
        self.modales(&ctx, &mut actions);

        for action in actions {
            self.appliquer(action);
        }
    }
}

// chemin absolu dans un volume
fn chemin_volume(dossier: &str, nom: &str) -> String {
    if dossier == "/" {
        format!("/{nom}")
    } else {
        format!("{dossier}/{nom}")
    }
}

// copie recursive au sein du systeme hote
fn copier_hote_recursif(source: &Path, destination: &Path) -> ResultatFs<()> {
    if source.is_dir() {
        std::fs::create_dir_all(destination)?;
        for entree in std::fs::read_dir(source)? {
            let entree = entree?;
            copier_hote_recursif(&entree.path(), &destination.join(entree.file_name()))?;
        }
    } else {
        std::fs::copy(source, destination)?;
    }
    Ok(())
}

// copie recursive au sein du volume, par fichier temporaire borne
fn copier_volume_recursif(session: &mut Session, source: &str, destination: &str) -> ResultatFs<()> {
    let entrees = session.lister(source);
    match entrees {
        Ok(entrees) => {
            // dossier : creation puis descente
            if let Err(e) = session.creer_dossier(destination) {
                if !matches!(e, gestionnaire_fichiers::ErreurFs::ExisteDeja(_)) {
                    return Err(e);
                }
            }
            for entree in entrees {
                copier_volume_recursif(
                    session,
                    &chemin_volume(source, &entree.nom),
                    &chemin_volume(destination, &entree.nom),
                )?;
            }
            Ok(())
        }
        Err(_) => {
            // fichier : passage par un tampon temporaire
            let tampon = std::env::temp_dir()
                .join(format!("monarque_interne_{}", std::process::id()));
            {
                let mut fichier = std::fs::File::create(&tampon)?;
                session.lire_flux(source, &mut fichier)?;
            }
            {
                let mut fichier = std::fs::File::open(&tampon)?;
                session.ecrire_flux(destination, &mut fichier)?;
            }
            std::fs::remove_file(&tampon).ok();
            Ok(())
        }
    }
}
