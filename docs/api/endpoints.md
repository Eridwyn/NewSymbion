# Référence Complète des Endpoints HTTP

> 📍 Documentation exhaustive des 90+ endpoints de l'API Symbion Kernel

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

### `POST /login`
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

### `POST /login/mfa`
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

### `POST /logout`
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

### `POST /mfa/setup`
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

### `POST /mfa/verify-setup`
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

### `POST /mfa/disable`
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

### `POST /webauthn/register/start`
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

### `POST /webauthn/register/finish`
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

### `POST /webauthn/auth/start`
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

### `POST /webauthn/auth/finish`
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

### `GET /webauthn/credentials`
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

### `DELETE /webauthn/credentials/{id}`
**Description** : Suppression passkey
**Auth** : JWT
**CSRF** : Requis
**Response** :
```json
{
  "success": true,
  "message": "Passkey deleted"
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
      "name": "firefox",
      "cpu": 15.2,
      "memory_mb": 1024
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

**Dernière mise à jour** : 2025-11-12
**Fichier source** : `symbion-kernel/src/http.rs` (2273 lignes)
**Total endpoints documentés** : 90+
