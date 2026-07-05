// bitmap d'allocation des blocs de donnees

use crate::bloc::{lire_bloc, ecrire_bloc, TAILLE_BLOC};
use crate::chiffrement::AlgorithmeChiffrement;
use crate::erreurs::{ErreurFs, ResultatFs};
use crate::stockage::Stockage;
use crate::superbloc::Superbloc;
use std::collections::HashSet;

pub struct Bitmap {
    // un bit par bloc de donnees
    bits: Vec<u8>,
    // blocs de bitmap modifies a reecrire
    sales: HashSet<usize>,
    nb_blocs_donnees: u64,
    // indice de depart pour accelerer la recherche
    indice_recherche: u64,
}

impl Bitmap {
    // bitmap vierge au formatage
    pub fn vierge(superbloc: &Superbloc) -> Self {
        let taille = (superbloc.nb_blocs_bitmap as usize) * TAILLE_BLOC;
        Self {
            bits: vec![0u8; taille],
            sales: (0..superbloc.nb_blocs_bitmap as usize).collect(),
            nb_blocs_donnees: superbloc.nb_blocs_donnees,
            indice_recherche: 0,
        }
    }

    // chargement depuis le volume
    pub fn charger(
        stockage: &mut dyn Stockage,
        algo: &dyn AlgorithmeChiffrement,
        cle: &[u8; 32],
        superbloc: &Superbloc,
    ) -> ResultatFs<Self> {
        let mut bits = Vec::with_capacity((superbloc.nb_blocs_bitmap as usize) * TAILLE_BLOC);
        for i in 0..superbloc.nb_blocs_bitmap {
            let bloc = lire_bloc(stockage, algo, cle, superbloc.bloc_debut_bitmap + i)?;
            bits.extend_from_slice(&bloc);
        }
        Ok(Self {
            bits,
            sales: HashSet::new(),
            nb_blocs_donnees: superbloc.nb_blocs_donnees,
            indice_recherche: 0,
        })
    }

    // allocation du premier bloc libre, index relatif a la zone de donnees
    pub fn allouer(&mut self) -> ResultatFs<u64> {
        for pas in 0..self.nb_blocs_donnees {
            let idx = (self.indice_recherche + pas) % self.nb_blocs_donnees;
            let octet = (idx / 8) as usize;
            let bit = (idx % 8) as u8;
            if self.bits[octet] & (1 << bit) == 0 {
                self.bits[octet] |= 1 << bit;
                self.sales.insert(octet / TAILLE_BLOC);
                self.indice_recherche = (idx + 1) % self.nb_blocs_donnees;
                return Ok(idx);
            }
        }
        Err(ErreurFs::EspacePlein)
    }

    // liberation d'un bloc
    pub fn liberer(&mut self, idx: u64) {
        let octet = (idx / 8) as usize;
        let bit = (idx % 8) as u8;
        self.bits[octet] &= !(1 << bit);
        self.sales.insert(octet / TAILLE_BLOC);
        self.indice_recherche = self.indice_recherche.min(idx);
    }

    // nombre de blocs libres
    pub fn nb_libres(&self) -> u64 {
        let mut occupes: u64 = 0;
        for i in 0..self.nb_blocs_donnees {
            let octet = (i / 8) as usize;
            if self.bits[octet] & (1 << (i % 8)) != 0 {
                occupes += 1;
            }
        }
        self.nb_blocs_donnees - occupes
    }

    // ecriture des blocs de bitmap modifies
    pub fn purger(
        &mut self,
        stockage: &mut dyn Stockage,
        algo: &dyn AlgorithmeChiffrement,
        cle: &[u8; 32],
        superbloc: &Superbloc,
    ) -> ResultatFs<()> {
        for &i in &self.sales {
            let debut = i * TAILLE_BLOC;
            ecrire_bloc(
                stockage,
                algo,
                cle,
                superbloc.bloc_debut_bitmap + i as u64,
                &self.bits[debut..debut + TAILLE_BLOC],
            )?;
        }
        self.sales.clear();
        Ok(())
    }
}
