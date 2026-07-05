// outil en ligne de commande monarque

mod commandes;
mod installation;

use std::process::ExitCode;

// aide generale
const AIDE: &str = "monarque - outil du systeme de stockage MonarqueFS

usage : monarque <commande> [arguments]

commandes disque :
  creer <image> <taille_mo>                  cree un disque logique
  partitionner <image> <nom> <taille_mo>     ajoute une partition
  supprimer_partition <image> <index>        retire une partition
  inspecter <image>                          affiche la table de partition
  peripheriques                              liste les peripheriques detectes
  preparer <support> <nom>                   table + partition + volume en une etape

commandes systeme :
  installer                                  installe binaires, veille et menus
  installer_udev                             installe la regle udev (administrateur)

commandes volume :
  formater <image> <index>                   formate une partition en MonarqueFS
  etat <image> <index>                       statistiques du volume monte

commandes fichiers :
  lister <image> <index> [chemin]            liste un dossier
  creer_dossier <image> <index> <chemin>     cree un dossier
  importer <image> <index> <source> <dest>   copie un fichier hote vers le volume
  exporter <image> <index> <source> <dest>   copie un fichier du volume vers l'hote
  afficher <image> <index> <chemin>          affiche un fichier texte
  effacer <image> <index> <chemin>           supprime un fichier ou dossier vide
  renommer <image> <index> <chemin> <nom>    renomme une entree
  meta <image> <index> <chemin> [cle valeur] lit ou definit une metadonnee

la phrase secrete est lue depuis MONARQUE_PHRASE ou demandee au clavier";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let Some(commande) = arguments.first() else {
        println!("{AIDE}");
        return ExitCode::FAILURE;
    };
    if commande == "aide" || commande == "--aide" || commande == "--help" {
        println!("{AIDE}");
        return ExitCode::SUCCESS;
    }
    let resultat = match commande.as_str() {
        "installer" => installation::installer(),
        "installer_udev" => installation::installer_udev(),
        _ => commandes::executer(commande, &arguments[1..]),
    };
    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("erreur : {erreur}");
            ExitCode::FAILURE
        }
    }
}
