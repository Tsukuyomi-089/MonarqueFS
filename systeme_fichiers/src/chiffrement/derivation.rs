// derivation de cle : hmac-sha256 et pbkdf2

use super::sha256::sha256;

// hmac-sha256 (rfc 2104)
pub fn hmac_sha256(cle: &[u8], message: &[u8]) -> [u8; 32] {
    let mut cle_bloc = [0u8; 64];
    if cle.len() > 64 {
        cle_bloc[..32].copy_from_slice(&sha256(cle));
    } else {
        cle_bloc[..cle.len()].copy_from_slice(cle);
    }

    let mut interne = Vec::with_capacity(64 + message.len());
    let mut externe = Vec::with_capacity(64 + 32);
    for o in &cle_bloc {
        interne.push(o ^ 0x36);
        externe.push(o ^ 0x5c);
    }
    interne.extend_from_slice(message);
    externe.extend_from_slice(&sha256(&interne));
    sha256(&externe)
}

// pbkdf2-hmac-sha256 (rfc 8018)
pub fn pbkdf2_sha256(phrase: &[u8], sel: &[u8], iterations: u32, taille: usize) -> Vec<u8> {
    let mut sortie = Vec::with_capacity(taille);
    let mut numero_bloc: u32 = 1;
    while sortie.len() < taille {
        // premier tour : sel concatene au numero de bloc
        let mut message = sel.to_vec();
        message.extend_from_slice(&numero_bloc.to_be_bytes());
        let mut u = hmac_sha256(phrase, &message);
        let mut t = u;
        for _ in 1..iterations {
            u = hmac_sha256(phrase, &u);
            for (a, b) in t.iter_mut().zip(u.iter()) {
                *a ^= b;
            }
        }
        sortie.extend_from_slice(&t);
        numero_bloc += 1;
    }
    sortie.truncate(taille);
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(donnees: &[u8]) -> String {
        donnees.iter().map(|o| format!("{o:02x}")).collect()
    }

    #[test]
    fn vecteur_hmac_rfc4231() {
        // cas de test 2 du rfc 4231
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn vecteur_pbkdf2_rfc7914() {
        // vecteur pbkdf2-hmac-sha256 du rfc 7914
        let derive = pbkdf2_sha256(b"passwd", b"salt", 1, 64);
        assert_eq!(
            hex(&derive),
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
             49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
        );
    }
}
