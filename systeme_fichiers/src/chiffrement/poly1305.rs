// authentificateur poly1305 (rfc 8439)
// boucles indexees conservees : plus lisibles pour l'arithmetique en membres
#![allow(clippy::needless_range_loop)]

// etat d'accumulation en membres de 26 bits
pub struct Poly1305 {
    r: [u32; 5],
    h: [u32; 5],
    s: [u32; 4],
}

impl Poly1305 {
    pub fn nouveau(cle: &[u8; 32]) -> Self {
        // decoupage de r avec application du masque
        let t0 = u32::from_le_bytes(cle[0..4].try_into().unwrap());
        let t1 = u32::from_le_bytes(cle[4..8].try_into().unwrap());
        let t2 = u32::from_le_bytes(cle[8..12].try_into().unwrap());
        let t3 = u32::from_le_bytes(cle[12..16].try_into().unwrap());

        let r = [
            t0 & 0x3ffffff,
            ((t0 >> 26) | (t1 << 6)) & 0x3ffff03,
            ((t1 >> 20) | (t2 << 12)) & 0x3ffc0ff,
            ((t2 >> 14) | (t3 << 18)) & 0x3f03fff,
            (t3 >> 8) & 0x00fffff,
        ];

        let s = [
            u32::from_le_bytes(cle[16..20].try_into().unwrap()),
            u32::from_le_bytes(cle[20..24].try_into().unwrap()),
            u32::from_le_bytes(cle[24..28].try_into().unwrap()),
            u32::from_le_bytes(cle[28..32].try_into().unwrap()),
        ];

        Self {
            r,
            h: [0; 5],
            s,
        }
    }

    // absorption d'un bloc de 16 octets au plus
    fn absorber_bloc(&mut self, bloc: &[u8]) {
        // chargement du bloc avec le bit haut
        let mut tampon = [0u8; 17];
        tampon[..bloc.len()].copy_from_slice(bloc);
        tampon[bloc.len()] = 1;

        let t0 = u32::from_le_bytes(tampon[0..4].try_into().unwrap());
        let t1 = u32::from_le_bytes(tampon[4..8].try_into().unwrap());
        let t2 = u32::from_le_bytes(tampon[8..12].try_into().unwrap());
        let t3 = u32::from_le_bytes(tampon[12..16].try_into().unwrap());
        let t4 = tampon[16] as u32;

        self.h[0] += t0 & 0x3ffffff;
        self.h[1] += ((t0 >> 26) | (t1 << 6)) & 0x3ffffff;
        self.h[2] += ((t1 >> 20) | (t2 << 12)) & 0x3ffffff;
        self.h[3] += ((t2 >> 14) | (t3 << 18)) & 0x3ffffff;
        self.h[4] += (t3 >> 8) | (t4 << 24);

        // multiplication modulaire h = h * r mod 2^130 - 5
        let r = &self.r;
        let h = &self.h;
        let mut d = [0u64; 5];
        for i in 0..5 {
            let mut somme: u64 = 0;
            for j in 0..5 {
                let coeff = if j <= i {
                    r[i - j] as u64
                } else {
                    // repli multiplie par 5
                    5 * (r[5 + i - j] as u64)
                };
                somme += (h[j] as u64) * coeff;
            }
            d[i] = somme;
        }

        // propagation des retenues
        let mut retenue: u64 = 0;
        for i in 0..5 {
            d[i] += retenue;
            self.h[i] = (d[i] & 0x3ffffff) as u32;
            retenue = d[i] >> 26;
        }
        self.h[0] += (retenue as u32) * 5;
        let r0 = self.h[0] >> 26;
        self.h[0] &= 0x3ffffff;
        self.h[1] += r0;
    }

    pub fn absorber(&mut self, donnees: &[u8]) {
        for bloc in donnees.chunks(16) {
            self.absorber_bloc(bloc);
        }
    }

    // calcul de l'etiquette finale
    pub fn finaliser(mut self) -> [u8; 16] {
        // propagation complete des retenues
        let mut retenue = self.h[1] >> 26;
        self.h[1] &= 0x3ffffff;
        for i in 2..5 {
            self.h[i] += retenue;
            retenue = self.h[i] >> 26;
            self.h[i] &= 0x3ffffff;
        }
        self.h[0] += retenue * 5;
        retenue = self.h[0] >> 26;
        self.h[0] &= 0x3ffffff;
        self.h[1] += retenue;

        // calcul de h + 5 - 2^130 pour la reduction finale
        let mut g = [0u32; 5];
        retenue = 5;
        for i in 0..5 {
            g[i] = self.h[i] + retenue;
            retenue = g[i] >> 26;
            g[i] &= 0x3ffffff;
        }

        // selection de g si h >= 2^130 - 5
        let masque = (retenue ^ 1).wrapping_sub(1);
        for i in 0..5 {
            self.h[i] = (self.h[i] & !masque) | (g[i] & masque);
        }

        // recomposition en quatre mots de 32 bits
        let mots = [
            self.h[0] | (self.h[1] << 26),
            (self.h[1] >> 6) | (self.h[2] << 20),
            (self.h[2] >> 12) | (self.h[3] << 14),
            (self.h[3] >> 18) | (self.h[4] << 8),
        ];

        // ajout du secret s
        let mut etiquette = [0u8; 16];
        let mut report: u64 = 0;
        for i in 0..4 {
            let somme = (mots[i] as u64) + (self.s[i] as u64) + report;
            etiquette[i * 4..i * 4 + 4].copy_from_slice(&(somme as u32).to_le_bytes());
            report = somme >> 32;
        }
        etiquette
    }
}

// comparaison a temps constant
pub fn etiquettes_egales(a: &[u8; 16], b: &[u8; 16]) -> bool {
    let mut difference = 0u8;
    for i in 0..16 {
        difference |= a[i] ^ b[i];
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(donnees: &[u8]) -> String {
        donnees.iter().map(|o| format!("{o:02x}")).collect()
    }

    #[test]
    fn vecteur_rfc8439() {
        // rfc 8439 section 2.5.2
        let mut cle = [0u8; 32];
        cle.copy_from_slice(
            &[
                0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52, 0xfe, 0x42,
                0xd5, 0x06, 0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d, 0xb2, 0xfd, 0x4a, 0xbf,
                0xf6, 0xaf, 0x41, 0x49, 0xf5, 0x1b,
            ],
        );
        let mut poly = Poly1305::nouveau(&cle);
        poly.absorber(b"Cryptographic Forum Research Group");
        assert_eq!(hex(&poly.finaliser()), "a8061dc1305136c6c22b8baf0c0127a9");
    }
}
