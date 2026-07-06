// detection des peripheriques de stockage et des volumes monarque

use std::collections::HashSet;
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
    // disque hebergeant le systeme d'exploitation : protege
    pub est_systeme: bool,
}

impl InfoPeripherique {
    // un support est formatable s'il n'heberge pas le systeme
    pub fn formatable(&self) -> bool {
        !self.est_systeme
    }
}

// lecture d'un fichier sysfs en texte
fn lire_sys(chemin: &str) -> Option<String> {
    std::fs::read_to_string(chemin)
        .ok()
        .map(|t| t.trim().to_string())
}

// disque physique portant le nom de partition donne, ex : nvme0n1p3 -> nvme0n1
fn disque_parent(partition: &str) -> String {
    // nvme et mmc utilisent le suffixe pN
    if let Some(pos) = partition.rfind('p') {
        if partition[pos + 1..].chars().all(|c| c.is_ascii_digit())
            && !partition[pos + 1..].is_empty()
            && (partition.starts_with("nvme") || partition.starts_with("mmcblk"))
        {
            return partition[..pos].to_string();
        }
    }
    // sda1 -> sda : on retire les chiffres finaux
    partition.trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

// disques physiques hebergeant le systeme (racine et amorcage)
fn disques_systeme() -> HashSet<String> {
    let mut disques = HashSet::new();
    let Ok(montages) = std::fs::read_to_string("/proc/mounts") else {
        return disques;
    };
    for ligne in montages.lines() {
        let mut champs = ligne.split_whitespace();
        let (Some(source), Some(point)) = (champs.next(), champs.next()) else {
            continue;
        };
        // seuls les points sensibles nous interessent
        if point != "/" && point != "/boot" && !point.starts_with("/boot/") {
            continue;
        }
        if let Some(nom) = source.strip_prefix("/dev/") {
            disques.insert(disque_parent(nom));
            // resolution des peripheriques mappes (luks, lvm) vers leurs disques physiques
            for esclave in esclaves_physiques(nom) {
                disques.insert(esclave);
            }
        }
    }
    disques
}

// disques physiques sous-jacents d'un peripherique mappe (dm, md)
fn esclaves_physiques(nom: &str) -> Vec<String> {
    let mut resultat = Vec::new();
    let chemin = format!("/sys/class/block/{nom}/slaves");
    if let Ok(entrees) = std::fs::read_dir(&chemin) {
        for entree in entrees.flatten() {
            let esclave = entree.file_name().to_string_lossy().into_owned();
            resultat.push(disque_parent(&esclave));
        }
    }
    resultat
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
    let systeme = disques_systeme();
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
        let est_systeme = systeme.contains(&nom);
        peripheriques.push(InfoPeripherique {
            est_systeme,
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
