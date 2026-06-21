# Symbion Plugin Contract v1

> **Version**: 1.0.0
> **Date**: 2 Février 2026
> **Status**: Specification

---

## Principes Fondamentaux

### Philosophie

Le système Symbion suit une architecture **hub-and-spoke** stricte :

- **Kernel** = Cerveau unique, centre décisionnel
- **Plugin** = Processus autonome, exécutant pur
- **MQTT** = Système nerveux, bus de communication

Un plugin n'a **aucune intelligence décisionnelle**. Il expose des capacités et exécute des commandes.

---

## Règles Inviolables

### Règle #1 : Action vs Event

| Concept | Direction | Origine | Comportement | Usage |
|---------|-----------|---------|--------------|-------|
| **Action** | Kernel → Plugin | Decision Engine | ACK obligatoire | Commandes avec impact |
| **Event** | Plugin → Kernel | Plugin | Best-effort, informatif | Signalement état/données |

**Actions** :
- Déclenchées uniquement par le Kernel
- Passent par le Decision Engine (évaluation impact)
- Requièrent un ACK synchrone
- Exemples : `create_note`, `send_notification`, `turn_on_light`

**Events** :
- Émis librement par le plugin
- Informatifs uniquement, pas de garantie de traitement
- Exemples : `note_created`, `sensor_value_changed`, `user_activity_detected`

### Règle #2 : Plugin ≠ Décideur

> **Un plugin ne peut JAMAIS modifier l'état global sans passer par une Action évaluée par le Decision Engine.**

Implications :
- Un plugin ne peut pas changer le mode contextuel
- Un plugin ne peut pas déclencher d'autres actions
- Un plugin ne peut pas modifier les automations
- Un plugin reçoit des ordres, il n'en donne pas

**Exemple interdit** :
```
Plugin Notes détecte "réunion importante" dans une note
  → NON: Plugin change automatiquement le mode en "focus"
  → OUI: Plugin émet un Event "meeting_keyword_detected"
         Kernel analyse, Decision Engine évalue, puis envoie Action si approprié
```

---

## Conventions MQTT

### Topics

```
symbion/plugins/{plugin_id}/actions    # Kernel → Plugin (commandes)
symbion/plugins/{plugin_id}/events     # Plugin → Kernel (informatif)
symbion/plugins/{plugin_id}/health     # Plugin → Kernel (heartbeat)
```

### Identifiants Plugin

Format : `kebab-case`, alphanumérique + tirets
- ✅ `notes`, `notifications`, `hue-lights`, `home-assistant`
- ❌ `Notes`, `my_plugin`, `plugin.test`

### Exemples

```
symbion/plugins/notes/actions         # Commandes vers plugin notes
symbion/plugins/notes/events          # Events du plugin notes
symbion/plugins/hue-lights/actions    # Commandes vers Hue
```

---

## Structures de Messages

### Versioning

**TOUS les messages** incluent `spec_version` pour compatibilité future :

```json
{
  "spec_version": "1.0",
  ...
}
```

### ActionRequest (Kernel → Plugin)

```json
{
  "spec_version": "1.0",
  "action_id": "uuid-v4",
  "action_type": "create_note",
  "payload": {
    "title": "Nouvelle note",
    "content": "Contenu de la note",
    "context": "intime"
  },
  "metadata": {
    "automation_id": "uuid-or-null",
    "triggered_by": "schedule|event|manual",
    "timestamp": "2026-02-02T20:00:00Z"
  }
}
```

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `spec_version` | string | ✅ | Version du contrat ("1.0") |
| `action_id` | UUID | ✅ | Identifiant unique de l'action |
| `action_type` | string | ✅ | Type d'action (défini dans capabilities) |
| `payload` | object | ✅ | Données spécifiques à l'action |
| `metadata` | object | ❌ | Contexte d'exécution |

### ActionResponse (Plugin → Kernel via HTTP)

**Modèle ACK : HTTP/Unix Socket Synchrone**

L'action est envoyée via POST sur le socket Unix du plugin. La réponse HTTP EST l'ACK.

```json
{
  "spec_version": "1.0",
  "action_id": "uuid-correspondant",
  "status": "success",
  "result": {
    "note_id": "created-note-uuid",
    "tags": ["auto", "intime"]
  },
  "error": null,
  "execution_time_ms": 45
}
```

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `spec_version` | string | ✅ | Version du contrat ("1.0") |
| `action_id` | UUID | ✅ | Référence à la requête |
| `status` | enum | ✅ | `success`, `error`, `rejected` |
| `result` | object | ❌ | Données de retour (si success) |
| `error` | object | ❌ | Détails erreur (si error/rejected) |
| `execution_time_ms` | u64 | ❌ | Temps d'exécution |

**Status possibles** :
- `success` : Action exécutée correctement
- `error` : Erreur technique (retry possible)
- `rejected` : Action refusée par le plugin (pas de retry)

### EventMessage (Plugin → Kernel via MQTT)

```json
{
  "spec_version": "1.0",
  "event_type": "note_created",
  "plugin_id": "notes",
  "payload": {
    "note_id": "uuid",
    "title": "Ma note",
    "context": "intime"
  },
  "timestamp": "2026-02-02T20:00:00Z"
}
```

| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `spec_version` | string | ✅ | Version du contrat ("1.0") |
| `event_type` | string | ✅ | Type d'événement |
| `plugin_id` | string | ✅ | Identifiant du plugin émetteur |
| `payload` | object | ✅ | Données de l'événement |
| `timestamp` | ISO8601 | ✅ | Horodatage UTC |

---

## Plugin Manifest

Chaque plugin déclare ses capacités au démarrage :

```json
{
  "spec_version": "1.0",
  "plugin_id": "notes",
  "name": "Symbion Notes",
  "version": "1.0.0",
  "description": "Gestion des notes contextuelles",
  "capabilities": [
    {
      "action_type": "create_note",
      "description": "Créer une nouvelle note",
      "impact_level": "low",
      "parameters": {
        "title": { "type": "string", "required": true },
        "content": { "type": "string", "required": false },
        "context": { "type": "string", "required": false }
      }
    },
    {
      "action_type": "delete_note",
      "description": "Supprimer une note",
      "impact_level": "medium",
      "parameters": {
        "note_id": { "type": "uuid", "required": true }
      }
    }
  ],
  "events": [
    {
      "event_type": "note_created",
      "description": "Une note a été créée"
    },
    {
      "event_type": "note_deleted",
      "description": "Une note a été supprimée"
    }
  ],
  "health_endpoint": "/health",
  "socket_path": "/run/symbion-plugins/notes.sock"
}
```

### Impact Levels

| Niveau | Description | Comportement Decision Engine |
|--------|-------------|------------------------------|
| `low` | Action réversible, impact minimal | Auto-approve si confiance > 0.5 |
| `medium` | Impact modéré, potentiellement réversible | Require confiance > 0.7 |
| `high` | Impact significatif, irréversible | Require validation explicite |
| `critical` | Impact critique système | Toujours require validation manuelle |

### Format alternatif : Actions templates pour rule builder PWA (mai 2026)

En complément du manifest MQTT historique (capabilities), un plugin peut exposer
ses actions structurées via le **PluginRegistration HTTP** envoyé au kernel
(`POST /v1/plugins/register`). Ces actions servent au rule builder PWA pour
générer un formulaire au lieu d'un payload JSON libre.

**Format** (`symbion-plugin-common/src/lib.rs::PluginAction`) :
```json
{
  "name": "power_on",
  "label": "Allumer la machine",
  "description": "Sort la machine du mode standby",
  "icon": "⚡",
  "route": "power",
  "method": "POST",
  "impact_level": "Low",
  "params": [
    {
      "name": "on",
      "label": "État",
      "type": "bool",
      "required": true,
      "default": true
    }
  ]
}
```

**Différence avec capabilities MQTT** :
- `capabilities` (MQTT) → contrat sémantique du plugin, utilisé pour
  ActionRequest/ActionResponse via `/actions` socket
- `actions` (PluginRegistration HTTP) → templates UI pour le rule builder.
  Le PWA construit `ActionDefinition::PluginCommand` directement, le plugin
  reçoit un POST classique sur `/{route}` (pas un wrap Contract v1.0)

Les deux peuvent coexister. Voir `docs/PLUGIN_DEVELOPMENT_GUIDE.md` §6 pour
l'implémentation Rust et `symbion-plugin-coffee` pour l'exemple de référence.

---

## Modèle ACK Explicite

### Choix d'architecture : HTTP/Unix Socket Synchrone

**Avantages** :
- Simplicité : pas de topic MQTT de réponse
- Garantie : réponse dans la même connexion TCP
- Timeout : facile à implémenter côté Kernel
- Debug : curl fonctionne directement

**Flow** :
```
┌─────────┐         ┌──────────┐
│ Kernel  │         │  Plugin  │
└────┬────┘         └────┬─────┘
     │                   │
     │ POST /actions     │
     │ {ActionRequest}   │
     │─────────────────→│
     │                   │
     │                   │ Execute
     │                   │
     │ HTTP 200          │
     │ {ActionResponse}  │
     │←─────────────────│
     │                   │
```

**Timeout** : 30 secondes par défaut, configurable par action

### Exemple cURL

```bash
curl -X POST \
  --unix-socket /run/symbion-plugins/notes.sock \
  http://localhost/actions \
  -H "Content-Type: application/json" \
  -d '{
    "spec_version": "1.0",
    "action_id": "550e8400-e29b-41d4-a716-446655440000",
    "action_type": "create_note",
    "payload": {
      "title": "Test",
      "content": "Hello World"
    }
  }'
```

---

## Cycle de Vie Plugin

### Démarrage

1. Plugin démarre et crée son socket Unix
2. Plugin publie manifest sur `symbion/plugins/{id}/manifest`
3. Kernel enregistre les capabilities
4. Plugin commence heartbeat sur `symbion/plugins/{id}/health`

> ### ⚠️ Écart implémentation — le manifest MQTT n'est PAS consommé (juin 2026)
>
> Les étapes 2 et 3 ci-dessus décrivent une cible, **pas le comportement réel du
> kernel**. Vérifié en traçant le code (juin 2026) :
>
> - Le kernel ne souscrit **jamais** à `symbion/plugins/+/manifest`. La liste des
>   `client.subscribe(...)` est fixe (`symbion-kernel/src/mqtt.rs` ~l.124-182) et
>   ne contient pas ce topic. Un plugin qui publie son manifest l'envoie dans le vide.
> - Deux types `PluginRegistry` coexistent :
>   - `plugin_proxy::PluginRegistry` → **le vrai**, branché dans `AppState`
>     (`http/mod.rs:82`) et `bootstrap/tasks.rs:18` via `discover_plugins()`.
>   - `plugins::registry::PluginRegistry` + `PluginManifest` + `contract::manifest()`
>     (builder du topic, `plugins/contract.rs:424`) → **code mort en prod** :
>     `register(manifest)` n'est appelé que dans les tests.
>
> **Chemin réellement actif** (HTTP, pas MQTT) :
> `POST /v1/plugins/register` → `plugin_proxy::register_plugin()`
> (`plugin_proxy.rs:128`), qui consomme `PluginRegistration { name, socket_path,
> routes, version, description, actions }` — **aucun champ `capabilities` ni
> `features`**. Les features sont ingérées séparément via le flux
> `symbion/features/update` (`mqtt.rs:568`, struct `ExternalFeatureUpdate`).
>
> **Conséquences pratiques pour un plugin** :
> - L'enregistrement HTTP (`PluginRegistrationBuilder::...register()`) est ce qui
>   rend le plugin découvrable et proxifie ses routes sur `/v1/plugin-api/{id}/*`.
>   C'est **obligatoire**.
> - Publier le manifest sur `symbion/plugins/{id}/manifest` reste **décoratif** :
>   tous les plugins le font (ssl, freebox, coffee…) par convention et pour le
>   debug via explorateur MQTT, mais le kernel l'ignore aujourd'hui.
>
> **Dette technique côté kernel** : soit câbler une souscription
> `symbion/plugins/+/manifest` → `plugins::registry::PluginRegistry.register()`,
> soit supprimer le type manifest mort et mettre cette doc en accord avec le
> chemin HTTP. À trancher avant de prétendre que le manifest fait foi.

### Heartbeat

```json
{
  "spec_version": "1.0",
  "plugin_id": "notes",
  "status": "healthy",
  "uptime_seconds": 3600,
  "last_action_at": "2026-02-02T19:55:00Z"
}
```

Fréquence : toutes les 30 secondes

### Arrêt

1. Plugin publie `{"status": "stopping"}` sur health
2. Plugin ferme son socket Unix
3. Kernel retire les capabilities du registry

---

## Plugin Registry (Session 2)

Le kernel maintient un registre centralisé de tous les plugins.

### États Plugin

| État | Description | Accepte Actions |
|------|-------------|-----------------|
| `available` | Plugin healthy, pleinement opérationnel | Oui |
| `degraded` | Plugin répond mais signale des problèmes | Oui (avec prudence) |
| `offline` | Plugin ne répond pas ou arrêté | Non |

### Transitions d'état

```
                    health: healthy
        ┌────────────────────────────────┐
        │                                │
        ▼                                │
   ┌─────────┐   health: degraded   ┌─────────┐
   │Available│◄────────────────────►│Degraded │
   └────┬────┘                      └────┬────┘
        │                                │
        │   3 health failures            │   3 health failures
        │   OR health: unhealthy         │   OR health: unhealthy
        ▼                                ▼
   ┌─────────────────────────────────────────┐
   │               Offline                   │
   └─────────────────────────────────────────┘
```

### Validation Dispatch

Avant d'envoyer une action, le kernel vérifie :

1. **Plugin existe** dans le registry
2. **Plugin accepte les actions** (available ou degraded)
3. **Capability déclarée** pour le type d'action

```rust
// Erreurs possibles
DispatchError::PluginNotFound      // Plugin inconnu
DispatchError::PluginOffline       // Plugin hors ligne
DispatchError::CapabilityNotFound  // Action non supportée
```

### Health Monitoring

- Heartbeat attendu : toutes les 30 secondes
- Seuil offline : 3 échecs consécutifs
- Récupération : un seul heartbeat `healthy` suffit

### Principe Fondamental

> **Pas de routing intelligent, pas de magie, pas d'orchestration cachée.**

Le registry est une table de lookup simple :
- Plugin ID → État + Capabilities
- Pas de load balancing
- Pas de failover automatique
- Pas de redirection

---

## Hors Scope (Sessions 1-2)

Les éléments suivants sont explicitement hors scope :

- Bus custom (MQTT suffit)
- Capabilities dynamiques (rechargement à chaud)
- Multi-version spec (négociation version)
- Sandbox/permissions granulaires
- UI gestion plugins
- Load balancing / failover
- Adaptation notes en plugin (Session 3)

---

## Changelog

### v1.1.0 (2 Février 2026) - Session 2
- Plugin Registry avec états (available/degraded/offline)
- Validation dispatch avant envoi d'action
- Health monitoring avec seuil d'échecs
- Structures Rust : PluginRegistry, PluginState, DispatchError

### v1.0.0 (2 Février 2026) - Session 1
- Règles fondamentales Action vs Event
- Convention topics MQTT
- Structures ActionRequest, ActionResponse, EventMessage
- Modèle ACK HTTP/Unix Socket
- Versioning au niveau message (spec_version)
- Niveaux d'impact (ImpactLevel)
