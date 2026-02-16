# Roadmap Technique - NewSymbion

**Version** : 2025-11 (Post PR1-PR6)
**Statut** : Fondations complètes, transition vers intégration physique
**Dernière mise à jour** : 16 Novembre 2025

---

## Contexte

### État Actuel (PR1-PR6)

**Production-Ready Core** : 67% (41/61 tâches)

| Phase | Status | Completion |
|-------|--------|------------|
| PR1 - Context Engine | ✅ Done | 100% P1 |
| PR2 - Security Hardening | ✅ Done | 100% |
| PR3 - Decision Engine | ✅ Done | 100% P1 |
| PR4 - Metrics & Observability | ✅ Done | 100% P1 |
| PR5 - Kernel Reliability | ✅ Done | 100% P1 |
| PR6 - Production Readiness | 🟡 In Progress | 18% (CSP only) |

**Fondations Techniques Disponibles** :
- ✅ **Kernel** : Event bus MQTT, API REST (93 endpoints), Plugin orchestration
- ✅ **Context Engine** : IANA timezone, 3 modes (Cravate/Intime/Neutre), hysteresis
- ✅ **Decision Engine** : Trust scoring multi-facteurs, validation workflow, audit trail
- ✅ **Security** : TLS 1.3, JWT+MFA+WebAuthn, CSRF, rate limiting, bcrypt cost 12
- ✅ **Observability** : 22 métriques Prometheus, health checks, structured logging
- ✅ **Reliability** : Systemd service, graceful shutdown, panic recovery
- ✅ **Agent-Host** : Multi-platform (Linux/Windows), auto-discovery, remote commands
- ✅ **PWA Dashboard** : Lit components, MQTT streaming, responsive

**Stack Technique** :
- Backend : Rust (Axum, Tokio, rumqttc, serde)
- Frontend : Lit (Web Components), Vite
- Communication : MQTT (Mosquitto), REST API, WebSocket
- Database : JSON files (migration PostgreSQL prévue PR6)
- Security : TLS 1.3, JWT HS256, bcrypt
- Deployment : Systemd, mkcert (dev), Let's Encrypt (production planned)

---

## Objectifs 2025-2026

### Vision Technique

Transition de **système logiciel pur** vers **intégration physique IoT** via 5 "organes" spécialisés.

**Contraintes Absolues** :
- **LAN-only** : Pas de cloud, 100% réseau local
- **Privacy-first** : Aucune donnée personnelle dans le code public
- **Generic design** : Abstraction pour réutilisabilité
- **Production-grade** : Tests unitaires, error handling, observability

### Déploiement Séquentiel

Implémentation **un organe à la fois** jusqu'à stabilité complète avant passage au suivant.

**Ordre d'implémentation** :
1. **F1** - Environment (chambre) : Fondations capteurs + alertes
2. **F2** - Digital Hygiene : Comportement utilisateur
3. **F3** - Intentions Log : Mémoire système
4. **F4** - Notifications : Multi-canal push
5. **F5** - Light Actuator : Premier actuateur physique

---

## F1 - Organe Environnement (Chambre)

**Effort** : 9 jours
**Statut** : 🔵 Priorité immédiate - Implémentation en cours
**Objectif** : Monitoring température/humidité avec alertes intelligentes

### Use Case

Surveillance environnement chambre pour :
- Prévention moisissure (humidité >70%)
- Alerte ventilation (humidité >65% pendant 30 min)
- Confort thermique (température <16°C la nuit)

### Architecture

**Pipeline** :
```
ESP32 + BME280 (chambre)
    ↓ MQTT QoS 1
symbion/sensors/chambre/env@v1
    ↓ Ingestion
Kernel RoomEnvironmentState (circular buffer 100 items)
    ↓ Decision Engine
Environment Rules (sustained thresholds)
    ↓ Intentions
symbion/dashboard/intentions@v1
    ↓ Display
PWA Widget Environment/Chambre
```

### Contrat MQTT

**Topic** : `symbion/sensors/chambre/env@v1`
**Publisher** : ESP32 firmware
**Subscribers** : Kernel environment module
**QoS** : 1 (at least once delivery)

**Payload JSON** :
```json
{
  "room_id": "chambre",
  "temperature_c": 21.5,
  "humidity_pct": 58.3,
  "timestamp": "2025-11-16T14:32:00Z"
}
```

**Fréquence** : 1 message / 5 min (optimisation batterie ESP32)

### Structures Kernel

**Fichier** : `symbion-kernel/src/environment.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvReading {
    pub temperature_c: f32,
    pub humidity_pct: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentStatus {
    Ok,
    Humid,       // 60-70%
    RiskMold,    // >70%
    Cold,        // <16°C la nuit
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomEnvironmentState {
    pub room_id: String,
    pub current: EnvReading,
    pub history: VecDeque<EnvReading>, // Circular buffer, max 100
    pub status: EnvironmentStatus,
    #[serde(skip)]
    max_history: usize,
}
```

**Features** :
- Circular buffer auto-eviction (VecDeque, max 100 items = 8h historique)
- Status enum avec seuils configurables
- Méthode `get_history(hours)` pour requêtes temporelles
- Sérialisation JSON pour persistence

### Decision Engine Rules

**Fichier** : `symbion-kernel/src/decision/environment.rs`

**Règles Implémentées** :

1. **ALERT_HUMIDITY_CHAMBRE** (Medium impact)
   - Condition : Humidité >65% sustained pendant 30 min
   - Validation : 6+ readings consécutifs >65%
   - Action suggérée : "Aérer la chambre"

2. **ALERT_HUMIDITY_CRITICAL** (High impact)
   - Condition : Humidité >75% sustained pendant 10 min
   - Validation : 2+ readings consécutifs >75%
   - Action suggérée : "Risque moisissure, aérer immédiatement"

3. **ALERT_COLD_NIGHT** (Low impact)
   - Condition : Température <16°C entre 22h-7h
   - Validation : Context mode check (nuit uniquement)
   - Action suggérée : "Augmenter chauffage"

**Integration Decision Engine** :
- Trust score calculation : 85% (capteur fiable, pas d'interaction humaine)
- Impact levels : Low (confort) / Medium (santé préventive) / High (santé critique)
- Idempotence : command_id basé sur room_id + rule + timestamp (window 5 min)

### API HTTP

**Endpoints** :

```
GET /v1/environment/chambre
    → RoomEnvironmentState JSON (current + status)

GET /v1/environment/chambre/history?hours=24
    → EnvReading[] JSON (array historique)
```

**Authentification** : JWT required (existing auth system)
**Rate limiting** : 60 req/min (lecture seule, pas critique)

### PWA Widget

**Fichier** : `pwa-dashboard/src/components/environment-chambre-widget.js`

**Features** :
- Affichage température + humidité en temps réel
- Status badge (Ok ✅ / Humid ⚠️ / Risk Mold 🚨 / Cold ❄️)
- Mini-chart historique 24h (canvas 2D)
- MQTT subscription pour updates live
- Fallback polling si MQTT disconnect (30s interval)

**Design** :
- Card layout responsive (grid 1fr, min 200px)
- Color coding : green (ok), yellow (warning), red (critical)
- Timestamp last update
- Click → navigation page détaillée (future F3)

### ESP32 Firmware

**Langage** : Rust embedded (esp-idf-hal + rumqttc-embedded)
**Hardware** : ESP32-WROOM-32 + BME280 I2C
**Power** : USB 5V (pas batterie pour v1)

**Structure** :

```
symbion-sensor-esp32/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point, WiFi init
│   ├── sensors.rs       # BME280 I2C driver wrapper
│   ├── mqtt.rs          # MQTT client + reconnect logic
│   └── config.rs        # WiFi SSID/password, MQTT broker
└── .cargo/
    └── config.toml      # ESP32 target config
```

**Fonctionnalités** :
- WiFi auto-reconnect (exponential backoff, max 5 retries)
- MQTT QoS 1 publish avec ack timeout
- Reading interval configurable (default 5 min)
- Watchdog timer 60s (reboot si hang)
- OTA update support (future)

**Calibration** :
- Offset température : -0.5°C (compensation self-heating ESP32)
- Offset humidité : +2% (empirique, BME280 drift)

### Tests Unitaires

**Fichiers** :

```rust
// symbion-kernel/src/environment.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_circular_buffer_eviction() { ... }

    #[test]
    fn test_status_calculation_humid() { ... }

    #[test]
    fn test_status_calculation_risk_mold() { ... }

    #[test]
    fn test_get_history_filter_by_hours() { ... }
}

// symbion-kernel/src/decision/environment.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_humidity_alert_sustained_30min() { ... }

    #[test]
    fn test_humidity_alert_not_sustained() { ... }

    #[test]
    fn test_critical_humidity_10min() { ... }

    #[test]
    fn test_cold_night_only() { ... }
}
```

**Coverage cible** : 80%+ (critical path environment rules)

### Checklist Implémentation

- [ ] `environment.rs` : RoomEnvironmentState + circular buffer
- [ ] `decision/environment.rs` : 3 règles alertes (humidity warning, critical, cold)
- [ ] MQTT ingestion : handler `symbion/sensors/chambre/env@v1`
- [ ] API endpoints : `/v1/environment/chambre` + `/history`
- [ ] PWA widget : `environment-chambre-widget.js` (Lit component)
- [ ] Tests unitaires : 8+ tests (state + decision rules)
- [ ] ESP32 firmware : Rust embedded (BME280 + MQTT publish)
- [ ] Documentation : Update `docs/mqtt/topics.md` (nouveau topic)
- [ ] Integration testing : End-to-end ESP32 → Kernel → PWA

---

## F2 - Organe Hygiène Digitale

**Effort** : 9 jours
**Statut** : 🟡 Pending (après F1 stable)
**Objectif** : Tracking activité PC (idle/work/game) et prévention burnout

### Use Case

Monitoring comportement utilisateur :
- Détection sessions work prolongées (>4h)
- Alerte burnout (>10h/jour pendant 3 jours)
- Stats hebdomadaires temps d'écran réel

### Architecture

**Pipeline** :
```
symbion-agent-host (ActivityTracker)
    ↓ MQTT QoS 1
symbion/agents/activity@v1
    ↓ Ingestion
Kernel DigitalHygieneState
    ↓ Decision Engine
Digital Hygiene Rules (sustained high activity)
    ↓ Intentions
PWA Widget + Dashboard Page
```

### Contrat MQTT

**Topic** : `symbion/agents/activity@v1`
**Publisher** : symbion-agent-host
**Subscribers** : Kernel digital_hygiene module

**Payload JSON** :
```json
{
  "agent_id": "pc-bureau",
  "state": "work",  // idle | work | game | unknown
  "duration_seconds": 7320,
  "app_name": "code",
  "processes": [
    {"name": "code", "cpu_pct": 15.2},
    {"name": "rust-analyzer", "cpu_pct": 8.1}
  ],
  "timestamp": "2025-11-16T16:45:00Z"
}
```

**Fréquence** : 1 message / 5 min (aligned avec agent heartbeat)

### Agent Implementation

**Fichier** : `symbion-agent-host/src/activity.rs`

**Features** :
- Process classification (work vs game keywords)
- Idle detection (no input >5 min)
- State transition tracking
- Top 5 processes CPU usage

### Decision Engine Rules

1. **SUGGEST_BREAK** (Low impact)
   - Condition : work state sustained >4h
   - Action : "Pause suggérée (4h travail continu)"

2. **ALERT_BURNOUT_RISK** (Medium impact)
   - Condition : >10h work/day pendant 3 jours
   - Action : "Risque burnout détecté, repos recommandé"

### PWA Components

- **digital-hygiene-widget.js** : Quick view temps d'écran jour
- **digital-hygiene-page.js** : Stats détaillées semaine/mois

---

## F3 - Organe Intentions Log

**Effort** : 5 jours
**Statut** : 🟡 Pending (après F2)
**Objectif** : Persistence et analytics historique toutes intentions Decision Engine

### Use Case

Mémoire système pour :
- Traçabilité décisions prises
- Analytics patterns (quelles alertes ignorées)
- Feedback loop amélioration continue

### Architecture

**Storage** : JSON file `intentions_log.json` (migration PostgreSQL future)
**Retention** : 90 jours rolling (auto-purge)
**Index** : intention_type + timestamp

### API Endpoints

```
GET /v1/intentions/log?type=ALERT_HUMIDITY_CHAMBRE&since=2025-11-01
    → Intention[] JSON (filtered history)

GET /v1/intentions/stats
    → { total: 1234, by_type: {...}, by_impact: {...} }
```

### PWA Page

**intentions-log-page.js** :
- Table paginée (25 items/page)
- Filters : type, impact, date range
- Export CSV (analytics externe)

---

## F4 - Organe Notifications

**Effort** : 6 jours
**Statut** : 🟡 Pending (après F3)
**Objectif** : Push notifications multi-canal (PWA + ntfy fallback)

### Use Case

Alertes critiques même hors domicile :
- PWA Web Notifications (si dashboard ouvert)
- Fallback ntfy.sh (push Android même app fermée)
- Escalation P0 → immédiat, P1 → retry si pas vu 15min

### Challenges Techniques

#### Problème VPN/PWA Access

**Contexte** : PWA sur `symbion.local` (mDNS) pas accessible via VPN standard

**Options** :

1. **Domaine Public + Let's Encrypt**
   - Pros : Accessible partout, certificat valide
   - Cons : Exposition publique (mitigation : auth forte + firewall strict)
   - Implémentation : DynDNS + Let's Encrypt ACME
   - Coût : 0€ (DuckDNS gratuit)

2. **VPN Split-Tunnel avec DNS Local**
   - Pros : Sécurité maximale (LAN-only maintenu)
   - Cons : Configuration complexe (WireGuard + dnsmasq)
   - Implémentation : WireGuard server + forwarding .local → LAN IP
   - Coût : 0€ (self-hosted)

3. **Sous-Domaine + Reverse Proxy**
   - Pros : Meilleur compromis (accès externe, auth existante)
   - Cons : Port forwarding requis (faille potentielle)
   - Implémentation : Nginx reverse proxy + fail2ban
   - Coût : 0€ (domaine existant)

**Recommandation** : Option 2 (VPN split-tunnel) pour F4 v1, évaluer option 3 si trop complexe

### Architecture Notifications

**Pipeline** :
```
Decision Engine Intention (impact High/Medium)
    ↓
Notifications Manager (kernel)
    ↓ Parallel dispatch
    ├─→ PWA Web Notifications API (si service worker actif)
    └─→ ntfy.sh POST (fallback si PWA unavailable)
```

### Contrat ntfy

**Topic** : `https://ntfy.sh/symbion-{user_hash}` (hash anonymisé)
**Auth** : Token ntfy (stocké .env)
**Priority** : High (P0), Default (P1), Low (P2)

**Message Format** :
```json
{
  "topic": "symbion-abc123",
  "title": "Symbion Alert",
  "message": "Chambre humide 78%, aérer urgent",
  "priority": "high",
  "tags": ["warning"]
}
```

### PWA Service Worker

**Fichier** : `pwa-dashboard/sw.js`

**Features** :
- Push subscription management
- Notification permission request
- Background sync (retry failed pushes)
- Click action → navigate to intentions log

---

## F5 - Organe Light Actuator

**Effort** : 10 jours
**Statut** : 🟡 Pending (après F4)
**Objectif** : Contrôle lumières connectées avec abstraction générique

### Use Case

Ambiances automatiques :
- Mode Cravate → lumière froide 100%
- Mode Intime → lumière chaude 30%
- Nuit → extinction progressive

### Clarification Matériel

**Ampoules** : SmartLife/Tuya ecosystem (pas Philips Hue)
**Protocole** : WiFi ou Zigbee (via hub Tuya)
**Contrainte** : **LAN-only, pas de cloud Tuya**

### Architecture Abstraction

**Interface Générique** : `LightActuator` trait

```rust
// symbion-kernel/src/lights/actuator.rs

pub trait LightActuator {
    fn turn_on(&self, light_id: &str) -> Result<(), Error>;
    fn turn_off(&self, light_id: &str) -> Result<(), Error>;
    fn set_brightness(&self, light_id: &str, pct: u8) -> Result<(), Error>;
    fn set_color_temp(&self, light_id: &str, kelvin: u16) -> Result<(), Error>;
    fn set_rgb(&self, light_id: &str, r: u8, g: u8, b: u8) -> Result<(), Error>;
    fn get_state(&self, light_id: &str) -> Result<LightState, Error>;
}

pub struct LightState {
    pub is_on: bool,
    pub brightness: u8,
    pub color_temp_k: Option<u16>,
    pub rgb: Option<(u8, u8, u8)>,
}
```

**Backends Possibles** (implémentation future) :
- `TuyaLocalActuator` : API locale Tuya (tuyapi protocol)
- `MQTTBridgeActuator` : Via broker MQTT (Zigbee2MQTT-like)
- `MockActuator` : Tests unitaires

**Décision Backend** : À confirmer après identification matériel exact

### Contrat MQTT

**Topics** :

```
symbion/lights/command@v1 (Kernel → Actuator)
    → { "light_id": "chambre_1", "action": "set_brightness", "value": 30 }

symbion/lights/state@v1 (Actuator → Kernel)
    → { "light_id": "chambre_1", "is_on": true, "brightness": 30 }
```

### Decision Engine Integration

**Règles Context-Aware** :

```rust
// symbion-kernel/src/decision/ambiance.rs

pub fn evaluate_context_change(
    old_mode: ContextMode,
    new_mode: ContextMode,
) -> Option<Intention> {
    match new_mode {
        ContextMode::Cravate => Some(Intention {
            intention_type: "SET_LIGHTS_WORK".to_string(),
            context: json!({
                "lights": [
                    {"id": "bureau_1", "brightness": 100, "temp_k": 6500}
                ]
            }),
            ...
        }),
        ContextMode::Intime => Some(Intention {
            intention_type: "SET_LIGHTS_RELAX".to_string(),
            context: json!({
                "lights": [
                    {"id": "salon_1", "brightness": 30, "temp_k": 2700}
                ]
            }),
            ...
        }),
        _ => None,
    }
}
```

### PWA Widget

**lights-control-widget.js** :
- Toggle on/off par lumière
- Slider brightness (0-100%)
- Color picker (RGB)
- Scénarios prédéfinis (Work, Relax, Night)

### Checklist Implémentation

- [ ] `lights/actuator.rs` : Trait LightActuator générique
- [ ] `lights/mock.rs` : MockActuator pour tests
- [ ] `decision/ambiance.rs` : Règles context-aware
- [ ] MQTT topics : `lights/command@v1` + `lights/state@v1`
- [ ] API endpoints : `/v1/lights` (list) + `/v1/lights/:id` (control)
- [ ] PWA widget : `lights-control-widget.js`
- [ ] Tests unitaires : Mock actuator + decision rules
- [ ] **PENDING** : Backend Tuya local (après confirmation matériel)

---

## Timeline Globale

### Q4 2025 (Nov-Dec)

**Semaine 1-2** : F1 Environment (9 jours)
- Kernel environment.rs + decision rules
- ESP32 firmware + BME280 integration
- PWA widget

**Semaine 3-4** : F2 Digital Hygiene (9 jours)
- Agent activity tracker
- Decision rules burnout
- PWA page stats

### Q1 2026 (Jan-Mar)

**Semaine 1** : F3 Intentions Log (5 jours)
- Persistence JSON
- API analytics
- PWA page log

**Semaine 2-3** : F4 Notifications (6 jours)
- Service Worker PWA
- ntfy integration
- VPN/PWA access solution

**Semaine 4-5** : F5 Light Actuator (10 jours)
- Abstraction LightActuator
- Backend Tuya local (après confirmation)
- Decision rules ambiance

### Total Effort

**5 Organes** : 39 jours (~8 semaines)
**Target v1.0.0** : Mars 2026

---

## Prochaines Étapes Immédiates

### Cette Semaine (16-22 Nov 2025)

1. ✅ Cleanup roadmaps obsolètes
2. ✅ Create private/vision.md
3. 🔵 **Démarrer F1 - Environment** :
   - [ ] Module kernel environment.rs
   - [ ] MQTT ingestion handler
   - [ ] Decision Engine rules
   - [ ] PWA widget basics

### Semaine Suivante (23-29 Nov 2025)

4. [ ] ESP32 firmware Rust embedded
5. [ ] Tests unitaires F1 (80%+ coverage)
6. [ ] Documentation MQTT topics
7. [ ] End-to-end testing ESP32 → PWA

---

## Métriques Succès

**F1 Environment** :
- ✅ 0 faux positifs alertes (precision >95%)
- ✅ <5min latency alerte critique (humidité >75%)
- ✅ 7 jours uptime ESP32 sans reboot

**F2 Digital Hygiene** :
- ✅ Détection burnout 100% (>10h/jour × 3 jours)
- ✅ <10% false positives idle detection

**F3 Intentions Log** :
- ✅ 0 perte données (persistence fiable)
- ✅ <500ms query response (analytics)

**F4 Notifications** :
- ✅ 100% delivery rate P0 alerts (ntfy fallback)
- ✅ <30s latency notification push

**F5 Light Actuator** :
- ✅ <2s response time commande lumière
- ✅ 100% state sync (lights ↔ kernel)

---

## Notes Techniques

### Contraintes Architecture

**LAN-Only** :
- Pas de cloud dependencies
- Local MQTT broker (Mosquitto)
- Self-hosted ntfy (future, v1 uses ntfy.sh)

**Privacy** :
- Aucun nom personnel dans code
- Intention types génériques
- Logs anonymisés

**Scalabilité** :
- Circular buffers (memory-bounded)
- JSON files → PostgreSQL migration (PR6)
- MQTT QoS 1 (balance perf/reliability)

### Dépendances Externes

**Hardware** :
- ESP32-WROOM-32 (~5€)
- BME280 I2C sensor (~3€)
- SmartLife/Tuya bulbs (existant)

**Software** :
- Mosquitto MQTT broker (installé)
- ntfy.sh (free tier 50 msg/day, puis self-hosted)
- WireGuard VPN (pour option 2 F4)

---

**Document Maintenu Par** : Claude Code + Mark
**Git Branch** : master
**Licence** : MIT
