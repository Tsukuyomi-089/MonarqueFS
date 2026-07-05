// chiffrement authentifie chacha20-poly1305 (rfc 8439)

use super::chacha20::{bloc_chacha20, chiffrer_flux};
use super::poly1305::{etiquettes_egales, Poly1305};

// taille de l'etiquette d'authentification
pub const TAILLE_TAG: usize = 16;

// derivation de la cle poly1305 depuis le bloc zero
fn cle_authentification(cle: &[u8; 32], nonce: &[u8; 12]) -> [u8; 32] {
    let bloc = bloc_chacha20(cle, nonce, 0);
    let mut cle_poly = [0u8; 32];
    cle_poly.copy_from_slice(&bloc[..32]);
    cle_poly
}

// calcul de l'etiquette sur donnees associees et chiffre
fn calculer_etiquette(
    cle_poly: &[u8; 32],
    donnees_associees: &[u8],
    chiffre: &[u8],
) -> [u8; 16] {
    // bourrage a 16 octets avant absorption
    let bourrer = |donnees: &[u8]| {
        let mut tampon = donnees.to_vec();
        while !tampon.len().is_multiple_of(16) {
            tampon.push(0);
        }
        tampon
    };
    let mut poly = Poly1305::nouveau(cle_poly);
    poly.absorber(&bourrer(donnees_associees));
    poly.absorber(&bourrer(chiffre));
    // longueurs finales
    let mut longueurs = [0u8; 16];
    longueurs[..8].copy_from_slice(&(donnees_associees.len() as u64).to_le_bytes());
    longueurs[8..].copy_from_slice(&(chiffre.len() as u64).to_le_bytes());
    poly.absorber(&longueurs);
    poly.finaliser()
}

// chiffrement : renvoie chiffre suivi de l'etiquette
pub fn sceller(
    cle: &[u8; 32],
    nonce: &[u8; 12],
    donnees_associees: &[u8],
    clair: &[u8],
) -> Vec<u8> {
    let mut sortie = clair.to_vec();
    chiffrer_flux(cle, nonce, 1, &mut sortie);
    let cle_poly = cle_authentification(cle, nonce);
    let etiquette = calculer_etiquette(&cle_poly, donnees_associees, &sortie);
    sortie.extend_from_slice(&etiquette);
    sortie
}

// dechiffrement avec verification de l'etiquette
pub fn ouvrir(
    cle: &[u8; 32],
    nonce: &[u8; 12],
    donnees_associees: &[u8],
    chiffre_et_tag: &[u8],
) -> Option<Vec<u8>> {
    if chiffre_et_tag.len() < TAILLE_TAG {
        return None;
    }
    let (chiffre, tag) = chiffre_et_tag.split_at(chiffre_et_tag.len() - TAILLE_TAG);
    let cle_poly = cle_authentification(cle, nonce);
    let attendu = calculer_etiquette(&cle_poly, donnees_associees, chiffre);
    let mut recu = [0u8; 16];
    recu.copy_from_slice(tag);
    if !etiquettes_egales(&attendu, &recu) {
        return None;
    }
    let mut clair = chiffre.to_vec();
    chiffrer_flux(cle, nonce, 1, &mut clair);
    Some(clair)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(donnees: &[u8]) -> String {
        donnees.iter().map(|o| format!("{o:02x}")).collect()
    }

    #[test]
    fn vecteur_aead_rfc8439() {
        // rfc 8439 section 2.8.2
        let mut cle = [0u8; 32];
        for i in 0..32 {
            cle[i] = 0x80 + i as u8;
        }
        let nonce = [0x07, 0, 0, 0, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47];
        let aad = [
            0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
        ];
        let clair = b"Ladies and Gentlemen of the class of '99: If I could offer you \
only one tip for the future, sunscreen would be it.";
        let scelle = sceller(&cle, &nonce, &aad, clair);
        // etiquette attendue du rfc
        assert_eq!(
            hex(&scelle[scelle.len() - 16..]),
            "1ae10b594f09e26a7e902ecbd0600691"
        );
        // ouverture correcte
        let ouvert = ouvrir(&cle, &nonce, &aad, &scelle).unwrap();
        assert_eq!(ouvert, clair);
        // toute alteration est rejetee
        let mut altere = scelle.clone();
        altere[3] ^= 1;
        assert!(ouvrir(&cle, &nonce, &aad, &altere).is_none());
    }
}
