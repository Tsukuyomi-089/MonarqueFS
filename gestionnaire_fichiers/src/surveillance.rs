// detection des peripheriques de stockage et des volumes monarque

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

// signature de la table de partition monarque
const MAGIE_TABLE: [u8; 8] = *b"MONARQUE";

// description d'un peripherique de stockage
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoPeripherique {
    pub nom: String,
    pub chemin: PathBuf,
    pub modele: String,
    pub taille_octets: u64,
    pub amovible: bool,
    // formate avec la table monarque
    pub est_monarque: bool,
    // lisible avec les droits actuels
    pub accessible: bool,
}

// lecture d'un fichier sysfs en texte
fn lire_sys(chemin: &str) -> Option<String> {
    std::fs::read_to_string(chemin)
        .ok()
        .map(|t| t.trim().to_string())
}

// verification de la signature monarque en tete de support
pub fn est_monarque(chemin: &Path) -> Option<bool> {
    let mut fichier = File::open(chemin).ok()?;
    let mut magie = [0u8; 8];
    fichier.read_exact(&mut magie).ok()?;
    Some(magie == MAGIE_TABLE)
}

// enumeration des peripheriques bloc du systeme
pub fn lister_peripheriques() -> Vec<InfoPeripherique> {
    let mut peripheriques = Vec::new();
    let Ok(entrees) = std::fs::read_dir("/sys/block") else {
        return peripheriques;
    };
    for entree in entrees.flatten() {
        let nom = entree.file_name().to_string_lossy().into_owned();
        // exclusion des peripheriques virtuels
        if nom.starts_with("loop")
            || nom.starts_with("ram")
            || nom.starts_with("zram")
            || nom.starts_with("dm-")
            || nom.starts_with("sr")
        {
            continue;
        }
        let base = format!("/sys/block/{nom}");
        let amovible = lire_sys(&format!("{base}/removable")).as_deref() == Some("1");
        let taille_octets = lire_sys(&format!("{base}/size"))
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(0)
            * 512;
        if taille_octets == 0 {
            continue;
        }
        let modele = lire_sys(&format!("{base}/device/model"))
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "peripherique".to_string());
        let chemin = PathBuf::from(format!("/dev/{nom}"));
        let verdict = est_monarque(&chemin);
        peripheriques.push(InfoPeripherique {
            nom,
            chemin,
            modele,
            taille_octets,
            amovible,
            est_monarque: verdict.unwrap_or(false),
            accessible: verdict.is_some(),
        });
    }
    // amovibles d'abord, puis par nom
    peripheriques.sort_by(|a, b| b.amovible.cmp(&a.amovible).then(a.nom.cmp(&b.nom)));
    peripheriques
}

// peripheriques amovibles formates monarque
pub fn peripheriques_monarque() -> Vec<InfoPeripherique> {
    lister_peripheriques()
        .into_iter()
        .filter(|p| p.est_monarque)
        .collect()
}
