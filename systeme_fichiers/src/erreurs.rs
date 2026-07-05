// erreurs du systeme de fichiers

use std::fmt;

pub type ResultatFs<T> = Result<T, ErreurFs>;

#[derive(Debug)]
pub enum ErreurFs {
    // erreur d'entree sortie
    Io(std::io::Error),
    // erreur du noyau de partition
    Noyau(noyau_partition::ErreurNoyau),
    // superbloc illisible ou signature absente
    VolumeInvalide(String),
    // phrase secrete incorrecte
    PhraseInvalide,
    // bloc altere ou cle incorrecte
    BlocCorrompu(u64),
    // plus de blocs libres
    EspacePlein,
    // plus d'inodes libres
    InodesEpuises,
    // chemin introuvable
    Introuvable(String),
    // entree deja existante
    ExisteDeja(String),
    // nom d'entree invalide
    NomInvalide(String),
    // operation attendait un dossier
    PasUnDossier(String),
    // operation attendait un fichier
    PasUnFichier(String),
    // suppression d'un dossier non vide
    DossierNonVide(String),
    // metadonnees etendues trop volumineuses
    MetaTropGrande,
    // fichier trop grand pour le volume
    FichierTropGrand,
}

impl fmt::Display for ErreurFs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurFs::Io(e) => write!(f, "erreur io : {e}"),
            ErreurFs::Noyau(e) => write!(f, "erreur noyau : {e}"),
            ErreurFs::VolumeInvalide(m) => write!(f, "volume invalide : {m}"),
            ErreurFs::PhraseInvalide => write!(f, "phrase secrete incorrecte"),
            ErreurFs::BlocCorrompu(b) => write!(f, "bloc {b} corrompu ou cle incorrecte"),
            ErreurFs::EspacePlein => write!(f, "volume plein"),
            ErreurFs::InodesEpuises => write!(f, "plus d'inodes disponibles"),
            ErreurFs::Introuvable(c) => write!(f, "introuvable : {c}"),
            ErreurFs::ExisteDeja(c) => write!(f, "existe deja : {c}"),
            ErreurFs::NomInvalide(n) => write!(f, "nom invalide : {n}"),
            ErreurFs::PasUnDossier(c) => write!(f, "pas un dossier : {c}"),
            ErreurFs::PasUnFichier(c) => write!(f, "pas un fichier : {c}"),
            ErreurFs::DossierNonVide(c) => write!(f, "dossier non vide : {c}"),
            ErreurFs::MetaTropGrande => write!(f, "metadonnees etendues trop volumineuses"),
            ErreurFs::FichierTropGrand => write!(f, "fichier trop grand"),
        }
    }
}

impl std::error::Error for ErreurFs {}

impl From<std::io::Error> for ErreurFs {
    fn from(e: std::io::Error) -> Self {
        ErreurFs::Io(e)
    }
}

impl From<noyau_partition::ErreurNoyau> for ErreurFs {
    fn from(e: noyau_partition::ErreurNoyau) -> Self {
        ErreurFs::Noyau(e)
    }
}
