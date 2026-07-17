# Notes de version — FunC v1.0 (proposition)

## ✅ Nouveautés

- Pipeline complet de compilation: parse, typecheck, génération LLVM, objet, exécutable.
- Imports de modules: résolution de chemins, extension `.fc`, déduplication de chargement.
- Messages d’erreur structurés par catégories (`SYN-001`, `SEM-001`, `BEG-001`, `MEM-001`, `IO-001`).
- Backend LLVM configurable:
  - `--backend-profile none|safe|aggressive`
- Debug info exécutable et améliorations de diagnostics backend/obj/exe.
- Suite de régression enrichie (parsing/typecheck/codegen/runtime).
- Documentation de migration et inventaire d’exemples.

## ⚠️ Limites connues

- Les warnings de variables potentiellement inutilisées peuvent entraîner un échec en mode `--check`.
- Certaines combinaisons syntaxiques sont stables mais la couverture des tests reste à compléter sur:
  - cas limites de contrôle de flux complexes
  - interactions mémoire avancées
- Certaines versions de LLVM peuvent ne pas exposer les mêmes options `llc/clang`.

## 🔄 Compatibilité et migration

- Compatible avec les scripts `--check`/`--emit-ir`/`--emit-obj`/`--emit-exe` existants.
- Voir `MIGRATION.md` pour la feuille de route de migration détaillée.
- Les options de backend changent seulement le comportement d’optimisation, pas le format des entrées.

## 📌 Checklist de validation recommandée

1. `funC compile --check <source>`
2. `funC compile --emit-ir <source> --out <fichier>.ll`
3. `funC compile --emit-obj <source> --out-obj <fichier>.o`
4. `funC compile --emit-exe <source> --out-exe <fichier>`

## 🧭 Prochaines étapes

- Finaliser la stabilisation de la sémantique sur de nouveaux cas d’utilisation de contrôle/objets.
- Préparer les assets de distribution et la documentation utilisateur finale.
