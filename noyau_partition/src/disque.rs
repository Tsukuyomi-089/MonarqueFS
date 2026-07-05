// gestion du disque logique (fichier image)

use crate::erreurs::{ErreurNoyau, Resultat};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

// taille d'un secteur en octets
pub const TAILLE_SECTEUR: u64 = 512;

pub struct DisqueLogique {
    fichier: File,
    taille_octets: u64,
}

// taille reelle du support, y compris peripherique bloc
fn taille_support(chemin: &Path, fichier: &File) -> Resultat<u64> {
    let taille = fichier.metadata()?.len();
    if taille > 0 {
        return Ok(taille);
    }
    // peripherique bloc : taille en secteurs via sysfs
    if let Some(nom) = chemin.file_name().and_then(|n| n.to_str()) {
        let chemin_sys = format!("/sys/class/block/{nom}/size");
        if let Ok(texte) = std::fs::read_to_string(&chemin_sys) {
            if let Ok(secteurs) = texte.trim().parse::<u64>() {
                return Ok(secteurs * TAILLE_SECTEUR);
            }
        }
    }
    Ok(taille)
}

impl DisqueLogique {
    // creation d'un disque logique vierge
    pub fn creer(chemin: &Path, taille_octets: u64) -> Resultat<Self> {
        let taille_alignee = taille_octets - (taille_octets % TAILLE_SECTEUR);
        if taille_alignee < TAILLE_SECTEUR * 64 {
            return Err(ErreurNoyau::EspaceInsuffisant);
        }
        let fichier = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(chemin)?;
        fichier.set_len(taille_alignee)?;
        Ok(Self {
            fichier,
            taille_octets: taille_alignee,
        })
    }

    // ouverture d'un disque existant, image ou peripherique bloc
    pub fn ouvrir(chemin: &Path) -> Resultat<Self> {
        let fichier = OpenOptions::new().read(true).write(true).open(chemin)?;
        let taille_octets = taille_support(chemin, &fichier)?;
        Ok(Self {
            fichier,
            taille_octets,
        })
    }

    // ouverture en lecture seule pour l'inspection
    pub fn ouvrir_lecture(chemin: &Path) -> Resultat<Self> {
        let fichier = OpenOptions::new().read(true).open(chemin)?;
        let taille_octets = taille_support(chemin, &fichier)?;
        Ok(Self {
            fichier,
            taille_octets,
        })
    }

    pub fn taille_octets(&self) -> u64 {
        self.taille_octets
    }

    pub fn nb_secteurs(&self) -> u64 {
        self.taille_octets / TAILLE_SECTEUR
    }

    // lecture brute a un decalage donne
    pub fn lire_a(&mut self, decalage: u64, tampon: &mut [u8]) -> Resultat<()> {
        if decalage + tampon.len() as u64 > self.taille_octets {
            return Err(ErreurNoyau::HorsBornes);
        }
        self.fichier.seek(SeekFrom::Start(decalage))?;
        self.fichier.read_exact(tampon)?;
        Ok(())
    }

    // ecriture brute a un decalage donne
    pub fn ecrire_a(&mut self, decalage: u64, donnees: &[u8]) -> Resultat<()> {
        if decalage + donnees.len() as u64 > self.taille_octets {
            return Err(ErreurNoyau::HorsBornes);
        }
        self.fichier.seek(SeekFrom::Start(decalage))?;
        self.fichier.write_all(donnees)?;
        Ok(())
    }

    // synchronisation sur le support physique
    pub fn synchroniser(&mut self) -> Resultat<()> {
        self.fichier.sync_all()?;
        Ok(())
    }
}
