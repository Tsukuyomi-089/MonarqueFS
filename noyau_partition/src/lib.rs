// noyau de gestion du disque logique et de la table de partition

pub mod disque;
pub mod erreurs;
pub mod table_partition;

pub use disque::{DisqueLogique, TAILLE_SECTEUR};
pub use erreurs::{ErreurNoyau, Resultat};
pub use table_partition::{EntreePartition, TablePartition, TypePartition, VuePartition};
