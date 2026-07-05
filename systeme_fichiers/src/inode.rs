// table d'inodes et metadonnees etendues

use crate::erreurs::{ErreurFs, ResultatFs};

// taille d'un inode serialise
pub const TAILLE_INODE: usize = 256;
// nombre de pointeurs directs
pub const NB_BLOCS_DIRECTS: usize = 12;
// zone reservee aux metadonnees etendues
pub const TAILLE_META: usize = 112;
// identifiant de l'inode racine
pub const INODE_RACINE: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeNoeud {
    Libre,
    Fichier,
    Dossier,
}

impl TypeNoeud {
    pub fn vers_u8(self) -> u8 {
        match self {
            TypeNoeud::Libre => 0,
            TypeNoeud::Fichier => 1,
            TypeNoeud::Dossier => 2,
        }
    }

    pub fn depuis_u8(v: u8) -> ResultatFs<Self> {
        match v {
            0 => Ok(TypeNoeud::Libre),
            1 => Ok(TypeNoeud::Fichier),
            2 => Ok(TypeNoeud::Dossier),
            autre => Err(ErreurFs::VolumeInvalide(format!(
                "type d'inode inconnu : {autre}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Inode {
    pub type_noeud: TypeNoeud,
    pub nb_liens: u32,
    pub taille: u64,
    pub cree: u64,
    pub modifie: u64,
    // pointeurs de blocs absolus, zero = vide
    pub blocs_directs: [u64; NB_BLOCS_DIRECTS],
    pub bloc_indirect: u64,
    pub bloc_double_indirect: u64,
    // metadonnees etendues cle valeur
    pub metas: Vec<(String, String)>,
}

impl Inode {
    pub fn vierge() -> Self {
        Self {
            type_noeud: TypeNoeud::Libre,
            nb_liens: 0,
            taille: 0,
            cree: 0,
            modifie: 0,
            blocs_directs: [0; NB_BLOCS_DIRECTS],
            bloc_indirect: 0,
            bloc_double_indirect: 0,
            metas: Vec::new(),
        }
    }

    pub fn nouveau(type_noeud: TypeNoeud, horodatage: u64) -> Self {
        Self {
            type_noeud,
            nb_liens: 1,
            taille: 0,
            cree: horodatage,
            modifie: horodatage,
            blocs_directs: [0; NB_BLOCS_DIRECTS],
            bloc_indirect: 0,
            bloc_double_indirect: 0,
            metas: Vec::new(),
        }
    }

    // serialisation en 256 octets
    pub fn serialiser(&self) -> ResultatFs<[u8; TAILLE_INODE]> {
        let mut tampon = [0u8; TAILLE_INODE];
        tampon[0] = self.type_noeud.vers_u8();
        tampon[4..8].copy_from_slice(&self.nb_liens.to_le_bytes());
        tampon[8..16].copy_from_slice(&self.taille.to_le_bytes());
        tampon[16..24].copy_from_slice(&self.cree.to_le_bytes());
        tampon[24..32].copy_from_slice(&self.modifie.to_le_bytes());
        for (i, bloc) in self.blocs_directs.iter().enumerate() {
            tampon[32 + i * 8..40 + i * 8].copy_from_slice(&bloc.to_le_bytes());
        }
        tampon[128..136].copy_from_slice(&self.bloc_indirect.to_le_bytes());
        tampon[136..144].copy_from_slice(&self.bloc_double_indirect.to_le_bytes());
        // metadonnees : longueur cle, longueur valeur, octets
        let mut curseur = 144;
        for (cle, valeur) in &self.metas {
            let c = cle.as_bytes();
            let v = valeur.as_bytes();
            if c.is_empty() || c.len() > 255 || v.len() > 255 {
                return Err(ErreurFs::MetaTropGrande);
            }
            if curseur + 2 + c.len() + v.len() > TAILLE_INODE {
                return Err(ErreurFs::MetaTropGrande);
            }
            tampon[curseur] = c.len() as u8;
            tampon[curseur + 1] = v.len() as u8;
            tampon[curseur + 2..curseur + 2 + c.len()].copy_from_slice(c);
            tampon[curseur + 2 + c.len()..curseur + 2 + c.len() + v.len()].copy_from_slice(v);
            curseur += 2 + c.len() + v.len();
        }
        Ok(tampon)
    }

    // lecture depuis 256 octets
    pub fn deserialiser(tampon: &[u8]) -> ResultatFs<Self> {
        let type_noeud = TypeNoeud::depuis_u8(tampon[0])?;
        let u64_a = |d: usize| u64::from_le_bytes(tampon[d..d + 8].try_into().unwrap());
        let mut blocs_directs = [0u64; NB_BLOCS_DIRECTS];
        for (i, bloc) in blocs_directs.iter_mut().enumerate() {
            *bloc = u64_a(32 + i * 8);
        }
        // lecture des metadonnees jusqu'a la cle vide
        let mut metas = Vec::new();
        let mut curseur = 144;
        while curseur + 2 <= TAILLE_INODE {
            let lc = tampon[curseur] as usize;
            if lc == 0 {
                break;
            }
            let lv = tampon[curseur + 1] as usize;
            if curseur + 2 + lc + lv > TAILLE_INODE {
                break;
            }
            let cle = String::from_utf8_lossy(&tampon[curseur + 2..curseur + 2 + lc]).into_owned();
            let valeur = String::from_utf8_lossy(
                &tampon[curseur + 2 + lc..curseur + 2 + lc + lv],
            )
            .into_owned();
            metas.push((cle, valeur));
            curseur += 2 + lc + lv;
        }
        Ok(Self {
            type_noeud,
            nb_liens: u32::from_le_bytes(tampon[4..8].try_into().unwrap()),
            taille: u64_a(8),
            cree: u64_a(16),
            modifie: u64_a(24),
            blocs_directs,
            bloc_indirect: u64_a(128),
            bloc_double_indirect: u64_a(136),
            metas,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_serialisation_inode() {
        let mut inode = Inode::nouveau(TypeNoeud::Fichier, 1234);
        inode.taille = 9999;
        inode.blocs_directs[0] = 42;
        inode.bloc_indirect = 77;
        inode.metas.push(("auteur".into(), "tsuky".into()));
        let tampon = inode.serialiser().unwrap();
        let relu = Inode::deserialiser(&tampon).unwrap();
        assert_eq!(relu.type_noeud, TypeNoeud::Fichier);
        assert_eq!(relu.taille, 9999);
        assert_eq!(relu.blocs_directs[0], 42);
        assert_eq!(relu.bloc_indirect, 77);
        assert_eq!(relu.metas, vec![("auteur".to_string(), "tsuky".to_string())]);
    }
}
