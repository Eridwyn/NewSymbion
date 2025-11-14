# Normes de Qualité du Code

Standards de développement pour le projet Symbion.

---

## 📏 Règle de Modularité

**Principe strict**: **1 fichier = 1 fonction/responsabilité** quand c'est possible

### Objectifs

✅ Éviter les fichiers monolithiques
✅ Faciliter la maintenance
✅ Améliorer la testabilité
✅ Clarifier les responsabilités

---

## 📐 Tailles Maximales Recommandées

### Fichiers Rust

| Type | Limite Recommandée |
|------|-------------------|
| **Fonctions** | < 50 lignes (idéal < 30) |
| **Fichiers modules** | < 300 lignes |
| **Fichiers principaux** | < 500 lignes (main.rs, lib.rs) |

### Fichiers JavaScript/Lit

| Type | Limite Recommandée |
|------|-------------------|
| **Composants Lit** | < 400 lignes |
| **Services** | < 200 lignes |
| **Widgets** | < 300 lignes |

### Dépassement Acceptable

Uniquement si:
- Logique métier complexe **indivisible**
- Refactoring dégraderait la lisibilité
- Documenté avec commentaires clairs

---

## 🚨 Signaux d'Alerte Monolithique

Refactoring nécessaire si:

- ❌ Fichier > 500 lignes
- ❌ Plus de 10 fonctions dans un module
- ❌ Imports circulaires
- ❌ Fichier modifié fréquemment pour raisons différentes
- ❌ Difficulté à nommer clairement le fichier

---

## 🔀 Stratégie de Découpage

### 1. Par Responsabilité

Chaque fonction dans son fichier.

```
❌ utils.rs (fourre-tout)
✅ format_date.rs
✅ validate_email.rs
✅ hash_password.rs
```

### 2. Par Domaine Métier

Grouper fonctions liées.

```
✅ auth/login.rs
✅ auth/session.rs
✅ auth/tokens.rs
```

### 3. Par Type

Séparer structures/traits/implémentations.

```
✅ models/user.rs
✅ services/user_service.rs
✅ handlers/user_handler.rs
```

---

## ❌ Anti-Patterns à Éviter

```
❌ mega_file.rs (2000 lignes)
❌ helpers.rs (fourre-tout)
❌ common.rs (tout partagé)
```

---

## ✅ Pattern Recommandé

```
✅ Fichiers courts et focalisés
✅ Noms explicites
✅ Responsabilité unique claire
```

---

## ✓ Checklist Avant Commit

Avant de committer du code, vérifier:

- [ ] Aucun fichier > 500 lignes (sans justification)
- [ ] Aucune fonction > 50 lignes (sans justification)
- [ ] Noms de fichiers explicites
- [ ] Responsabilité unique par fichier
- [ ] Pas de code mort (commenté ou inutilisé)
- [ ] Imports organisés et utilisés

---

## 🎯 Manifeste de Code

> **Code court, clair, modulaire.**
> **Une fonction = un fichier quand c'est possible.**
> **Zéro monolithe.**
