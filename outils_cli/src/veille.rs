// demon de veille : detection des peripheriques monarque branches

use gestionnaire_fichiers::peripheriques_monarque;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

// intervalle entre deux analyses
const INTERVALLE: Duration = Duration::from_secs(2);

// localisation du gestionnaire graphique
fn chemin_gui() -> PathBuf {
    // meme dossier que le demon, sinon recherche dans le chemin systeme
    if let Ok(exe) = std::env::current_exe() {
        let voisin = exe.with_file_name("monarque_gui");
        if voisin.exists() {
            return voisin;
        }
    }
    PathBuf::from("monarque_gui")
}

fn main() {
    println!("monarque_veille : surveillance des peripheriques monarque");
    let gui = chemin_gui();
    // peripheriques deja traites, reinitialises au debranchement
    let mut connus: HashSet<PathBuf> = HashSet::new();
    loop {
        let presents = peripheriques_monarque();
        let chemins: HashSet<PathBuf> = presents.iter().map(|p| p.chemin.clone()).collect();
        // oubli des peripheriques debranches
        connus.retain(|c| chemins.contains(c));
        for peripherique in &presents {
            if connus.contains(&peripherique.chemin) {
                continue;
            }
            connus.insert(peripherique.chemin.clone());
            println!(
                "volume monarque detecte : {} ({})",
                peripherique.chemin.display(),
                peripherique.modele
            );
            // ouverture du gestionnaire de fichiers sur le peripherique
            let lancement = std::process::Command::new(&gui)
                .arg(&peripherique.chemin)
                .spawn();
            if let Err(e) = lancement {
                eprintln!("impossible de lancer le gestionnaire : {e}");
            }
        }
        std::thread::sleep(INTERVALLE);
    }
}
