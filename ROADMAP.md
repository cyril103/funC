# 🗺️ Feuille de Route : Compilateur FunC v2

## 🧭 Reprise immédiate (à partir d'aujourd'hui)
1. ✅ Finaliser `Objectif 1`
2. ✅ Prioriser les 3 premières cartes, valables en l'état :
3. ✅ Ajouter `else if` et l’`else if` chaîné
4. ✅ Accepter la condition `if` entre parenthèses
5. ✅ Ajouter l’opérateur `!` booléen
6. ✅ Préparer l’erreur parser/typecheck avec `line:column`, extrait source et suggestion
7. ✅ Implémenter `funC compile --check` (analyse seule)
8. ✅ Implémenter `funC asm` via `--emit-asm`
9. ✅ Uniformiser la sortie exécutable avec `--out-exe` sur toutes les cibles

## 🧭 Objectif 1 — MVP robuste (2–4 séances)
1. `if / else` plus riche
   - ✅ Ajouter `else if`
   - ✅ Accepter la parenthèse conditionnelle optionnelle
   - ✅ Ajouter l'opérateur booléen `!`
2. Diagnostic UX
   - ✅ Enrichir toutes les erreurs parser/typecheck avec `line:column`
   - ✅ Afficher un extrait de source et une suggestion de correction
3. Commande CLI `check`
   - ✅ Ajouter `funC compile` avec mode d'analyse-only (`--check`)
   - ✅ Parse + typage sans génération d'IR
4. Commande CLI `asm`
   - ✅ Ajouter `--emit-asm` via `llc -filetype=asm`
5. Cohérence de sortie
   - ✅ Normaliser le nommage des exécutables (`--out-exe`) sur toutes les cibles

## 🚀 Objectif 2 — Langage plus expressif (4–8 séances)
1. Boucles
   - ✅ Ajouter `while`
   - ✅ Ajouter `for`
2. Retour explicite
   - ✅ Ajouter `return` comme mot-clé utilisable dans les blocs
3. Mutabilité
   - ✅ Introduire `let mut`
4. Fonctions
   - ✅ Améliorer la robustesse des appels
   - ✅ Vérification stricte du type de retour et de l'arité
5. Collections de base
   - ✅ Ajouter les tableaux statiques `[T; N]`
   - ✅ Ajouter l'indexation `arr[i]`

## 🧠 Objectif 3 — Sémantique forte (8–12 séances)
1. Table des symboles
   - ✅ Détecter le shadowing involontaire
   - ✅ Détecter les variables non utilisées
2. Vérification de flux
   - ✅ Vérifier qu'un bloc `if/else` (et branches) retourne un type cohérent dans toutes les issues
3. Gestion mémoire
   - `alloc` / `free` avec vérification heuristique des leaks (`--warn-memory`)
4. Erreurs de types avancées
   - Messages plus précis pour incompatibilités (`i32` vs `i64`, signed/unsigned, pointeurs)
5. Optimisations minimales
   - Faire du constant folding simple dans l’AST (ex: `2 + 3`, `true && x`, `x || false`)

## 🧪 Objectif 4 — Outils & ergonomie (12–16 séances)
1. Commande de formatage
   - Ajouter `funC fmt` pour une mise en forme de base
2. Référentiel d’exemples
   - `examples/if_else`, `examples/loops`, `examples/pointers`, `examples/cross-target`
3. Documentation utilisateur
   - README simplifié avec flux de compilation en 3 lignes
4. CI
   - Workflow GitHub Actions : build + tests + check docs
5. Tests d’intégration
   - Ajouter une suite régression parser / typecheck / codegen

## 🌌 Objectif 5 — Pistes avancées
1. Modules
   - Ajouter un système `import` simple pour compiler plusieurs fichiers
2. Types complexes
   - Ajouter `struct` et `enum`
3. Builtins mémoire
   - Ajouter `memcpy`, `memset`, `realloc`
4. Debug info
   - Générer une option `--debug-info`
