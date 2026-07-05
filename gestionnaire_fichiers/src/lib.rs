// api de haut niveau pour la gestion du disque et des fichiers

pub mod administration;
pub mod session;

pub use administration::{
    ajouter_partition, creer_disque, formater_partition, lister_partitions, ouvrir_partition,
    supprimer_partition, InfoPartition,
};
pub use session::Session;
pub use systeme_fichiers::{ErreurFs, InfoEntree, ResultatFs, Statistiques, TypeNoeud};
