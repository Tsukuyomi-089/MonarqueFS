// tests d'integration du volume monarque

use systeme_fichiers::{formater, monter, ErreurFs, StockageMemoire, TypeNoeud};

const TAILLE_VOLUME: usize = 8 * 1024 * 1024;
const PHRASE: &str = "phrase de test tres secrete";

fn volume_pret() -> StockageMemoire {
    let mut stockage = StockageMemoire::nouveau(TAILLE_VOLUME);
    formater(&mut stockage, PHRASE).unwrap();
    stockage
}

#[test]
fn cycle_fichier_simple() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.ecrire_fichier("/bonjour.txt", b"salut monarque").unwrap();
    assert_eq!(volume.lire_fichier("/bonjour.txt").unwrap(), b"salut monarque");
}

#[test]
fn hierarchie_dossiers() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.creer_dossier("/documents").unwrap();
    volume.creer_dossier("/documents/travail").unwrap();
    volume
        .ecrire_fichier("/documents/travail/rapport.txt", b"contenu du rapport")
        .unwrap();
    let entrees = volume.lister("/documents").unwrap();
    assert_eq!(entrees.len(), 1);
    assert_eq!(entrees[0].nom, "travail");
    assert_eq!(entrees[0].type_noeud, TypeNoeud::Dossier);
    let entrees = volume.lister("/documents/travail").unwrap();
    assert_eq!(entrees[0].nom, "rapport.txt");
    assert_eq!(entrees[0].taille, 18);
}

#[test]
fn persistance_apres_remontage() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.creer_dossier("/archives").unwrap();
    volume
        .ecrire_fichier("/archives/note.txt", b"persiste bien")
        .unwrap();
    let stockage = volume.demonter().unwrap();
    // remontage complet depuis le support
    let mut volume = monter(stockage, PHRASE).unwrap();
    assert_eq!(
        volume.lire_fichier("/archives/note.txt").unwrap(),
        b"persiste bien"
    );
}

#[test]
fn phrase_incorrecte_refusee() {
    let stockage = volume_pret();
    match monter(stockage, "mauvaise phrase") {
        Err(ErreurFs::PhraseInvalide) => {}
        Err(autre) => panic!("attendu PhraseInvalide, obtenu {autre:?}"),
        Ok(_) => panic!("montage accepte avec une mauvaise phrase"),
    }
}

#[test]
fn aucune_donnee_en_clair_sur_disque() {
    let secret = b"SECRET_ABSOLUMENT_UNIQUE_12345";
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.ecrire_fichier("/secret.txt", secret).unwrap();
    let stockage = volume.demonter().unwrap();
    // le contenu ne doit apparaitre nulle part en clair
    let brut = stockage.octets_bruts();
    let trouve = brut.windows(secret.len()).any(|f| f == secret);
    assert!(!trouve, "donnees en clair detectees sur le disque");
    // le nom de fichier non plus
    let nom = b"secret.txt";
    let trouve_nom = brut.windows(nom.len()).any(|f| f == nom);
    assert!(!trouve_nom, "nom de fichier en clair detecte");
}

#[test]
fn gros_fichier_indirections() {
    // fichier de 3 mo : force l'indirection simple et double
    let stockage = StockageMemoire::nouveau(16 * 1024 * 1024);
    let mut stockage = stockage;
    formater(&mut stockage, PHRASE).unwrap();
    let mut volume = monter(stockage, PHRASE).unwrap();
    let donnees: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    volume.ecrire_fichier("/gros.bin", &donnees).unwrap();
    let relu = volume.lire_fichier("/gros.bin").unwrap();
    assert_eq!(relu.len(), donnees.len());
    assert_eq!(relu, donnees);
    // la reecriture libere puis realloue sans fuite
    let libres_avant = volume.statistiques().blocs_libres;
    volume.ecrire_fichier("/gros.bin", &donnees).unwrap();
    assert_eq!(volume.statistiques().blocs_libres, libres_avant);
    // la suppression rend tous les blocs
    volume.supprimer("/gros.bin").unwrap();
    let stats = volume.statistiques();
    assert!(stats.blocs_libres > libres_avant);
}

#[test]
fn ecriture_lecture_en_flux() {
    // ecriture par flux puis relecture par flux, sans passer par la memoire complete
    let mut stockage = StockageMemoire::nouveau(16 * 1024 * 1024);
    formater(&mut stockage, PHRASE).unwrap();
    let mut volume = monter(stockage, PHRASE).unwrap();
    // 5 Mo : force les indirections simple et double
    let source: Vec<u8> = (0..5_000_000u32).map(|i| (i.wrapping_mul(7) % 251) as u8).collect();
    let mut lecteur = std::io::Cursor::new(source.clone());
    let ecrit = volume.ecrire_fichier_flux("/flux.bin", &mut lecteur).unwrap();
    assert_eq!(ecrit, source.len() as u64);
    assert_eq!(volume.taille_fichier("/flux.bin").unwrap(), source.len() as u64);
    // relecture par flux
    let mut sortie = Vec::new();
    volume.lire_fichier_flux("/flux.bin", &mut sortie).unwrap();
    assert_eq!(sortie, source);
    // coherence avec la lecture classique
    assert_eq!(volume.lire_fichier("/flux.bin").unwrap(), source);
}

#[test]
fn suppression_et_regles() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.creer_dossier("/plein").unwrap();
    volume.ecrire_fichier("/plein/f.txt", b"x").unwrap();
    // dossier non vide refuse
    assert!(matches!(
        volume.supprimer("/plein"),
        Err(ErreurFs::DossierNonVide(_))
    ));
    volume.supprimer("/plein/f.txt").unwrap();
    volume.supprimer("/plein").unwrap();
    assert!(volume.lister("/").unwrap().is_empty());
    // la racine est protegee
    assert!(volume.supprimer("/").is_err());
}

#[test]
fn renommage_et_index() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.creer_dossier("/ancien").unwrap();
    volume.ecrire_fichier("/ancien/f.txt", b"contenu").unwrap();
    volume.renommer("/ancien", "nouveau").unwrap();
    // l'index suit le sous arbre
    assert_eq!(volume.lire_fichier("/nouveau/f.txt").unwrap(), b"contenu");
    assert!(volume.lire_fichier("/ancien/f.txt").is_err());
}

#[test]
fn metadonnees_etendues() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    volume.ecrire_fichier("/photo.jpg", b"image").unwrap();
    volume.definir_meta("/photo.jpg", "auteur", "tsuky").unwrap();
    volume.definir_meta("/photo.jpg", "etiquette", "vacances").unwrap();
    volume.definir_meta("/photo.jpg", "auteur", "monarque").unwrap();
    let metas = volume.lire_metas("/photo.jpg").unwrap();
    assert_eq!(metas.len(), 2);
    assert!(metas.contains(&("auteur".to_string(), "monarque".to_string())));
    // les metadonnees persistent au remontage
    let stockage = volume.demonter().unwrap();
    let mut volume = monter(stockage, PHRASE).unwrap();
    assert_eq!(volume.lire_metas("/photo.jpg").unwrap().len(), 2);
}

#[test]
fn erreurs_de_chemin() {
    let stockage = volume_pret();
    let mut volume = monter(stockage, PHRASE).unwrap();
    assert!(volume.lire_fichier("/absent.txt").is_err());
    assert!(volume.creer_dossier("/a/b/c").is_err());
    assert!(volume.ecrire_fichier("relatif.txt", b"x").is_err());
    volume.creer_dossier("/d").unwrap();
    assert!(matches!(
        volume.creer_dossier("/d"),
        Err(ErreurFs::ExisteDeja(_))
    ));
    // un dossier ne se lit pas comme un fichier
    assert!(matches!(
        volume.lire_fichier("/d"),
        Err(ErreurFs::PasUnFichier(_))
    ));
}
