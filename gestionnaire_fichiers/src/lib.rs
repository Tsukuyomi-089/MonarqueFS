// api de haut niveau pour la gestion du disque et des fichiers

pub mod administration;
pub mod arborescence;
pub mod mise_a_jour;
pub mod privileges;
pub mod session;
pub mod surveillance;

pub use administration::{
    ajouter_partition, creer_disque, formater_partition, lister_partitions, ouvrir_partition,
    preparer_support, supprimer_partition, InfoPartition,
};
pub use arborescence::{Arborescence, ArbreHote, ArbreVolume, EntreeArbre};
pub use privileges::{autoriser_peripheriques, pkexec_disponible};
pub use session::Session;
pub use surveillance::{est_monarque, lister_peripheriques, peripheriques_monarque, InfoPeripherique};
pub use systeme_fichiers::{ErreurFs, InfoEntree, ResultatFs, Statistiques, TypeNoeud};
