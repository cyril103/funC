# Guide de migration FunC

## 1) État de version

- Projet en pré-release `0.1.0`.
- Aucune contrainte de compatibilité stricte de format de sortie binaire n’est encore promise.
- Les changements de syntaxe sont généralement rétrocompatibles au sein de la branche actuelle.

## 2) Migration vers `0.1.0`

### CLI

- La commande passe à un subcommand explicite: `compile|fmt|validate|list-targets`.
- Les options d’objets/exécutables sont consolidées sur:
  - `--emit-obj`
  - `--emit-exe`
  - `--backend-profile`
- `--backend-profile` accepte `none`, `safe`, `aggressive`
  - `none` (défaut) = pas d’optimisation `opt` agressive
  - `safe` = `-O1`
  - `aggressive` = `-O3`

### Débogage

- `--debug-info` active la transmission d’options de debug au linker:
  - Linux/macOS: `-g` (ou mode équivalent côté linker)
  - Windows: `/DEBUG`

### Import de modules

- `import "nom";` résout encore les chemins relatifs par rapport au fichier appelant.
- Extension `.fc` ajoutée automatiquement si absente.
- Réutilisation de modules déjà chargés pour éviter les imports en boucle.

### Règles de mémoire renforcées

- Les diagnostics mémoire conservent une sortie catégorisée (`MEM-001`, `SEM-001`, `BEG-001`, ...).
- `--check` reste le point d’entrée recommandé avant génération IR/obj/exe.

## 3) Points de rupture connus

- Les programmes reposant sur des schémas de déclaration non conformes à la langue (ou incomplets)
  peuvent être rejetés avec des messages de précision plus stricts qu’en `0.0.x`.
- Les diagnostics peuvent désormais stopper la compilation sur des cas qui étaient auparavant
  acceptés avec warnings implicites.

## 4) Vérification post-migration (recommandée)

```bash
cargo run -- compile examples/getting-started/sample_if_else.fc --check
cargo run -- compile examples/getting-started/sample.fc --emit-ir --out /tmp/sample.ll
cargo run -- compile examples/getting-started/sample_if_else.fc --emit-exe --out-exe /tmp/sample
```

## 5) Migration inter-version (si nécessaire)

- Migrer par étape: `--check` -> `--emit-ir` -> `--emit-obj` -> `--emit-exe`.
- En cas de doute, tester d’abord `--emit-obj` sur une cible connue puis élargir.
- Conserver les artefacts générés dans `/tmp` pour comparer les écarts lors d’un rollback.
