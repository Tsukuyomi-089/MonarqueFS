# Architecture de MonarqueFS

## Vue d'ensemble

MonarqueFS est un système de stockage complet écrit en Rust pur, en couches strictes :

```
interface_graphique (GUI)      outils_cli (monarque)
            \                       /
             gestionnaire_fichiers (API)
                      |
              systeme_fichiers (volume chiffré)
                      |
              noyau_partition (disque + table)
```

## Modules

| module | rôle | dépend de |
|---|---|---|
| `noyau_partition` | disque logique, table de partition propriétaire | — |
| `systeme_fichiers` | volume chiffré : blocs, inodes, dossiers, index | `noyau_partition` |
| `gestionnaire_fichiers` | API de haut niveau : administration + session | les deux couches basses |
| `outils_cli` | binaire `monarque` : formatage, inspection, fichiers | `gestionnaire_fichiers` |
| `interface_graphique` | binaire `monarque_gui` : explorateur graphique | `gestionnaire_fichiers` + `eframe` |

## Règles

- tout en français, snake_case
- aucune dépendance externe dans les couches critiques
  (le chiffrement est implémenté dans le projet et validé par les vecteurs RFC)
- chaque module est isolé et testé
- `graphity/graphity.json` reflète l'état réel du code après chaque tâche

## Priorités

1. vitesse — index O(1) des chemins, bitmap avec indice de recherche, blocs de 4096 octets
2. stabilité — chiffrement authentifié : toute corruption est détectée à la lecture
3. simplicité — formats binaires fixes documentés dans `specification/`
4. extensibilité — algorithme de chiffrement interchangeable par trait
