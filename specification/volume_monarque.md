# Volume MonarqueFS (MFS1)

Système de fichiers chiffré par défaut, par blocs, avec table d'inodes.

## Géométrie

- bloc utile : 4096 octets
- bloc sur disque : 12 (nonce) + 4096 (chiffré) + 16 (étiquette) = 4124 octets
- disposition : `[superbloc][bitmap][inodes][données]`
- environ un inode pour huit blocs (minimum 128)

## Superbloc (bloc 0, non chiffré sauf la clé enveloppée)

| décalage | taille | champ |
|---|---|---|
| 0 | 4 | magie `MFS1` |
| 4 | 2 | version |
| 6 | 2 | identifiant d'algorithme de chiffrement |
| 8 | 4 | taille de bloc |
| 12 | 8 | nombre total de blocs |
| 20 | 8 | nombre d'inodes |
| 28 | 8×5 | débuts et tailles des zones (bitmap, inodes, données) |
| 68 | 8 | nombre de blocs de données |
| 76 | 4 | itérations kdf |
| 80 | 16 | sel kdf |
| 96 | 12 | nonce de l'enveloppe de clé |
| 108 | 48 | clé de volume enveloppée (32 + étiquette 16) |

## Inode (256 octets, 16 par bloc)

| décalage | taille | champ |
|---|---|---|
| 0 | 1 | type : 0 libre, 1 fichier, 2 dossier |
| 4 | 4 | nombre de liens |
| 8 | 8 | taille en octets |
| 16 | 16 | horodatages création / modification |
| 32 | 96 | 12 pointeurs directs (u64) |
| 128 | 8 | pointeur indirect simple (512 pointeurs) |
| 136 | 8 | pointeur indirect double (512×512 pointeurs) |
| 144 | 112 | métadonnées étendues (longueur clé, longueur valeur, octets) |

- inode 1 : dossier racine
- capacité maximale d'un fichier : (12 + 512 + 512²) × 4096 ≈ 1 Go

## Dossiers

Le contenu d'un dossier est un fichier d'entrées de 64 octets :
identifiant d'inode (8), type (1), longueur du nom (1), nom (54).

## Index rapide

Au montage, l'arborescence est parcourue une fois et chaque chemin absolu
est indexé dans une table de hachage → résolution de chemin en O(1).
L'index est maintenu à chaque création, suppression et renommage.
