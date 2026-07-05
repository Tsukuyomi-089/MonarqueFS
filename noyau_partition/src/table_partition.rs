// table de partition proprietaire monarque

use crate::disque::{DisqueLogique, TAILLE_SECTEUR};
use crate::erreurs::{ErreurNoyau, Resultat};

// signature de la table
pub const MAGIE_TABLE: [u8; 8] = *b"MONARQUE";
pub const VERSION_TABLE: u16 = 1;
// nombre maximal de partitions
pub const MAX_PARTITIONS: usize = 32;
// taille d'une entree serialisee
pub const TAILLE_ENTREE: usize = 64;
// premier secteur disponible pour les donnees
pub const SECTEUR_DEBUT_DONNEES: u64 = 8;
// longueur maximale du nom
pub const LONGUEUR_NOM_MAX: usize = 23;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePartition {
    Libre,
    MonarqueFs,
    Brute,
}

impl TypePartition {
    fn vers_u32(self) -> u32 {
        match self {
            TypePartition::Libre => 0,
            TypePartition::MonarqueFs => 1,
            TypePartition::Brute => 2,
        }
    }

    fn depuis_u32(v: u32) -> Resultat<Self> {
        match v {
            0 => Ok(TypePartition::Libre),
            1 => Ok(TypePartition::MonarqueFs),
            2 => Ok(TypePartition::Brute),
            autre => Err(ErreurNoyau::TableInvalide(format!(
                "type de partition inconnu : {autre}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntreePartition {
    pub nom: String,
    pub debut_secteur: u64,
    pub nb_secteurs: u64,
    pub type_partition: TypePartition,
    pub drapeaux: u32,
    pub id: u64,
}

impl EntreePartition {
    // serialisation en 64 octets
    fn serialiser(&self) -> [u8; TAILLE_ENTREE] {
        let mut tampon = [0u8; TAILLE_ENTREE];
        let nom = self.nom.as_bytes();
        tampon[..nom.len().min(LONGUEUR_NOM_MAX)]
            .copy_from_slice(&nom[..nom.len().min(LONGUEUR_NOM_MAX)]);
        tampon[24..32].copy_from_slice(&self.debut_secteur.to_le_bytes());
        tampon[32..40].copy_from_slice(&self.nb_secteurs.to_le_bytes());
        tampon[40..44].copy_from_slice(&self.type_partition.vers_u32().to_le_bytes());
        tampon[44..48].copy_from_slice(&self.drapeaux.to_le_bytes());
        tampon[48..56].copy_from_slice(&self.id.to_le_bytes());
        tampon
    }

    // lecture depuis 64 octets
    fn deserialiser(tampon: &[u8]) -> Resultat<Self> {
        let fin_nom = tampon[..24].iter().position(|&o| o == 0).unwrap_or(24);
        let nom = String::from_utf8(tampon[..fin_nom].to_vec())
            .map_err(|_| ErreurNoyau::TableInvalide("nom non utf8".into()))?;
        let u64_a = |debut: usize| {
            u64::from_le_bytes(tampon[debut..debut + 8].try_into().unwrap())
        };
        let debut_secteur = u64_a(24);
        let nb_secteurs = u64_a(32);
        let id = u64_a(48);
        let type_partition =
            TypePartition::depuis_u32(u32::from_le_bytes(tampon[40..44].try_into().unwrap()))?;
        let drapeaux = u32::from_le_bytes(tampon[44..48].try_into().unwrap());
        Ok(Self {
            nom,
            debut_secteur,
            nb_secteurs,
            type_partition,
            drapeaux,
            id,
        })
    }

    pub fn taille_octets(&self) -> u64 {
        self.nb_secteurs * TAILLE_SECTEUR
    }
}

pub struct TablePartition {
    pub version: u16,
    pub entrees: Vec<EntreePartition>,
    prochain_id: u64,
}

impl TablePartition {
    // ecriture d'une table vierge sur le disque
    pub fn initialiser(disque: &mut DisqueLogique) -> Resultat<Self> {
        let table = Self {
            version: VERSION_TABLE,
            entrees: Vec::new(),
            prochain_id: 1,
        };
        table.sauvegarder(disque)?;
        Ok(table)
    }

    // chargement de la table depuis le disque
    pub fn charger(disque: &mut DisqueLogique) -> Resultat<Self> {
        let mut en_tete = [0u8; TAILLE_SECTEUR as usize];
        disque.lire_a(0, &mut en_tete)?;
        if en_tete[..8] != MAGIE_TABLE {
            return Err(ErreurNoyau::TableInvalide("signature absente".into()));
        }
        let version = u16::from_le_bytes(en_tete[8..10].try_into().unwrap());
        let nb_entrees = u16::from_le_bytes(en_tete[10..12].try_into().unwrap()) as usize;
        let prochain_id = u64::from_le_bytes(en_tete[16..24].try_into().unwrap());
        if nb_entrees > MAX_PARTITIONS {
            return Err(ErreurNoyau::TableInvalide("trop d'entrees".into()));
        }
        // lecture des entrees dans les secteurs suivants
        let mut zone = vec![0u8; MAX_PARTITIONS * TAILLE_ENTREE];
        disque.lire_a(TAILLE_SECTEUR, &mut zone)?;
        let mut entrees = Vec::with_capacity(nb_entrees);
        for i in 0..nb_entrees {
            let debut = i * TAILLE_ENTREE;
            entrees.push(EntreePartition::deserialiser(&zone[debut..debut + TAILLE_ENTREE])?);
        }
        Ok(Self {
            version,
            entrees,
            prochain_id,
        })
    }

    // ecriture de la table sur le disque
    pub fn sauvegarder(&self, disque: &mut DisqueLogique) -> Resultat<()> {
        let mut en_tete = [0u8; TAILLE_SECTEUR as usize];
        en_tete[..8].copy_from_slice(&MAGIE_TABLE);
        en_tete[8..10].copy_from_slice(&self.version.to_le_bytes());
        en_tete[10..12].copy_from_slice(&(self.entrees.len() as u16).to_le_bytes());
        en_tete[16..24].copy_from_slice(&self.prochain_id.to_le_bytes());
        disque.ecrire_a(0, &en_tete)?;
        let mut zone = vec![0u8; MAX_PARTITIONS * TAILLE_ENTREE];
        for (i, entree) in self.entrees.iter().enumerate() {
            let debut = i * TAILLE_ENTREE;
            zone[debut..debut + TAILLE_ENTREE].copy_from_slice(&entree.serialiser());
        }
        disque.ecrire_a(TAILLE_SECTEUR, &zone)?;
        disque.synchroniser()?;
        Ok(())
    }

    // ajout d'une partition par recherche du premier espace libre
    pub fn ajouter(
        &mut self,
        disque: &mut DisqueLogique,
        nom: &str,
        nb_secteurs: u64,
        type_partition: TypePartition,
    ) -> Resultat<usize> {
        if nom.is_empty() || nom.len() > LONGUEUR_NOM_MAX {
            return Err(ErreurNoyau::NomInvalide);
        }
        if self.entrees.len() >= MAX_PARTITIONS {
            return Err(ErreurNoyau::TablePleine);
        }
        let debut_secteur = self.chercher_espace(disque, nb_secteurs)?;
        let entree = EntreePartition {
            nom: nom.to_string(),
            debut_secteur,
            nb_secteurs,
            type_partition,
            drapeaux: 0,
            id: self.prochain_id,
        };
        self.prochain_id += 1;
        self.entrees.push(entree);
        self.entrees.sort_by_key(|e| e.debut_secteur);
        self.sauvegarder(disque)?;
        Ok(self
            .entrees
            .iter()
            .position(|e| e.id == self.prochain_id - 1)
            .unwrap())
    }

    // suppression d'une partition par index
    pub fn supprimer(&mut self, disque: &mut DisqueLogique, index: usize) -> Resultat<()> {
        if index >= self.entrees.len() {
            return Err(ErreurNoyau::Introuvable);
        }
        self.entrees.remove(index);
        self.sauvegarder(disque)
    }

    // recherche du premier intervalle libre assez grand
    fn chercher_espace(&self, disque: &DisqueLogique, nb_secteurs: u64) -> Resultat<u64> {
        let fin_disque = disque.nb_secteurs();
        let mut curseur = SECTEUR_DEBUT_DONNEES;
        // les entrees sont triees par debut
        for entree in &self.entrees {
            if entree.debut_secteur >= curseur + nb_secteurs {
                return Ok(curseur);
            }
            curseur = curseur.max(entree.debut_secteur + entree.nb_secteurs);
        }
        if curseur + nb_secteurs <= fin_disque {
            Ok(curseur)
        } else {
            Err(ErreurNoyau::EspaceInsuffisant)
        }
    }
}

// vue bornee sur une partition, possede le disque
pub struct VuePartition {
    disque: DisqueLogique,
    debut_octets: u64,
    taille_octets: u64,
}

impl VuePartition {
    pub fn nouvelle(disque: DisqueLogique, entree: &EntreePartition) -> Self {
        Self {
            disque,
            debut_octets: entree.debut_secteur * TAILLE_SECTEUR,
            taille_octets: entree.taille_octets(),
        }
    }

    pub fn taille_octets(&self) -> u64 {
        self.taille_octets
    }

    // lecture bornee a la partition
    pub fn lire_a(&mut self, decalage: u64, tampon: &mut [u8]) -> Resultat<()> {
        if decalage + tampon.len() as u64 > self.taille_octets {
            return Err(ErreurNoyau::HorsBornes);
        }
        self.disque.lire_a(self.debut_octets + decalage, tampon)
    }

    // ecriture bornee a la partition
    pub fn ecrire_a(&mut self, decalage: u64, donnees: &[u8]) -> Resultat<()> {
        if decalage + donnees.len() as u64 > self.taille_octets {
            return Err(ErreurNoyau::HorsBornes);
        }
        self.disque.ecrire_a(self.debut_octets + decalage, donnees)
    }

    pub fn synchroniser(&mut self) -> Resultat<()> {
        self.disque.synchroniser()
    }

    // restitue le disque sous jacent
    pub fn liberer(self) -> DisqueLogique {
        self.disque
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn chemin_test(nom: &str) -> PathBuf {
        let mut chemin = std::env::temp_dir();
        chemin.push(format!("monarque_test_{nom}_{}.img", std::process::id()));
        chemin
    }

    #[test]
    fn cycle_table_partition() {
        let chemin = chemin_test("table");
        let mut disque = DisqueLogique::creer(&chemin, 16 * 1024 * 1024).unwrap();
        let mut table = TablePartition::initialiser(&mut disque).unwrap();
        table
            .ajouter(&mut disque, "volume_a", 1024, TypePartition::MonarqueFs)
            .unwrap();
        table
            .ajouter(&mut disque, "volume_b", 2048, TypePartition::MonarqueFs)
            .unwrap();
        // rechargement et verification
        let table2 = TablePartition::charger(&mut disque).unwrap();
        assert_eq!(table2.entrees.len(), 2);
        assert_eq!(table2.entrees[0].nom, "volume_a");
        assert_eq!(table2.entrees[1].nom, "volume_b");
        // les partitions ne se chevauchent pas
        assert!(
            table2.entrees[0].debut_secteur + table2.entrees[0].nb_secteurs
                <= table2.entrees[1].debut_secteur
        );
        std::fs::remove_file(&chemin).ok();
    }

    #[test]
    fn suppression_et_reutilisation_espace() {
        let chemin = chemin_test("suppression");
        let mut disque = DisqueLogique::creer(&chemin, 8 * 1024 * 1024).unwrap();
        let mut table = TablePartition::initialiser(&mut disque).unwrap();
        table
            .ajouter(&mut disque, "a", 1000, TypePartition::Brute)
            .unwrap();
        table
            .ajouter(&mut disque, "b", 1000, TypePartition::Brute)
            .unwrap();
        table.supprimer(&mut disque, 0).unwrap();
        // le trou laisse par a est reutilise
        let index = table
            .ajouter(&mut disque, "c", 500, TypePartition::Brute)
            .unwrap();
        assert_eq!(table.entrees[index].debut_secteur, SECTEUR_DEBUT_DONNEES);
        std::fs::remove_file(&chemin).ok();
    }

    #[test]
    fn vue_partition_bornee() {
        let chemin = chemin_test("vue");
        let mut disque = DisqueLogique::creer(&chemin, 8 * 1024 * 1024).unwrap();
        let mut table = TablePartition::initialiser(&mut disque).unwrap();
        table
            .ajouter(&mut disque, "v", 100, TypePartition::Brute)
            .unwrap();
        let entree = table.entrees[0].clone();
        let mut vue = VuePartition::nouvelle(disque, &entree);
        vue.ecrire_a(0, b"bonjour").unwrap();
        let mut tampon = [0u8; 7];
        vue.lire_a(0, &mut tampon).unwrap();
        assert_eq!(&tampon, b"bonjour");
        // acces hors bornes refuse
        assert!(vue.ecrire_a(entree.taille_octets() - 1, &[0, 0]).is_err());
        std::fs::remove_file(&chemin).ok();
    }
}
