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

## Hors Scope (Session 1)

Les éléments suivants sont explicitement hors scope pour cette version :

- Bus custom (MQTT suffit)
- Capabilities dynamiques (rechargement à chaud)
- Multi-version spec (négociation version)
- Sandbox/permissions granulaires
- UI gestion plugins
- Plugin Registry centralisé (Session 2)
- Adaptation notes en plugin (Session 3)

---

## Changelog

### v1.0.0 (2 Février 2026)
- Règles fondamentales Action vs Event
- Convention topics MQTT
- Structures ActionRequest, ActionResponse, EventMessage
- Modèle ACK HTTP/Unix Socket
- Versioning au niveau message (spec_version)
- Niveaux d'impact (ImpactLevel)
