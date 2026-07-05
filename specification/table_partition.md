# Table de partition Monarque (MPT)

Format propriétaire, indépendant de MBR/GPT, optimisé pour une lecture en deux accès.

## Disque logique

- fichier image, taille alignée sur le secteur (512 octets)
- secteurs 0 à 7 réservés à la table, données à partir du secteur 8

## Secteur 0 — en-tête

| décalage | taille | champ |
|---|---|---|
| 0 | 8 | magie `MONARQUE` |
| 8 | 2 | version (u16 LE) |
| 10 | 2 | nombre d'entrées (u16 LE) |
| 16 | 8 | prochain identifiant (u64 LE) |

## Secteurs 1 à 4 — entrées

32 entrées maximum, 64 octets chacune :

| décalage | taille | champ |
|---|---|---|
| 0 | 24 | nom (utf8, terminé par zéro, 23 octets utiles) |
| 24 | 8 | secteur de début (u64 LE) |
| 32 | 8 | nombre de secteurs (u64 LE) |
| 40 | 4 | type : 0 libre, 1 monarque_fs, 2 brute (u32 LE) |
| 44 | 4 | drapeaux (u32 LE) |
| 48 | 8 | identifiant unique (u64 LE) |
| 56 | 8 | réservé |

## Allocation

- les entrées sont maintenues triées par secteur de début
- l'ajout cherche le premier intervalle libre suffisant (premier ajustement)
- la suppression laisse un trou réutilisable
