# Roadmap P0 - Consolidation Symbion Kernel

## 🎯 Objectif Phase P0

**Consolidation infrastructure décisionnelle** pour passer d'un système réactif à un système **proactif et fiable** capable de prendre des décisions contextuelles intelligentes avec guardrails de sécurité.

**Durée estimée**: 2-3 semaines
**Priorité**: P0 (critique - fondation pour toutes features futures)

---

## ⚠️ NOTE CRITIQUE - INTÉGRATION FRONTEND

**IMPORTANT**: Chaque PR doit inclure l'intégration PWA correspondante pour éviter que l'interface devienne hors service.

**Raison**: L'ajout de protections backend (CSRF, auth, etc.) peut rendre des routes inaccessibles depuis le PWA si celui-ci n'est pas mis à jour en parallèle.

**Procédure**: À la fin de chaque PR, avant le merge:
1. Implémenter les adaptations PWA nécessaires (services, widgets)
2. Tester les fonctionnalités depuis l'interface web
3. S'assurer qu'aucune régression n'affecte l'utilisation quotidienne
4. Commit séparé ou intégré selon complexité

---

## 📋 Liste des PRs P0

### ✅ PR1: Context Engine - Timezone + Hysteresis (COMPLETED)

**Status**: ✅ Merged
**Commit**: `6b1ca90`
**Date**: 25 Octobre 2025

**Objectifs**:
- ✅ Détection timezone automatique (Europe/Paris)
- ✅ Règles temporelles (Nuit 23h-7h, Week-end)
- ✅ Override manuel avec durée configurable
- ✅ API `/context/override` et `/context/clear`
- ✅ Widget PWA avec contrôles manuels 3 modes

**Delivered**:
- Context detection automatique avec fallback intelligent
- Hysteresis pour éviter oscillations mode
- Interface PWA complète avec sélecteur durée (1h/2h/4h/8h)
- Auto-injection contexte dans notes

---

### 🔄 PR2: API v1 + Auth JWT + MFA + CSRF + Rate Limiting (IN PROGRESS - 90%)

**Branch**: `feature/pr2-api-v1-auth-mfa`
**Status**: 🔄 Backend 90% done, Frontend PWA integration en cours

**Objectifs Backend**:
- ✅ Endpoints MFA complets (`/v1/auth/mfa/*`)
- ✅ Protection CSRF avec nonces single-use (TTL 5min)
- ⏳ Rate Limiting Tower (en cours)

**Objectifs Frontend** (AJOUTÉ SUITE FEEDBACK):
- ⏳ Service CSRF automatique pour PWA (`csrf-service.js`)
- ⏳ Intégration CSRF dans widgets (context, agents, plugins)
- ⏳ Tests fonctionnels depuis interface web

**Détails Implémentation**:

#### Auth & MFA
- `POST /v1/auth/login` - JWT tokens (exp 8h)
- `GET /v1/auth/mfa/status` - État MFA utilisateur
- `POST /v1/auth/mfa/setup` - Setup TOTP + QR code
- `POST /v1/auth/mfa/verify` - Vérification code TOTP
- `POST /v1/auth/mfa/disable` - Désactivation MFA

#### CSRF Protection
- `GET /auth/csrf/nonce` - Génération nonce (JWT requis)
- Middleware `require_csrf` sur 11 routes destructrices:
  - Agent control: shutdown, reboot, hibernate, kill process
  - Context control: override, clear
  - Plugin control: start, stop, restart
  - Data mutations: delete/update memo, delete port data

#### Rate Limiting (TODO)
```rust
// Configuration prévue
- Auth routes: 10 req/min par IP
- API routes: 5 req/sec par IP
- Headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
```

**Tests Validés**:
- ✅ Login + JWT tokens
- ✅ MFA setup/verify/disable flow complet
- ✅ CSRF nonce generation
- ✅ CSRF middleware (4/4 scénarios)

**Commit Actuel**: `bbb8952` - feat(PR2): protection CSRF complète avec middleware et tests validés

**Prochaines Étapes**:
1. Implémenter PWA CSRF service (priorité urgente)
2. Rate Limiting Tower backend
3. Tests finaux + rapport email
4. Merge dans master

---

### 📋 PR3: Decision Engine - Guards-First + Weights

**Status**: ⏳ Pending
**Dépendances**: PR2 (auth/rate limiting)

**Objectifs**:
- Système de guards décisionnels (conditions avant action)
- Poids de confiance pour décisions contextuelles
- Validation multi-critères avant actions critiques
- Audit trail des décisions refusées

**Architecture Prévue**:
```rust
struct DecisionGuard {
    name: String,
    condition: GuardCondition,
    priority: u8,
    failure_mode: FailureMode,
}

enum GuardCondition {
    ContextMode(Vec<ContextMode>),  // Whitelist modes autorisés
    TimeRange(TimeRange),            // Plage horaire autorisée
    UserPresence(bool),              // Présence utilisateur requise
    Custom(Box<dyn Fn(&State) -> bool>),
}
```

**Exemples Guards**:
- "Shutdown agent seulement si mode Neutre ou Intime"
- "Context override limité à 4h max si jour de travail"
- "Plugin restart seulement si aucune activité critique"

**Intégration PWA**:
- Indicateurs visuels guards actifs
- Tooltips expliquant refus décisions
- Logs décisions dans widget système

---

### 📋 PR4: Observability Minimale

**Status**: ⏳ Pending
**Dépendances**: PR3 (guards/decisions)

**Objectifs**:
- Métriques temps réel (Prometheus-compatible)
- Logs structurés avec niveaux (error/warn/info/debug)
- Health checks enrichis (latence, mémorisation, décisions/sec)
- Tracing décisions critiques

**Métriques Prévues**:
```
symbion_decisions_total{mode, result}
symbion_context_switches_total{from, to}
symbion_agent_actions_total{action, agent_id}
symbion_api_requests_total{endpoint, status}
symbion_mqtt_messages_total{topic}
```

**Intégration PWA**:
- Widget observability avec graphiques temps réel
- Alertes visuelles si métriques anormales
- Export métriques JSON pour analyse externe

---

### 📋 PR5: Fail-Safe Mechanisms

**Status**: ⏳ Pending
**Dépendances**: PR4 (observability)

**Objectifs**:
- Modes dégradés si composants critiques tombent
- Circuit breakers pour services externes (MQTT, plugins)
- Auto-recovery avec backoff exponentiel
- Fallback comportements si context engine fail

**Fail-Safe Behaviors**:
```rust
// Si MQTT down → mode local uniquement
// Si plugin crash → désactivation automatique + alerte
// Si context engine fail → fallback mode "Neutre" safe
// Si storage fail → cache en RAM temporaire
```

**Tests Résilience**:
- Kill MQTT broker → kernel continue en mode dégradé
- Crash plugin → isolation sans impact kernel
- Corruption fichier users.json → fallback auth désactivé + alerte

**Intégration PWA**:
- Indicateurs mode dégradé visibles
- Notifications recovery automatique
- Boutons recovery manuel si auto-recovery échoue

---

### 📋 PR6: Intentions & Lifecycle Management

**Status**: ⏳ Pending
**Dépendances**: PR5 (fail-safe)

**Objectifs**:
- Système d'intentions utilisateur (goals à moyen terme)
- Lifecycle hooks pour plugins (init/start/stop/cleanup)
- Gestion graceful shutdown (sauvegarde état + cleanup)
- Persistence intentions cross-restarts

**Intentions Examples**:
```json
{
  "intention": "focus_work_session",
  "duration_minutes": 90,
  "behaviors": {
    "context_mode": "cravate",
    "notifications_filter": "work_only",
    "break_reminders": true
  }
}
```

**Lifecycle Hooks**:
```rust
trait PluginLifecycle {
    async fn on_init(&mut self) -> Result<()>;
    async fn on_start(&mut self) -> Result<()>;
    async fn on_stop(&mut self) -> Result<()>;
    async fn on_cleanup(&mut self) -> Result<()>;
}
```

**Intégration PWA**:
- Widget intentions avec presets (Focus, Détente, Cuisine, Sport)
- Timeline intentions actives/passées
- Statistiques accomplissement intentions

---

## 🎯 Définition de "Done" pour Chaque PR

Chaque PR est considérée complète seulement si:

1. ✅ **Backend implémenté** - Tous endpoints/features fonctionnels
2. ✅ **Tests validés** - Scénarios principaux testés manuellement ou via tests automatisés
3. ✅ **PWA intégré** - Interface web compatible, aucune régression fonctionnelle
4. ✅ **Build stable** - `cargo build --release` sans erreurs critiques
5. ✅ **Documentation** - README ou CLAUDE.md mis à jour si nécessaire
6. ✅ **Rapport email** - Email récapitulatif envoyé (habit de documentation)
7. ✅ **Git clean** - Commit + push sur branche feature, prêt pour merge

---

## 📊 Progression Globale P0

```
Phase P0: Consolidation Infrastructure Décisionnelle
┌─────────────────────────────────────────────────────────────┐
│ PR1: Context + Timezone            ✅ 100% (Merged)         │
│ PR2: API v1 + Auth + MFA + CSRF    🔄  90% (In Progress)    │
│ PR3: Decision Guards + Weights     ⏳   0% (Pending)        │
│ PR4: Observability Minimale        ⏳   0% (Pending)        │
│ PR5: Fail-Safe Mechanisms          ⏳   0% (Pending)        │
│ PR6: Intentions & Lifecycle        ⏳   0% (Pending)        │
└─────────────────────────────────────────────────────────────┘

Total P0: 15% (1/6 PRs merged, 1 en cours)
ETA: 2-3 semaines (si 2-3 jours par PR)
```

---

## 🚀 Après P0 - Phases Suivantes

**Phase P1 - Intelligence Contextuelle**:
- Machine learning patterns comportementaux
- Prédictions contextuelles avancées
- Suggestions proactives

**Phase P2 - Écosystème Étendu**:
- Modules métier (Cuisine, Finance, Santé, Famille)
- Intégrations services externes (calendrier, météo, banques)
- Multi-utilisateurs avec profils personnalisés

**Phase P3 - Production Hardening**:
- Let's Encrypt certificats automatiques
- Déploiement Kubernetes/Docker
- Monitoring production (Grafana/Prometheus)
- Backup automatisé données critiques

---

## 📝 Notes d'Architecture

### Principes de Design P0

1. **Security First**: Chaque feature doit inclure auth/authz dès le départ
2. **Fail-Safe by Default**: Comportement dégradé toujours défini
3. **Observability Built-In**: Logs/métriques intégrés, pas après-coup
4. **User Control Priority**: Utilisateur garde toujours contrôle manuel
5. **PWA Integration Mandatory**: Chaque PR inclut frontend (évite interface hors service)

### Dépendances Techniques

**Backend**:
- Rust stable (1.70+)
- Axum framework (API REST)
- Tower middleware (auth, rate limiting, tracing)
- rumqttc (MQTT client)
- serde_json (persistence)

**Frontend**:
- Vanilla JS (Lit framework)
- WebSocket temps réel
- Service Workers (PWA)
- marked.js (markdown rendering)

**Infrastructure**:
- MQTT broker (Mosquitto)
- Filesystem storage (JSON files)
- HTTPS TLS (auto-signé dev, Let's Encrypt prod)

---

## 🔗 Liens Utiles

- **Repo GitHub**: (privé ou configurer selon projet)
- **Documentation Principale**: `/home/eridwyn/RustroverProjects/NewSymbion/CLAUDE.md`
- **Incidents Résolus**: `/home/eridwyn/RustroverProjects/NewSymbion/incidents/resolu/`
- **Scripts Monitoring**: `/home/eridwyn/RustroverProjects/NewSymbion/scripts/` (exclus repo - sécurité)

---

**Dernière mise à jour**: 29 Octobre 2025
**Mainteneur**: Mark (avec assistance Claude Code)
