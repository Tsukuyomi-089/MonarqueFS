// volume monarque : formatage, montage et operations

use crate::bitmap::Bitmap;
use crate::bloc::{ecrire_bloc, lire_bloc, TAILLE_BLOC, TAILLE_BLOC_DISQUE};
use crate::chiffrement::{
    aead, alea, algorithme_depuis_identifiant, derivation, AlgorithmeChiffrement,
    ChaCha20Poly1305,
};
use crate::dossier::{
    deserialiser_entrees, serialiser_entrees, valider_nom, EntreeDossier,
};
use crate::erreurs::{ErreurFs, ResultatFs};
use crate::index::IndexRapide;
use crate::inode::{Inode, TypeNoeud, INODE_RACINE, NB_BLOCS_DIRECTS, TAILLE_INODE};
use crate::stockage::Stockage;
use crate::superbloc::{Superbloc, ITERATIONS_KDF, VERSION_VOLUME};
use std::io::{Read, Write};

// inodes par bloc
const INODES_PAR_BLOC: u64 = (TAILLE_BLOC / TAILLE_INODE) as u64;
// pointeurs par bloc d'indirection
const POINTEURS_PAR_BLOC: u64 = (TAILLE_BLOC / 8) as u64;
// capacite de chaque zone d'indirection en blocs de donnees
const CAP_INDIRECT: u64 = POINTEURS_PAR_BLOC;
const CAP_DOUBLE: u64 = POINTEURS_PAR_BLOC * POINTEURS_PAR_BLOC;
const CAP_TRIPLE: u64 = POINTEURS_PAR_BLOC * POINTEURS_PAR_BLOC * POINTEURS_PAR_BLOC;
// capacite maximale d'un fichier en blocs (12 directs + 3 niveaux d'indirection)
// avec des blocs de 4 Ko : environ 549 To, soit une taille pratiquement illimitee
const MAX_BLOCS_FICHIER: u64 =
    NB_BLOCS_DIRECTS as u64 + CAP_INDIRECT + CAP_DOUBLE + CAP_TRIPLE;
// donnees associees pour l'enveloppe de cle
const AAD_CLE: &[u8] = b"cle_volume_monarque";

// horodatage en secondes
fn maintenant() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// remplissage complet d'un tampon depuis un flux, tolere les lectures partielles
fn remplir_tampon<R: Read + ?Sized>(source: &mut R, tampon: &mut [u8]) -> ResultatFs<usize> {
    let mut total = 0;
    while total < tampon.len() {
        match source.read(&mut tampon[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(ErreurFs::Io(e)),
        }
    }
    Ok(total)
}

// informations d'une entree listee
#[derive(Debug, Clone)]
pub struct InfoEntree {
    pub nom: String,
    pub type_noeud: TypeNoeud,
    pub taille: u64,
    pub modifie: u64,
}

// etat global du volume
#[derive(Debug, Clone)]
pub struct Statistiques {
    pub nb_blocs_donnees: u64,
    pub blocs_libres: u64,
    pub nb_inodes: u64,
    pub taille_bloc: usize,
    pub nb_chemins_indexes: usize,
}

pub struct Volume<S: Stockage> {
    stockage: S,
    superbloc: Superbloc,
    cle_volume: [u8; 32],
    algo: Box<dyn AlgorithmeChiffrement>,
    bitmap: Bitmap,
    index: IndexRapide,
}

// calcul de la geometrie du volume
fn calculer_geometrie(taille_octets: u64) -> ResultatFs<Superbloc> {
    let nb_blocs_total = taille_octets / TAILLE_BLOC_DISQUE as u64;
    if nb_blocs_total < 16 {
        return Err(ErreurFs::VolumeInvalide("volume trop petit".into()));
    }
    let restant = nb_blocs_total - 1;
    // environ un inode pour huit blocs
    let nb_blocs_inodes = ((restant / 8).max(128) / INODES_PAR_BLOC + 1).min(restant / 2);
    let nb_inodes = nb_blocs_inodes * INODES_PAR_BLOC;
    let restant = restant - nb_blocs_inodes;
    // un bit de bitmap par bloc de donnees
    let bits_par_bloc = (TAILLE_BLOC * 8) as u64;
    let nb_blocs_bitmap = restant.div_ceil(bits_par_bloc + 1);
    let nb_blocs_donnees = restant - nb_blocs_bitmap;
    if nb_blocs_donnees < 4 {
        return Err(ErreurFs::VolumeInvalide("volume trop petit".into()));
    }
    Ok(Superbloc {
        version: VERSION_VOLUME,
        algorithme: 0,
        nb_blocs_total,
        nb_inodes,
        bloc_debut_bitmap: 1,
        nb_blocs_bitmap,
        bloc_debut_inodes: 1 + nb_blocs_bitmap,
        nb_blocs_inodes,
        bloc_debut_donnees: 1 + nb_blocs_bitmap + nb_blocs_inodes,
        nb_blocs_donnees,
        iterations_kdf: ITERATIONS_KDF,
        sel_kdf: [0; 16],
        nonce_cle: [0; 12],
        cle_enveloppee: [0; 48],
    })
}

// formatage d'un support en volume monarque
pub fn formater<S: Stockage>(stockage: &mut S, phrase: &str) -> ResultatFs<()> {
    let algo = ChaCha20Poly1305;
    let mut superbloc = calculer_geometrie(stockage.taille_octets())?;
    superbloc.algorithme = algo.identifiant();

    // generation et enveloppement de la cle de volume
    let cle_volume = alea::cle_aleatoire()?;
    alea::remplir_aleatoire(&mut superbloc.sel_kdf)?;
    superbloc.nonce_cle = alea::nonce_aleatoire()?;
    let cle_enveloppe = derivation::pbkdf2_sha256(
        phrase.as_bytes(),
        &superbloc.sel_kdf,
        superbloc.iterations_kdf,
        32,
    );
    let mut cle_kdf = [0u8; 32];
    cle_kdf.copy_from_slice(&cle_enveloppe);
    let enveloppe = aead::sceller(&cle_kdf, &superbloc.nonce_cle, AAD_CLE, &cle_volume);
    superbloc.cle_enveloppee.copy_from_slice(&enveloppe);
    superbloc.sauvegarder(stockage)?;

    // bitmap vierge
    let mut bitmap = Bitmap::vierge(&superbloc);
    bitmap.purger(stockage, &algo, &cle_volume, &superbloc)?;

    // table d'inodes vide avec la racine dans le premier bloc
    let racine = Inode::nouveau(TypeNoeud::Dossier, maintenant());
    let mut premier_bloc = vec![0u8; TAILLE_BLOC];
    let decalage_racine = (INODE_RACINE as usize) * TAILLE_INODE;
    premier_bloc[decalage_racine..decalage_racine + TAILLE_INODE]
        .copy_from_slice(&racine.serialiser()?);
    ecrire_bloc(
        stockage,
        &algo,
        &cle_volume,
        superbloc.bloc_debut_inodes,
        &premier_bloc,
    )?;
    let bloc_vide = vec![0u8; TAILLE_BLOC];
    for i in 1..superbloc.nb_blocs_inodes {
        ecrire_bloc(
            stockage,
            &algo,
            &cle_volume,
            superbloc.bloc_debut_inodes + i,
            &bloc_vide,
        )?;
    }
    stockage.synchroniser()
}

// montage d'un volume existant
pub fn monter<S: Stockage>(mut stockage: S, phrase: &str) -> ResultatFs<Volume<S>> {
    let superbloc = Superbloc::charger(&mut stockage)?;
    let algo = algorithme_depuis_identifiant(superbloc.algorithme)
        .ok_or_else(|| ErreurFs::VolumeInvalide("algorithme inconnu".into()))?;

    // deballage de la cle de volume
    let cle_enveloppe = derivation::pbkdf2_sha256(
        phrase.as_bytes(),
        &superbloc.sel_kdf,
        superbloc.iterations_kdf,
        32,
    );
    let mut cle_kdf = [0u8; 32];
    cle_kdf.copy_from_slice(&cle_enveloppe);
    let cle_claire = aead::ouvrir(
        &cle_kdf,
        &superbloc.nonce_cle,
        AAD_CLE,
        &superbloc.cle_enveloppee,
    )
    .ok_or(ErreurFs::PhraseInvalide)?;
    let mut cle_volume = [0u8; 32];
    cle_volume.copy_from_slice(&cle_claire);

    let bitmap = Bitmap::charger(&mut stockage, algo.as_ref(), &cle_volume, &superbloc)?;
    let mut volume = Volume {
        stockage,
        superbloc,
        cle_volume,
        algo,
        bitmap,
        index: IndexRapide::nouveau(),
    };
    volume.reconstruire_index()?;
    Ok(volume)
}

impl<S: Stockage> Volume<S> {
    // ------- gestion des inodes -------

    fn position_inode(&self, id: u64) -> ResultatFs<(u64, usize)> {
        if id == 0 || id >= self.superbloc.nb_inodes {
            return Err(ErreurFs::VolumeInvalide(format!("inode {id} hors table")));
        }
        let bloc = self.superbloc.bloc_debut_inodes + id / INODES_PAR_BLOC;
        let decalage = ((id % INODES_PAR_BLOC) as usize) * TAILLE_INODE;
        Ok((bloc, decalage))
    }

    fn lire_inode(&mut self, id: u64) -> ResultatFs<Inode> {
        let (bloc, decalage) = self.position_inode(id)?;
        let contenu = lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
        Inode::deserialiser(&contenu[decalage..decalage + TAILLE_INODE])
    }

    fn ecrire_inode(&mut self, id: u64, inode: &Inode) -> ResultatFs<()> {
        let (bloc, decalage) = self.position_inode(id)?;
        let mut contenu =
            lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
        contenu[decalage..decalage + TAILLE_INODE].copy_from_slice(&inode.serialiser()?);
        ecrire_bloc(
            &mut self.stockage,
            self.algo.as_ref(),
            &self.cle_volume,
            bloc,
            &contenu,
        )
    }

    // recherche du premier inode libre
    fn allouer_inode(&mut self) -> ResultatFs<u64> {
        for bloc_idx in 0..self.superbloc.nb_blocs_inodes {
            let bloc = self.superbloc.bloc_debut_inodes + bloc_idx;
            let contenu =
                lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
            for slot in 0..INODES_PAR_BLOC {
                let id = bloc_idx * INODES_PAR_BLOC + slot;
                if id <= INODE_RACINE {
                    continue;
                }
                if contenu[(slot as usize) * TAILLE_INODE] == TypeNoeud::Libre.vers_u8() {
                    return Ok(id);
                }
            }
        }
        Err(ErreurFs::InodesEpuises)
    }

    // ------- gestion des blocs de contenu -------

    fn allouer_bloc(&mut self) -> ResultatFs<u64> {
        let relatif = self.bitmap.allouer()?;
        Ok(self.superbloc.bloc_debut_donnees + relatif)
    }

    fn liberer_bloc(&mut self, absolu: u64) {
        if absolu >= self.superbloc.bloc_debut_donnees {
            self.bitmap.liberer(absolu - self.superbloc.bloc_debut_donnees);
        }
    }

    fn lire_bloc_pointeurs(&mut self, bloc: u64) -> ResultatFs<Vec<u64>> {
        let contenu =
            lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
        Ok(contenu
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect())
    }

    fn ecrire_bloc_pointeurs(&mut self, bloc: u64, pointeurs: &[u64]) -> ResultatFs<()> {
        let mut contenu = vec![0u8; TAILLE_BLOC];
        for (i, p) in pointeurs.iter().enumerate() {
            contenu[i * 8..i * 8 + 8].copy_from_slice(&p.to_le_bytes());
        }
        ecrire_bloc(
            &mut self.stockage,
            self.algo.as_ref(),
            &self.cle_volume,
            bloc,
            &contenu,
        )
    }

    // parcours d'un arbre d'indirection : collecte les blocs de donnees
    fn collecter_indirection(
        &mut self,
        racine: u64,
        niveau: u32,
        restant: &mut u64,
        sortie: &mut Vec<u64>,
    ) -> ResultatFs<()> {
        if *restant == 0 {
            return Ok(());
        }
        let pointeurs = self.lire_bloc_pointeurs(racine)?;
        if niveau == 1 {
            let prendre = (*restant).min(POINTEURS_PAR_BLOC);
            sortie.extend_from_slice(&pointeurs[..prendre as usize]);
            *restant -= prendre;
        } else {
            for p in pointeurs {
                if *restant == 0 {
                    break;
                }
                self.collecter_indirection(p, niveau - 1, restant, sortie)?;
            }
        }
        Ok(())
    }

    // construction d'un arbre d'indirection, renvoie le pointeur racine
    fn construire_indirection(&mut self, blocs: &[u64], niveau: u32) -> ResultatFs<u64> {
        if niveau == 1 {
            let racine = self.allouer_bloc()?;
            self.ecrire_bloc_pointeurs(racine, blocs)?;
            return Ok(racine);
        }
        // capacite d'un sous-arbre de niveau inferieur
        let capacite = POINTEURS_PAR_BLOC.pow(niveau - 1) as usize;
        let mut enfants = Vec::new();
        for groupe in blocs.chunks(capacite) {
            enfants.push(self.construire_indirection(groupe, niveau - 1)?);
        }
        let racine = self.allouer_bloc()?;
        self.ecrire_bloc_pointeurs(racine, &enfants)?;
        Ok(racine)
    }

    // liberation des seuls blocs de pointeurs d'un arbre d'indirection
    fn liberer_indirection(&mut self, racine: u64, niveau: u32) -> ResultatFs<()> {
        if niveau > 1 {
            let pointeurs = self.lire_bloc_pointeurs(racine)?;
            for p in pointeurs {
                if p != 0 {
                    self.liberer_indirection(p, niveau - 1)?;
                }
            }
        }
        self.liberer_bloc(racine);
        Ok(())
    }

    // liste ordonnee des blocs de donnees d'un inode
    fn collecter_blocs(&mut self, inode: &Inode) -> ResultatFs<Vec<u64>> {
        let nb = (inode.taille as usize).div_ceil(TAILLE_BLOC) as u64;
        let mut blocs = Vec::with_capacity(nb.min(1 << 20) as usize);
        let mut restant = nb;
        let directs = restant.min(NB_BLOCS_DIRECTS as u64);
        for i in 0..directs {
            blocs.push(inode.blocs_directs[i as usize]);
        }
        restant -= directs;
        if restant > 0 && inode.bloc_indirect != 0 {
            self.collecter_indirection(inode.bloc_indirect, 1, &mut restant, &mut blocs)?;
        }
        if restant > 0 && inode.bloc_double_indirect != 0 {
            self.collecter_indirection(inode.bloc_double_indirect, 2, &mut restant, &mut blocs)?;
        }
        if restant > 0 && inode.bloc_triple_indirect != 0 {
            self.collecter_indirection(inode.bloc_triple_indirect, 3, &mut restant, &mut blocs)?;
        }
        Ok(blocs)
    }

    // liberation de tous les blocs d'un inode, indirections comprises
    fn liberer_contenu(&mut self, inode: &Inode) -> ResultatFs<()> {
        let blocs = self.collecter_blocs(inode)?;
        for bloc in blocs {
            self.liberer_bloc(bloc);
        }
        if inode.bloc_indirect != 0 {
            self.liberer_indirection(inode.bloc_indirect, 1)?;
        }
        if inode.bloc_double_indirect != 0 {
            self.liberer_indirection(inode.bloc_double_indirect, 2)?;
        }
        if inode.bloc_triple_indirect != 0 {
            self.liberer_indirection(inode.bloc_triple_indirect, 3)?;
        }
        Ok(())
    }

    // pose la liste des blocs de donnees dans l'inode : directs puis indirections
    fn poser_blocs_et_arbre(&mut self, id: u64, blocs: Vec<u64>, taille: u64) -> ResultatFs<()> {
        let mut inode = self.lire_inode(id)?;
        inode.blocs_directs = [0; NB_BLOCS_DIRECTS];
        inode.bloc_indirect = 0;
        inode.bloc_double_indirect = 0;
        inode.bloc_triple_indirect = 0;

        if blocs.len() as u64 > MAX_BLOCS_FICHIER {
            return Err(ErreurFs::FichierTropGrand);
        }

        // pointeurs directs
        for (i, &bloc) in blocs.iter().take(NB_BLOCS_DIRECTS).enumerate() {
            inode.blocs_directs[i] = bloc;
        }
        // zones d'indirection successives
        let mut reste = if blocs.len() > NB_BLOCS_DIRECTS {
            &blocs[NB_BLOCS_DIRECTS..]
        } else {
            &[][..]
        };
        for (niveau, capacite) in [(1u32, CAP_INDIRECT), (2, CAP_DOUBLE), (3, CAP_TRIPLE)] {
            if reste.is_empty() {
                break;
            }
            let n = reste.len().min(capacite as usize);
            let racine = self.construire_indirection(&reste[..n], niveau)?;
            match niveau {
                1 => inode.bloc_indirect = racine,
                2 => inode.bloc_double_indirect = racine,
                _ => inode.bloc_triple_indirect = racine,
            }
            reste = &reste[n..];
        }

        inode.taille = taille;
        inode.modifie = maintenant();
        self.ecrire_inode(id, &inode)?;
        self.purger_bitmap()
    }

    // remplacement complet du contenu d'un inode (donnees en memoire)
    fn ecrire_contenu(&mut self, id: u64, donnees: &[u8]) -> ResultatFs<()> {
        let inode = self.lire_inode(id)?;
        self.liberer_contenu(&inode)?;
        // ecriture des blocs de donnees
        let mut blocs = Vec::with_capacity(donnees.len().div_ceil(TAILLE_BLOC));
        for morceau in donnees.chunks(TAILLE_BLOC) {
            let bloc = self.allouer_bloc()?;
            let mut tampon = vec![0u8; TAILLE_BLOC];
            tampon[..morceau.len()].copy_from_slice(morceau);
            ecrire_bloc(
                &mut self.stockage,
                self.algo.as_ref(),
                &self.cle_volume,
                bloc,
                &tampon,
            )?;
            blocs.push(bloc);
        }
        self.poser_blocs_et_arbre(id, blocs, donnees.len() as u64)
    }

    // remplacement du contenu depuis un flux, sans charger le fichier en memoire
    fn ecrire_contenu_flux<R: Read + ?Sized>(&mut self, id: u64, source: &mut R) -> ResultatFs<u64> {
        let inode = self.lire_inode(id)?;
        self.liberer_contenu(&inode)?;
        let mut blocs = Vec::new();
        let mut taille: u64 = 0;
        let mut tampon = vec![0u8; TAILLE_BLOC];
        loop {
            let lu = remplir_tampon(source, &mut tampon)?;
            if lu == 0 {
                break;
            }
            for octet in tampon.iter_mut().skip(lu) {
                *octet = 0;
            }
            let bloc = self.allouer_bloc()?;
            ecrire_bloc(
                &mut self.stockage,
                self.algo.as_ref(),
                &self.cle_volume,
                bloc,
                &tampon,
            )?;
            blocs.push(bloc);
            taille += lu as u64;
            if lu < TAILLE_BLOC {
                break;
            }
        }
        self.poser_blocs_et_arbre(id, blocs, taille)?;
        Ok(taille)
    }

    // lecture complete du contenu d'un inode
    fn lire_contenu(&mut self, inode: &Inode) -> ResultatFs<Vec<u8>> {
        let blocs = self.collecter_blocs(inode)?;
        let mut donnees = Vec::with_capacity(inode.taille as usize);
        for bloc in blocs {
            let contenu =
                lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
            donnees.extend_from_slice(&contenu);
        }
        donnees.truncate(inode.taille as usize);
        Ok(donnees)
    }

    // ecriture du contenu d'un inode dans un flux, bloc par bloc
    fn lire_contenu_flux<W: Write + ?Sized>(&mut self, inode: &Inode, sortie: &mut W) -> ResultatFs<()> {
        let blocs = self.collecter_blocs(inode)?;
        let mut restant = inode.taille;
        for bloc in blocs {
            let contenu =
                lire_bloc(&mut self.stockage, self.algo.as_ref(), &self.cle_volume, bloc)?;
            let n = restant.min(TAILLE_BLOC as u64) as usize;
            sortie.write_all(&contenu[..n])?;
            restant -= n as u64;
        }
        Ok(())
    }

    fn purger_bitmap(&mut self) -> ResultatFs<()> {
        self.bitmap.purger(
            &mut self.stockage,
            self.algo.as_ref(),
            &self.cle_volume,
            &self.superbloc,
        )
    }

    // ------- gestion des dossiers -------

    fn lire_entrees(&mut self, id: u64) -> ResultatFs<Vec<EntreeDossier>> {
        let inode = self.lire_inode(id)?;
        let contenu = self.lire_contenu(&inode)?;
        deserialiser_entrees(&contenu)
    }

    fn ecrire_entrees(&mut self, id: u64, entrees: &[EntreeDossier]) -> ResultatFs<()> {
        let contenu = serialiser_entrees(entrees);
        self.ecrire_contenu(id, &contenu)
    }

    // ------- resolution des chemins -------

    // normalisation d'un chemin absolu
    pub fn normaliser(chemin: &str) -> ResultatFs<String> {
        if !chemin.starts_with('/') {
            return Err(ErreurFs::NomInvalide(chemin.to_string()));
        }
        let segments: Vec<&str> = chemin.split('/').filter(|s| !s.is_empty()).collect();
        if segments.is_empty() {
            return Ok("/".to_string());
        }
        for segment in &segments {
            valider_nom(segment)?;
        }
        Ok(format!("/{}", segments.join("/")))
    }

    // separation en chemin parent et nom
    fn separer_parent(chemin: &str) -> (String, String) {
        let position = chemin.rfind('/').unwrap();
        let parent = if position == 0 {
            "/".to_string()
        } else {
            chemin[..position].to_string()
        };
        (parent, chemin[position + 1..].to_string())
    }

    fn resoudre(&self, chemin: &str) -> ResultatFs<u64> {
        self.index
            .chercher(chemin)
            .ok_or_else(|| ErreurFs::Introuvable(chemin.to_string()))
    }

    // reconstruction complete de l'index au montage
    fn reconstruire_index(&mut self) -> ResultatFs<()> {
        self.index = IndexRapide::nouveau();
        let mut pile = vec![("/".to_string(), INODE_RACINE)];
        while let Some((chemin, id)) = pile.pop() {
            for entree in self.lire_entrees(id)? {
                let chemin_enfant = if chemin == "/" {
                    format!("/{}", entree.nom)
                } else {
                    format!("{}/{}", chemin, entree.nom)
                };
                self.index.inserer(chemin_enfant.clone(), entree.id_inode);
                if entree.type_noeud == TypeNoeud::Dossier {
                    pile.push((chemin_enfant, entree.id_inode));
                }
            }
        }
        Ok(())
    }

    // creation d'un noeud rattache a son parent
    fn creer_noeud(&mut self, chemin: &str, type_noeud: TypeNoeud) -> ResultatFs<u64> {
        let chemin = Self::normaliser(chemin)?;
        if self.index.chercher(&chemin).is_some() {
            return Err(ErreurFs::ExisteDeja(chemin));
        }
        let (parent, nom) = Self::separer_parent(&chemin);
        let id_parent = self.resoudre(&parent)?;
        let inode_parent = self.lire_inode(id_parent)?;
        if inode_parent.type_noeud != TypeNoeud::Dossier {
            return Err(ErreurFs::PasUnDossier(parent));
        }

        let id = self.allouer_inode()?;
        let inode = Inode::nouveau(type_noeud, maintenant());
        self.ecrire_inode(id, &inode)?;

        // ajout de l'entree au dossier parent
        let mut entrees = self.lire_entrees(id_parent)?;
        entrees.push(EntreeDossier {
            id_inode: id,
            type_noeud,
            nom,
        });
        self.ecrire_entrees(id_parent, &entrees)?;
        self.index.inserer(chemin, id);
        Ok(id)
    }

    // ------- operations publiques -------

    // creation d'un dossier
    pub fn creer_dossier(&mut self, chemin: &str) -> ResultatFs<()> {
        self.creer_noeud(chemin, TypeNoeud::Dossier)?;
        Ok(())
    }

    // ecriture d'un fichier, creation ou remplacement
    pub fn ecrire_fichier(&mut self, chemin: &str, donnees: &[u8]) -> ResultatFs<()> {
        let chemin = Self::normaliser(chemin)?;
        let id = match self.index.chercher(&chemin) {
            Some(id) => {
                let inode = self.lire_inode(id)?;
                if inode.type_noeud != TypeNoeud::Fichier {
                    return Err(ErreurFs::PasUnFichier(chemin));
                }
                id
            }
            None => self.creer_noeud(&chemin, TypeNoeud::Fichier)?,
        };
        self.ecrire_contenu(id, donnees)
    }

    // ecriture d'un fichier depuis un flux, taille pratiquement illimitee
    // les donnees ne transitent jamais entierement par la memoire
    pub fn ecrire_fichier_flux<R: Read + ?Sized>(
        &mut self,
        chemin: &str,
        source: &mut R,
    ) -> ResultatFs<u64> {
        let chemin = Self::normaliser(chemin)?;
        let id = match self.index.chercher(&chemin) {
            Some(id) => {
                let inode = self.lire_inode(id)?;
                if inode.type_noeud != TypeNoeud::Fichier {
                    return Err(ErreurFs::PasUnFichier(chemin));
                }
                id
            }
            None => self.creer_noeud(&chemin, TypeNoeud::Fichier)?,
        };
        self.ecrire_contenu_flux(id, source)
    }

    // lecture complete d'un fichier, dechiffrement en memoire seulement
    pub fn lire_fichier(&mut self, chemin: &str) -> ResultatFs<Vec<u8>> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        let inode = self.lire_inode(id)?;
        if inode.type_noeud != TypeNoeud::Fichier {
            return Err(ErreurFs::PasUnFichier(chemin));
        }
        self.lire_contenu(&inode)
    }

    // lecture d'un fichier vers un flux, bloc par bloc
    pub fn lire_fichier_flux<W: Write + ?Sized>(&mut self, chemin: &str, sortie: &mut W) -> ResultatFs<()> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        let inode = self.lire_inode(id)?;
        if inode.type_noeud != TypeNoeud::Fichier {
            return Err(ErreurFs::PasUnFichier(chemin));
        }
        self.lire_contenu_flux(&inode, sortie)
    }

    // taille d'un fichier sans lire son contenu
    pub fn taille_fichier(&mut self, chemin: &str) -> ResultatFs<u64> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        Ok(self.lire_inode(id)?.taille)
    }

    // liste des entrees d'un dossier
    pub fn lister(&mut self, chemin: &str) -> ResultatFs<Vec<InfoEntree>> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        let inode = self.lire_inode(id)?;
        if inode.type_noeud != TypeNoeud::Dossier {
            return Err(ErreurFs::PasUnDossier(chemin));
        }
        let entrees = self.lire_entrees(id)?;
        let mut infos = Vec::with_capacity(entrees.len());
        for entree in entrees {
            let inode = self.lire_inode(entree.id_inode)?;
            infos.push(InfoEntree {
                nom: entree.nom,
                type_noeud: entree.type_noeud,
                taille: inode.taille,
                modifie: inode.modifie,
            });
        }
        infos.sort_by(|a, b| a.nom.cmp(&b.nom));
        Ok(infos)
    }

    // suppression d'un fichier ou d'un dossier vide
    pub fn supprimer(&mut self, chemin: &str) -> ResultatFs<()> {
        let chemin = Self::normaliser(chemin)?;
        if chemin == "/" {
            return Err(ErreurFs::NomInvalide("/".into()));
        }
        let id = self.resoudre(&chemin)?;
        let inode = self.lire_inode(id)?;
        if inode.type_noeud == TypeNoeud::Dossier && !self.lire_entrees(id)?.is_empty() {
            return Err(ErreurFs::DossierNonVide(chemin));
        }

        // retrait de l'entree du parent
        let (parent, nom) = Self::separer_parent(&chemin);
        let id_parent = self.resoudre(&parent)?;
        let mut entrees = self.lire_entrees(id_parent)?;
        entrees.retain(|e| e.nom != nom);
        self.ecrire_entrees(id_parent, &entrees)?;

        // liberation des blocs et de l'inode
        self.liberer_contenu(&inode)?;
        self.ecrire_inode(id, &Inode::vierge())?;
        self.index.retirer(&chemin);
        self.purger_bitmap()
    }

    // renommage dans le meme dossier
    pub fn renommer(&mut self, chemin: &str, nouveau_nom: &str) -> ResultatFs<()> {
        let chemin = Self::normaliser(chemin)?;
        valider_nom(nouveau_nom)?;
        let (parent, ancien_nom) = Self::separer_parent(&chemin);
        let nouveau_chemin = if parent == "/" {
            format!("/{nouveau_nom}")
        } else {
            format!("{parent}/{nouveau_nom}")
        };
        if self.index.chercher(&nouveau_chemin).is_some() {
            return Err(ErreurFs::ExisteDeja(nouveau_chemin));
        }
        let id_parent = self.resoudre(&parent)?;
        let mut entrees = self.lire_entrees(id_parent)?;
        let entree = entrees
            .iter_mut()
            .find(|e| e.nom == ancien_nom)
            .ok_or_else(|| ErreurFs::Introuvable(chemin.clone()))?;
        entree.nom = nouveau_nom.to_string();
        self.ecrire_entrees(id_parent, &entrees)?;
        self.index.renommer_prefixe(&chemin, &nouveau_chemin);
        Ok(())
    }

    // ajout ou remplacement d'une metadonnee etendue
    pub fn definir_meta(&mut self, chemin: &str, cle: &str, valeur: &str) -> ResultatFs<()> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        let mut inode = self.lire_inode(id)?;
        inode.metas.retain(|(c, _)| c != cle);
        inode.metas.push((cle.to_string(), valeur.to_string()));
        inode.modifie = maintenant();
        self.ecrire_inode(id, &inode)
    }

    // lecture des metadonnees etendues
    pub fn lire_metas(&mut self, chemin: &str) -> ResultatFs<Vec<(String, String)>> {
        let chemin = Self::normaliser(chemin)?;
        let id = self.resoudre(&chemin)?;
        Ok(self.lire_inode(id)?.metas)
    }

    // etat global du volume
    pub fn statistiques(&self) -> Statistiques {
        Statistiques {
            nb_blocs_donnees: self.superbloc.nb_blocs_donnees,
            blocs_libres: self.bitmap.nb_libres(),
            nb_inodes: self.superbloc.nb_inodes,
            taille_bloc: TAILLE_BLOC,
            nb_chemins_indexes: self.index.nb_entrees(),
        }
    }

    // ecriture de tout l'etat en attente
    pub fn synchroniser(&mut self) -> ResultatFs<()> {
        self.purger_bitmap()?;
        self.stockage.synchroniser()
    }

    // restitue le support apres synchronisation
    pub fn demonter(mut self) -> ResultatFs<S> {
        self.synchroniser()?;
        Ok(self.stockage)
    }
}
