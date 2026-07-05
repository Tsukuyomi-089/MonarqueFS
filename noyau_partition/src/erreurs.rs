// erreurs du noyau de partition

use std::fmt;

pub type Resultat<T> = Result<T, ErreurNoyau>;

#[derive(Debug)]
pub enum ErreurNoyau {
    // erreur d'entree sortie
    Io(std::io::Error),
    // table de partition illisible
    TableInvalide(String),
    // pas d'espace pour la partition demandee
    EspaceInsuffisant,
    // table pleine
    TablePleine,
    // acces hors des bornes du disque ou de la partition
    HorsBornes,
    // nom de partition invalide
    NomInvalide,
    // partition introuvable
    Introuvable,
}

impl fmt::Display for ErreurNoyau {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurNoyau::Io(e) => write!(f, "erreur io : {e}"),
            ErreurNoyau::TableInvalide(m) => write!(f, "table de partition invalide : {m}"),
            ErreurNoyau::EspaceInsuffisant => write!(f, "espace disque insuffisant"),
            ErreurNoyau::TablePleine => write!(f, "table de partition pleine"),
            ErreurNoyau::HorsBornes => write!(f, "acces hors bornes"),
            ErreurNoyau::NomInvalide => write!(f, "nom de partition invalide"),
            ErreurNoyau::Introuvable => write!(f, "partition introuvable"),
        }
    }
}

impl std::error::Error for ErreurNoyau {}

impl From<std::io::Error> for ErreurNoyau {
    fn from(e: std::io::Error) -> Self {
        ErreurNoyau::Io(e)
    }
}
