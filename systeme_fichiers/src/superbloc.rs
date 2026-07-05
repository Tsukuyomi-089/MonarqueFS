// superbloc du volume monarque

use crate::bloc::{TAILLE_BLOC, TAILLE_BLOC_DISQUE};
use crate::erreurs::{ErreurFs, ResultatFs};
use crate::stockage::Stockage;

// signature du volume
pub const MAGIE_VOLUME: [u8; 4] = *b"MFS1";
pub const VERSION_VOLUME: u16 = 1;
// iterations de derivation de cle
pub const ITERATIONS_KDF: u32 = 60_000;

#[derive(Debug, Clone)]
pub struct Superbloc {
    pub version: u16,
    pub algorithme: u16,
    pub nb_blocs_total: u64,
    pub nb_inodes: u64,
    pub bloc_debut_bitmap: u64,
    pub nb_blocs_bitmap: u64,
    pub bloc_debut_inodes: u64,
    pub nb_blocs_inodes: u64,
    pub bloc_debut_donnees: u64,
    pub nb_blocs_donnees: u64,
    pub iterations_kdf: u32,
    pub sel_kdf: [u8; 16],
    pub nonce_cle: [u8; 12],
    // cle de volume chiffree par la cle derivee de la phrase
    pub cle_enveloppee: [u8; 48],
}

impl Superbloc {
    // ecriture dans le bloc zero
    pub fn sauvegarder(&self, stockage: &mut dyn Stockage) -> ResultatFs<()> {
        let mut tampon = vec![0u8; TAILLE_BLOC_DISQUE];
        tampon[0..4].copy_from_slice(&MAGIE_VOLUME);
        tampon[4..6].copy_from_slice(&self.version.to_le_bytes());
        tampon[6..8].copy_from_slice(&self.algorithme.to_le_bytes());
        tampon[8..12].copy_from_slice(&(TAILLE_BLOC as u32).to_le_bytes());
        tampon[12..20].copy_from_slice(&self.nb_blocs_total.to_le_bytes());
        tampon[20..28].copy_from_slice(&self.nb_inodes.to_le_bytes());
        tampon[28..36].copy_from_slice(&self.bloc_debut_bitmap.to_le_bytes());
        tampon[36..44].copy_from_slice(&self.nb_blocs_bitmap.to_le_bytes());
        tampon[44..52].copy_from_slice(&self.bloc_debut_inodes.to_le_bytes());
        tampon[52..60].copy_from_slice(&self.nb_blocs_inodes.to_le_bytes());
        tampon[60..68].copy_from_slice(&self.bloc_debut_donnees.to_le_bytes());
        tampon[68..76].copy_from_slice(&self.nb_blocs_donnees.to_le_bytes());
        tampon[76..80].copy_from_slice(&self.iterations_kdf.to_le_bytes());
        tampon[80..96].copy_from_slice(&self.sel_kdf);
        tampon[96..108].copy_from_slice(&self.nonce_cle);
        tampon[108..156].copy_from_slice(&self.cle_enveloppee);
        stockage.ecrire_a(0, &tampon)
    }

    // lecture depuis le bloc zero
    pub fn charger(stockage: &mut dyn Stockage) -> ResultatFs<Self> {
        let mut tampon = vec![0u8; TAILLE_BLOC_DISQUE];
        stockage.lire_a(0, &mut tampon)?;
        if tampon[0..4] != MAGIE_VOLUME {
            return Err(ErreurFs::VolumeInvalide("signature absente".into()));
        }
        let u16_a = |d: usize| u16::from_le_bytes(tampon[d..d + 2].try_into().unwrap());
        let u32_a = |d: usize| u32::from_le_bytes(tampon[d..d + 4].try_into().unwrap());
        let u64_a = |d: usize| u64::from_le_bytes(tampon[d..d + 8].try_into().unwrap());
        if u32_a(8) as usize != TAILLE_BLOC {
            return Err(ErreurFs::VolumeInvalide("taille de bloc inattendue".into()));
        }
        let mut sel_kdf = [0u8; 16];
        sel_kdf.copy_from_slice(&tampon[80..96]);
        let mut nonce_cle = [0u8; 12];
        nonce_cle.copy_from_slice(&tampon[96..108]);
        let mut cle_enveloppee = [0u8; 48];
        cle_enveloppee.copy_from_slice(&tampon[108..156]);
        Ok(Self {
            version: u16_a(4),
            algorithme: u16_a(6),
            nb_blocs_total: u64_a(12),
            nb_inodes: u64_a(20),
            bloc_debut_bitmap: u64_a(28),
            nb_blocs_bitmap: u64_a(36),
            bloc_debut_inodes: u64_a(44),
            nb_blocs_inodes: u64_a(52),
            bloc_debut_donnees: u64_a(60),
            nb_blocs_donnees: u64_a(68),
            iterations_kdf: u32_a(76),
            sel_kdf,
            nonce_cle,
            cle_enveloppee,
        })
    }
}
