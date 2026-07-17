# Exemples FunC par domaine

## Contrôle

- `examples/getting-started/sample_if_else.fc`
  - `if/else` avec expressions de branchement de type `i64`.
- `examples/loops/loops.fc`
  - `while` et `for`, appels de fonctions, calcul d’accumulations.
- `examples/if_else/basic.fc`
  - Comparaison et retour de valeur selon branche.

## Mémoire

- `examples/pointers/pointers.fc`
  - `alloc`, `store`, `load`, `free` avec primitives builtin mémoire.

## Modules / Imports

- `examples/modules/main.fc` + `examples/modules/math.fc`
  - `import "math";` et composition multi-fichiers.

## CLI / Chaîne LLVM

- Générer l’IR: `--emit-ir`
  - `cargo run -- compile examples/getting-started/sample.fc --emit-ir --out /tmp/sample.ll`
- Générer objet: `--emit-obj`
  - `cargo run -- compile examples/getting-started/sample.fc --emit-obj --out /tmp/sample.o`
- Générer exécutable: `--emit-exe`
  - `cargo run -- compile examples/getting-started/sample.fc --emit-exe --out-exe /tmp/sample`
- Profil backend: `--backend-profile` (`none`/`safe`/`aggressive`)
- Afficher les cibles: `cargo run -- list-targets`

## Intégrations ciblées

- `examples/cross-target/cross-target.fc`
  - Vérification rapide des alias/cibles disponibles.
