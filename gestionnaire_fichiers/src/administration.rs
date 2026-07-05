// administration du disque : creation, partitionnement, formatage

use noyau_partition::{DisqueLogique, TablePartition, TypePartition, VuePartition, TAILLE_SECTEUR};
use systeme_fichiers::ResultatFs;
use std::path::Path;

// description d'une partition pour l'affichage
#[derive(Debug, Clone)]
pub struct InfoPartition {
    pub index: usize,
    pub nom: String,
    pub taille_octets: u64,
    pub debut_secteur: u64,
    pub id: u64,
}

// creation d'un disque logique avec table vierge
pub fn creer_disque(chemin: &Path, taille_octets: u64) -> ResultatFs<()> {
    let mut disque = DisqueLogique::creer(chemin, taille_octets)?;
    TablePartition::initialiser(&mut disque)?;
    Ok(())
}

// ajout d'une partition monarque
pub fn ajouter_partition(chemin: &Path, nom: &str, taille_octets: u64) -> ResultatFs<usize> {
    let mut disque = DisqueLogique::ouvrir(chemin)?;
    let mut table = TablePartition::charger(&mut disque)?;
    let nb_secteurs = taille_octets.div_ceil(TAILLE_SECTEUR);
    let index = table.ajouter(&mut disque, nom, nb_secteurs, TypePartition::MonarqueFs)?;
    Ok(index)
}

// suppression d'une partition
pub fn supprimer_partition(chemin: &Path, index: usize) -> ResultatFs<()> {
    let mut disque = DisqueLogique::ouvrir(chemin)?;
    let mut table = TablePartition::charger(&mut disque)?;
    table.supprimer(&mut disque, index)?;
    Ok(())
}

// liste des partitions du disque
pub fn lister_partitions(chemin: &Path) -> ResultatFs<Vec<InfoPartition>> {
    let mut disque = DisqueLogique::ouvrir(chemin)?;
    let table = TablePartition::charger(&mut disque)?;
    Ok(table
        .entrees
        .iter()
        .enumerate()
        .map(|(index, e)| InfoPartition {
            index,
            nom: e.nom.clone(),
            taille_octets: e.taille_octets(),
            debut_secteur: e.debut_secteur,
            id: e.id,
        })
        .collect())
}

// ouverture d'une vue sur une partition
pub fn ouvrir_partition(chemin: &Path, index: usize) -> ResultatFs<VuePartition> {
    let mut disque = DisqueLogique::ouvrir(chemin)?;
    let table = TablePartition::charger(&mut disque)?;
    let entree = table
        .entrees
        .get(index)
        .ok_or(noyau_partition::ErreurNoyau::Introuvable)?
        .clone();
    Ok(VuePartition::nouvelle(disque, &entree))
}

// formatage d'une partition en volume monarque
pub fn formater_partition(chemin: &Path, index: usize, phrase: &str) -> ResultatFs<()> {
    let mut vue = ouvrir_partition(chemin, index)?;
    systeme_fichiers::formater(&mut vue, phrase)?;
    vue.synchroniser()?;
    Ok(())
}

// preparation complete d'un peripherique ou d'une image existante :
// table monarque, partition unique et volume chiffre proteges par phrase
pub fn preparer_support(chemin: &Path, nom_volume: &str, phrase: &str) -> ResultatFs<()> {
    let mut disque = DisqueLogique::ouvrir(chemin)?;
    let mut table = TablePartition::initialiser(&mut disque)?;
    // partition unique sur tout l'espace disponible
    let nb_secteurs = disque
        .nb_secteurs()
        .saturating_sub(noyau_partition::table_partition::SECTEUR_DEBUT_DONNEES);
    table.ajouter(&mut disque, nom_volume, nb_secteurs, TypePartition::MonarqueFs)?;
    drop(disque);
    formater_partition(chemin, 0, phrase)
}
