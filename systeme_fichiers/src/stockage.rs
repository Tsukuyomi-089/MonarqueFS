// abstraction du support de stockage

use crate::erreurs::ResultatFs;
use noyau_partition::VuePartition;

// support d'octets adressable par decalage
pub trait Stockage: Send {
    fn lire_a(&mut self, decalage: u64, tampon: &mut [u8]) -> ResultatFs<()>;
    fn ecrire_a(&mut self, decalage: u64, donnees: &[u8]) -> ResultatFs<()>;
    fn taille_octets(&self) -> u64;
    fn synchroniser(&mut self) -> ResultatFs<()>;
}

// une partition est un support de stockage
impl Stockage for VuePartition {
    fn lire_a(&mut self, decalage: u64, tampon: &mut [u8]) -> ResultatFs<()> {
        VuePartition::lire_a(self, decalage, tampon)?;
        Ok(())
    }

    fn ecrire_a(&mut self, decalage: u64, donnees: &[u8]) -> ResultatFs<()> {
        VuePartition::ecrire_a(self, decalage, donnees)?;
        Ok(())
    }

    fn taille_octets(&self) -> u64 {
        VuePartition::taille_octets(self)
    }

    fn synchroniser(&mut self) -> ResultatFs<()> {
        VuePartition::synchroniser(self)?;
        Ok(())
    }
}

// support en memoire pour les tests
pub struct StockageMemoire {
    donnees: Vec<u8>,
}

impl StockageMemoire {
    pub fn nouveau(taille: usize) -> Self {
        Self {
            donnees: vec![0u8; taille],
        }
    }

    // acces brut pour verifier l'absence de donnees en clair
    pub fn octets_bruts(&self) -> &[u8] {
        &self.donnees
    }
}

impl Stockage for StockageMemoire {
    fn lire_a(&mut self, decalage: u64, tampon: &mut [u8]) -> ResultatFs<()> {
        let debut = decalage as usize;
        tampon.copy_from_slice(&self.donnees[debut..debut + tampon.len()]);
        Ok(())
    }

    fn ecrire_a(&mut self, decalage: u64, donnees: &[u8]) -> ResultatFs<()> {
        let debut = decalage as usize;
        self.donnees[debut..debut + donnees.len()].copy_from_slice(donnees);
        Ok(())
    }

    fn taille_octets(&self) -> u64 {
        self.donnees.len() as u64
    }

    fn synchroniser(&mut self) -> ResultatFs<()> {
        Ok(())
    }
}
