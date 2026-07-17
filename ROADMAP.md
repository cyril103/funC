# 🗺️ Feuille de route FunC — Bilan & suite

## ✅ Fonctions livrées (État actuel)

### Objectif 1 — MVP robuste
- ✅ `if / else` enrichi (`else if`, parenthèses optionnelles, opérateur `!`)
- ✅ Diagnostics parser/typecheck avec `line:column`, extrait source et suggestion
- ✅ `funC compile --check`
- ✅ `--emit-asm` pour la génération assembleur
- ✅ Normalisation de `--out-exe` multi-cibles

### Objectif 2 — Langage plus expressif
- ✅ `while`, `for`
- ✅ `return` explicite
- ✅ `let mut`
- ✅ Robustesse des appels (signature/arity/types)
- ✅ Tableaux statiques `[T; N]` et indexation `arr[i]`

### Objectif 3 — Sémantique forte
- ✅ Détection de shadowing
- ✅ Variables non utilisées
- ✅ Vérification de flux `if/else`
- ✅ Gestion mémoire (`alloc`, `free`, `--warn-memory`)
- ✅ Erreurs de type enrichies (entiers/signes/pointeurs)
- ✅ Constant folding de base (arithmétique, booléens, opérations d’égalité)

### Objectif 4 — Outils & ergonomie
- ✅ `funC fmt` + mode `--check`
- ✅ Exemples structurés dans `examples/*`
- ✅ README orienté usage
- ✅ CI GitHub Actions opérationnelle
- ✅ Régression parser / typecheck / codegen

### Objectif 5 — Pistes avancées
- ✅ Imports multi-fichiers (`import`)
- ✅ Types complexes (`struct`, `enum`)
- ✅ Builtins mémoire (`memcpy`, `memset`, `realloc`)
- ✅ `--debug-info` (LLDB/GDB-friendly via `llc -g`)

## 🚩 Bilan qualité (post-roadmap)
- ✅ Parse + typecheck + génération IR/objet/exécutable fonctionnels.
- ✅ Les `.fc` sont correctement isolés dans `examples/`.
- ✅ Le projet peut être compilé et vérifié en `--check`.
- ⚠️ Ajustements de qualité en cours: suppression des warnings de compilation.

## 🔜 Prochaine vague recommandée (stabilisation)
- ✅ Nettoyage complet des warnings compiler (objectif immédiat).
- ✅ Meilleure couverture des cas `size_of`, struct/enum en backend.
- ✅ Erreurs de modules/CLI plus ciblées (cas d’échec de compilation lisibles).
- ✅ Tests d’intégration “pipeline complet” (source → exe par cible).
