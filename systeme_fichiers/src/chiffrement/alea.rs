// generation d'octets aleatoires via la source systeme

use std::fs::File;
use std::io::Read;

// remplissage d'un tampon avec de l'alea systeme
pub fn remplir_aleatoire(tampon: &mut [u8]) -> std::io::Result<()> {
    let mut source = File::open("/dev/urandom")?;
    source.read_exact(tampon)?;
    Ok(())
}

// generation d'un nonce de 12 octets
pub fn nonce_aleatoire() -> std::io::Result<[u8; 12]> {
    let mut nonce = [0u8; 12];
    remplir_aleatoire(&mut nonce)?;
    Ok(nonce)
}

// generation d'une cle de 32 octets
pub fn cle_aleatoire() -> std::io::Result<[u8; 32]> {
    let mut cle = [0u8; 32];
    remplir_aleatoire(&mut cle)?;
    Ok(cle)
}
