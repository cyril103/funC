# FunC 🦀⚡

**FunC** est un langage de programmation expérimental qui marie la rigueur et l'expressivité du **paradigme fonctionnel** avec le contrôle ultime du **développement bas niveau**. 

Écrit en **Rust** et propulsé par l'infrastructure de compilation **LLVM**, FunC propose une approche où tout est expression, tout en laissant l'utilisateur gérer manuellement l'allocation et la libération de la mémoire.

---

## 💡 Philosophie

1. **Pureté Fonctionnelle & Expressionnisme :** Dans FunC, tout est expression. Les blocs, les conditions et les fonctions retournent tous une valeur. L'immuabilité est la règle par défaut.
2. **Contrôle Total de la Mémoire :** Pas de Garbage Collector, pas de système de possession (ownership) complexe. L'allocation sur le tas se fait manuellement via des primitives explicites.
3. **Zéro Abstraction Coûteuse :** Le compilateur traduit directement le code FunC en LLVM IR hautement optimisé, garantissant des performances proches du C.

---

## 📐 Grammaire du Langage (EBNF)

La grammaire actuelle intègre le système de types complet, le modulo, ainsi que l'ensemble des opérateurs d'égalité, de comparaison et logiques.

```ebnf
(* Structure globale *)
Program       ::= Function*
Function      ::= "fn" Identifier "(" ParameterList? ")" "->" Type Block
ParameterList ::= Parameter ("," Parameter)*
Parameter     ::= Identifier ":" Type

(* Système de types *)
Type          ::= "void" | "bool"
                | "i8"  | "i16" | "i32" | "i64"  (* Entiers signés *)
                | "u8"  | "u16" | "u32" | "u64"  (* Entiers non signés *)
                | "f32" | "f64"                  (* Flottants *)
                | "*" Type                       (* Pointeurs *)

(* Blocs et Expressions *)
Block         ::= "{" Expression* "}"
Expression    ::= LetBinding
                | Assignment
                | IfElse
                | BinaryExpr
                | PrimaryExpr

LetBinding    ::= "let" Identifier (":" Type)? "=" Expression ";"
Assignment    ::= "store" "(" Expression "," Expression ")" ";"  (* store(valeur, pointeur) *)
IfElse        ::= "if" Expression Block "else" Block

BinaryExpr    ::= Expression BinaryOp Expression

(* Opérateurs par ordre de priorité (à gérer dans le parser) *)
BinaryOp      ::= "||" 
                | "&&" 
                | "==" | "!=" 
                | "<"  | "<=" | ">" | ">=" 
                | "+"  | "-" 
                | "*"  | "/"  | "%"

PrimaryExpr   ::= IntegerLiteral
                | FloatLiteral
                | BooleanLiteral
                | Identifier
                | FunctionCall
                | Block
                | "alloc" "(" Expression ")"   (* Allocation par taille en octets *)
                | "free" "(" Expression ")"    (* Libération d'un pointeur *)
                | "load" "(" Expression ")"    (* Lecture à une adresse *)
                | "sizeof" "(" Type ")"        (* Taille d'un type en octets *)
                | "(" Expression ")"

FunctionCall  ::= Identifier "(" ArgumentList? ")"
ArgumentList  ::= Expression ("," Expression)*

## 🔨 Guide de compilation

Commandes disponibles avec le CLI:

  - `cargo run -- compile examples/getting-started/sample.fc --emit-ir --out /tmp/sample.ll`
  - `cargo run -- compile examples/getting-started/sample.fc --emit-obj`
  - `cargo run -- compile examples/getting-started/sample.fc --emit-exe --out-exe /tmp/fc_sample`

Flux rapide en 3 lignes:
1. `cargo run -- compile examples/getting-started/sample.fc --check`
2. `cargo run -- compile examples/getting-started/sample.fc --emit-ir --out /tmp/sample.ll`
3. `cargo run -- compile examples/getting-started/sample.fc --emit-obj --emit-exe --out-exe ./sample`

Exemple de flux complet:
1. Générer l'IR: `--emit-ir`
2. Générer un objet: `--emit-obj`
3. Générer un exécutable: `--emit-exe`

Le compilateur conserve la chaîne:
`FunC -> LLVM IR -> objet objet (.o/.obj) -> exécutable natif`.

## 🌐 Compilation cross-compilée

Syntaxe `--target` (LLVM triple):
- `cargo run -- compile examples/getting-started/sample.fc --emit-obj --target x86_64-pc-windows-msvc`
- `cargo run -- compile examples/getting-started/sample.fc --emit-exe --target x86_64-pc-windows-msvc --out-exe sample.exe`
- `cargo run -- compile examples/getting-started/sample.fc --emit-obj --target aarch64-unknown-linux-gnu`

Aliases supportées:
- `native` (triple de l’hôte)
- `x86_64`, `aarch64` (`arm64`), `x86` (`i386`) pour des cibles courantes

Lister les cibles et alias:

- `cargo run -- list-targets`

Comportement:
- Objet: `obj` si la cible contient `windows`, sinon `o`.
- Exécutable: `exe` si la cible contient `windows`, sinon sans suffixe.
- `emit_obj`/`emit_exe` réutilise `llc -mtriple` et `clang -target` (ou `cc` en fallback).
- Si la chaîne LLVM/Clang ne supporte pas la cible, vous obtenez une erreur explicite.

## 📦 Gestion des imports

Les modules sont chargés via le mot-clé `import` avec une chaîne:

- Syntaxe: `import "chemin";`
- Exemple: `import "math";`

Résolution:
- si le chemin est relatif, il est résolu depuis le dossier du fichier appelant;
- l'extension `.fc` est ajoutée automatiquement si elle n'est pas présente;
- si le chemin est absolu, il est utilisé tel quel;
- les imports déjà chargés ne sont pas recompilés (anti-cycle simple via cache interne).

Comportement d'erreur:
- `Impossible de localiser le module` si le chemin ne peut pas être résolu;
- `Impossible de lire le module` en cas d'absence de fichier ou de problème d'accès;
- Les erreurs de lexeur/parser des modules importés remontent avec des diagnostics syntaxiques.

## ✅ Exemples de validation

- `examples/getting-started/sample_if_else.fc` : valide `if/else` avec retour d'un même type dans chaque branche.
- `examples/getting-started/sample_logic_shortcircuit.fc` : valide le court-circuit de `&&` et `||` (la partie droite peut contenir une expression non-safe qui ne doit pas être exécutée).

## 🧪 Référentiel d’exemples

- `examples/if_else/basic.fc` : comparateur avec `if/else`.
- `examples/loops/loops.fc` : exemples de `while` et `for`.
- `examples/pointers/pointers.fc` : `alloc`, `store`, `load`, `free`.
- `examples/cross-target/cross-target.fc` : exemple compact pour valider des builds multi-cibles.

## 🧭 Documentation de migration

- `MIGRATION.md` : guide de migration, vérifications recommandées et points de rupture connus.

## 🧪 Périmètre d’exemples

- `EXAMPLES.md` : inventaire d’exemples classés par domaine (contrôle, mémoire, modules, CLI).

## 📣 Notes de version

- `RELEASE_NOTES_v1.0.md` : notes de release, limites connues et points de compatibilité.
- `RELEASE_CHECKLIST_v1.1.md` : checklist de validation pour la préparation de la release 1.1.
