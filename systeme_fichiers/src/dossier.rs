// entrees de dossier

use crate::erreurs::{ErreurFs, ResultatFs};
use crate::inode::TypeNoeud;

// taille d'une entree serialisee
pub const TAILLE_ENTREE_DOSSIER: usize = 64;
// longueur maximale d'un nom
pub const LONGUEUR_NOM_MAX: usize = 54;

#[derive(Debug, Clone)]
pub struct EntreeDossier {
    pub id_inode: u64,
    pub type_noeud: TypeNoeud,
    pub nom: String,
}

// verification d'un nom d'entree
pub fn valider_nom(nom: &str) -> ResultatFs<()> {
    if nom.is_empty()
        || nom.len() > LONGUEUR_NOM_MAX
        || nom.contains('/')
        || nom == "."
        || nom == ".."
    {
        return Err(ErreurFs::NomInvalide(nom.to_string()));
    }
    Ok(())
}

// serialisation d'une liste d'entrees
pub fn serialiser_entrees(entrees: &[EntreeDossier]) -> Vec<u8> {
    let mut sortie = Vec::with_capacity(entrees.len() * TAILLE_ENTREE_DOSSIER);
    for entree in entrees {
        let mut tampon = [0u8; TAILLE_ENTREE_DOSSIER];
        tampon[..8].copy_from_slice(&entree.id_inode.to_le_bytes());
        tampon[8] = entree.type_noeud.vers_u8();
        let nom = entree.nom.as_bytes();
        tampon[9] = nom.len() as u8;
        tampon[10..10 + nom.len()].copy_from_slice(nom);
        sortie.extend_from_slice(&tampon);
    }
    sortie
}

// lecture d'une liste d'entrees
pub fn deserialiser_entrees(donnees: &[u8]) -> ResultatFs<Vec<EntreeDossier>> {
    let mut entrees = Vec::new();
    for tampon in donnees.chunks_exact(TAILLE_ENTREE_DOSSIER) {
        let id_inode = u64::from_le_bytes(tampon[..8].try_into().unwrap());
        if id_inode == 0 {
            continue;
        }
        let type_noeud = TypeNoeud::depuis_u8(tampon[8])?;
        let longueur = tampon[9] as usize;
        if longueur > LONGUEUR_NOM_MAX {
            return Err(ErreurFs::VolumeInvalide("entree de dossier alteree".into()));
        }
        let nom = String::from_utf8(tampon[10..10 + longueur].to_vec())
            .map_err(|_| ErreurFs::VolumeInvalide("nom d'entree non utf8".into()))?;
        entrees.push(EntreeDossier {
            id_inode,
            type_noeud,
            nom,
        });
    }
    Ok(entrees)
}
