# Checklist de release FunC v1.1

Ce document définit les vérifications minimales pour stabiliser et publier la version 1.1.

## 1) Qualité compilateur
- [ ] Vérifier `cargo build` en mode `dev` et `release`.
- [ ] Vérifier l’absence de régression de parsing sur les exemples clés (`sample*.fc`).
- [ ] Vérifier la couverture `typecheck` sur cas négatifs déjà couverts + régression de diagnostics.
- [ ] Vérifier la couverture de debug info sur au moins un binaire exécutable.

## 2) Chaîne LLVM / backend
- [ ] Valider `--emit-obj` pour les triplets: `native`, `x86_64`, `aarch64`.
- [ ] Valider `--emit-exe` avec `clang`/`cc` pour `native` et alias connus.
- [ ] Valider le fallback propre quand une cible n’est pas supportée par la toolchain locale.
- [ ] Vérifier les messages d’erreur exécutable/objet avec cas d’échec réel.

## 3) Runtime
- [ ] Passer le test de build+execution multi-cibles sur Linux pour les exemples runtime critiques.
- [ ] Vérifier au moins:
  - `if/else` avec valeur de retour uniforme,
  - import de modules,
  - alias cible (`x86_64`).
- [ ] Ajouter une stratégie de nettoyage des artefacts de test temporaires (pas de fichiers `.fc/.o/.exe` persistants).

## 4) Outil / CLI
- [ ] Vérifier les messages d’aide de base (`funC help`, `funC help compile`, `funC list-targets`).
- [ ] Vérifier `funC compile --check` sur code invalide et valide.
- [ ] Vérifier les codes d’erreurs structurés (`SYN-xxx`, `SEM-xxx`, `BEG-xxx`, `MEM-xxx`, `IO-xxx`).

## 5) Documentation
- [ ] Mettre à jour README avec un point clair sur la feuille de route post-1.0.
- [ ] Vérifier la cohérence entre `RELEASE_NOTES`, `MIGRATION`, `ROADMAP` et `EXAMPLES.md`.
- [ ] Produire un résumé de compatibilité connu / limites.

## 6) Pré-release
- [ ] Tagger la version.
- [ ] Générer le changelog de release 1.1.
- [ ] Valider les artefacts distribuables (binaire + notes).
- [ ] Ouvrir la PR de release avec les vérifications ci-dessus.
