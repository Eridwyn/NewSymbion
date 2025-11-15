# Référence Complète des Endpoints HTTP

> 📍 Documentation exhaustive des 85+ endpoints de l'API Symbion Kernel
>
> ⚠️ **NOTE DE MIGRATION (Novembre 2025)**: 31 endpoints additionnels existent dans `http.rs` mais ne sont pas encore documentés ici:
> - **Context Engine**: `/context/history`, `/context/stats`, `/context/patterns`, `/context/productivity`
> - **Decision Engine**: `/decision/metrics`, `/decision/config`, `/decision/stats`, `/decision/audit/trail`, `/decision/audit/stats`, `/decision/validation/stats`, `/decision/trust/scores`, `/decision/context/latest`
> - **Agent Management**: `/agents/{id}/processes`, `/agents/{id}/command`, `/agents/{id}/reboot`, `/agents/{id}/kill/{pid}`, `/agents/discovery/scan`, `/agents/discovery/status`, `/agents/{id}/network/scan`
> - **Ports**: `/ports`, `/ports/{port}/open`, `/ports/{port}/close`
> - **Auth**: `/auth/session`, `/auth/status`, `/auth/reload`, `/auth/discoverable`
> - **Autres**: `/ws/notes/stream` (WebSocket), `/api/notes` (alias), `/system/reboot`, `/system/shutdown`, `/csrf/token`
>
> Ces endpoints sont fonctionnels et testés mais nécessitent une documentation complète. Voir `symbion-kernel/src/http.rs` pour détails d'implémentation.
>
> ⚠️ **PHANTOM ENDPOINTS (Non implémentés)**: Les endpoints ci-dessous sont documentés mais n'existent PAS dans le code actuel:
> - **Auth**: `/csrf-token` (utiliser `/auth/csrf/nonce`), `/jwt/verify` (utiliser `/auth/verify`), `/sessions`, `/sessions/{id}` (DELETE), `/refresh`
> - **Users**: `/users/{id}` (PUT), `/users/{id}/password` (PUT) - Note: `/users` existe comme `/v1/users`
> - **Agents**: `/agents/{id}` (DELETE), `/agents/{id}/logs`, `/agents/{id}/restart`, `/agents/{id}/capabilities`
> - **Plugins**: `/plugins/{name}/health`
> - **Decision**: `/decision/consents`, `/decision/consents/{id}` (DELETE), `/decision/trust-score` (utiliser `/decision/agent-health`)
> - **Config**: `/config` (GET/PUT) - Note: utiliser `/decision/config` pour configuration Decision Engine
>
> Ces endpoints devraient être retirés de la documentation ou implémentés selon les besoins futurs.

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

### `GET /system/status`
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

### `POST /jwt/verify`
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

### `GET /csrf-token`
**Description** : Génération nonce CSRF (5 min TTL)
**Auth** : JWT
**Response** :
```json
{
  "token": "csrf-nonce-abcd1234",
  "expires_at": 1699887300
}
```

### `GET /sessions`
**Description** : Liste sessions actives utilisateur
**Auth** : JWT
**Response** :
```json
{
  "sessions": [
    {
      "id": "session-1",
      "device": "Firefox on Linux",
      "ip": "192.168.1.14",
      "created_at": 1699800800,
      "last_active": 1699887200
    }
  ]
}
```

### `DELETE /sessions/{session_id}`
**Description** : Révocation session spécifique
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Session revoked"
}
```

### `POST /refresh`
**Description** : Renouvellement token JWT avant expiration
**Auth** : JWT (valide mais proche expiration)
**Response** :
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_at": 1699974000
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

### `GET /users`
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

### `POST /users`
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

### `GET /users/{id}`
**Description** : Détails utilisateur
**Auth** : JWT (admin ou self)
**Response** :
```json
{
  "id": "user-123",
  "username": "admin",
  "is_admin": true,
  "mfa_enabled": true,
  "webauthn_credentials": 2,
  "created_at": 1699800800,
  "last_login": 1699887200
}
```

### `PUT /users/{id}`
**Description** : Modification utilisateur
**Auth** : JWT (admin ou self pour password)
**CSRF** : Requis
**Request** :
```json
{
  "username": "newusername", // Admin only
  "password": "newpassword", // Self ou admin
  "is_admin": true // Admin only
}
```

### `DELETE /users/{id}`
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

### `PUT /users/{id}/password`
**Description** : Changement mot de passe
**Auth** : JWT (self)
**CSRF** : Requis
**Request** :
```json
{
  "current_password": "oldpassword",
  "new_password": "newpassword"
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

### `DELETE /agents/{id}`
**Description** : Suppression agent du registry
**Auth** : JWT (admin)
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Agent eridwyn-Salon removed from registry"
}
```

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

### `GET /agents/{id}/logs`
**Description** : Logs agent (dernières 100 lignes)
**Auth** : JWT
**Response** :
```json
{
  "logs": [
    "[2025-11-12 10:30:45] Agent started",
    "[2025-11-12 10:31:00] Registered with kernel",
    "[2025-11-12 10:31:15] Heartbeat sent"
  ]
}
```

### `POST /agents/{id}/restart`
**Description** : Redémarrage agent process
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Agent restart command sent"
}
```

### `GET /agents/{id}/capabilities`
**Description** : Capacités agent (features disponibles)
**Auth** : JWT
**Response** :
```json
{
  "capabilities": [
    "presence_detection",
    "energy_monitoring",
    "smart_scheduling",
    "context_learning",
    "wake_on_lan"
  ]
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

### `GET /plugins/{name}/health`
**Description** : Santé plugin
**Auth** : JWT
**Response** :
```json
{
  "status": "healthy",
  "last_check": 1699887200,
  "errors": []
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

### `GET /decision/consents`
**Description** : Consentements durables actifs
**Auth** : JWT
**Response** :
```json
{
  "consents": [
    {
      "id": "consent-456",
      "action_type": "agent_shutdown",
      "scope": {
        "target": "eridwyn-Bureau",
        "hours": "23:00-07:00",
        "conditions": ["inactivity > 2h"]
      },
      "created_at": 1699800800,
      "expires_at": 1707576800
    }
  ]
}
```

### `DELETE /decision/consents/{id}`
**Description** : Révocation consentement
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Consent revoked"
}
```

### `GET /decision/pending`
**Description** : Intentions en attente validation
**Auth** : JWT
**Response** :
```json
{
  "pending": [
    {
      "id": "intention-789",
      "action": "purchase_groceries",
      "impact_level": "H",
      "reason": "Automated grocery order",
      "expires_at": 1699889000
    }
  ]
}
```

### `GET /decision/trust-score`
**Description** : Calcul trust score actuel
**Auth** : JWT
**Response** :
```json
{
  "trust_score": 0.78,
  "factors": {
    "sensor_consistency": 0.9,
    "telemetry_freshness": 0.85,
    "historical_success": 0.7,
    "network_latency": 0.6,
    "local_presence": 1.0
  }
}
```

---

## ⚙️ Système & Configuration

### `GET /config`
**Description** : Configuration système
**Auth** : JWT (admin)
**Response** :
```json
{
  "mqtt": {
    "broker": "127.0.0.1:1883",
    "qos": 1
  },
  "api": {
    "port": 8443,
    "tls_enabled": true
  },
  "context": {
    "auto_detection": true,
    "override_timeout_minutes": 480
  }
}
```

### `PUT /config`
**Description** : Modification configuration
**Auth** : JWT (admin)
**CSRF** : Requis
**Request** :
```json
{
  "context": {
    "auto_detection": true
  }
}
```

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

### `GET /system/metrics`
**Description** : Métriques kernel (uptime, memory)
**Auth** : JWT
**Response** :
```json
{
  "uptime_seconds": 86400,
  "memory_mb": 23.6,
  "mqtt_messages_processed": 15432,
  "http_requests_total": 8976
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

**Dernière mise à jour** : 2025-11-15
**Fichier source** : `symbion-kernel/src/http.rs` (2709 lignes)
**Total endpoints documentés** : 93 (90 + 3 metrics)
