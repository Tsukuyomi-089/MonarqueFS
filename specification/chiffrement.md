# Chiffrement de MonarqueFS

## Principe

Aucune donnée n'est stockée en clair : contenu des fichiers, table d'inodes,
entrées de dossiers, bitmap et blocs d'indirection sont tous chiffrés.
Le déchiffrement n'a lieu qu'en mémoire, à la lecture.

## Algorithme par défaut

ChaCha20-Poly1305 (RFC 8439), implémenté dans le projet sans dépendance
externe et validé par les vecteurs de test officiels :

- `chacha20.rs` — chiffrement de flux (vecteurs §2.3.2 et §2.4.2)
- `poly1305.rs` — authentification (vecteur §2.5.2)
- `aead.rs` — composition scellée (vecteur §2.8.2)
- `sha256.rs` + `derivation.rs` — pbkdf2-hmac-sha256 (vecteurs RFC 4231 et 7914)

## Format d'un bloc chiffré

```
[ nonce 12 octets | charge chiffrée 4096 octets | étiquette 16 octets ]
```

- nonce aléatoire frais à chaque écriture (source : /dev/urandom)
- le numéro de bloc sert de donnée associée : un bloc déplacé est rejeté
- toute altération est détectée par l'étiquette Poly1305

## Gestion des clés

- une clé de volume de 32 octets, générée aléatoirement au formatage
- la phrase secrète dérive une clé d'enveloppe : pbkdf2-hmac-sha256,
  sel de 16 octets, 60 000 itérations
- la clé de volume est stockée enveloppée (chiffrée et authentifiée)
  dans le superbloc : jamais de clé en clair sur le disque
- une phrase incorrecte est détectée par l'échec d'authentification
  de l'enveloppe, sans lire aucune donnée

## Interchangeabilité

Le trait `AlgorithmeChiffrement` isole l'algorithme ; l'identifiant stocké
dans le superbloc sélectionne l'implémentation au montage. Ajouter un
algorithme = une implémentation du trait + une entrée dans la fabrique.
