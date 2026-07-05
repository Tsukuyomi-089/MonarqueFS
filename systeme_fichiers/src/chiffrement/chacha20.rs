// chiffrement de flux chacha20 (rfc 8439)

// quart de tour sur l'etat
#[inline(always)]
fn quart_de_tour(etat: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    etat[a] = etat[a].wrapping_add(etat[b]);
    etat[d] = (etat[d] ^ etat[a]).rotate_left(16);
    etat[c] = etat[c].wrapping_add(etat[d]);
    etat[b] = (etat[b] ^ etat[c]).rotate_left(12);
    etat[a] = etat[a].wrapping_add(etat[b]);
    etat[d] = (etat[d] ^ etat[a]).rotate_left(8);
    etat[c] = etat[c].wrapping_add(etat[d]);
    etat[b] = (etat[b] ^ etat[c]).rotate_left(7);
}

// generation d'un bloc de 64 octets de flux
pub fn bloc_chacha20(cle: &[u8; 32], nonce: &[u8; 12], compteur: u32) -> [u8; 64] {
    let mut etat = [0u32; 16];
    // constantes "expand 32-byte k"
    etat[0] = 0x61707865;
    etat[1] = 0x3320646e;
    etat[2] = 0x79622d32;
    etat[3] = 0x6b206574;
    for i in 0..8 {
        etat[4 + i] = u32::from_le_bytes(cle[i * 4..i * 4 + 4].try_into().unwrap());
    }
    etat[12] = compteur;
    for i in 0..3 {
        etat[13 + i] = u32::from_le_bytes(nonce[i * 4..i * 4 + 4].try_into().unwrap());
    }

    let mut travail = etat;
    for _ in 0..10 {
        // tours colonnes
        quart_de_tour(&mut travail, 0, 4, 8, 12);
        quart_de_tour(&mut travail, 1, 5, 9, 13);
        quart_de_tour(&mut travail, 2, 6, 10, 14);
        quart_de_tour(&mut travail, 3, 7, 11, 15);
        // tours diagonales
        quart_de_tour(&mut travail, 0, 5, 10, 15);
        quart_de_tour(&mut travail, 1, 6, 11, 12);
        quart_de_tour(&mut travail, 2, 7, 8, 13);
        quart_de_tour(&mut travail, 3, 4, 9, 14);
    }

    let mut sortie = [0u8; 64];
    for i in 0..16 {
        let mot = travail[i].wrapping_add(etat[i]);
        sortie[i * 4..i * 4 + 4].copy_from_slice(&mot.to_le_bytes());
    }
    sortie
}

// chiffrement ou dechiffrement par ou exclusif avec le flux
pub fn chiffrer_flux(cle: &[u8; 32], nonce: &[u8; 12], compteur_initial: u32, donnees: &mut [u8]) {
    for (i, morceau) in donnees.chunks_mut(64).enumerate() {
        let flux = bloc_chacha20(cle, nonce, compteur_initial.wrapping_add(i as u32));
        for (octet, f) in morceau.iter_mut().zip(flux.iter()) {
            *octet ^= f;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(donnees: &[u8]) -> String {
        donnees.iter().map(|o| format!("{o:02x}")).collect()
    }

    fn cle_test() -> [u8; 32] {
        let mut cle = [0u8; 32];
        for i in 0..32 {
            cle[i] = i as u8;
        }
        cle
    }

    #[test]
    fn vecteur_bloc_rfc8439() {
        // rfc 8439 section 2.3.2
        let nonce = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let bloc = bloc_chacha20(&cle_test(), &nonce, 1);
        assert_eq!(
            hex(&bloc[..16]),
            "10f1e7e4d13b5915500fdd1fa32071c4"
        );
    }

    #[test]
    fn vecteur_chiffrement_rfc8439() {
        // rfc 8439 section 2.4.2
        let nonce = [0, 0, 0, 0, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let mut donnees = b"Ladies and Gentlemen of the class of '99: If I could offer you \
only one tip for the future, sunscreen would be it."
            .to_vec();
        chiffrer_flux(&cle_test(), &nonce, 1, &mut donnees);
        assert_eq!(
            hex(&donnees[..32]),
            "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b"
        );
        // le dechiffrement restitue le texte clair
        chiffrer_flux(&cle_test(), &nonce, 1, &mut donnees);
        assert!(donnees.starts_with(b"Ladies and Gentlemen"));
    }
}
