// execution des commandes du cli

use gestionnaire_fichiers::{
    ajouter_partition, creer_disque, formater_partition, lister_partitions,
    lister_peripheriques, preparer_support, supprimer_monarque, supprimer_partition, Session,
    TypeNoeud,
};
use std::io::{BufRead, Write};
use std::path::Path;

type ResultatCli = Result<(), Box<dyn std::error::Error>>;

// lecture de la phrase secrete
fn obtenir_phrase() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(phrase) = std::env::var("MONARQUE_PHRASE") {
        return Ok(phrase);
    }
    print!("phrase secrete : ");
    std::io::stdout().flush()?;
    let mut phrase = String::new();
    std::io::stdin().lock().read_line(&mut phrase)?;
    Ok(phrase.trim_end_matches('\n').to_string())
}

// analyse d'un argument obligatoire
fn argument<'a>(arguments: &'a [String], position: usize, nom: &str) -> Result<&'a str, String> {
    arguments
        .get(position)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("argument manquant : {nom}"))
}

fn taille_mo(texte: &str) -> Result<u64, String> {
    texte
        .parse::<u64>()
        .map(|mo| mo * 1024 * 1024)
        .map_err(|_| format!("taille invalide : {texte}"))
}

fn index_partition(texte: &str) -> Result<usize, String> {
    texte
        .parse::<usize>()
        .map_err(|_| format!("index invalide : {texte}"))
}

// ouverture d'une session avec phrase
fn ouvrir_session(arguments: &[String]) -> Result<Session, Box<dyn std::error::Error>> {
    let image = argument(arguments, 0, "image")?;
    let index = index_partition(argument(arguments, 1, "index")?)?;
    let phrase = obtenir_phrase()?;
    Ok(Session::ouvrir(Path::new(image), index, &phrase)?)
}

// affichage lisible d'une taille
fn taille_lisible(octets: u64) -> String {
    if octets >= 1024 * 1024 {
        format!("{:.1} Mo", octets as f64 / (1024.0 * 1024.0))
    } else if octets >= 1024 {
        format!("{:.1} Ko", octets as f64 / 1024.0)
    } else {
        format!("{octets} o")
    }
}

pub fn executer(commande: &str, arguments: &[String]) -> ResultatCli {
    match commande {
        "creer" => {
            let image = argument(arguments, 0, "image")?;
            let taille = taille_mo(argument(arguments, 1, "taille_mo")?)?;
            creer_disque(Path::new(image), taille)?;
            println!("disque cree : {image} ({})", taille_lisible(taille));
        }
        "partitionner" => {
            let image = argument(arguments, 0, "image")?;
            let nom = argument(arguments, 1, "nom")?;
            let taille = taille_mo(argument(arguments, 2, "taille_mo")?)?;
            let index = ajouter_partition(Path::new(image), nom, taille)?;
            println!("partition {index} creee : {nom} ({})", taille_lisible(taille));
        }
        "supprimer_partition" => {
            let image = argument(arguments, 0, "image")?;
            let index = index_partition(argument(arguments, 1, "index")?)?;
            supprimer_partition(Path::new(image), index)?;
            println!("partition {index} supprimee");
        }
        "inspecter" => {
            let image = argument(arguments, 0, "image")?;
            let partitions = lister_partitions(Path::new(image))?;
            println!("table de partition monarque : {} partition(s)", partitions.len());
            for p in partitions {
                println!(
                    "  [{}] {:<24} debut secteur {:<10} {}",
                    p.index,
                    p.nom,
                    p.debut_secteur,
                    taille_lisible(p.taille_octets)
                );
            }
        }
        "peripheriques" => {
            let peripheriques = lister_peripheriques();
            if peripheriques.is_empty() {
                println!("aucun peripherique detecte");
            }
            for p in peripheriques {
                let etat = if p.est_systeme {
                    "disque systeme (protege)"
                } else if !p.accessible {
                    "acces refuse (voir : monarque installer_udev)"
                } else if p.est_monarque {
                    "MonarqueFS"
                } else {
                    "non formate monarque"
                };
                let amovible = if p.amovible { "amovible" } else { "interne" };
                println!(
                    "{:<12} {:<24} {:>10}  {:<8} {}",
                    p.chemin.display(),
                    p.modele,
                    taille_lisible(p.taille_octets),
                    amovible,
                    etat
                );
            }
        }
        "preparer" => {
            let support = argument(arguments, 0, "support")?;
            let nom = argument(arguments, 1, "nom")?;
            let phrase = obtenir_phrase()?;
            preparer_support(Path::new(support), nom, &phrase)?;
            println!("support prepare : {support} (volume \"{nom}\" chiffre)");
        }
        "supprimer_monarque" => {
            let support = argument(arguments, 0, "support")?;
            supprimer_monarque(Path::new(support))?;
            println!("monarque supprime de {support} : donnees definitivement illisibles");
        }
        "formater" => {
            let image = argument(arguments, 0, "image")?;
            let index = index_partition(argument(arguments, 1, "index")?)?;
            let phrase = obtenir_phrase()?;
            formater_partition(Path::new(image), index, &phrase)?;
            println!("partition {index} formatee en MonarqueFS");
        }
        "etat" => {
            let session = ouvrir_session(arguments)?;
            let stats = session.statistiques();
            println!("volume MonarqueFS");
            println!("  taille de bloc     : {} octets", stats.taille_bloc);
            println!("  blocs de donnees   : {}", stats.nb_blocs_donnees);
            println!(
                "  blocs libres       : {} ({})",
                stats.blocs_libres,
                taille_lisible(stats.blocs_libres * stats.taille_bloc as u64)
            );
            println!("  inodes             : {}", stats.nb_inodes);
            println!("  chemins indexes    : {}", stats.nb_chemins_indexes);
            session.fermer()?;
        }
        "lister" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = arguments.get(2).map(|s| s.as_str()).unwrap_or("/");
            for entree in session.lister(chemin)? {
                let marque = match entree.type_noeud {
                    TypeNoeud::Dossier => "d",
                    _ => "-",
                };
                println!(
                    "{marque} {:>10}  {}",
                    taille_lisible(entree.taille),
                    entree.nom
                );
            }
            session.fermer()?;
        }
        "creer_dossier" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = argument(arguments, 2, "chemin")?;
            session.creer_dossier(chemin)?;
            println!("dossier cree : {chemin}");
            session.fermer()?;
        }
        "importer" => {
            let mut session = ouvrir_session(arguments)?;
            let source = argument(arguments, 2, "source")?;
            let destination = argument(arguments, 3, "destination")?;
            session.importer(Path::new(source), destination)?;
            println!("importe : {source} -> {destination}");
            session.fermer()?;
        }
        "exporter" => {
            let mut session = ouvrir_session(arguments)?;
            let source = argument(arguments, 2, "source")?;
            let destination = argument(arguments, 3, "destination")?;
            session.exporter(source, Path::new(destination))?;
            println!("exporte : {source} -> {destination}");
            session.fermer()?;
        }
        "afficher" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = argument(arguments, 2, "chemin")?;
            let donnees = session.lire_fichier(chemin)?;
            print!("{}", String::from_utf8_lossy(&donnees));
            session.fermer()?;
        }
        "effacer" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = argument(arguments, 2, "chemin")?;
            session.supprimer(chemin)?;
            println!("supprime : {chemin}");
            session.fermer()?;
        }
        "renommer" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = argument(arguments, 2, "chemin")?;
            let nom = argument(arguments, 3, "nouveau_nom")?;
            session.renommer(chemin, nom)?;
            println!("renomme : {chemin} -> {nom}");
            session.fermer()?;
        }
        "meta" => {
            let mut session = ouvrir_session(arguments)?;
            let chemin = argument(arguments, 2, "chemin")?;
            match (arguments.get(3), arguments.get(4)) {
                (Some(cle), Some(valeur)) => {
                    session.definir_meta(chemin, cle, valeur)?;
                    println!("metadonnee definie : {cle}={valeur}");
                }
                _ => {
                    for (cle, valeur) in session.lire_metas(chemin)? {
                        println!("{cle}={valeur}");
                    }
                }
            }
            session.fermer()?;
        }
        "mettre_a_jour" => {
            use gestionnaire_fichiers::mise_a_jour::{mettre_a_jour, EtapeMaj};
            let (emetteur, recepteur) = std::sync::mpsc::channel();
            let fil = std::thread::spawn(move || mettre_a_jour(emetteur));
            // affichage de la progression en direct
            for etape in recepteur {
                match etape {
                    EtapeMaj::Info(texte) => println!("{texte}"),
                    EtapeMaj::Terminee(Ok(_)) => {
                        println!("mise a jour terminee — relancer les outils pour en profiter")
                    }
                    EtapeMaj::Terminee(Err(e)) => return Err(e.into()),
                }
            }
            fil.join().ok();
        }
        inconnue => return Err(format!("commande inconnue : {inconnue}").into()),
    }
    Ok(())
}
