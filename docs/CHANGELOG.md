# Changelog - Symbion

Historique des améliorations et changements majeurs du projet.

---

## 🔐 Phase 2: Security Hardening (14 Novembre 2025)

**Status**: 🟢 100% complété (5/5 tâches)

### Vulnérabilités Corrigées

#### VULN-005: Bcrypt Cost Factor
- ✅ Augmentation: 10 → 12 (~400ms hashing)
- Fichiers: `symbion-kernel/src/auth.rs:107`, `symbion-kernel/src/auth.rs:367`

#### VULN-009: Rate Limiting Auth
- ✅ Retrait `tower_governor` (HTTP 500 localhost/VPN)
- ✅ Maintien rate limiting basé username (5/15min)
- Fichiers: `symbion-kernel/Cargo.toml`, `symbion-kernel/src/http.rs`

#### VULN-004: Secrets Hardcodés
- ✅ Rotation complète JWT_SECRET + API_KEY (OpenSSL)
- ✅ `.env` mis à jour, `.env.example` créé
- Prochaine rotation: 12 février 2026 (90 jours)

#### VULN-001: Permissions Certificats TLS
- ✅ `/etc/mosquitto/certs/key-mkcert.pem` → 600 (propriétaire seul)
- Avant: 640 (lisible groupe)

#### PR1: MQTT Retain Context
- ✅ Déjà implémenté `retain=true` (`symbion-kernel/src/dashboard_events.rs:46`)

**Documentation**: `docs/security/`

---

## 🔧 Fix Critique MQTT (15 Octobre 2025)

### Problème
- MQTT status: "connecting" permanent
- Agents non synchronisés

### Root Cause
`mark_mqtt_connected()` jamais appelé après subscriptions MQTT

### Solution
`symbion-kernel/src/mqtt.rs:81-85` - Ajout appel après subscriptions

### Impact
- ✅ Agents synchronisés
- ✅ Métriques CPU/RAM temps réel
- ✅ Status online/offline précis
- ✅ Plugin notes fonctionnel

**Documentation**: `incidents/resolu/INCIDENT-2025-10-15-mqtt-not-connected.md`

---

## 📧 Monitoring Automatique (15 Octobre 2025)

### Surveillance Proactive
- **Fréquence**: Toutes les 15 minutes (cron)
- **Alertes**: Gmail (Markchavatte@gmail.com) via msmtp

### Scripts Créés
- `scripts/monitor-symbion.sh` - Surveillance complète
- `scripts/install-monitoring.sh` - Installation cron
- `scripts/README.md` - Documentation

### Checks
- ✅ Kernel alive (http://localhost:8080)
- ✅ MQTT connected/disconnected
- ✅ Agents online/offline + métriques
- ✅ Plugins running/failed

### Alertes Configurées
- 🚨 Kernel DOWN
- 🚨 MQTT Disconnected
- 🚨 All Agents Offline
- 🚨 Plugin Failed
- ✅ System Recovered

---

## 📝 Widget Notes UX (15 Octobre 2025)

### Améliorations
- ✅ Markdown rendering (`marked.js`)
- ✅ Preview 3 lignes + gradient fondu
- ✅ Expand/collapse ("📖 Lire plus" / "⬆️ Réduire")
- ✅ Styling avancé (titres, code, listes)

**Fichier**: `pwa-dashboard/src/widgets/notes-widget.js`

---

## 🎨 Context Engine (25 Octobre 2025)

### Détection Automatique
- ✅ Règles temporelles: Nuit (23h-7h) → Mode Neutre
- ✅ Week-end automatique → Mode Intime
- ✅ Expiration override

### API Contrôle Manuel
```
POST /context/override
POST /context/clear
GET /context/current
```

**Fichiers**:
- `symbion-kernel/src/context.rs:107-148,192-235`
- `pwa-dashboard/src/widgets/context-widget.js` (498 lignes)

---

## 🏷️ Notes Contextuelles (25 Octobre 2025)

### Auto-injection Backend
- ✅ Tagging automatique notes avec contexte actuel
- ✅ Format lowercase: "cravate", "intime", "neutre"
- ✅ Fallback intelligent

**Fichiers**:
- `symbion-kernel/src/http.rs:588-625`
- `pwa-dashboard/src/widgets/notes-widget.js:837-950`

---

## 📱 Interface Mobile Responsive (25 Octobre 2025)

### Tabs Catégorisées
- 🎛️ Contrôle: Context + Agent controls
- ⚙️ Système: Health + Agents list
- 📝 Données: Notes widget

### Theming Dynamique
Variables CSS selon mode actif:
- `--context-primary`
- `--context-bg`
- `--context-accent`

**Fichiers**:
- `pwa-dashboard/src/components/dashboard-app.js:107-178,268-297,567-624`
- `pwa-dashboard/src/services/context-service.js`

---

## 🔐 Setup Certificat TLS (26 Octobre 2025)

### Workflow Automatisé
- ✅ Endpoint public: `GET /ca-certificate`
- ✅ Téléchargement direct `symbion-ca.crt`
- ✅ Instructions plateformes (Windows/Linux/macOS/iOS/Android)
- ✅ Vérification installation automatique

### Navigation Mobile Fixe
- ✅ Tabs ancrés en bas viewport (`position: fixed`)
- ✅ Backdrop blur effet iOS
- ✅ Padding container (70px)

**Fichiers**:
- `symbion-kernel/src/http.rs:136,1105-1131`
- `pwa-dashboard/src/components/boot-terminal.js:716-779,904-914`
- `pwa-dashboard/src/components/dashboard-app.js:331-349`

---

## 📚 Légende

- 🟢 Complété
- 🟡 En cours
- 🔴 Bloqué
- ⚪ Non démarré
