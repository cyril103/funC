# 🗺️ Feuille de Route : Compilateur FunC

Ce document retrace les étapes de développement pour concevoir le compilateur **FunC** en Rust avec LLVM. Cochez les cases au fur et à mesure de votre progression !

---

## 📦 Phase 1 : Lexer (Analyse Lexicale)
L'objectif est de découper le code source brut (chaîne de caractères) en une liste de jetons (Tokens) exploitables.

- [ ] **Définir l'énumération `Token` en Rust**
  - [ ] Mots-clés : `fn`, `let`, `if`, `else`, `alloc`, `free`, `load`, `store`, `sizeof`
  - [ ] Types : `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`, `void`
  - [ ] Identifiants (noms de variables/fonctions) et Littéraux (nombres entiers, flottants, booléens)
  - [ ] Opérateurs arithmétiques : `+`, `-`, `*`, `/`, `%`
  - [ ] Opérateurs logiques et de comparaison : `&&`, `||`, `==`, `!=`, `<`, `<=`, `>`, `>=`
  - [ ] Symboles syntaxiques : `(`, `)`, `{`, `}`, `[`, `]`, `:`, `->`, `=`, `,`, `;`, `*`
- [ ] **Écrire le Lexer**
  - [ ] Option A : À la main (boucle de lecture caractère par caractère, idéal pour apprendre).
  - [ ] Option B : Avec une bibliothèque de combinateurs (ex: `nom` ou `logos` pour des performances maximales).
- [ ] **Écrire des tests unitaires pour le Lexer**
  - [ ] Valider la détection des flottants (`3.14`).
  - [ ] Valider la distinction entre `=` (affectation) et `==` (comparaison).
  - [ ] Valider la gestion des espaces et des commentaires.

---

## 📐 Phase 2 : Parser & AST (Analyse Syntaxique)
L'objectif est de transformer la suite de Tokens en un Arbre de Syntaxe Abstraite (AST) représentant la structure logique du programme.

- [ ] **Définir la structure de l'AST en Rust**
  - [ ] Structures/Enums pour `Program`, `Function`, `Type`, `Expression` (avec toutes les variantes).
- [ ] **Implémenter le Parser**
  - [ ] Parser de descente récursive pour les structures globales (`fn`, paramètres, blocs `{}`).
  - [ ] Implémenter la priorité des opérateurs (Precedence Climbing ou algorithme Shunting-Yard) pour que `1 + 2 * 3` soit bien analysé comme `1 + (2 * 3)`.
  - [ ] Gérer l'associativité à gauche/droite (notamment pour les opérateurs logiques `&&` et `||`).
- [ ] **Écrire des tests pour l'AST**
  - [ ] Vérifier que les parenthèses forcent la priorité d'évaluation : `(1 + 2) * 3`.
  - [ ] Valider la structure imbriquée des expressions `if-else`.

---

## 🔍 Phase 3 : Analyse Sémantique & Typage (Type Checker)
Avant de générer du code, il faut s'assurer que le programme est cohérent. C'est ici que l'on rejette les programmes incorrects.

- [ ] **Table des symboles (Symbol Table)**
  - [ ] Enregistrer les fonctions déclarées et leurs signatures.
  - [ ] Gérer la portée (scope) des variables (une variable dans un bloc `{}` cache-t-elle une variable globale ?).
- [ ] **Le Type Checker**
  - [ ] Vérifier que les variables utilisées sont bien déclarées.
  - [ ] Valider la cohérence des types lors des opérations (ex: interdire `5 + true`).
  - [ ] S'assurer que les deux côtés d'une comparaison (`==`, `<`, etc.) possèdent le même type.
  - [ ] Vérifier que le type retourné par un bloc `{}` ou un `if-else` correspond à la signature de la fonction.
- [ ] **Évaluation statique de `sizeof`**
  - [ ] Résoudre `sizeof(T)` et le remplacer directement dans l'AST par une constante entière correspondant à la taille en octets du type ciblé.

---

## ⚡ Phase 4 : Génération de Code LLVM (CodeGen)
Traduire notre AST typé et validé en instructions LLVM IR en utilisant la bibliothèque Rust `inkwell` (ou `llvm-sys`).

- [ ] **Configuration de LLVM**
  - [ ] Initialiser le contexte LLVM, le Builder et le Module.
  - [ ] Configurer la cible système par défaut (Target Machine).
- [ ] **Génération des types et fonctions**
  - [ ] Traduire les types de FunC vers les types LLVM correspondants (`i32` de FunC -> `i32` de LLVM, `f64` -> `double`, etc.).
  - [ ] Enregistrer la signature des fonctions dans le module LLVM.
- [ ] **Génération des expressions de base**
  - [ ] Expressions arithmétiques.
  - [ ] Comparaisons :
    - [ ] Entiers / Pointeurs : utiliser l'instruction LLVM `icmp` (avec prédicats signés `sgt`, `slt` ou non signés `ugt`, `ult` selon le type FunC).
    - [ ] Flottants : utiliser `fcmp`.
  - [ ] Divisions et Modulos (`%`) :
    - [ ] Signés : `sdiv` / `srem`.
    - [ ] Non signés : `udiv` / `urem`.
    - [ ] Flottants : `fdiv` / `frem`.
- [ ] **Gestion du contrôle de flux**
  - [ ] Implémenter les branchements conditionnels pour les expressions `if-else`.
  - [ ] Implémenter le **court-circuitage** pour `&&` et `||` (générer des sauts conditionnels pour ne pas évaluer la seconde expression si la première suffit à déterminer le résultat).
- [ ] **Gestion de la mémoire**
  - [ ] Déclarer les fonctions externes de la libc : `malloc(i64) -> *i8` et `free(*i8) -> void`.
  - [ ] Traduire `alloc(taille)` en un appel à `malloc` puis convertir le pointeur retourné (`*i8` de LLVM) vers le type attendu.
  - [ ] Traduire `free(ptr)` en appel à `free`.
  - [ ] Traduire `store(val, ptr)` et `load(ptr)` en instructions de lecture/écriture mémoire LLVM (`load` / `store`).

---

## 🚀 Phase 5 : Driver, Compilation Croisée et Packaging
Faire de votre compilateur un outil en ligne de commande (CLI) utilisable par d'autres.

- [ ] **Créer l'interface CLI en Rust** (avec `clap`)
  - [ ] Permettre de spécifier le fichier d'entrée (`func compile main.fc`).
  - [ ] Ajouter une option pour exporter l'IR LLVM textuelle sous forme de fichier `.ll` (très utile pour débugger).
- [ ] **Liaison (Linking) et exécutables natifs**
  - [ ] Compiler l'IR LLVM en code objet machine (fichier `.o` ou `.obj`).
  - [ ] Appeler le linker du système (comme `lld`, `ld`, ou `link.exe`) pour lier le fichier objet avec la bibliothèque standard (libc) et produire l'exécutable final.
- [ ] **Compilation croisée (Cross-Compilation)**
  - [ ] Ajouter des options CLI pour spécifier la cible (ex: `--target x86_64-pc-windows-msvc` ou `--target aarch64-unknown-linux-gnu`).