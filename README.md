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

- `cargo run -- compile sample.fc --emit-ir --out /tmp/sample.ll`
- `cargo run -- compile sample.fc --emit-obj`
- `cargo run -- compile sample.fc --emit-exe --out-exe /tmp/fc_sample`

Exemple de flux complet:
1. Générer l'IR: `--emit-ir`
2. Générer un objet: `--emit-obj`
3. Générer un exécutable: `--emit-exe`

Le compilateur conserve la chaîne:
`FunC -> LLVM IR -> objet objet (.o/.obj) -> exécutable natif`.
