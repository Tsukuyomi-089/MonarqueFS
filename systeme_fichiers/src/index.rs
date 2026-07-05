// index rapide chemin vers inode

use std::collections::HashMap;

pub struct IndexRapide {
    chemins: HashMap<String, u64>,
}

impl IndexRapide {
    pub fn nouveau() -> Self {
        let mut chemins = HashMap::new();
        // la racine est l'inode 1
        chemins.insert("/".to_string(), crate::inode::INODE_RACINE);
        Self { chemins }
    }

    // recherche directe en o(1)
    pub fn chercher(&self, chemin: &str) -> Option<u64> {
        self.chemins.get(chemin).copied()
    }

    pub fn inserer(&mut self, chemin: String, id_inode: u64) {
        self.chemins.insert(chemin, id_inode);
    }

    pub fn retirer(&mut self, chemin: &str) {
        self.chemins.remove(chemin);
    }

    // deplacement d'un sous arbre entier
    pub fn renommer_prefixe(&mut self, ancien: &str, nouveau: &str) {
        let prefixe_enfants = format!("{ancien}/");
        let concernes: Vec<String> = self
            .chemins
            .keys()
            .filter(|c| c.as_str() == ancien || c.starts_with(&prefixe_enfants))
            .cloned()
            .collect();
        for chemin in concernes {
            if let Some(id) = self.chemins.remove(&chemin) {
                let suffixe = &chemin[ancien.len()..];
                self.chemins.insert(format!("{nouveau}{suffixe}"), id);
            }
        }
    }

    pub fn nb_entrees(&self) -> usize {
        self.chemins.len()
    }
}
