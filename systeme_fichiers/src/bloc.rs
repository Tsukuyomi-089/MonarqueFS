// lecture et ecriture des blocs chiffres

use crate::chiffrement::{alea, AlgorithmeChiffrement, TAILLE_NONCE, TAILLE_TAG};
use crate::erreurs::{ErreurFs, ResultatFs};
use crate::stockage::Stockage;

// taille utile d'un bloc
pub const TAILLE_BLOC: usize = 4096;
// taille d'un bloc sur disque : nonce + charge + etiquette
pub const TAILLE_BLOC_DISQUE: usize = TAILLE_NONCE + TAILLE_BLOC + TAILLE_TAG;

// decalage d'un bloc dans le volume
#[inline]
pub fn decalage_bloc(index: u64) -> u64 {
    index * TAILLE_BLOC_DISQUE as u64
}

// ecriture chiffree d'un bloc avec nonce frais
pub fn ecrire_bloc(
    stockage: &mut dyn Stockage,
    algo: &dyn AlgorithmeChiffrement,
    cle: &[u8; 32],
    index: u64,
    contenu: &[u8],
) -> ResultatFs<()> {
    debug_assert_eq!(contenu.len(), TAILLE_BLOC);
    let nonce = alea::nonce_aleatoire()?;
    // le numero de bloc lie le chiffre a sa position
    let scelle = algo.chiffrer(cle, &nonce, &index.to_le_bytes(), contenu);
    let mut tampon = Vec::with_capacity(TAILLE_BLOC_DISQUE);
    tampon.extend_from_slice(&nonce);
    tampon.extend_from_slice(&scelle);
    stockage.ecrire_a(decalage_bloc(index), &tampon)
}

// lecture et dechiffrement d'un bloc
pub fn lire_bloc(
    stockage: &mut dyn Stockage,
    algo: &dyn AlgorithmeChiffrement,
    cle: &[u8; 32],
    index: u64,
) -> ResultatFs<Vec<u8>> {
    let mut tampon = vec![0u8; TAILLE_BLOC_DISQUE];
    stockage.lire_a(decalage_bloc(index), &mut tampon)?;
    let mut nonce = [0u8; TAILLE_NONCE];
    nonce.copy_from_slice(&tampon[..TAILLE_NONCE]);
    algo.dechiffrer(cle, &nonce, &index.to_le_bytes(), &tampon[TAILLE_NONCE..])
        .ok_or(ErreurFs::BlocCorrompu(index))
}
