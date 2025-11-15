# Philosophie Architecturale Symbion

> Symbion n'est pas un simple projet logiciel. C'est une architecture systémique vivante, organisée autour d'un Kernel central conscient du contexte et de modules agents strictement subordonnés.

---

## 🧬 Hiérarchie Système

```
Kernel (Cerveau)
   ↓ décisions, contexte, vérité
Agents (Muscles)
   ↓ exécution passive, stateless
PWA (Sens)
   ↓ respiration sensorielle
Utilisateur
```

---

## 🧠 Le Kernel : Cerveau Central

Le Kernel est le **seul point de vérité et de décision**.

### Responsabilités

✅ **Ne délègue JAMAIS de logique métier** aux agents
✅ **Gère les autorisations**, priorités et contexte global
✅ **Maintient l'intégrité symbolique** (cohérence, mémoire, sens)
✅ **Pense et décide** pour tout l'écosystème

### Exemples

- **Context Engine**: Détecte SSID, horaires, patterns → décide du mode
- **Auth Manager**: Gère sessions JWT et autorisations
- **Agent Registry**: Maintient l'état global des agents
- **Plugin Orchestration**: Active/désactive modules selon contexte

---

## 🤖 Les Agents : Exécutants Passifs

Les agents sont des **exécutants passifs, stateless autant que possible**.

### Rôle

Rapporter, exécuter, mourir proprement.

✅ **Collectent et rapportent**: Métriques système (CPU, RAM, uptime)
✅ **Exécutent sans question**: Commandes du Kernel (shutdown, hibernate)
✅ **Pas de logique métier**: Juste observation et action
✅ **Heartbeats réguliers**: Signaux de vie MQTT toutes les 30s

### Anti-patterns à éviter

❌ Agents qui décident du mode contextuel
❌ Agents qui interprètent les données
❌ Agents qui stockent de l'état complexe
❌ Agents qui communiquent entre eux directement

---

## 📱 La PWA : Interface Sensorielle

La PWA représente **l'interface "sensorielle" du Kernel** — une extension humaine, pas un tableau de bord.

### Rôle

Traduire la pensée du système sous forme accessible.

✅ **Traduit les modes**: 👔 Focus Pro, 🏡 Maison, 🌱 Veille
✅ **États visuels**: Thèmes, couleurs, gradients selon contexte
✅ **Signaux d'ambiance**: Widgets adaptatifs (matin: agenda, soir: détente)
✅ **Respire le contexte**: Interface qui s'adapte, pas qui contrôle

**La PWA ne décide jamais** — elle reflète l'état du Kernel.

---

## 📐 Règles de Développement

### Pour toute évolution de code

1. ✅ **Respecter la hiérarchie**: Kernel > Agents > Clients
2. ✅ **Préserver la cohérence symbolique** + fonctionnelle
3. ✅ **Code court, clair, modulaire**: Pas de magie implicite
4. ✅ **Un composant = une responsabilité** claire

---

## 🌊 Manifeste

> **Le Kernel pense, les agents obéissent, et la PWA respire.**

---

## 🎯 Principes Symbion

**🌱 Grandit avec toi**
Plus tu l'utilises, mieux il te connaît et anticipe tes besoins.

**👻 S'efface naturellement**
Les meilleures technologies sont invisibles — Symbion agit en arrière-plan.

**🗝️ Libère ton énergie**
En automatisant le répétitif, il te redonne du temps pour tes passions et tes proches.

**🏠 Respecte ton intimité**
Toutes les données restent dans ton écosystème domestique, pas de cloud externe.

**🔧 Extensible à volonté**
Architecture modulaire — ajoute seulement les modules de vie qui t'intéressent.

---

## ⚖️ Gestion des Décisions et Automatisations

Symbion fonctionne selon une **hiérarchie stricte de décision**.

### Hiérarchie

✅ **Le Kernel** est l'unique entité autorisée à décider et déclencher des actions
✅ **Les agents** exécutent et rapportent, sans initiative autonome
✅ **L'application mobile** (future) sert d'interface d'approbation humaine

### Niveaux d'Impact

**🟢 Niveau L (Low)**: Actions locales, réversibles, sans risque
- Le Kernel agit seul, sans validation
- Ex: ajuster luminosité écran, activer mode concentration

**🟡 Niveau M (Medium)**: Impact modéré
- Le Kernel agit seul **si `trust_score ≥ 0.7`**
- Sinon, demande validation via app mobile
- Ex: éteindre machine, modifier température

**🔴 Niveau H (High)**: Critiques (sécurité, vie privée, argent)
- **Nécessitent validation explicite**
- Jamais d'action autonome
- Ex: achat automatique, partage données, accès réseau externe

### Trust Score Dynamique

Calculé **par décision** selon:
- ✅ Cohérence des capteurs (données concordantes)
- ✅ Fraîcheur télémétrie (agents online, heartbeats récents)
- ✅ Historique de succès (pattern répété validé)
- ✅ Latence réseau (communication stable)
- ✅ Présence locale détectée (vs absent)

**Formule** (résultat toujours dans [0.0, 1.0]):
```
trust_score = (
  sensor_consistency * 0.3 +
  telemetry_freshness * 0.25 +
  historical_success * 0.25 +
  network_latency * 0.1 +
  local_presence * 0.1
)
```

### Consentements Durables

Les décisions validées peuvent donner lieu à un **consentement durable**:

✅ **Scope limité**: Type d'action, horaire, durée maximale
✅ **Révocable à tout moment** via app mobile
✅ **Expire automatiquement** si:
  - Inactivité prolongée (> 30 jours)
  - Baisse confiance (trust_score < 0.5)
  - Changement contexte (nouveau SSID, déménagement)

### Fail-Safe

En cas d'absence de réponse utilisateur:
- **Niveau L** → Ignore (décision triviale)
- **Niveau M** → Option "safe minimal" (ex: mode économie)
- **Niveau H** → **Fail-safe**: ne rien faire

**Timeout**:
- Niveau M: 5 minutes
- Niveau H: 30 minutes

---

## 📊 Résumé du Modèle

```
┌─────────────────────────────────────┐
│   Le Kernel agit seul              │
│   quand c'est sûr (L, M high-trust)│
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Il consulte l'humain             │
│   quand c'est risqué (M low-trust, H)│
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Il apprend uniquement             │
│   des validations explicites        │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│   Aucune automatisation n'échappe   │
│   au contrôle de sens               │
└─────────────────────────────────────┘
```
