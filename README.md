# 👑 MonarqueFS

**Système de stockage complet, chiffré par défaut, écrit en Rust pur.**

MonarqueFS est un écosystème de stockage autonome : une table de partition propriétaire, un système de fichiers chiffré, des outils en ligne de commande et un gestionnaire de fichiers graphique — sans aucune dépendance externe dans les couches critiques.

```
┌─────────────────────┐   ┌─────────────────────┐
│ interface_graphique │   │     outils_cli      │
│    (monarque_gui)   │   │     (monarque)      │
└──────────┬──────────┘   └──────────┬──────────┘
           └───────────┬─────────────┘
                       ▼
           ┌───────────────────────┐
           │ gestionnaire_fichiers │   api de haut niveau
           └───────────┬───────────┘
                       ▼
           ┌───────────────────────┐
           │   systeme_fichiers    │   volume chiffré : blocs,
           └───────────┬───────────┘   inodes, dossiers, index
                       ▼
           ┌───────────────────────┐
           │    noyau_partition    │   disque logique et
           └───────────────────────┘   table de partition
```

---

## ✨ Caractéristiques

### Table de partition propriétaire
- format `MONARQUE`, indépendant de MBR/GPT
- jusqu'à 32 partitions par disque logique
- allocation par premier ajustement, lecture en deux accès disque

### Système de fichiers MonarqueFS
- allocation par blocs de 4096 octets
- table d'inodes : 12 pointeurs directs, indirection simple et double (fichiers jusqu'à ~1 Go)
- hiérarchie complète de dossiers
- **index rapide** : résolution de chemin en O(1) via table de hachage construite au montage
- **métadonnées étendues** clé/valeur sur chaque fichier et dossier

### Chiffrement natif — rien n'est stocké en clair
- chaque bloc sur disque : `nonce (12) │ données chiffrées (4096) │ étiquette (16)`
- ChaCha20-Poly1305 (RFC 8439) **implémenté dans le projet** et validé par les vecteurs de test officiels
- contenu, noms de fichiers, inodes, bitmap : tout est chiffré
- clé de volume aléatoire, enveloppée par PBKDF2-HMAC-SHA256 (60 000 itérations) — jamais de clé en clair
- déchiffrement en mémoire uniquement ; toute altération du disque est détectée à la lecture
- algorithme interchangeable via le trait `AlgorithmeChiffrement`

---

## 🚀 Démarrage rapide

### Compilation

```bash
cargo build --release
```

### Créer un disque, le partitionner, le formater

```bash
export MONARQUE_PHRASE="ma phrase secrete"
monarque=./target/release/monarque

$monarque creer disque.img 64                  # disque logique de 64 Mo
$monarque partitionner disque.img systeme 24   # partition 0
$monarque partitionner disque.img donnees 24   # partition 1
$monarque inspecter disque.img                 # table de partition
$monarque formater disque.img 1                # volume MonarqueFS chiffré
```

### Stocker et lire des fichiers chiffrés

```bash
$monarque creer_dossier disque.img 1 /documents
$monarque importer disque.img 1 note.txt /documents/note.txt
$monarque lister disque.img 1 /documents
$monarque afficher disque.img 1 /documents/note.txt
$monarque meta disque.img 1 /documents/note.txt auteur tsuky
$monarque etat disque.img 1
```

> Sans `MONARQUE_PHRASE`, la phrase secrète est demandée au clavier.

### Explorer graphiquement

```bash
cargo run --release -p interface_graphique
```

Connexion à une image disque, navigation dans l'arborescence, import/export,
renommage, suppression, aperçu des fichiers texte et édition des métadonnées.

---

## 🧰 Commandes du CLI

| commande | rôle |
|---|---|
| `creer <image> <taille_mo>` | crée un disque logique |
| `partitionner <image> <nom> <taille_mo>` | ajoute une partition |
| `supprimer_partition <image> <index>` | retire une partition |
| `inspecter <image>` | affiche la table de partition |
| `formater <image> <index>` | formate en MonarqueFS |
| `etat <image> <index>` | statistiques du volume |
| `lister <image> <index> [chemin]` | liste un dossier |
| `creer_dossier <image> <index> <chemin>` | crée un dossier |
| `importer <image> <index> <source> <dest>` | copie hôte → volume |
| `exporter <image> <index> <source> <dest>` | copie volume → hôte |
| `afficher <image> <index> <chemin>` | affiche un fichier texte |
| `effacer <image> <index> <chemin>` | supprime fichier ou dossier vide |
| `renommer <image> <index> <chemin> <nom>` | renomme une entrée |
| `meta <image> <index> <chemin> [cle valeur]` | lit ou définit une métadonnée |

---

## 🔒 Sécurité

- **Confidentialité native** : un accès brut au disque ne révèle ni contenu, ni noms, ni structure — vérifié par un test automatisé qui balaye l'image octet par octet.
- **Intégrité** : chaque bloc est authentifié (Poly1305) et lié à sa position — un bloc altéré, tronqué ou déplacé est rejeté.
- **Phrase secrète** : une phrase incorrecte échoue à l'ouverture de l'enveloppe de clé, sans lire la moindre donnée.
- **Cryptographie vérifiée** : SHA-256, HMAC, PBKDF2, ChaCha20, Poly1305 et l'AEAD passent tous les vecteurs officiels (RFC 8439, RFC 4231, RFC 7914, FIPS 180-4).

---

## 📁 Structure du projet

```
noyau_partition/        disque logique, table de partition propriétaire
systeme_fichiers/       volume chiffré : superbloc, bitmap, inodes, dossiers, index
  └── chiffrement/      chacha20, poly1305, aead, sha256, pbkdf2, aléa système
gestionnaire_fichiers/  api de haut niveau : administration + session
outils_cli/             binaire `monarque`
interface_graphique/    binaire `monarque_gui` (eframe)
specification/          formats binaires et architecture documentés
graphity/               graphity.json — carte vivante du projet
```

La documentation de référence est dans [`specification/`](specification/) :
[architecture](specification/architecture.md) ·
[table de partition](specification/table_partition.md) ·
[volume](specification/volume_monarque.md) ·
[chiffrement](specification/chiffrement.md)

---

## 🧪 Tests

```bash
cargo test --workspace
```

24 tests : vecteurs cryptographiques officiels, cycle complet
disque → partition → volume → fichier, persistance après remontage,
indirections sur gros fichiers, rejet des mauvaises phrases,
et vérification de l'absence totale de données en clair sur le disque.

---

## 🧭 Conventions

- **langage unique : Rust** — aucune logique système hors Rust
- **tout en français** : fichiers, fonctions, variables, modules, commentaires
- `snake_case` obligatoire
- `graphity/graphity.json` est mis à jour après chaque évolution structurelle

## Priorités de conception

1. **vitesse** — index O(1), bitmap incrémentale, formats binaires fixes
2. **stabilité** — chiffrement authentifié, erreurs typées, tests d'intégration
3. **simplicité** — formats documentés, couches strictes
4. **extensibilité** — algorithme de chiffrement et supports de stockage interchangeables
