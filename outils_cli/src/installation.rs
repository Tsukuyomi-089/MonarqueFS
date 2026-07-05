// installation de monarque sur le systeme (toutes distributions, via xdg)

use std::path::{Path, PathBuf};

type ResultatCli = Result<(), Box<dyn std::error::Error>>;

// dossier personnel de l'utilisateur
fn dossier_personnel() -> Result<PathBuf, Box<dyn std::error::Error>> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "variable HOME absente".into())
}

// copie d'un binaire voisin vers le dossier d'installation
fn copier_binaire(source_dir: &Path, nom: &str, destination: &Path) -> ResultatCli {
    let source = source_dir.join(nom);
    if !source.exists() {
        return Err(format!(
            "binaire introuvable : {} (compiler avec cargo build --release)",
            source.display()
        )
        .into());
    }
    let cible = destination.join(nom);
    std::fs::copy(&source, &cible)?;
    println!("  installe : {}", cible.display());
    Ok(())
}

// regle udev : acces aux peripheriques usb pour les utilisateurs locaux
const REGLE_UDEV: &str = r#"# monarquefs : acces des utilisateurs aux disques usb amovibles
KERNEL=="sd[a-z]", SUBSYSTEM=="block", ENV{ID_BUS}=="usb", MODE="0660", TAG+="uaccess"
"#;

pub fn installer() -> ResultatCli {
    let personnel = dossier_personnel()?;
    let dossier_exe = std::env::current_exe()?
        .parent()
        .ok_or("dossier du binaire introuvable")?
        .to_path_buf();

    // binaires dans ~/.local/bin
    let bin = personnel.join(".local/bin");
    std::fs::create_dir_all(&bin)?;
    println!("installation des binaires :");
    copier_binaire(&dossier_exe, "monarque", &bin)?;
    copier_binaire(&dossier_exe, "monarque_gui", &bin)?;
    copier_binaire(&dossier_exe, "monarque_veille", &bin)?;

    // demarrage automatique du demon de veille (norme xdg, toutes distributions)
    let autostart = personnel.join(".config/autostart");
    std::fs::create_dir_all(&autostart)?;
    let bureau_veille = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=MonarqueFS Veille\n\
         Comment=Detection des peripheriques MonarqueFS\n\
         Exec={}/monarque_veille\n\
         X-GNOME-Autostart-enabled=true\n",
        bin.display()
    );
    let chemin_veille = autostart.join("monarque_veille.desktop");
    std::fs::write(&chemin_veille, bureau_veille)?;
    println!("  demarrage automatique : {}", chemin_veille.display());

    // entree du gestionnaire dans le menu des applications
    let applications = personnel.join(".local/share/applications");
    std::fs::create_dir_all(&applications)?;
    let bureau_gui = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=MonarqueFS\n\
         Comment=Gestionnaire de fichiers chiffres MonarqueFS\n\
         Exec={}/monarque_gui\n\
         Categories=System;FileManager;\n\
         Terminal=false\n",
        bin.display()
    );
    let chemin_gui = applications.join("monarquefs.desktop");
    std::fs::write(&chemin_gui, bureau_gui)?;
    println!("  menu des applications : {}", chemin_gui.display());

    // regle udev pour l'acces aux cles usb sans droits administrateur
    let chemin_regle = Path::new("/etc/udev/rules.d/70-monarquefs.rules");
    match std::fs::write(chemin_regle, REGLE_UDEV) {
        Ok(()) => println!("  regle udev : {}", chemin_regle.display()),
        Err(_) => {
            println!("\nregle udev non installee (droits insuffisants).");
            println!("pour acceder aux cles usb sans etre administrateur, executer :");
            println!("  sudo monarque installer_udev");
        }
    }

    println!("\ninstallation terminee.");
    println!("le demon de veille demarrera a la prochaine session ;");
    println!("pour le lancer immediatement : {}/monarque_veille &", bin.display());
    Ok(())
}

// installation de la seule regle udev (a executer en administrateur)
pub fn installer_udev() -> ResultatCli {
    let chemin_regle = Path::new("/etc/udev/rules.d/70-monarquefs.rules");
    std::fs::write(chemin_regle, REGLE_UDEV)?;
    println!("regle udev installee : {}", chemin_regle.display());
    println!("rechargement : udevadm control --reload-rules (ou rebrancher la cle)");
    Ok(())
}
