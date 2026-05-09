# Référence Complète des Endpoints HTTP

> 📍 Documentation de l'API Symbion Kernel
>
> ⚠️ **OBSOLÈTE PARTIEL (audit 9 mai 2026)** : ce fichier date de la restructuration du 13 Mars 2026 (suppression des préfixes `/v1/` pour beaucoup de routes). Couverture actuelle estimée à **62 %** :
> - **186 endpoints réels** dans le code (150 kernel + 36 plugins)
> - **116 documentés ici** → 118 routes non documentées (notamment agents file management, automations v2, modes, notifications enrichies) + 34 routes documentées mais supprimées
>
> **Plugins actifs (8)** : sensors, notes, ssl, library, telegram, freebox, common, **coffee** (ce dernier branché à l'Intelligence Engine le 9 mai 2026 — voir CHANGELOG)
>
> **Action recommandée** : régénérer cette doc depuis le code via OpenAPI/utoipa derive (déjà annoté côté kernel pour /docs Swagger). Tant que ce n'est pas fait, le code reste la source de vérité — `grep -rn ".route(" symbion-kernel/src/` donne la liste réelle.
>
> Modules présents et globalement fiables ci-dessous : Auth, CSRF, Intelligence (ces sections n'ont quasi pas bougé). Modules à risque : Agents file mgmt, Automations, Modes, Notifications.

---

## ⚠️ ENDPOINTS DÉPRÉCIÉS (Non Implémentés)

Les endpoints suivants sont **RETIRÉS** de la documentation car non implémentés dans le code actuel :

### Auth (Dépréciés)
- ❌ `GET /csrf-token` → Utiliser `/auth/csrf/nonce`
- ❌ `POST /jwt/verify` → Utiliser `/auth/verify`
- ❌ `GET /sessions` → Non implémenté
- ❌ `DELETE /sessions/{id}` → Non implémenté
- ❌ `POST /refresh` → Non implémenté

### Users (Dépréciés)
- ❌ `GET /users` → Utiliser `/v1/users`
- ❌ `PUT /users/{id}` → Non implémenté
- ❌ `PUT /users/{id}/password` → Non implémenté

### Agents (Dépréciés)
- ❌ `DELETE /agents/{id}` → Non implémenté
- ❌ `GET /agents/{id}/logs` → Non implémenté
- ❌ `POST /agents/{id}/restart` → Non implémenté
- ❌ `GET /agents/{id}/capabilities` → Non implémenté

### Plugins (Dépréciés)
- ❌ `GET /plugins/{name}/health` → Non implémenté

### Decision Engine (Dépréciés)
- ❌ `GET /decision/consents` → Non implémenté
- ❌ `DELETE /decision/consents/{id}` → Non implémenté
- ❌ `GET /decision/trust-score` → Utiliser `/decision/agent-health`
- ❌ `GET /decision/pending` → Utiliser `/decision/validations/pending`

### Système (Dépréciés)
- ❌ `GET /config` → Utiliser `/decision/config`
- ❌ `PUT /config` → Non implémenté
- ❌ `GET /system/status` → Utiliser `/system/health`

---

## 🟢 Endpoints Publics (Sans Authentification)

### `GET /health`
**Description** : Vérification santé système
**Auth** : Non requis
**Response** :
```json
{
  "status": "healthy",
  "mqtt_connected": true,
  "agents_online": 2,
  "uptime_seconds": 86400
}
```

### `GET /system/health`
**Description** : Statut détaillé du système
**Auth** : Non requis
**Response** :
```json
{
  "version": "0.1.0",
  "mqtt": {
    "connected": true,
    "broker": "127.0.0.1:1883"
  },
  "agents": {
    "total": 2,
    "online": 2,
    "offline": 0
  },
  "plugins": {
    "memo": "running"
  }
}
```

### `GET /ca-certificate`
**Description** : Téléchargement certificat CA pour TLS
**Auth** : Non requis
**Response** : Fichier PEM (`symbion-ca.crt`)

### `GET /swagger-ui/*`
**Description** : Interface Swagger UI pour exploration API
**Auth** : Non requis
**Source** : `symbion-kernel/src/http/mod.rs:447`

### `GET /api-docs/openapi.json`
**Description** : Spécification OpenAPI JSON
**Auth** : Non requis
**Source** : `symbion-kernel/src/openapi.rs`

### `GET /v1/public/library/*`
**Description** : Proxy lecture seule vers le plugin bibliothèque
**Auth** : Non requis (public)
**Méthodes** : GET uniquement (405 pour POST/PUT/DELETE)
**CORS** : `Access-Control-Allow-Origin: *`
**Source** : `symbion-kernel/src/plugin_proxy.rs:508-606`

**Sous-routes disponibles** :
- `GET /v1/public/library/sections` — Liste sections
- `GET /v1/public/library/nodes?limit=N&offset=N` — Liste nodes
- `GET /v1/public/library/nodes/{id}` — Détail node
- `GET /v1/public/library/search?q=...` — Recherche FTS5
- `GET /v1/public/library/templates` — Liste templates
- `GET /v1/public/library/graph` — Données graphe

---

## 📊 Metrics & Observability (PR4)

### `GET /metrics`
**Description** : Métriques Prometheus (format text/plain)
**Auth** : Non requis
**Response** : Format Prometheus text
```
# HELP symbion_decisions_total Total decision engine evaluations
# TYPE symbion_decisions_total counter
symbion_decisions_total{outcome="approved"} 42
symbion_decisions_total{outcome="blocked"} 5

# HELP symbion_guards_total Guard executions
# TYPE symbion_guards_total counter
symbion_guards_total{result="passed"} 38
symbion_guards_total{result="blocked"} 9
```

### `GET /v1/metrics/agents`
**Description** : Métriques agents (format JSON)
**Auth** : JWT
**Response** :
```json
{
  "agents": [
    {
      "agent_id": "eridwyn-Salon",
      "status": "online",
      "uptime_seconds": 86400,
      "last_heartbeat": 1732396800,
      "commands_executed": 42
    }
  ],
  "total": 2,
  "online": 2
}
```

### `GET /v1/metrics/system`
**Description** : Métriques système (format JSON)
**Auth** : JWT
**Response** :
```json
{
  "uptime_seconds": 172800,
  "version": "1.1.7",
  "mqtt": {
    "connected": true,
    "messages_published": 1542,
    "messages_received": 1834
  },
  "http": {
    "requests_total": 2341,
    "active_connections": 12
  },
  "decision_engine": {
    "decisions_total": 89,
    "approvals": 67,
    "blocks": 15,
    "validations": 7
  }
}
```

### `GET /ws/notes/stream`
**Description** : WebSocket streaming notes (temps réel)
**Auth** : JWT via query param `?token=...`
**Protocol** : WebSocket
**Response** : Stream MQTT
```json
{"type": "note", "data": {...}}
{"type": "note", "data": {...}}
{"type": "list_end"}
```

---

## 🔐 Authentification & Sessions

> ⚠️ **NOTE DE MIGRATION (Novembre 2025)**: Tous les endpoints d'authentification utilisent maintenant le préfixe `/auth/*`:
> - `/login` → `/auth/login`
> - `/logout` → `/auth/logout`
> - `/mfa/*` → `/auth/mfa/*`
> - `/webauthn/*` → `/auth/webauthn/*`
> - `/csrf/nonce` → `/auth/csrf/nonce`
>
> Les endpoints ci-dessous sont documentés AVEC le préfixe `/auth/` pour refléter l'implémentation actuelle.
>
> **Source**: `symbion-kernel/src/http.rs:203-235`

### `POST /auth/login`
**Description** : Connexion utilisateur (JWT + MFA)
**Auth** : Non requis
**Request** :
```json
{
  "username": "admin",
  "password": "securepassword",
  "device_fingerprint": "browser-uuid-1234" // Optionnel
}
```
**Response (sans MFA)** :
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_at": 1699887600,
  "requires_mfa": false
}
```
**Response (avec MFA)** :
```json
{
  "requires_mfa": true,
  "mfa_token": "temp-mfa-token-123",
  "expires_in": 300
}
```

### `POST /auth/mfa/verify`
**Description** : Complétion login avec code TOTP
**Auth** : MFA Token (temporaire)
**Request** :
```json
{
  "mfa_token": "temp-mfa-token-123",
  "code": "123456",
  "trust_device": true,
  "device_fingerprint": "browser-uuid-1234"
}
```
**Response** :
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_at": 1699887600,
  "device_trusted": true,
  "trust_expires_at": 1702479600
}
```

### `GET /auth/verify`
**Description** : Validation token JWT
**Auth** : JWT
**Response** :
```json
{
  "valid": true,
  "user": {
    "id": "user-123",
    "username": "admin",
    "mfa_verified": true
  },
  "expires_at": 1699887600
}
```

### `POST /auth/logout`
**Description** : Déconnexion (révocation JWT)
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Logged out successfully"
}
```

### `GET /auth/csrf/nonce`
**Description** : Génération nonce CSRF (5 min TTL)
**Auth** : JWT
**Response** :
```json
{
  "token": "csrf-nonce-abcd1234",
  "expires_at": 1699887300
}
```

### `GET /auth/session`
**Description** : Information session utilisateur actuelle
**Auth** : JWT
**Response** :
```json
{
  "user_id": "user-123",
  "username": "admin",
  "is_admin": true,
  "mfa_verified": true,
  "session_started": 1699800800,
  "expires_at": 1699887600
}
```

### `POST /auth/reload`
**Description** : Rechargement base utilisateurs depuis fichier (admin uniquement)
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "User database reloaded",
  "users_count": 5
}
```

---

## 🔑 Multi-Factor Authentication (MFA)

### `POST /auth/mfa/setup`
**Description** : Activation MFA (génère QR code TOTP)
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...",
  "backup_codes": [
    "ABCD-1234",
    "EFGH-5678",
    "IJKL-9012"
  ]
}
```

### `POST /auth/mfa/verify`
**Description** : Validation activation MFA avec premier code
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "code": "123456"
}
```
**Response** :
```json
{
  "success": true,
  "mfa_enabled": true
}
```

### `POST /auth/mfa/disable`
**Description** : Désactivation MFA
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "code": "123456" // Code TOTP actuel ou backup code
}
```
**Response** :
```json
{
  "success": true,
  "mfa_enabled": false
}
```

### `GET /auth/mfa/status`
**Description** : Statut MFA pour l'utilisateur actuel
**Auth** : JWT
**Response** :
```json
{
  "mfa_enabled": true,
  "backup_codes_remaining": 2
}
```

---

## 🗝️ WebAuthn (Passkeys Biométriques)

### `POST /auth/webauthn/register-start`
**Description** : Démarrage enregistrement passkey
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "username": "admin"
}
```
**Response** :
```json
{
  "challenge": "random-challenge-base64",
  "rp": {
    "id": "symbion.local",
    "name": "Symbion Hub"
  },
  "user": {
    "id": "user-123-base64",
    "name": "admin",
    "displayName": "Administrator"
  },
  "pubKeyCredParams": [
    { "type": "public-key", "alg": -7 }
  ],
  "timeout": 60000,
  "attestation": "none"
}
```

### `POST /auth/webauthn/register-finish`
**Description** : Finalisation enregistrement passkey
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "id": "credential-id-base64",
  "rawId": "credential-id-base64",
  "response": {
    "attestationObject": "...",
    "clientDataJSON": "..."
  },
  "type": "public-key"
}
```
**Response** :
```json
{
  "success": true,
  "credential_id": "cred-123",
  "message": "Passkey registered successfully"
}
```

### `POST /auth/webauthn/authenticate-start`
**Description** : Démarrage authentification passkey
**Auth** : Non requis
**Request** :
```json
{
  "username": "admin"
}
```
**Response** :
```json
{
  "challenge": "random-challenge-base64",
  "rpId": "symbion.local",
  "allowCredentials": [
    {
      "type": "public-key",
      "id": "credential-id-base64"
    }
  ],
  "timeout": 60000
}
```

### `POST /auth/webauthn/authenticate-finish`
**Description** : Finalisation authentification passkey
**Auth** : Non requis
**Request** :
```json
{
  "id": "credential-id-base64",
  "rawId": "credential-id-base64",
  "response": {
    "authenticatorData": "...",
    "clientDataJSON": "...",
    "signature": "..."
  },
  "type": "public-key"
}
```
**Response** :
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_at": 1699887600
}
```

### `POST /auth/webauthn/authenticate-discoverable-start`
**Description** : Authentification passkey sans username (discoverable credentials)
**Auth** : Non requis
**Request** :
```json
{}
```
**Response** :
```json
{
  "challenge": "random-challenge-base64",
  "rpId": "symbion.local",
  "timeout": 60000,
  "userVerification": "required"
}
```

### `GET /auth/webauthn/passkeys`
**Description** : Liste passkeys enregistrées
**Auth** : JWT
**Response** :
```json
{
  "credentials": [
    {
      "id": "cred-123",
      "name": "TouchID MacBook",
      "created_at": 1699800800,
      "last_used": 1699887200
    }
  ]
}
```

---

## 👤 Gestion Utilisateurs (Admin Only)

### `GET /v1/users`
**Description** : Liste tous les utilisateurs
**Auth** : JWT (admin)
**Response** :
```json
{
  "users": [
    {
      "id": "user-123",
      "username": "admin",
      "is_admin": true,
      "mfa_enabled": true,
      "created_at": 1699800800
    }
  ]
}
```

### `POST /v1/users`
**Description** : Création utilisateur
**Auth** : JWT (admin)
**CSRF** : Requis
**Request** :
```json
{
  "username": "newuser",
  "password": "securepassword",
  "is_admin": false
}
```
**Response** :
```json
{
  "id": "user-456",
  "username": "newuser",
  "is_admin": false
}
```

### `DELETE /v1/users/{username}`
**Description** : Suppression utilisateur
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "User deleted"
}
```

### `PUT /v1/users/{username}/password`
**Description** : Mise à jour mot de passe utilisateur
**Auth** : JWT (admin ou utilisateur lui-même)
**CSRF** : Requis
**Request** :
```json
{
  "new_password": "newSecurePassword123"
}
```
**Response** :
```json
{
  "success": true,
  "message": "Password updated successfully"
}
```

---

## 🤖 Gestion Agents Domestiques

### `GET /agents`
**Description** : Liste agents avec métriques temps réel
**Auth** : JWT
**Response** :
```json
{
  "agents": [
    {
      "id": "eridwyn-Salon",
      "hostname": "eridwyn-Salon",
      "status": "online",
      "last_seen": 1699887200,
      "platform": {
        "os": "linux",
        "arch": "x86_64"
      },
      "metrics": {
        "cpu_usage": 23.5,
        "memory": {
          "total_mb": 16384,
          "used_mb": 8192,
          "percent": 50.0
        },
        "uptime_seconds": 266400
      },
      "network": {
        "ssid": "HomeNetwork",
        "local_ip": "192.168.1.14"
      }
    }
  ]
}
```

### `GET /agents/{id}`
**Description** : Détails agent spécifique
**Auth** : JWT
**Response** :
```json
{
  "id": "eridwyn-Salon",
  "hostname": "eridwyn-Salon",
  "status": "online",
  "last_seen": 1699887200,
  "platform": {
    "os": "linux",
    "arch": "x86_64",
    "kernel": "6.14.0-33-generic"
  },
  "metrics": {
    "cpu_usage": 23.5,
    "memory": {
      "total_mb": 16384,
      "used_mb": 8192,
      "percent": 50.0
    },
    "disk": {
      "total_gb": 512,
      "used_gb": 256,
      "percent": 50.0
    },
    "uptime_seconds": 266400
  },
  "network": {
    "ssid": "HomeNetwork",
    "local_ip": "192.168.1.14",
    "public_ip": "203.0.113.42"
  },
  "processes": [
    {
      "name": "symbion-kernel",
      "cpu": 2.1,
      "memory_mb": 24
    },
    {
      "name": "docker-containerd",
      "cpu": 0.8,
      "memory_mb": 156
    },
    {
      "name": "postgres",
      "cpu": 1.5,
      "memory_mb": 512
    }
  ]
}
```

### `POST /agents/{id}/shutdown`
**Description** : Extinction machine distante
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Shutdown command sent to eridwyn-Salon"
}
```

### `POST /agents/{id}/hibernate`
**Description** : Mise en hibernation
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Hibernate command sent"
}
```

### `POST /agents/{id}/reboot`
**Description** : Redémarrage machine distante
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Reboot command sent to eridwyn-Salon"
}
```

### `POST /v1/agents/{id}/reconnect`
**Description** : Demande de reconnexion de l'agent au kernel (force re-registration)
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "command_id": "cmd_abc123",
  "message": "Reconnect command sent to agent via kernel"
}
```

### `POST /agents/{id}/command`
**Description** : Exécution commande whitelistée
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "command": "systemctl status bluetooth"
}
```
**Response** :
```json
{
  "success": true,
  "output": "● bluetooth.service - Bluetooth service\n   Loaded: loaded...",
  "exit_code": 0
}
```

### `GET /agents/{id}/processes`
**Description** : Liste processus actifs de l'agent
**Auth** : JWT
**Response** :
```json
{
  "processes": [
    {
      "pid": 1234,
      "name": "symbion-agent",
      "cpu_percent": 2.1,
      "memory_mb": 24,
      "status": "running"
    },
    {
      "pid": 5678,
      "name": "nginx",
      "cpu_percent": 0.8,
      "memory_mb": 45,
      "status": "sleeping"
    }
  ],
  "total_count": 256
}
```

### `POST /agents/{id}/processes/{pid}/kill`
**Description** : Arrêt forcé d'un processus sur l'agent
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Process 1234 killed on eridwyn-Salon"
}
```

### `GET /agents/{id}/commands`
**Description** : Liste historique commandes envoyées à l'agent
**Auth** : JWT
**Response** :
```json
{
  "commands": [
    {
      "id": "cmd-123",
      "command": "systemctl status nginx",
      "status": "completed",
      "exit_code": 0,
      "created_at": 1699887200,
      "completed_at": 1699887202
    }
  ]
}
```

### `POST /agents/{id}/commands`
**Description** : Envoi commande asynchrone à l'agent
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "command": "apt update && apt upgrade -y"
}
```
**Response** :
```json
{
  "command_id": "cmd-456",
  "status": "pending",
  "message": "Command queued for execution"
}
```

### `GET /commands/{command_id}/status`
**Description** : Statut d'une commande asynchrone
**Auth** : JWT
**Response** :
```json
{
  "command_id": "cmd-456",
  "status": "running",
  "output": "Partial output...",
  "exit_code": null,
  "created_at": 1699887200,
  "started_at": 1699887201,
  "completed_at": null
}
```

### `POST /commands/{command_id}/cancel`
**Description** : Annulation d'une commande en cours
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Command cmd-456 cancelled"
}
```

### `GET /agents/{id}/metrics`
**Description** : Métriques détaillées (historique)
**Auth** : JWT
**Query** : `?period=1h|24h|7d`
**Response** :
```json
{
  "period": "1h",
  "samples": 60,
  "cpu": {
    "min": 10.2,
    "max": 45.7,
    "avg": 23.5,
    "current": 25.1
  },
  "memory": {
    "avg_percent": 52.3,
    "current_mb": 8192
  }
}
```

### `GET /hosts`
**Description** : Liste hosts/agents (alias de /agents)
**Auth** : JWT
**Response** : Identique à `GET /agents`

### `GET /hosts/{id}`
**Description** : Détails host spécifique (alias de /agents/{id})
**Auth** : JWT
**Response** : Identique à `GET /agents/{id}`

### `POST /wake`
**Description** : Wake-on-LAN (réveil machine)
**Auth** : JWT
**CSRF** : Requis
**Query** : `?host_id=eridwyn-Bureau`
**Response** :
```json
{
  "success": true,
  "message": "WOL magic packet sent to eridwyn-Bureau"
}
```

---

## 🎭 Context Engine (Modes Contextuels)

### `GET /context/current`
**Description** : Mode contextuel actuel
**Auth** : JWT
**Response** :
```json
{
  "mode": "intime",
  "reason": "weekend_auto_detection",
  "override_active": false,
  "override_expires_at": null,
  "auto_detection": {
    "is_weekend": true,
    "hour": 14,
    "ssid": "HomeNetwork"
  }
}
```

### `POST /context/override`
**Description** : Override manuel mode contextuel
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "mode": "cravate",
  "duration_minutes": 240,
  "reason": "Focus développement"
}
```
**Response** :
```json
{
  "success": true,
  "mode": "cravate",
  "expires_at": 1699901600,
  "message": "Mode override activated for 240 minutes"
}
```

### `POST /context/clear`
**Description** : Annulation override (retour auto-détection)
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "mode": "intime",
  "message": "Override cleared, auto-detection resumed"
}
```

### `GET /context/history`
**Description** : Historique transitions contextuelles
**Auth** : JWT
**Query** : `?period=24h|7d|30d`
**Response** :
```json
{
  "transitions": [
    {
      "from": "cravate",
      "to": "intime",
      "timestamp": 1699887200,
      "reason": "work_hours_ended",
      "auto_detected": true
    },
    {
      "from": "intime",
      "to": "cravate",
      "timestamp": 1699800800,
      "reason": "manual_override",
      "auto_detected": false
    }
  ],
  "total_transitions": 42
}
```

### `GET /context/stats`
**Description** : Statistiques utilisation modes
**Auth** : JWT
**Query** : `?period=7d|30d|90d`
**Response** :
```json
{
  "period": "7d",
  "mode_distribution": {
    "cravate": {
      "duration_hours": 45.2,
      "percentage": 26.8,
      "transitions_count": 12
    },
    "intime": {
      "duration_hours": 98.5,
      "percentage": 58.6,
      "transitions_count": 15
    },
    "neutre": {
      "duration_hours": 24.5,
      "percentage": 14.6,
      "transitions_count": 8
    }
  },
  "auto_detection_accuracy": 0.92
}
```

### `GET /context/patterns`
**Description** : Patterns détectés (apprentissage habitudes)
**Auth** : JWT
**Response** :
```json
{
  "patterns": [
    {
      "type": "work_hours",
      "description": "Cravate mode typically 9am-6pm weekdays",
      "confidence": 0.95,
      "occurrences": 42
    },
    {
      "type": "weekend_routine",
      "description": "Intime mode all day Saturday-Sunday",
      "confidence": 0.88,
      "occurrences": 8
    }
  ]
}
```

### `GET /context/productivity`
**Description** : Métriques productivité par contexte
**Auth** : JWT
**Query** : `?period=7d|30d`
**Response** :
```json
{
  "period": "7d",
  "by_mode": {
    "cravate": {
      "active_hours": 38.5,
      "tasks_completed": 127,
      "avg_focus_duration_min": 45.2
    },
    "intime": {
      "active_hours": 22.3,
      "tasks_completed": 48,
      "avg_focus_duration_min": 28.7
    }
  }
}
```

---

## 🌡️ Environment & IoT Sensors (F1)

> 🆕 **Feature F1 - Environment Monitoring** (November 2025): API REST pour capteurs IoT (ESP32 + BME280)
>
> - Endpoints HTTP pour consultation données environnementales
> - Support multi-room évolutif (chambre, salon, bureau...)
> - Auto-registration via MQTT (symbion/sensors/registration@v1)
> - History filtrage par période (hourly grouping)

### `GET /v1/environment/sensors`
**Description** : Liste tous les capteurs enregistrés
**Auth** : X-API-Key
**Response** :
```json
{
  "sensors": [
    {
      "sensor_id": "ESP32-CDE370",
      "room_id": "chambre",
      "sensor_type": "BME280",
      "location": "Bedroom ceiling",
      "registered_at": "2025-11-18T14:32:00Z",
      "last_seen": "2025-11-18T16:45:12Z",
      "signal_rssi": -42,
      "firmware_version": "1.0.0",
      "status": "online"
    }
  ],
  "count": 1,
  "online_count": 1
}
```

### `GET /v1/environment/sensors/{sensor_id}`
**Description** : Détails d'un capteur avec état environnemental actuel
**Auth** : X-API-Key
**Params** : `sensor_id` - Identifiant du capteur (ex: ESP32-CDE370)
**Response** :
```json
{
  "sensor": {
    "sensor_id": "ESP32-CDE370",
    "room_id": "chambre",
    "sensor_type": "BME280",
    "location": "Bedroom ceiling",
    "registered_at": "2025-11-18T14:32:00Z",
    "last_seen": "2025-11-18T16:45:12Z",
    "signal_rssi": -42,
    "firmware_version": "1.0.0",
    "status": "online"
  },
  "environment": {
    "room_id": "chambre",
    "current": {
      "temperature_c": 22.9,
      "humidity_pct": 79.6,
      "timestamp": "2025-11-18T16:45:12Z"
    },
    "status": "risk_mold",
    "alerts": [
      "High humidity detected (>70%). Risk of mold growth."
    ],
    "history": [
      {
        "temperature_c": 22.9,
        "humidity_pct": 79.6,
        "timestamp": "2025-11-18T16:45:12Z"
      }
    ],
    "avg_last_24h": {
      "temperature_c": 22.5,
      "humidity_pct": 75.2
    }
  }
}
```

### `GET /v1/environment/sensors/{sensor_id}/history`
**Description** : Historique des mesures d'un capteur
**Auth** : X-API-Key
**Params** : `sensor_id` - Identifiant du capteur
**Query** : `?hours=24` - Période en heures (défaut: 24h)
**Response** :
```json
{
  "room_id": "chambre",
  "current": {
    "temperature_c": 22.9,
    "humidity_pct": 79.6,
    "timestamp": "2025-11-18T16:45:12Z"
  },
  "status": "risk_mold",
  "alerts": [
    "High humidity detected (>70%). Risk of mold growth."
  ],
  "history": [
    {
      "temperature_c": 22.8,
      "humidity_pct": 78.9,
      "timestamp": "2025-11-18T15:45:12Z"
    },
    {
      "temperature_c": 22.7,
      "humidity_pct": 77.5,
      "timestamp": "2025-11-18T14:45:12Z"
    }
  ],
  "avg_last_24h": {
    "temperature_c": 22.5,
    "humidity_pct": 75.2
  }
}
```

### `DELETE /v1/environment/sensors/{sensor_id}`
**Description** : Désinscrire un capteur (suppression manuelle)
**Auth** : X-API-Key + CSRF
**Params** : `sensor_id` - Identifiant du capteur
**Response** : `204 No Content`

### `GET /v1/environment/{room_id}`
**Description** : État environnemental actuel d'une pièce (aggregation multi-sensors)
**Auth** : X-API-Key
**Params** : `room_id` - Identifiant de la pièce (ex: chambre, salon, bureau)
**Response** :
```json
{
  "room_id": "chambre",
  "current": {
    "temperature_c": 22.9,
    "humidity_pct": 79.6,
    "timestamp": "2025-11-18T16:45:12Z"
  },
  "status": "risk_mold",
  "alerts": [
    "High humidity detected (>70%). Risk of mold growth."
  ],
  "history": [
    {
      "temperature_c": 22.9,
      "humidity_pct": 79.6,
      "timestamp": "2025-11-18T16:45:12Z"
    }
  ],
  "avg_last_24h": {
    "temperature_c": 22.5,
    "humidity_pct": 75.2
  }
}
```

**Notes** :
- Si plusieurs capteurs dans la même pièce → sélection de la lecture la plus récente
- Statut calculé automatiquement par Decision Engine :
  - `normal` : Température 18-24°C, Humidité 40-60%
  - `humid` : Humidité 60-70%
  - `risk_mold` : Humidité >70% (risque moisissure)
  - `cold` : Température <18°C

### `GET /v1/environment/{room_id}/history`
**Description** : Historique des mesures pour une pièce (filtrées par période)
**Auth** : X-API-Key
**Params** : `room_id` - Identifiant de la pièce
**Query** : `?hours=24` - Période en heures (défaut: 24h)
**Response** :
```json
[
  {
    "temperature_c": 22.9,
    "humidity_pct": 79.6,
    "timestamp": "2025-11-18T16:45:12Z"
  },
  {
    "temperature_c": 22.8,
    "humidity_pct": 78.9,
    "timestamp": "2025-11-18T15:45:12Z"
  }
]
```

**Endpoints count** : +5 endpoints (sensors list, sensor detail, sensor history, room environment, room history)

---

## 📝 Notes/Memo (Plugin)

### `GET /ports/memo`
**Description** : Liste notes avec contexte
**Auth** : JWT
**Response** :
```json
{
  "notes": [
    {
      "id": "note-123",
      "content": "Acheter lait + pain",
      "context": "intime",
      "tags": ["courses"],
      "created_at": 1699887200,
      "updated_at": 1699887200
    }
  ]
}
```

### `POST /ports/memo`
**Description** : Création note (contexte auto-injecté)
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "content": "Rendez-vous client 15h",
  "tags": ["travail"]
  // context auto-injecté selon mode actif
}
```
**Response** :
```json
{
  "id": "note-456",
  "content": "Rendez-vous client 15h",
  "context": "cravate",
  "tags": ["travail"],
  "created_at": 1699887300
}
```

### `PUT /ports/memo/{id}`
**Description** : Modification note
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "content": "Rendez-vous client 16h (reporté)",
  "tags": ["travail", "urgent"]
}
```

### `DELETE /ports/memo/{id}`
**Description** : Suppression note
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Note deleted"
}
```

### `WebSocket /ws/notes/stream`
**Description** : Streaming temps réel des notes (pagination MQTT)
**Auth** : JWT (via query param `?token=...`)
**Protocol** : WebSocket
**Messages** :
```json
// Message note individuelle
{
  "type": "note",
  "data": {
    "id": "note-123",
    "content": "Acheter lait",
    "context": "intime",
    "tags": ["courses"],
    "created_at": 1699887200
  }
}

// Message fin de liste
{
  "type": "list_end",
  "total": 42
}

// Message erreur
{
  "type": "error",
  "message": "Failed to fetch notes"
}
```

### `GET /ports/{port_name}`
**Description** : Lecture générique d'un port (équivalent memo)
**Auth** : JWT
**Response** : Format dépend du plugin

### `POST /ports/{port_name}`
**Description** : Écriture générique sur un port
**Auth** : JWT
**CSRF** : Requis
**Request** : Format dépend du plugin

### `DELETE /ports/{port_name}/{id}`
**Description** : Suppression générique d'une entrée de port
**Auth** : JWT
**CSRF** : Requis

### `GET /ports`
**Description** : Liste tous les ports disponibles
**Auth** : JWT
**Response** :
```json
{
  "ports": [
    {
      "name": "memo",
      "plugin": "symbion-plugin-notes",
      "status": "active",
      "entry_count": 42
    }
  ]
}
```

---

## 🔌 Gestion Plugins

### `GET /plugins`
**Description** : Liste plugins actifs
**Auth** : JWT
**Response** :
```json
{
  "plugins": [
    {
      "name": "memo",
      "status": "running",
      "version": "0.1.0",
      "uptime_seconds": 86400
    }
  ]
}
```

### `POST /plugins/{name}/start`
**Description** : Démarrage plugin
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Plugin memo started"
}
```

### `POST /plugins/{name}/stop`
**Description** : Arrêt plugin
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Plugin memo stopped"
}
```

### `POST /plugins/{name}/restart`
**Description** : Redémarrage plugin sans coupure
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Plugin memo restarted"
}
```

---

## 🧠 Decision Engine (PR3)

### `POST /decision/evaluate`
**Description** : Évaluation action avec trust score
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "action": {
    "type": "agent_shutdown",
    "target": "eridwyn-Bureau",
    "reason": "inactivity_2h"
  },
  "context": {
    "mode": "intime",
    "time_of_day": "night",
    "presence": "absent"
  }
}
```
**Response** :
```json
{
  "decision": "auto_approve",
  "trust_score": 0.85,
  "impact_level": "M",
  "reasoning": "High trust + medium impact + absence confirmed",
  "action_id": "action-123"
}
```

### `POST /decision/validation/{id}/resolve`
**Description** : Approbation/refus intention
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "approved": true,
  "create_consent": true,
  "consent_duration_days": 90
}
```
**Response** :
```json
{
  "success": true,
  "action_executed": true,
  "consent_created": true,
  "consent_id": "consent-456"
}
```

### `POST /decision/override`
**Description** : Master override (force action - MFA requis)
**Auth** : JWT + MFA
**CSRF** : Requis
**Request** :
```json
{
  "action_id": "action-123",
  "mfa_code": "123456",
  "reason": "Emergency override"
}
```
**Response** :
```json
{
  "success": true,
  "message": "Action force-executed"
}
```

### `GET /decision/audit`
**Description** : Historique décisions
**Auth** : JWT
**Query** : `?period=7d&impact_level=M,H&approved=true`
**Response** :
```json
{
  "decisions": [
    {
      "id": "action-123",
      "action_type": "agent_shutdown",
      "impact_level": "M",
      "trust_score": 0.85,
      "decision": "auto_approve",
      "timestamp": 1699887200
    }
  ]
}
```

### `GET /decision/metrics`
**Description** : Métriques Decision Engine
**Auth** : JWT
**Response** :
```json
{
  "decisions_total": 1247,
  "decisions_approved": 1089,
  "decisions_blocked": 158,
  "auto_approval_rate": 0.87,
  "avg_trust_score": 0.82,
  "validations_pending": 3,
  "overrides_active": 1
}
```

### `GET /decision/stats`
**Description** : Statistiques détaillées décisions
**Auth** : JWT
**Query** : `?period=7d|30d`
**Response** :
```json
{
  "period": "7d",
  "by_action_type": {
    "agent_shutdown": {
      "total": 42,
      "approved": 38,
      "blocked": 4,
      "avg_trust_score": 0.85
    },
    "context_override": {
      "total": 18,
      "approved": 15,
      "blocked": 3,
      "avg_trust_score": 0.78
    }
  },
  "by_impact_level": {
    "L": { "count": 25, "approval_rate": 0.96 },
    "M": { "count": 28, "approval_rate": 0.85 },
    "H": { "count": 7, "approval_rate": 0.57 }
  }
}
```

### `GET /decision/validations/pending`
**Description** : Validations en attente approbation utilisateur
**Auth** : JWT
**Response** :
```json
{
  "pending": [
    {
      "id": "validation-789",
      "action_type": "purchase_groceries",
      "impact_level": "H",
      "reason": "Automated grocery order",
      "created_at": 1699887200,
      "expires_at": 1699889000
    }
  ],
  "total": 3
}
```

### `GET /decision/validations/expired`
**Description** : Liste validations expirées
**Auth** : JWT
**Response** :
```json
{
  "expired": [
    {
      "id": "validation-456",
      "action_type": "agent_shutdown",
      "expired_at": 1699887000,
      "status": "timeout"
    }
  ],
  "total": 12
}
```

### `DELETE /decision/validations/expired`
**Description** : Nettoyage validations expirées
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "deleted_count": 12
}
```

### `DELETE /decision/validation/{id}`
**Description** : Annulation validation spécifique
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Validation cancelled"
}
```

### `GET /decision/overrides/active`
**Description** : Overrides actifs (force-execution)
**Auth** : JWT
**Response** :
```json
{
  "overrides": [
    {
      "id": "override-123",
      "action_id": "action-789",
      "reason": "Emergency override",
      "created_at": 1699887200,
      "created_by": "admin"
    }
  ],
  "total": 1
}
```

### `DELETE /decision/override/{id}`
**Description** : Révocation override
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Override revoked"
}
```

### `GET /decision/config`
**Description** : Configuration Decision Engine
**Auth** : JWT (admin)
**Response** :
```json
{
  "trust_thresholds": {
    "auto_approve_low_impact": 0.7,
    "auto_approve_medium_impact": 0.85,
    "auto_approve_high_impact": 0.95
  },
  "validation_timeout_seconds": 1800,
  "max_active_overrides": 5
}
```

### `GET /decision/agent-health`
**Description** : Score santé agents pour trust calculation
**Auth** : JWT
**Response** :
```json
{
  "agents": [
    {
      "agent_id": "eridwyn-Salon",
      "health_score": 0.92,
      "factors": {
        "telemetry_freshness": 0.95,
        "network_stability": 0.88,
        "sensor_consistency": 0.93
      },
      "last_check": 1699887200
    }
  ]
}
```

---

## ⚙️ Système & Configuration

### `GET /contracts`
**Description** : Contrats MQTT enregistrés
**Auth** : JWT
**Response** :
```json
{
  "contracts": [
    {
      "topic": "symbion/agents/heartbeat@v1",
      "version": 1,
      "schema": {
        "type": "object",
        "required": ["agent_id", "timestamp"]
      }
    }
  ]
}
```

### `GET /contracts/{name}`
**Description** : Détails d'un contrat MQTT spécifique
**Auth** : JWT
**Response** :
```json
{
  "name": "heartbeat",
  "topic": "symbion/agents/heartbeat@v1",
  "version": 1,
  "schema": {
    "type": "object",
    "required": ["agent_id", "timestamp"],
    "properties": {
      "agent_id": { "type": "string" },
      "timestamp": { "type": "integer" }
    }
  }
}
```

---

## 🔍 Recherche et Filtrage

La plupart des endpoints `GET` supportent des query params de filtrage :

**Exemples** :
```bash
# Agents par statut
GET /agents?status=online

# Notes par contexte et tags
GET /ports/memo?context=cravate&tags=travail,urgent

# Décisions par niveau impact
GET /decision/audit?impact_level=H&period=30d

# Utilisateurs admin uniquement
GET /users?is_admin=true
```

---

## 📈 Metrics & Observability (PR4)

**Public Endpoints** (pas d'authentification requise - pour outils monitoring)

### GET /metrics

**Description** : Endpoint de scraping Prometheus (exposition format)

**Format** : `text/plain` (Prometheus exposition format)

**Total métriques** : 36

**Catégories** :
- **Decision Engine** (20 metrics) : decisions_total, decisions_approved, decisions_blocked, guards_passed/blocked, validations_created/pending/approved, overrides_active, audit_records, agents_total/online/active/degraded/offline
- **MQTT** (4 metrics) : mqtt_connected (1=connected, 0=disconnected), mqtt_reconnects_total, mqtt_messages_per_minute, mqtt_messages_total
- **Agents** (3 metrics) : kernel_agents_total, kernel_agents_online, kernel_agents_offline
- **Context Engine** (2 metrics) : context_mode (0=neutre, 1=cravate, 2=intime), context_confidence (0.0-1.0)
- **Plugins** (3 metrics) : plugins_total, plugins_running, plugins_failed
- **Kernel** (4 metrics) : kernel_uptime_seconds, kernel_memory_usage_mb, contracts_loaded

**Exemple réponse** :
```
# HELP symbion_decision_total Total number of decisions made
# TYPE symbion_decision_total counter
symbion_decision_total 42
# HELP symbion_mqtt_connected MQTT broker connection status
# TYPE symbion_mqtt_connected gauge
symbion_mqtt_connected 1
# HELP symbion_kernel_agents_online Number of agents currently online
# TYPE symbion_kernel_agents_online gauge
symbion_kernel_agents_online 3
```

**Fichier source** : `symbion-kernel/src/http.rs:2579-2708`

---

### GET /v1/metrics/agents

**Description** : Métriques détaillées par agent (format JSON)

**Format** : `application/json` (array)

**Authentification** : ❌ Public

**Réponse** : Array d'objets `AgentMetrics`

**Champs** :
- `agent_id` (string) - Identifiant unique agent (MAC sans colons)
- `hostname` (string) - Nom d'hôte
- `status` (string) - Statut ("online", "offline")
- `last_seen` (integer) - Timestamp Unix dernière activité
- `uptime_seconds` (integer) - Temps de fonctionnement
- `cpu` (object) :
  - `percent` (float) - Utilisation CPU %
  - `load_avg` (array[float]) - Charge moyenne [1min, 5min, 15min]
  - `core_count` (integer) - Nombre de cœurs
- `memory` (object) :
  - `total_mb` (integer) - RAM totale
  - `used_mb` (integer) - RAM utilisée
  - `available_mb` (integer) - RAM disponible
  - `percent_used` (float) - % utilisation
- `disk` (array[object]) :
  - `path` (string) - Point de montage
  - `total_gb` (float) - Espace total
  - `used_gb` (float) - Espace utilisé
  - `free_gb` (float) - Espace libre
  - `percent_used` (float) - % utilisation
- `network` (array[object]) :
  - `name` (string) - Nom interface
  - `bytes_sent` (integer) - Octets envoyés
  - `bytes_recv` (integer) - Octets reçus
  - `is_up` (boolean) - Interface active
- `processes` (object) :
  - `total_count` (integer) - Nombre total processus
  - `running_count` (integer) - Processus en cours

**Exemple réponse** :
```json
[
  {
    "agent_id": "7070fc0481d8",
    "hostname": "eridwyn-Salon",
    "status": "online",
    "last_seen": 1763207035,
    "uptime_seconds": 75900,
    "cpu": {
      "percent": 10.17,
      "load_avg": [1.18, 0.85, 0.72],
      "core_count": 16
    },
    "memory": {
      "total_mb": 27803,
      "used_mb": 2460,
      "available_mb": 25342,
      "percent_used": 8.85
    },
    "disk": [
      {
        "path": "/",
        "total_gb": 937.0,
        "used_gb": 60.0,
        "free_gb": 830.0,
        "percent_used": 7.0
      }
    ],
    "network": [],
    "processes": {
      "total_count": 946,
      "running_count": 17
    }
  }
]
```

**Fichier source** : `symbion-kernel/src/http.rs:2308-2442`

---

### GET /v1/metrics/system

**Description** : Vue d'ensemble métriques kernel (format JSON)

**Format** : `application/json` (object)

**Authentification** : ❌ Public

**Réponse** : Objet `SystemMetrics`

**Champs** :
- `kernel` (object) :
  - `uptime_seconds` (integer) - Temps de fonctionnement
  - `memory_usage_mb` (float) - Utilisation mémoire kernel
  - `contracts_loaded` (integer) - Contrats MQTT chargés
- `mqtt` (object) :
  - `status` (string) - "connected", "disconnected", "reconnecting"
  - `reconnects_total` (integer) - Nombre de reconnexions
  - `messages_per_minute` (float) - Débit messages
  - `messages_total` (integer) - Total messages reçus
- `agents` (object) :
  - `total` (integer) - Nombre total agents
  - `online` (integer) - Agents en ligne
  - `offline` (integer) - Agents hors ligne
- `plugins` (object) :
  - `total` (integer) - Nombre total plugins
  - `running` (integer) - Plugins actifs
  - `failed` (integer) - Plugins en erreur
- `context` (object) :
  - `current_mode` (string) - "neutre", "cravate", "intime"
  - `confidence` (float) - Confiance détection (0.0-1.0)
- `decision_engine` (object) :
  - `decisions_total` (integer) - Décisions totales
  - `decisions_approved` (integer) - Décisions approuvées
  - `decisions_blocked` (integer) - Décisions bloquées
  - `validations_pending` (integer) - Validations en attente
  - `overrides_active` (integer) - Overrides actifs

**Exemple réponse** :
```json
{
  "kernel": {
    "uptime_seconds": 92,
    "memory_usage_mb": 13.54,
    "contracts_loaded": 0
  },
  "mqtt": {
    "status": "connected",
    "reconnects_total": 0,
    "messages_per_minute": 4.0,
    "messages_total": 6
  },
  "agents": {
    "total": 3,
    "online": 2,
    "offline": 1
  },
  "plugins": {
    "total": 1,
    "running": 1,
    "failed": 0
  },
  "context": {
    "current_mode": "intime",
    "confidence": 0.9
  },
  "decision_engine": {
    "decisions_total": 0,
    "decisions_approved": 0,
    "decisions_blocked": 0,
    "validations_pending": 0,
    "overrides_active": 0
  }
}
```

**Fichier source** : `symbion-kernel/src/http.rs:2444-2573`

---

## 📊 Codes de Statut HTTP

| Code | Signification | Exemple |
|------|---------------|---------|
| 200 | OK | GET /agents |
| 201 | Created | POST /users |
| 204 | No Content | DELETE /users/{id} |
| 400 | Bad Request | JSON invalide |
| 401 | Unauthorized | Token JWT manquant/invalide |
| 403 | Forbidden | Token valide mais droits insuffisants |
| 404 | Not Found | Ressource inexistante |
| 409 | Conflict | Username déjà existant |
| 422 | Unprocessable Entity | Validation échouée |
| 429 | Too Many Requests | Rate limit dépassé |
| 500 | Internal Server Error | Erreur serveur |

---

## 📊 Performance & Latency

Voir **[docs/PERFORMANCE.md](../PERFORMANCE.md)** pour métriques détaillées :

**Latence Typique** (P50/P95/P99) :
- `GET /health` : 2/5/8 ms
- `GET /agents` : 12/25/40 ms
- `POST /login` : 180/250/320 ms (bcrypt cost 12)
- `GET /notes` : 45/90/150 ms (MQTT plugin roundtrip)
- `POST /agents/:id/command` : 25/60/100 ms

**Throughput** :
- Rate limit : 5 req/sec per IP (burst 10)
- Max sustainable : ~850 req/sec (4-core CPU limit)
- Beyond limit : HTTP 429 + `Retry-After` header

**Response Sizes** :
- Compression : gzip level 6 (auto > 1KB)
- Typical : 50B (/health) to 50KB (/notes large list)
- Headers : ~200-300 bytes overhead

---

## 🔗 Documentation Connexe

### API Security & Authentication
- **[authentication.md](authentication.md)** - JWT, MFA, WebAuthn passkeys
- **[security.md](security.md)** - CSRF, Rate Limiting, TLS 1.3

### Communication Patterns
- **[../mqtt/topics.md](../mqtt/topics.md)** - MQTT topics triggered by API
  - `POST /agents/:id/command` → `symbion/agents/command@v1`
  - `POST /notes` → `symbion/notes/command@v1`
- **[../mqtt/flows.md](../mqtt/flows.md)** - End-to-end workflows

### Architecture & Infrastructure
- **[../architecture/SYSTEM_OVERVIEW.md](../architecture/SYSTEM_OVERVIEW.md)** - System overview
- **[../DEPLOYMENT.md](../DEPLOYMENT.md)** - Production deployment guide
- **[../TROUBLESHOOTING.md](../TROUBLESHOOTING.md)** - API error resolution
- **[../PERFORMANCE.md](../PERFORMANCE.md)** - Benchmarks & profiling

### Development
- **[../CODE_STANDARDS.md](../CODE_STANDARDS.md)** - Coding conventions
- **[../QUICK_REFERENCE.md](../QUICK_REFERENCE.md)** - Cheat sheet

---

## 💡 Best Practices

### Error Handling Patterns

**Always check status code** :
```javascript
const response = await fetch('/agents', {
  headers: { 'Authorization': `Bearer ${token}` }
});

if (!response.ok) {
  const error = await response.json();
  console.error(`API Error ${response.status}:`, error.message);

  // Handle specific errors
  switch (response.status) {
    case 401: // Redirect to login
      window.location.href = '/login';
      break;
    case 429: // Rate limited, retry after delay
      const retryAfter = response.headers.get('Retry-After');
      setTimeout(() => fetchAgents(), (retryAfter || 60) * 1000);
      break;
    case 500: // Server error, alert user
      alert('Server error, please try again later');
      break;
  }
  return;
}

const data = await response.json();
```

**Error Response Format** (standard) :
```json
{
  "error": "Unauthorized",
  "message": "JWT token expired",
  "timestamp": 1699887200,
  "path": "/agents",
  "request_id": "req-abc123"
}
```

### Authentication Flow

```javascript
// 1. Login
const loginRes = await fetch('/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ username: 'admin', password: 'pass' })
});
const { token } = await loginRes.json();

// 2. Store token (secure)
localStorage.setItem('jwt_token', token);  // Or sessionStorage

// 3. Use token for authenticated requests
const agentsRes = await fetch('/agents', {
  headers: { 'Authorization': `Bearer ${token}` }
});

// 4. Handle token expiration (auto-refresh or re-login)
if (agentsRes.status === 401) {
  localStorage.removeItem('jwt_token');
  window.location.href = '/login';
}
```

### Rate Limiting Handling

```javascript
async function fetchWithRetry(url, options = {}, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    const res = await fetch(url, options);

    if (res.status !== 429) return res;

    // Rate limited, wait and retry
    const retryAfter = parseInt(res.headers.get('Retry-After') || '60');
    console.warn(`Rate limited, retrying after ${retryAfter}s...`);
    await new Promise(resolve => setTimeout(resolve, retryAfter * 1000));
  }

  throw new Error('Max retries exceeded');
}
```

### Pagination Best Practices

```javascript
// For large datasets (notes, agents with many items)
async function fetchAllNotes() {
  let offset = 0;
  const limit = 50;  // Reasonable chunk size
  let allNotes = [];

  while (true) {
    const res = await fetch(`/notes?offset=${offset}&limit=${limit}`);
    const notes = await res.json();

    if (notes.length === 0) break;  // No more data

    allNotes = allNotes.concat(notes);
    offset += limit;

    // Optional: Progress feedback
    console.log(`Loaded ${allNotes.length} notes...`);
  }

  return allNotes;
}
```

### WebSocket for Real-Time Updates

```javascript
// Alternative to polling: Use MQTT WebSocket for live updates
import mqtt from 'mqtt';

const client = mqtt.connect('wss://symbion.local:9001');

client.on('connect', () => {
  // Subscribe to dashboard updates
  client.subscribe('symbion/dashboard/agents@v1');
  client.subscribe('symbion/dashboard/health@v1');
});

client.on('message', (topic, payload) => {
  const data = JSON.parse(payload);

  switch (topic) {
    case 'symbion/dashboard/agents@v1':
      updateAgentsUI(data);  // Real-time agent status
      break;
    case 'symbion/dashboard/health@v1':
      updateHealthUI(data);  // Real-time system metrics
      break;
  }
});

// Much more efficient than polling /agents every second
```

---

---

## 🔌 Plugin API - Service Discovery

> **Architecture** : Reverse Proxy HTTP → Unix Domain Sockets
> **Prefix** : `/v1/plugin-api/{plugin_name}/{route}`
> **Auth** : JWT requis pour tous les endpoints
> **Format** : JSON

Les plugins s'enregistrent dynamiquement au démarrage via Service Discovery (`POST /v1/plugins/register`) et exposent leurs routes HTTP via Unix sockets. Le kernel agit comme reverse proxy authentifié.

### Plugins Disponibles

#### **sensors** (F1 Environment Monitoring)
Routes exposées : 2

##### `GET /v1/plugin-api/sensors/sensors`
**Description** : Liste tous les capteurs d'environnement enregistrés
**Auth** : JWT
**Response** :
```json
{
  "count": 1,
  "sensors": [
    {
      "room_id": "chambre",
      "device_id": "esp32-bme280-001",
      "last_seen": "2025-11-26T22:30:00Z"
    }
  ]
}
```

##### `GET /v1/plugin-api/sensors/environment/:room_id`
**Description** : Données environnement pour une pièce spécifique (parameterized route)
**Auth** : JWT
**Path Params** : `room_id` (ex: "chambre", "bureau", "salon")
**Response** :
```json
{
  "room_id": "chambre",
  "current": {
    "temperature_c": 21.5,
    "humidity_pct": 45.0,
    "timestamp": "2025-11-26T22:35:12Z"
  },
  "status": "comfort",
  "thresholds": {
    "temp_min": 18.0,
    "temp_max": 24.0,
    "humidity_min": 30.0,
    "humidity_max": 60.0
  },
  "alerts": []
}
```
**Errors** :
- `404` si `room_id` inexistant

---

#### **notes** (Notes/Memo Management)
Routes exposées : 2

##### `GET /v1/plugin-api/notes/notes`
**Description** : Liste toutes les notes (avec MQTT streaming pagination)
**Auth** : JWT
**Response** :
```json
{
  "notes": [
    {
      "id": "note_abc123",
      "title": "Réunion projet Symbion",
      "content": "Discussion architecture Service Discovery...",
      "tags": ["project", "architecture"],
      "created_at": "2025-11-26T14:20:00Z",
      "updated_at": "2025-11-26T18:45:00Z"
    }
  ],
  "total": 42
}
```

##### `GET /v1/plugin-api/notes/notes/:id`
**Description** : Récupération d'une note spécifique (parameterized route)
**Auth** : JWT
**Path Params** : `id` (UUID de la note)
**Response** :
```json
{
  "id": "note_abc123",
  "title": "Réunion projet Symbion",
  "content": "Discussion architecture Service Discovery...",
  "tags": ["project", "architecture"],
  "created_at": "2025-11-26T14:20:00Z",
  "updated_at": "2025-11-26T18:45:00Z"
}
```
**Errors** :
- `404` si note inexistante

---

#### **notifications-manager** (F4 Notifications)
Routes exposées : 4

##### `GET /v1/plugin-api/notifications/notifications`
**Description** : Liste toutes les notifications
**Auth** : JWT
**Response** :
```json
{
  "notifications": [
    {
      "id": "notif_001",
      "priority": "P2",
      "title": "Alerte température chambre",
      "body": "Température descendue à 16°C",
      "source": "sensors-manager",
      "timestamp": 1732664400,
      "acknowledged": false,
      "actions": ["dismiss", "acknowledge"]
    }
  ],
  "total": 5,
  "unread": 2
}
```

##### `POST /v1/plugin-api/notifications/notifications/send`
**Description** : Envoi d'une nouvelle notification (MQTT + FCM)
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "priority": "P1",
  "title": "Action requise",
  "body": "Validation manuelle nécessaire pour automation",
  "source": "decision-engine",
  "actions": ["approve", "reject"]
}
```
**Response** :
```json
{
  "id": "notif_002",
  "status": "sent",
  "timestamp": 1732664500
}
```

##### `POST /v1/plugin-api/notifications/notifications/acknowledge`
**Description** : Marquage notification comme lue
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "notification_id": "notif_001"
}
```
**Response** :
```json
{
  "success": true,
  "notification_id": "notif_001",
  "acknowledged_at": 1732664600
}
```

##### `POST /v1/plugin-api/notifications/fcm/register`
**Description** : Enregistrement d'un token Firebase Cloud Messaging (mobile)
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "fcm_token": "dXA8G...",
  "device_type": "android",
  "device_name": "Pixel 7"
}
```
**Response** :
```json
{
  "success": true,
  "registered_at": 1732664700
}
```

---

### Service Discovery Flow

```
1. Plugin Startup
   ├─ Plugin crée Unix socket (/tmp/symbion-plugin-{name}.sock)
   ├─ Plugin lance serveur HTTP sur socket
   └─ Plugin envoie POST /v1/plugins/register au kernel

2. Registration Payload
   {
     "name": "sensors",
     "version": "0.1.0",
     "socket_path": "/tmp/symbion-plugin-sensors.sock",
     "routes": ["/sensors", "/environment/:room_id"],
     "contracts": ["environment.data@v1", "sensors.health@v1"]
   }

3. Kernel Processing
   ├─ Valide manifest et socket
   ├─ Enregistre routes dans router dynamique
   ├─ Map /v1/plugin-api/{name}/* → socket Unix
   └─ Health checks périodiques (30s interval)

4. Client Request Flow
   Client → Kernel HTTPS :8443/v1/plugin-api/sensors/environment/chambre
           ↓ JWT validation
           ↓ CSRF check (si POST/PUT/DELETE)
           ↓ Rate limiting
   Kernel → Plugin Unix Socket /tmp/symbion-plugin-sensors.sock
           ↓ HTTP reverse proxy
   Plugin → Business Logic + Response
           ↓
   Kernel ← JSON response
           ↓
   Client ← HTTPS response
```

### Parameterized Routes

Service Discovery supporte les routes paramétrées avec pattern `:param` :

| Pattern | Example | Extracted Params |
|---------|---------|------------------|
| `/environment/:room_id` | `/environment/chambre` | `room_id = "chambre"` |
| `/notes/:id` | `/notes/abc123` | `id = "abc123"` |
| `/devices/:mac/config` | `/devices/AA:BB:CC:DD/config` | `mac = "AA:BB:CC:DD"` |

Le kernel effectue le matching et passe les params via headers HTTP au plugin :
```
X-Param-room_id: chambre
X-Param-id: abc123
```

### Testing

E2E tests disponibles : `/home/eridwyn/RustroverProjects/NewSymbion/scripts/test-service-discovery-e2e.sh`

**Tests Coverage** :
- ✅ Plugin registry listing (`GET /v1/plugins`)
- ✅ Route registration verification
- ✅ Static routes (`/sensors`, `/notes`)
- ✅ Parameterized routes (`/environment/:room_id`, `/notes/:id`)
- ✅ Invalid routes (404 handling)
- ✅ Multi-segment path matching

**Test Results** : 11/12 passing (92% success rate)

---

## 📊 Récapitulatif Endpoints par Catégorie

| Catégorie | Endpoints | Status |
|-----------|-----------|---------|
| **Public (sans auth)** | 3 | ✅ |
| **Metrics** | 3 | ✅ |
| **Authentification** | 6 | ✅ |
| **MFA** | 3 | ✅ |
| **WebAuthn** | 5 | ✅ |
| **Utilisateurs** | 3 | ✅ |
| **Agents** | 11 | ✅ |
| **Commandes** | 4 | ✅ |
| **Context Engine** | 8 | ✅ |
| **Notes/Memo** | 3 | ✅ |
| **Ports** | 4 | ✅ |
| **WebSocket** | 1 | ✅ |
| **Plugins (Core)** | 4 | ✅ |
| **Plugin API (Service Discovery)** | 8 | ✅ |
| **Decision Engine** | 13 | ✅ |
| **Système & Config** | 2 | ✅ |
| **Automations Engine** | 10 | ✅ 🆕 |
| **Modes Dynamiques** | 6 | ✅ 🆕 |
| **Notifications Système** | 10 | ✅ 🆕 |
| **TOTAL** | **107** | **✅ 100% sync** |

### Changements Effectués
- ✅ **7 path mismatches corrigés**:
  - `/system/status` → `/system/health`
  - `/csrf-token` → `/auth/csrf/nonce`
  - `/jwt/verify` → `/auth/verify`
  - `/users/*` → `/v1/users/*`
  - `/decision/pending` → `/decision/validations/pending`

- ❌ **19 phantom endpoints retirés**:
  - Auth: `/sessions`, `/sessions/{id}`, `/refresh`
  - Users: `/users/{id}` (PUT), `/users/{id}/password`
  - Agents: `/agents/{id}` (DELETE), `/agents/{id}/logs`, `/agents/{id}/restart`, `/agents/{id}/capabilities`
  - Decision: `/decision/consents`, `/decision/consents/{id}`, `/decision/trust-score`
  - System: `/config` (GET/PUT)

- ✅ **37 nouveaux endpoints documentés**:
  - Context Engine: `/context/history`, `/context/stats`, `/context/patterns`, `/context/productivity`
  - Decision Engine: `/decision/metrics`, `/decision/stats`, `/decision/config`, `/decision/agent-health`, `/decision/validations/*`, `/decision/overrides/*`
  - Agents: `/agents/{id}/processes`, `/agents/{id}/reboot`, `/agents/{id}/processes/{pid}/kill`, `/agents/{id}/commands`
  - Commands: `/commands/{id}/status`, `/commands/{id}/cancel`
  - Auth: `/auth/session`, `/auth/reload`
  - Ports: `/ports`, `/ports/{port_name}`
  - WebSocket: `/ws/notes/stream`

---

## 🤖 Automations Engine (PR5 - Janvier 2026)

> 🆕 **Feature Automations** (Janvier 2026): Moteur d'automatisations basé sur événements avec triggers, conditions et actions.
>
> **Source**: `symbion-kernel/src/automations.rs`, `symbion-kernel/src/automations_http.rs`

### `GET /v1/automations`
**Description** : Liste toutes les automations
**Auth** : JWT
**Response** :
```json
{
  "automations": [
    {
      "id": "auto-123",
      "name": "Éteindre PC Salon si inactif",
      "enabled": true,
      "trigger": { "type": "sensor_threshold", "sensor_id": "..." },
      "conditions": [],
      "actions": [{ "type": "agent_command", "agent_id": "...", "command_type": "shutdown" }],
      "cooldown_seconds": 3600,
      "last_executed_at": "2026-01-31T22:00:00Z"
    }
  ],
  "count": 5,
  "enabled_count": 3
}
```

### `GET /v1/automations/{id}`
**Description** : Détails d'une automation spécifique
**Auth** : JWT
**Response** : Objet Automation complet

### `GET /v1/automations/schema`
**Description** : Schéma pour le rule builder (triggers, conditions, actions disponibles)
**Auth** : JWT
**Response** :
```json
{
  "triggers": [
    { "type": "sensor_threshold", "label": "Seuil capteur", "params": [...] },
    { "type": "schedule", "label": "Horaire", "params": [...] },
    { "type": "mode_change", "label": "Changement de mode", "params": [...] }
  ],
  "conditions": [...],
  "actions": [...],
  "dynamic_values": {
    "agents": [...],
    "rooms": [...],
    "sensors": [...],
    "modes": [...]
  }
}
```

### `GET /v1/automations/history`
**Description** : Historique d'exécution des automations
**Auth** : JWT
**Query** : `?limit=50`
**Response** :
```json
[
  {
    "automation_id": "auto-123",
    "automation_name": "Éteindre PC Salon",
    "executed_at": "2026-01-31T22:00:00Z",
    "trigger_event": "sensor_threshold",
    "conditions_met": true,
    "success": true,
    "actions_executed": [...]
  }
]
```

### `POST /v1/automations`
**Description** : Création d'une nouvelle automation
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "name": "Alerte humidité chambre",
  "trigger": { "type": "sensor_threshold", "sensor_id": "esp32-chambre", "threshold": 70 },
  "conditions": [],
  "actions": [{ "type": "send_notification", "title": "Humidité élevée", "body": "..." }],
  "cooldown_seconds": 1800
}
```
**Response** : `201 Created` avec l'automation créée

### `PUT /v1/automations/{id}`
**Description** : Mise à jour d'une automation
**Auth** : JWT
**CSRF** : Requis
**Response** : Automation mise à jour

### `DELETE /v1/automations/{id}`
**Description** : Suppression (soft-delete) d'une automation
**Auth** : JWT
**CSRF** : Requis
**Response** : `204 No Content`

### `PATCH /v1/automations/{id}/enable`
**Description** : Activer/Désactiver une automation
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "enabled": true
}
```
**Response** : Automation avec statut mis à jour

### `POST /v1/automations/{id}/test`
**Description** : Test dry-run (preview sans exécution)
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "automation_id": "auto-123",
  "automation_name": "...",
  "would_execute": true,
  "cooldown": null,
  "actions_preview": ["Send notification: Humidité élevée", "..."]
}
```

### `POST /v1/automations/{id}/run`
**Description** : Exécution manuelle d'une automation
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "automation_id": "auto-123",
  "automation_name": "...",
  "executed": true,
  "success": true,
  "actions_count": 2,
  "actions": [
    { "action_type": "send_notification", "success": true, "duration_ms": 45 }
  ],
  "trust_score": 0.85,
  "decision_outcome": "auto_approve"
}
```

---

## 🎨 Modes Contextuels (PR5 - Janvier 2026)

> 🆕 **Gestion Modes Dynamiques** (Janvier 2026): Création et gestion de modes personnalisés au-delà de Cravate/Intime/Neutre.
>
> **Source**: `symbion-kernel/src/http.rs`

### `GET /v1/modes`
**Description** : Liste tous les modes disponibles (prédéfinis + personnalisés)
**Auth** : JWT
**Response** :
```json
{
  "modes": [
    { "id": "cravate", "slug": "cravate", "name": "Cravate", "icon": "👔", "theme": {...}, "is_builtin": true },
    { "id": "intime", "slug": "intime", "name": "Intime", "icon": "🏡", "theme": {...}, "is_builtin": true },
    { "id": "custom-focus", "slug": "focus", "name": "Focus Profond", "icon": "🎯", "theme": {...}, "is_builtin": false }
  ]
}
```

### `GET /v1/modes/{id}`
**Description** : Détails d'un mode spécifique
**Auth** : JWT
**Response** : Objet Mode complet

### `POST /v1/modes`
**Description** : Création d'un nouveau mode personnalisé
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "name": "Gaming",
  "icon": "🎮",
  "theme": { "primary": "#8b5cf6", "bg": "#1e1b4b", "accent": "#a78bfa" }
}
```
**Response** : `201 Created` avec mode créé

### `PUT /v1/modes/{id}`
**Description** : Mise à jour d'un mode
**Auth** : JWT
**CSRF** : Requis
**Response** : Mode mis à jour

### `DELETE /v1/modes/{id}`
**Description** : Suppression d'un mode personnalisé (modes builtin non supprimables)
**Auth** : JWT
**CSRF** : Requis
**Response** : `204 No Content`

### `GET /v1/schedule/current`
**Description** : Mode actuel selon le planning horaire
**Auth** : JWT
**Response** :
```json
{
  "current_mode": "cravate",
  "scheduled_until": "2026-02-01T18:00:00Z",
  "next_mode": "intime"
}
```

### `PUT /v1/schedule/default`
**Description** : Définir le mode par défaut du planning
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "default_mode": "veille"
}
```
**Response** : Confirmation

---

## 🔔 Notifications Système (PR5 - Janvier 2026)

> 🆕 **Système Notifications Centralisé** (Janvier 2026): Gestion des notifications, FCM tokens, et configurations par type.
>
> **Source**: `symbion-kernel/src/notifications.rs`, `symbion-kernel/src/http.rs`

### `GET /v1/notifications`
**Description** : Liste toutes les notifications
**Auth** : JWT
**Response** :
```json
{
  "notifications": [
    {
      "id": "notif-123",
      "title": "Humidité chambre élevée",
      "body": "79% - Risque moisissure",
      "priority": "P2",
      "source": "automation",
      "timestamp": "2026-02-01T15:30:00Z",
      "acknowledged": false
    }
  ],
  "total": 15,
  "unread": 3
}
```

### `GET /v1/notifications/active`
**Description** : Notifications non-acquittées uniquement
**Auth** : JWT
**Response** : Liste filtrée

### `POST /v1/notifications`
**Description** : Envoi d'une nouvelle notification
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "title": "Test notification",
  "body": "Contenu du message",
  "priority": "P2",
  "source": "manual"
}
```
**Response** : Notification créée

### `POST /v1/notifications/{id}/acknowledge`
**Description** : Marquer notification comme lue
**Auth** : JWT
**CSRF** : Requis
**Response** : Notification mise à jour

### `DELETE /v1/notifications/{id}`
**Description** : Supprimer une notification
**Auth** : JWT
**CSRF** : Requis
**Response** : `204 No Content`

### `GET /v1/notifications/tokens`
**Description** : Liste des tokens FCM enregistrés
**Auth** : JWT
**Response** :
```json
{
  "tokens": [
    { "user_id": "admin", "device_name": "Pixel 7", "registered_at": "..." }
  ]
}
```

### `POST /v1/notifications/tokens`
**Description** : Enregistrer un nouveau token FCM
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "token": "fcm-token-abc...",
  "device_name": "iPhone 14"
}
```
**Response** : Confirmation

### `GET /v1/notification-types`
**Description** : Liste des types de notifications configurables
**Auth** : JWT
**Response** :
```json
{
  "types": [
    { "type_id": "humidity_alert", "enabled": true, "priority": "P2", "channels": ["push", "mqtt"] }
  ]
}
```

### `GET /v1/notification-types/{type_id}`
**Description** : Configuration d'un type de notification
**Auth** : JWT
**Response** : Configuration complète

### `PUT /v1/notification-types/{type_id}`
**Description** : Mettre à jour configuration d'un type
**Auth** : JWT
**CSRF** : Requis
**Request** :
```json
{
  "enabled": true,
  "priority": "P1",
  "channels": ["push", "mqtt", "email"]
}
```
**Response** : Configuration mise à jour

---

**Dernière mise à jour** : 1 Février 2026
**Fichier source** : `symbion-kernel/src/http.rs` (103 routes) + Service Discovery
**Total endpoints documentés** : 107 (99 kernel + 8 plugin API)
**Synchronisation** : ✅ 100% (tous les endpoints implémentés sont documentés)

### Nouveaux Endpoints (Février 2026)
- ✅ **Automations Engine** (10 endpoints): CRUD + schema + history + test + run
- ✅ **Modes Dynamiques** (6 endpoints): CRUD + schedule
- ✅ **Notifications Système** (10 endpoints): CRUD + FCM tokens + configs

### Endpoints (26 Nov 2025)
- ✅ **Service Discovery - Plugin API** (8 endpoints):
  - sensors: `/v1/plugin-api/sensors/sensors`, `/v1/plugin-api/sensors/environment/:room_id`
  - notes: `/v1/plugin-api/notes/notes`, `/v1/plugin-api/notes/notes/:id`
  - notifications: `/v1/plugin-api/notifications/notifications`, `/v1/plugin-api/notifications/notifications/send`, `/v1/plugin-api/notifications/notifications/acknowledge`, `/v1/plugin-api/notifications/fcm/register`
- ✅ **Parameterized Routes** : Support `:param` dans paths (ex: `:room_id`, `:id`)
- ✅ **Reverse Proxy Architecture** : Kernel → Unix sockets (HTTP proxy dynamique)
