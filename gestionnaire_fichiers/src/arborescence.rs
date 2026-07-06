// arborescence navigable : systeme hote ou volume monarque, avec copie en flux

use crate::session::Session;
use systeme_fichiers::{ErreurFs, ResultatFs, TypeNoeud};
use std::fs::File;
use std::path::PathBuf;

// une entree affichable dans un panneau
#[derive(Debug, Clone)]
pub struct EntreeArbre {
    pub nom: String,
    pub est_dossier: bool,
    pub taille: u64,
}

// operations communes aux deux sources de fichiers
pub trait Arborescence {
    // nom court de la source (pour l'onglet)
    fn etiquette(&self) -> String;
    // chemin courant affichable
    fn chemin_courant(&self) -> String;
    // contenu du dossier courant, dossiers d'abord
    fn lister(&mut self) -> ResultatFs<Vec<EntreeArbre>>;
    // descente dans un sous dossier
    fn entrer(&mut self, nom: &str) -> ResultatFs<()>;
    // remontee au dossier parent
    fn remonter(&mut self);
    // retour a la racine
    fn aller_racine(&mut self);
    // creation d'un dossier dans le dossier courant
    fn creer_dossier(&mut self, nom: &str) -> ResultatFs<()>;
    // suppression recursive d'une entree du dossier courant
    fn supprimer(&mut self, nom: &str) -> ResultatFs<()>;
    // renommage d'une entree du dossier courant
    fn renommer(&mut self, nom: &str, nouveau: &str) -> ResultatFs<()>;
    // vrai si l'entree est un dossier
    fn est_dossier(&mut self, nom: &str) -> bool;
    // sauvegarde et restauration de la position (pour la copie recursive)
    fn position(&self) -> String;
    fn positionner(&mut self, chemin: &str) -> ResultatFs<()>;
    // chemin hote reel d'une entree, si la source est le systeme hote
    fn chemin_hote(&self, nom: &str) -> Option<PathBuf>;
    // lecture en flux d'un fichier du dossier courant
    fn lire_flux(&mut self, nom: &str, sortie: &mut dyn std::io::Write) -> ResultatFs<()>;
    // ecriture en flux d'un fichier dans le dossier courant
    fn ecrire_flux(&mut self, nom: &str, source: &mut dyn std::io::Read) -> ResultatFs<()>;
}

// tri commun : dossiers avant fichiers, puis par nom
fn trier(entrees: &mut [EntreeArbre]) {
    entrees.sort_by(|a, b| {
        b.est_dossier
            .cmp(&a.est_dossier)
            .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
    });
}

// ---------------- systeme de fichiers hote ----------------

pub struct ArbreHote {
    courant: PathBuf,
}

impl ArbreHote {
    // demarrage dans le dossier personnel
    pub fn nouveau() -> Self {
        let depart = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));
        Self { courant: depart }
    }
}

impl Arborescence for ArbreHote {
    fn etiquette(&self) -> String {
        "Cet ordinateur".to_string()
    }

    fn chemin_courant(&self) -> String {
        self.courant.display().to_string()
    }

    fn lister(&mut self) -> ResultatFs<Vec<EntreeArbre>> {
        let mut entrees = Vec::new();
        for entree in std::fs::read_dir(&self.courant)? {
            let entree = entree?;
            let nom = entree.file_name().to_string_lossy().into_owned();
            // on masque les fichiers caches
            if nom.starts_with('.') {
                continue;
            }
            let meta = entree.metadata();
            let est_dossier = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let taille = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            entrees.push(EntreeArbre {
                nom,
                est_dossier,
                taille,
            });
        }
        trier(&mut entrees);
        Ok(entrees)
    }

    fn entrer(&mut self, nom: &str) -> ResultatFs<()> {
        let cible = self.courant.join(nom);
        if cible.is_dir() {
            self.courant = cible;
        }
        Ok(())
    }

    fn remonter(&mut self) {
        if let Some(parent) = self.courant.parent() {
            self.courant = parent.to_path_buf();
        }
    }

    fn aller_racine(&mut self) {
        self.courant = PathBuf::from("/");
    }

    fn creer_dossier(&mut self, nom: &str) -> ResultatFs<()> {
        std::fs::create_dir(self.courant.join(nom))?;
        Ok(())
    }

    fn supprimer(&mut self, nom: &str) -> ResultatFs<()> {
        let cible = self.courant.join(nom);
        if cible.is_dir() {
            std::fs::remove_dir_all(cible)?;
        } else {
            std::fs::remove_file(cible)?;
        }
        Ok(())
    }

    fn renommer(&mut self, nom: &str, nouveau: &str) -> ResultatFs<()> {
        std::fs::rename(self.courant.join(nom), self.courant.join(nouveau))?;
        Ok(())
    }

    fn est_dossier(&mut self, nom: &str) -> bool {
        self.courant.join(nom).is_dir()
    }

    fn position(&self) -> String {
        self.courant.display().to_string()
    }

    fn positionner(&mut self, chemin: &str) -> ResultatFs<()> {
        self.courant = PathBuf::from(chemin);
        Ok(())
    }

    fn chemin_hote(&self, nom: &str) -> Option<PathBuf> {
        Some(self.courant.join(nom))
    }

    fn lire_flux(&mut self, nom: &str, sortie: &mut dyn std::io::Write) -> ResultatFs<()> {
        let mut fichier = File::open(self.courant.join(nom))?;
        std::io::copy(&mut fichier, sortie)?;
        Ok(())
    }

    fn ecrire_flux(&mut self, nom: &str, source: &mut dyn std::io::Read) -> ResultatFs<()> {
        let mut fichier = File::create(self.courant.join(nom))?;
        std::io::copy(source, &mut fichier)?;
        Ok(())
    }
}

// ---------------- volume monarque ----------------

pub struct ArbreVolume {
    session: Session,
    courant: String,
    etiquette: String,
}

impl ArbreVolume {
    pub fn nouveau(session: Session, etiquette: String) -> Self {
        Self {
            session,
            courant: "/".to_string(),
            etiquette,
        }
    }

    // chemin absolu dans le volume
    fn absolu(&self, nom: &str) -> String {
        if self.courant == "/" {
            format!("/{nom}")
        } else {
            format!("{}/{nom}", self.courant)
        }
    }

    // acces a la session sous jacente
    pub fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn fermer(self) -> ResultatFs<()> {
        self.session.fermer()
    }
}

impl Arborescence for ArbreVolume {
    fn etiquette(&self) -> String {
        self.etiquette.clone()
    }

    fn chemin_courant(&self) -> String {
        self.etiquette.clone() + ":" + &self.courant
    }

    fn lister(&mut self) -> ResultatFs<Vec<EntreeArbre>> {
        let brut = self.session.lister(&self.courant)?;
        let mut entrees: Vec<EntreeArbre> = brut
            .into_iter()
            .map(|e| EntreeArbre {
                nom: e.nom,
                est_dossier: e.type_noeud == TypeNoeud::Dossier,
                taille: e.taille,
            })
            .collect();
        trier(&mut entrees);
        Ok(entrees)
    }

    fn entrer(&mut self, nom: &str) -> ResultatFs<()> {
        let cible = self.absolu(nom);
        // on ne descend que dans un dossier
        if self.est_dossier(nom) {
            self.courant = cible;
        }
        Ok(())
    }

    fn remonter(&mut self) {
        if self.courant != "/" {
            let pos = self.courant.rfind('/').unwrap_or(0);
            self.courant = if pos == 0 {
                "/".to_string()
            } else {
                self.courant[..pos].to_string()
            };
        }
    }

    fn aller_racine(&mut self) {
        self.courant = "/".to_string();
    }

    fn creer_dossier(&mut self, nom: &str) -> ResultatFs<()> {
        let chemin = self.absolu(nom);
        self.session.creer_dossier(&chemin)
    }

    fn supprimer(&mut self, nom: &str) -> ResultatFs<()> {
        let chemin = self.absolu(nom);
        supprimer_recursif(&mut self.session, &chemin)
    }

    fn renommer(&mut self, nom: &str, nouveau: &str) -> ResultatFs<()> {
        let chemin = self.absolu(nom);
        self.session.renommer(&chemin, nouveau)
    }

    fn est_dossier(&mut self, nom: &str) -> bool {
        self.session
            .lister(&self.courant)
            .map(|entrees| {
                entrees
                    .iter()
                    .any(|e| e.nom == nom && e.type_noeud == TypeNoeud::Dossier)
            })
            .unwrap_or(false)
    }

    fn position(&self) -> String {
        self.courant.clone()
    }

    fn positionner(&mut self, chemin: &str) -> ResultatFs<()> {
        self.courant = chemin.to_string();
        Ok(())
    }

    fn chemin_hote(&self, _nom: &str) -> Option<PathBuf> {
        None
    }

    fn lire_flux(&mut self, nom: &str, sortie: &mut dyn std::io::Write) -> ResultatFs<()> {
        let chemin = self.absolu(nom);
        self.session.lire_flux(&chemin, sortie)
    }

    fn ecrire_flux(&mut self, nom: &str, source: &mut dyn std::io::Read) -> ResultatFs<()> {
        let chemin = self.absolu(nom);
        self.session.ecrire_flux(&chemin, source)?;
        Ok(())
    }
}

// suppression recursive dans un volume : vide les dossiers avant de les retirer
fn supprimer_recursif(session: &mut Session, chemin: &str) -> ResultatFs<()> {
    if let Ok(entrees) = session.lister(chemin) {
        for entree in entrees {
            let enfant = if chemin == "/" {
                format!("/{}", entree.nom)
            } else {
                format!("{}/{}", chemin, entree.nom)
            };
            supprimer_recursif(session, &enfant)?;
        }
    }
    session.supprimer(chemin)
}

// ---------------- copie entre deux arborescences ----------------

// copie recursive d'une entree du panneau source vers le panneau destination
pub fn copier(
    source: &mut dyn Arborescence,
    nom: &str,
    destination: &mut dyn Arborescence,
) -> ResultatFs<()> {
    if source.est_dossier(nom) {
        // creation du dossier cible (ignore s'il existe deja)
        match destination.creer_dossier(nom) {
            Ok(()) | Err(ErreurFs::ExisteDeja(_)) => {}
            Err(e) => return Err(e),
        }
        let pos_source = source.position();
        let pos_dest = destination.position();
        source.entrer(nom)?;
        destination.entrer(nom)?;
        for entree in source.lister()? {
            copier(source, &entree.nom, destination)?;
        }
        source.positionner(&pos_source)?;
        destination.positionner(&pos_dest)?;
        Ok(())
    } else {
        copier_fichier(source, nom, destination)
    }
}

// copie d'un seul fichier avec le chemin de flux le plus direct
fn copier_fichier(
    source: &mut dyn Arborescence,
    nom: &str,
    destination: &mut dyn Arborescence,
) -> ResultatFs<()> {
    match (source.chemin_hote(nom), destination.chemin_hote(nom)) {
        // hote vers hote : copie directe du systeme
        (Some(src), Some(dst)) => {
            std::fs::copy(src, dst)?;
            Ok(())
        }
        // hote vers volume : lecture directe du fichier hote
        (Some(src), None) => {
            let mut fichier = File::open(src)?;
            destination.ecrire_flux(nom, &mut fichier)
        }
        // volume vers hote : ecriture directe du fichier hote
        (None, Some(dst)) => {
            let mut fichier = File::create(dst)?;
            source.lire_flux(nom, &mut fichier)
        }
        // volume vers volume : passage par un fichier temporaire, memoire bornee
        (None, None) => {
            let tampon = std::env::temp_dir()
                .join(format!("monarque_copie_{}_{nom}", std::process::id()));
            {
                let mut fichier = File::create(&tampon)?;
                source.lire_flux(nom, &mut fichier)?;
            }
            {
                let mut fichier = File::open(&tampon)?;
                destination.ecrire_flux(nom, &mut fichier)?;
            }
            std::fs::remove_file(&tampon).ok();
            Ok(())
        }
    }
}
