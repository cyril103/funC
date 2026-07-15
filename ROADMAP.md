# 🗺️ Feuille de Route : Compilateur FunC

Ce document retrace les étapes de développement pour concevoir le compilateur **FunC** en Rust avec LLVM. Cochez les cases au fur et à mesure de votre progression !

---

## 📦 Phase 1 : Lexer (Analyse Lexicale)
L'objectif est de découper le code source brut (chaîne de caractères) en une liste de jetons (Tokens) exploitables.

- [x] **Définir l'énumération `Token` en Rust**
  - [x] Mots-clés : `fn`, `let`, `if`, `else`, `alloc`, `free`, `load`, `store`, `sizeof`
  - [x] Types : `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`, `void`
  - [x] Identifiants (noms de variables/fonctions) et Littéraux (nombres entiers, flottants, booléens)
  - [x] Opérateurs arithmétiques : `+`, `-`, `*`, `/`, `%`
  - [x] Opérateurs logiques et de comparaison : `&&`, `||`, `==`, `!=`, `<`, `<=`, `>`, `>=`
  - [x] Symboles syntaxiques : `(`, `)`, `{`, `}`, `[`, `]`, `:`, `->`, `=`, `,`, `;`, `*`
- [x] **Écrire le Lexer**
  - [x] Option A : À la main (boucle de lecture caractère par caractère, idéal pour apprendre).
  - [ ] Option B : Avec une bibliothèque de combinateurs (ex: `nom` ou `logos` pour des performances maximales).
- [ ] **Écrire des tests unitaires pour le Lexer**
  - [ ] Valider la détection des flottants (`3.14`).
  - [ ] Valider la distinction entre `=` (affectation) et `==` (comparaison).
  - [ ] Valider la gestion des espaces et des commentaires.

---

## 📐 Phase 2 : Parser & AST (Analyse Syntaxique)
L'objectif est de transformer la suite de Tokens en un Arbre de Syntaxe Abstraite (AST) représentant la structure logique du programme.

- [x] **Définir la structure de l'AST en Rust**
  - [x] Structures/Enums pour `Program`, `Function`, `Type`, `Expression` (avec toutes les variantes).
- [x] **Implémenter le Parser**
  - [x] Parser de descente récursive pour les structures globales (`fn`, paramètres, blocs `{}`).
  - [x] Implémenter la priorité des opérateurs (Precedence Climbing ou algorithme Shunting-Yard) pour que `1 + 2 * 3` soit bien analysé comme `1 + (2 * 3)`.
  - [x] Gérer l'associativité à gauche/droite (notamment pour les opérateurs logiques `&&` et `||`).
- [ ] **Écrire des tests pour l'AST**
  - [ ] Vérifier que les parenthèses forcent la priorité d'évaluation : `(1 + 2) * 3`.
  - [ ] Valider la structure imbriquée des expressions `if-else`.

---

## 🔍 Phase 3 : Analyse Sémantique & Typage (Type Checker)
Avant de générer du code, il faut s'assurer que le programme est cohérent. C'est ici que l'on rejette les programmes incorrects.

- [x] **Table des symboles (Symbol Table)**
  - [x] Enregistrer les fonctions déclarées et leurs signatures.
  - [x] Gérer la portée (scope) des variables (une variable dans un bloc `{}` cache-t-elle une variable globale ?).
- [x] **Le Type Checker**
  - [x] Vérifier que les variables utilisées sont bien déclarées.
  - [x] Valider la cohérence des types lors des opérations (ex: interdire `5 + true`).
  - [x] S'assurer que les deux côtés d'une comparaison (`==`, `<`, etc.) possèdent le même type.
  - [x] Vérifier que le type retourné par un bloc `{}` ou un `if-else` correspond à la signature de la fonction.
- [x] **Évaluation statique de `sizeof`**
  - [x] Résoudre `sizeof(T)` et le remplacer directement dans l'AST par une constante entière correspondant à la taille en octets du type ciblé.

---

## ⚡ Phase 4 : Génération de Code LLVM (CodeGen)
Traduire notre AST typé et validé en instructions LLVM IR en utilisant la bibliothèque Rust `inkwell` (ou `llvm-sys`).

- [ ] **Configuration de LLVM**
  - [x] Initialiser le contexte LLVM, le Builder et le Module.
  - [x] Configurer la cible système par défaut (Target Machine).
- [x] **Génération des types et fonctions**
  - [x] Traduire les types de FunC vers les types LLVM correspondants (`i32` de FunC -> `i32` de LLVM, `f64` -> `double`, etc.).
  - [x] Enregistrer la signature des fonctions dans le module LLVM (émission textuelle).
- [x] **Génération des expressions de base**
  - [x] Expressions arithmétiques.
  - [x] Comparaisons :
    - [x] Entiers / Pointeurs : `icmp` avec prédicats signés/non signés selon le type.
    - [x] Flottants : `fcmp`.
  - [x] Divisions et Modulos (`%`) :
    - [x] Signés : `sdiv` / `srem`.
    - [x] Non signés : `udiv` / `urem`.
    - [x] Flottants : `fdiv` / `frem`.
- [x] **Gestion du contrôle de flux**
  - [x] Implémenter les branchements conditionnels pour les expressions `if-else`.
  - [x] Implémenter le **court-circuitage** pour `&&` et `||` (sauts conditionnels).
- [x] **Gestion de la mémoire**
  - [x] Déclarer les fonctions externes de la libc : `malloc(i64) -> *i8` et `free(*i8) -> void`.
  - [x] Traduire `alloc(taille)` en un appel à `malloc` puis convertir le pointeur retourné (`*i8` de LLVM) vers le type attendu.
  - [x] Traduire `free(ptr)` en appel à `free`.
  - [x] Traduire `store(val, ptr)` et `load(ptr)` en instructions de lecture/écriture mémoire LLVM (`load` / `store`).

---

## 🚀 Phase 5 : Driver, Compilation Croisée et Packaging
Faire de votre compilateur un outil en ligne de commande (CLI) utilisable par d'autres.

- [x] **Créer l'interface CLI en Rust** (avec `clap`)
  - [x] Permettre de spécifier le fichier d'entrée (`func compile main.fc`).
  - [x] Ajouter une option pour exporter l'IR LLVM textuelle sous forme de fichier `.ll`.
- [ ] **Liaison (Linking) et exécutables natifs**
  - [x] Compiler l'IR LLVM en code objet machine (fichier `.o` ou `.obj`).
  - [ ] Appeler le linker du système (comme `lld`, `ld`, ou `link.exe`) pour lier le fichier objet avec la bibliothèque standard (libc) et produire l'exécutable final.
- [ ] **Compilation croisée (Cross-Compilation)**
  - [ ] Ajouter des options CLI pour spécifier la cible (ex: `--target x86_64-pc-windows-msvc` ou `--target aarch64-unknown-linux-gnu`).
