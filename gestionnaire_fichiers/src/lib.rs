// api de haut niveau pour la gestion du disque et des fichiers

pub mod administration;
pub mod mise_a_jour;
pub mod session;
pub mod surveillance;

pub use administration::{
    ajouter_partition, creer_disque, formater_partition, lister_partitions, ouvrir_partition,
    preparer_support, supprimer_partition, InfoPartition,
};
pub use session::Session;
pub use surveillance::{est_monarque, lister_peripheriques, peripheriques_monarque, InfoPeripherique};
pub use systeme_fichiers::{ErreurFs, InfoEntree, ResultatFs, Statistiques, TypeNoeud};
