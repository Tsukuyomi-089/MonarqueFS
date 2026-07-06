// autorisation d'acces aux peripheriques via une fenetre graphique (polkit)

use std::path::PathBuf;
use std::process::Command;

// localisation du binaire monarque installe
fn binaire_monarque() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        let voisin = exe.with_file_name("monarque");
        if voisin.exists() {
            return voisin;
        }
    }
    // repli sur le chemin d'installation standard
    if let Ok(personnel) = std::env::var("HOME") {
        let installe = PathBuf::from(personnel).join(".local/bin/monarque");
        if installe.exists() {
            return installe;
        }
    }
    PathBuf::from("monarque")
}

// verification de la presence de pkexec
pub fn pkexec_disponible() -> bool {
    Command::new("sh")
        .args(["-c", "command -v pkexec"])
        .output()
        .map(|s| s.status.success())
        .unwrap_or(false)
}

// demande d'acces : installe la regle udev via une fenetre de mot de passe
// aucune commande a taper, polkit affiche une boite de dialogue graphique
pub fn autoriser_peripheriques() -> Result<(), String> {
    if !pkexec_disponible() {
        return Err(
            "pkexec introuvable — installez polkit ou executez : \
             sudo monarque installer_udev"
                .to_string(),
        );
    }
    let monarque = binaire_monarque();
    let sortie = Command::new("pkexec")
        .arg(&monarque)
        .arg("installer_udev")
        .output()
        .map_err(|e| format!("lancement de pkexec impossible : {e}"))?;
    if sortie.status.success() {
        Ok(())
    } else {
        let code = sortie.status.code().unwrap_or(-1);
        // pkexec renvoie 126 si l'utilisateur annule ou echoue l'authentification
        if code == 126 || code == 127 {
            Err("autorisation refusee ou annulee".to_string())
        } else {
            let details = String::from_utf8_lossy(&sortie.stderr);
            Err(format!("echec de l'autorisation : {}", details.trim()))
        }
    }
}
