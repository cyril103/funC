# Roadmap FunC — Refonte

## Bilan de départ
- Le compilateur est opérationnel pour un cycle complet: parsing, typecheck, génération LLVM IR, génération objet et exécutable.
- Les fonctionnalités de langage de base et intermédiaires sont implémentées (contrôle de flux, fonctions, mémoire, struct/enum, modules, builtin mémoire, debug info).
- La qualité code compile sans warnings.

## Nouvelle feuille de route

### Phase 0 — Fiabilisation (1 semaine)
- [x] Stabiliser la sémantique de `size_of` (support complet pour les types composés).
- [x] Finaliser la couverture de types pour l’IR (struct/enum/array).
- [x] Ajouter des tests d’intégration « source -> exécutable » pour Linux/Windows.

### Phase 1 — UX compilateur (2 semaines)
- [x] Harmoniser les erreurs/diagnostics avec codes (catégories): syntaxe, sémantique, backend.
- [x] Afficher des spans multi-lignes quand c’est utile (`if`, `for`, `import`).
- [x] Ajouter une commande `funC validate` qui combine parse + typecheck + diagnostics mémoire.
- [x] Documenter le comportement `import` (résolution des chemins, erreurs).

### Phase 2 — Productivité langage (2–3 semaines)
- [x] Élargir le constant folding (comparaisons booléennes et propagation de constantes simples).
- [x] Optimisations locales sûres (élimination de code mort local / branchements inutiles).
- [x] Ajouter des helpers de style (`assert`, `panic` minimal) au niveau bibliothèque standard.
- [x] Définir une mini-std (`func::`) pour les utilitaires mémoire et I/O minimal.

### Phase 3 — Backend et runtime (2–3 semaines)
- [x] Vérifier les pipelines cross-cible sur `llc/clang` (obj/exe).
- [x] Ajouter des passes backend optionnelles (désactivation du `opt` agressif par défaut, profil d’activation).
- [x] Renforcer le support de debug info en mode exécutable.
- [ ] Améliorer la sortie objet/exécutable avec messages d’échec contextuels.

### Phase 4 — Préparation release 1.0 (1–2 semaines)
- [ ] Finaliser une suite de régression complète (parsing/typecheck/codegen/runtime).
- [ ] Rédiger le guide de migration / upgrade.
- [ ] Publier une page `Exemples` par domaine (memoire, contrôle, modules, CLI).
- [ ] Rédiger notes de version v1.0 avec limites connues et compatibilité.

## Indicateur d’avancement
- Priorité: livrer chaque phase par lot, avec vérification `cargo build` + jeux de tests pertinents par phase.
- Statut actuel: Phase 3 en cours — debug info exécutable activée avec `--debug-info` (flag transmis au linker), tests `backend_regression_respects_backend_pass_profiles` et `integration_regression_linux_build_executable_with_debug_info`.
- Objectif cible: stabiliser avant d’ajouter de nouvelles fonctionnalités de syntaxe.
