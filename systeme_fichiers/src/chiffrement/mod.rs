// couche de chiffrement du volume

pub mod aead;
pub mod alea;
pub mod chacha20;
pub mod derivation;
pub mod poly1305;
pub mod sha256;

pub use aead::TAILLE_TAG;

// taille du nonce
pub const TAILLE_NONCE: usize = 12;
// taille d'une cle de volume
pub const TAILLE_CLE: usize = 32;

// algorithme de chiffrement authentifie interchangeable
pub trait AlgorithmeChiffrement: Send {
    // identifiant stocke dans le superbloc
    fn identifiant(&self) -> u16;
    // surcout par bloc (etiquette)
    fn surcout(&self) -> usize;
    // chiffre et renvoie chiffre + etiquette
    fn chiffrer(
        &self,
        cle: &[u8; TAILLE_CLE],
        nonce: &[u8; TAILLE_NONCE],
        donnees_associees: &[u8],
        clair: &[u8],
    ) -> Vec<u8>;
    // dechiffre apres verification de l'etiquette
    fn dechiffrer(
        &self,
        cle: &[u8; TAILLE_CLE],
        nonce: &[u8; TAILLE_NONCE],
        donnees_associees: &[u8],
        chiffre: &[u8],
    ) -> Option<Vec<u8>>;
}

// implementation par defaut : chacha20-poly1305
pub struct ChaCha20Poly1305;

impl AlgorithmeChiffrement for ChaCha20Poly1305 {
    fn identifiant(&self) -> u16 {
        1
    }

    fn surcout(&self) -> usize {
        TAILLE_TAG
    }

    fn chiffrer(
        &self,
        cle: &[u8; TAILLE_CLE],
        nonce: &[u8; TAILLE_NONCE],
        donnees_associees: &[u8],
        clair: &[u8],
    ) -> Vec<u8> {
        aead::sceller(cle, nonce, donnees_associees, clair)
    }

    fn dechiffrer(
        &self,
        cle: &[u8; TAILLE_CLE],
        nonce: &[u8; TAILLE_NONCE],
        donnees_associees: &[u8],
        chiffre: &[u8],
    ) -> Option<Vec<u8>> {
        aead::ouvrir(cle, nonce, donnees_associees, chiffre)
    }
}

// fabrique d'algorithme depuis l'identifiant du superbloc
pub fn algorithme_depuis_identifiant(id: u16) -> Option<Box<dyn AlgorithmeChiffrement>> {
    match id {
        1 => Some(Box::new(ChaCha20Poly1305)),
        _ => None,
    }
}
