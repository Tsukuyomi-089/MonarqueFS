// mise a jour de l'application depuis le depot officiel

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

// depot officiel du projet
pub const DEPOT: &str = "https://github.com/Tsukuyomi-089/MonarqueFS";
// version compilee de l'application
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// progression envoyee a l'interface
pub enum EtapeMaj {
    Info(String),
    // chemin du nouveau gestionnaire graphique en cas de succes
    Terminee(Result<PathBuf, String>),
}

fn dossier_personnel() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "variable HOME absente".to_string())
}

// execution d'un programme avec remontee des erreurs
fn executer(dossier: Option<&Path>, programme: &str, arguments: &[&str]) -> Result<(), String> {
    let mut commande = Command::new(programme);
    if let Some(d) = dossier {
        commande.current_dir(d);
    }
    let sortie = commande
        .args(arguments)
        .output()
        .map_err(|e| format!("{programme} indisponible : {e}"))?;
    if !sortie.status.success() {
        let erreurs = String::from_utf8_lossy(&sortie.stderr);
        let dernieres: Vec<&str> = erreurs.lines().rev().take(5).collect();
        return Err(format!(
            "echec de {programme} {} :\n{}",
            arguments.first().unwrap_or(&""),
            dernieres.into_iter().rev().collect::<Vec<_>>().join("\n")
        ));
    }
    Ok(())
}

// deroulement complet, a executer dans un fil dedie
pub fn mettre_a_jour(emetteur: Sender<EtapeMaj>) {
    let resultat = derouler(&emetteur);
    emetteur.send(EtapeMaj::Terminee(resultat)).ok();
}

fn derouler(emetteur: &Sender<EtapeMaj>) -> Result<PathBuf, String> {
    let info = |texte: &str| {
        emetteur.send(EtapeMaj::Info(texte.to_string())).ok();
    };
    let personnel = dossier_personnel()?;
    let source = personnel.join(".local/share/monarquefs/source");
    std::fs::create_dir_all(source.parent().unwrap()).map_err(|e| e.to_string())?;

    // recuperation du code : clonage initial puis tirages
    if source.join(".git").exists() {
        info("récupération des dernières modifications…");
        executer(Some(&source), "git", &["pull", "--ff-only"])?;
    } else {
        info("téléchargement du dépôt…");
        executer(
            None,
            "git",
            &[
                "clone",
                "--depth",
                "1",
                DEPOT,
                source.to_str().ok_or("chemin de source invalide")?,
            ],
        )?;
    }

    info("compilation optimisée — cela peut prendre quelques minutes…");
    executer(Some(&source), "cargo", &["build", "--release", "--workspace"])?;

    info("installation des nouveaux binaires…");
    let bin = personnel.join(".local/bin");
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    for nom in ["monarque", "monarque_gui", "monarque_veille"] {
        let origine = source.join("target/release").join(nom);
        let cible = bin.join(nom);
        // suppression prealable : un binaire en cours d'execution ne s'ecrase pas
        std::fs::remove_file(&cible).ok();
        std::fs::copy(&origine, &cible).map_err(|e| format!("copie de {nom} : {e}"))?;
    }
    info("mise à jour installée");
    Ok(bin.join("monarque_gui"))
}

// redemarrage de l'application et du demon de veille
pub fn relancer(chemin_gui: &Path) -> ! {
    // arret de l'ancien demon puis relance de la nouvelle version
    Command::new("pkill").args(["-x", "monarque_veille"]).status().ok();
    Command::new(chemin_gui.with_file_name("monarque_veille")).spawn().ok();
    Command::new(chemin_gui).spawn().ok();
    std::process::exit(0);
}
