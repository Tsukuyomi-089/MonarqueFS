// session de travail sur un volume monte

use crate::administration::ouvrir_partition;
use noyau_partition::VuePartition;
use systeme_fichiers::{monter, InfoEntree, ResultatFs, Statistiques, Volume};
use std::io::{Read, Write};
use std::path::Path;

pub struct Session {
    volume: Volume<VuePartition>,
}

impl Session {
    // montage d'un volume depuis une image disque
    pub fn ouvrir(chemin: &Path, index_partition: usize, phrase: &str) -> ResultatFs<Self> {
        let vue = ouvrir_partition(chemin, index_partition)?;
        Ok(Self {
            volume: monter(vue, phrase)?,
        })
    }

    pub fn lister(&mut self, chemin: &str) -> ResultatFs<Vec<InfoEntree>> {
        self.volume.lister(chemin)
    }

    pub fn creer_dossier(&mut self, chemin: &str) -> ResultatFs<()> {
        self.volume.creer_dossier(chemin)
    }

    pub fn lire_fichier(&mut self, chemin: &str) -> ResultatFs<Vec<u8>> {
        self.volume.lire_fichier(chemin)
    }

    pub fn ecrire_fichier(&mut self, chemin: &str, donnees: &[u8]) -> ResultatFs<()> {
        self.volume.ecrire_fichier(chemin, donnees)
    }

    pub fn supprimer(&mut self, chemin: &str) -> ResultatFs<()> {
        self.volume.supprimer(chemin)
    }

    pub fn renommer(&mut self, chemin: &str, nouveau_nom: &str) -> ResultatFs<()> {
        self.volume.renommer(chemin, nouveau_nom)
    }

    pub fn definir_meta(&mut self, chemin: &str, cle: &str, valeur: &str) -> ResultatFs<()> {
        self.volume.definir_meta(chemin, cle, valeur)
    }

    pub fn lire_metas(&mut self, chemin: &str) -> ResultatFs<Vec<(String, String)>> {
        self.volume.lire_metas(chemin)
    }

    pub fn statistiques(&self) -> Statistiques {
        self.volume.statistiques()
    }

    // ecriture d'un fichier depuis un flux (copie de taille illimitee)
    pub fn ecrire_flux(&mut self, chemin: &str, source: &mut dyn Read) -> ResultatFs<u64> {
        self.volume.ecrire_fichier_flux(chemin, source)
    }

    // lecture d'un fichier vers un flux
    pub fn lire_flux(&mut self, chemin: &str, sortie: &mut dyn Write) -> ResultatFs<()> {
        self.volume.lire_fichier_flux(chemin, sortie)
    }

    // taille d'un fichier sans lire son contenu
    pub fn taille_fichier(&mut self, chemin: &str) -> ResultatFs<u64> {
        self.volume.taille_fichier(chemin)
    }

    // import d'un fichier de l'hote vers le volume
    pub fn importer(&mut self, source: &Path, destination: &str) -> ResultatFs<()> {
        let donnees = std::fs::read(source)?;
        self.volume.ecrire_fichier(destination, &donnees)
    }

    // export d'un fichier du volume vers l'hote
    pub fn exporter(&mut self, source: &str, destination: &Path) -> ResultatFs<()> {
        let donnees = self.volume.lire_fichier(source)?;
        std::fs::write(destination, donnees)?;
        Ok(())
    }

    // fermeture propre de la session
    pub fn fermer(self) -> ResultatFs<()> {
        self.volume.demonter()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::administration::{
        ajouter_partition, creer_disque, formater_partition, lister_partitions,
    };
    use std::path::PathBuf;

    fn chemin_test() -> PathBuf {
        let mut chemin = std::env::temp_dir();
        chemin.push(format!("monarque_session_{}.img", std::process::id()));
        chemin
    }

    #[test]
    fn cycle_complet_disque_a_fichier() {
        let chemin = chemin_test();
        // disque de 32 mo avec deux partitions
        creer_disque(&chemin, 32 * 1024 * 1024).unwrap();
        ajouter_partition(&chemin, "systeme", 12 * 1024 * 1024).unwrap();
        ajouter_partition(&chemin, "donnees", 12 * 1024 * 1024).unwrap();
        assert_eq!(lister_partitions(&chemin).unwrap().len(), 2);

        formater_partition(&chemin, 1, "phrase donnees").unwrap();
        let mut session = Session::ouvrir(&chemin, 1, "phrase donnees").unwrap();
        session.creer_dossier("/projets").unwrap();
        session
            .ecrire_fichier("/projets/idee.txt", b"contenu chiffre")
            .unwrap();
        session.fermer().unwrap();

        // reouverture et verification
        let mut session = Session::ouvrir(&chemin, 1, "phrase donnees").unwrap();
        assert_eq!(
            session.lire_fichier("/projets/idee.txt").unwrap(),
            b"contenu chiffre"
        );
        session.fermer().unwrap();
        std::fs::remove_file(&chemin).ok();
    }
}
