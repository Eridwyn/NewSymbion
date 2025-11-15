# Authentification - Symbion Kernel

> 🔐 Système multi-couches d'authentification : JWT, MFA, WebAuthn, Device Trust

## 🎯 Vue d'Ensemble

Symbion implémente **4 modes d'authentification** avec sécurité progressive :

| Mode | Sécurité | Use Case | MFA Requis |
|------|----------|----------|------------|
| **JWT** | Base | Session utilisateur standard | Optionnel |
| **JWT + MFA** | Élevée | Opérations sensibles | Oui |
| **WebAuthn** | Très élevée | Passwordless biométrique | Non (passkey = MFA) |
| **API Key** | Service-to-Service | Communication inter-composants | Non |

---

## 🔑 JWT Authentication (Mode Principal)

### Principe

**JSON Web Tokens (RFC 7519)** : tokens signés HS256 avec claims utilisateur

**Caractéristiques** :
- Algorithme : **HS256** (HMAC-SHA256)
- Durée de vie : **8 heures** (configurable via `SYMBION_TOKEN_EXPIRY_HOURS`)
- Secret : Variable d'environnement `SYMBION_JWT_SECRET` (64+ caractères)
- Claims : `sub` (user_id), `username`, `exp`, `mfa_verified`

### Flow Complet

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  POST /login                               │
     │  {"username": "admin", "password": "***"}  │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Vérification
     │                                            │ bcrypt password
     │                                            │
     │  200 OK                                    │
     │  {"token": "eyJ0eXAi...", "expires_at": X} │
     │◄───────────────────────────────────────────┤
     │                                            │
     │                                            │
     │  GET /agents                               │
     │  Authorization: Bearer eyJ0eXAi...        │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation JWT
     │                                            │ + extraction claims
     │                                            │
     │  200 OK                                    │
     │  {"agents": [...]}                         │
     │◄───────────────────────────────────────────┤
     │                                            │
```

### Génération Token

**Fichier** : `symbion-kernel/src/http.rs:1204-1297`

```rust
use jsonwebtoken::{encode, Header, EncodingKey};

#[derive(Serialize)]
struct Claims {
    sub: String,        // User ID
    username: String,
    exp: i64,          // Expiration timestamp
    mfa_verified: bool,
}

let claims = Claims {
    sub: user.id.clone(),
    username: user.username.clone(),
    exp: (Utc::now() + Duration::hours(24)).timestamp(),
    mfa_verified: false,
};

let token = encode(
    &Header::default(),
    &claims,
    &EncodingKey::from_secret(jwt_secret.as_bytes())
)?;
```

### Validation Token

**Middleware** : `require_auth` (`symbion-kernel/src/http.rs:74-115`)

```rust
use jsonwebtoken::{decode, DecodingKey, Validation};

// 1. Extraction header Authorization
let auth_header = request.headers()
    .get("authorization")
    .and_then(|h| h.to_str().ok())?;

let token = auth_header.strip_prefix("Bearer ")?;

// 2. Validation JWT
let token_data = decode::<Claims>(
    token,
    &DecodingKey::from_secret(jwt_secret.as_bytes()),
    &Validation::default()
)?;

// 3. Injection user dans Extension
request.extensions_mut().insert(User {
    id: token_data.claims.sub,
    username: token_data.claims.username,
    mfa_verified: token_data.claims.mfa_verified,
});
```

### Renouvellement Token

**Endpoint** : `POST /refresh`

```bash
# Avant expiration (< 1h restante)
curl -X POST https://localhost:8443/refresh \
  -H "Authorization: Bearer $OLD_TOKEN"

# Réponse : nouveau token avec 24h supplémentaires
{
  "token": "eyJ0eXAiOiJKV1Qi...",
  "expires_at": 1699974000
}
```

---

## 🔐 Multi-Factor Authentication (MFA/TOTP)

### Principe

**TOTP (Time-based One-Time Password, RFC 6238)** : codes à 6 chiffres rotatifs (30s)

**Caractéristiques** :
- Algorithme : **SHA1** (standard TOTP)
- Fenêtre : **30 secondes**
- Longueur code : **6 chiffres**
- Backup codes : **3 codes** one-time (format `ABCD-1234`)
- Compatible : Google Authenticator, Authy, 1Password

### Flow Setup MFA

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  POST /mfa/setup                           │
     │  Authorization: Bearer <JWT>               │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Génération secret
     │                                            │ + QR code
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "secret": "JBSWY3DPEHPK3PXP",           │
     │    "qr_code": "data:image/png;base64,...", │
     │    "backup_codes": ["ABCD-1234", ...]      │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
     │  [User scans QR code in authenticator app] │
     │                                            │
     │  POST /mfa/verify-setup                    │
     │  {"secret": "JBSWY...", "code": "123456"}  │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation code
     │                                            │ + activation MFA
     │                                            │
     │  200 OK                                    │
     │  {"success": true, "mfa_enabled": true}    │
     │◄───────────────────────────────────────────┤
     │                                            │
```

**Fichier setup** : `symbion-kernel/src/http.rs:1476-1586`

### Flow Login avec MFA

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  POST /login                               │
     │  {"username": "admin", "password": "***"}  │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Password OK
     │                                            │ MFA enabled? YES
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "requires_mfa": true,                   │
     │    "mfa_token": "temp-123",                │
     │    "expires_in": 300                       │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
     │  [User opens authenticator app]            │
     │  [Reads current TOTP code: 654321]         │
     │                                            │
     │  POST /login/mfa                           │
     │  {                                         │
     │    "mfa_token": "temp-123",                │
     │    "code": "654321",                       │
     │    "trust_device": true                    │
     │  }                                         │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation TOTP
     │                                            │ + device trust
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "token": "eyJ0eXAiOiJKV1Qi...",         │
     │    "device_trusted": true,                 │
     │    "trust_expires_at": 1702479600          │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
```

### MFA pour Opérations Sensibles

Certains endpoints **requièrent MFA même avec JWT valide** :

**Endpoints MFA-Protected** :
- `POST /decision/override` - Master override décisions
- `DELETE /users/{id}` - Suppression utilisateurs
- `POST /plugins/{name}/stop` - Arrêt plugins critiques

**Vérification claim** :
```rust
// Middleware vérifie `mfa_verified: true` dans JWT claims
if !user.mfa_verified {
    return Err((StatusCode::FORBIDDEN, "MFA required".to_string()));
}
```

### Backup Codes

En cas de perte accès authenticator app :

```bash
# Utiliser backup code au lieu de TOTP
curl -X POST https://localhost:8443/login/mfa \
  -H "Content-Type: application/json" \
  -d '{
    "mfa_token": "temp-123",
    "code": "ABCD-1234"  // Backup code au lieu de TOTP
  }'
```

**Important** : Backup code utilisable **une seule fois**, puis invalidé.

---

## 🔐 WebAuthn (Passkeys Biométriques)

### Principe

**WebAuthn (FIDO2)** : Authentification passwordless avec clés cryptographiques

**Caractéristiques** :
- Standard : **W3C WebAuthn Level 2**
- Authenticators : TouchID, FaceID, Windows Hello, YubiKey
- Cryptographie : **Paires de clés publiques/privées**
- Phishing-resistant : Challenge-response avec domaine binding

### Avantages

✅ **Pas de mot de passe** : clé privée stockée dans hardware sécurisé
✅ **Biométrie locale** : empreinte/visage jamais envoyé au serveur
✅ **Anti-phishing** : clé liée au domaine (`symbion.local`)
✅ **Multi-device** : plusieurs passkeys (laptop, phone, YubiKey)

### Flow Registration (Enregistrement Passkey)

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  POST /webauthn/register/start             │
     │  Authorization: Bearer <JWT>               │
     │  {"username": "admin"}                     │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Génération challenge
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "challenge": "random-bytes-base64",     │
     │    "rp": {"id": "symbion.local"},          │
     │    "user": {"id": "user-123", ...},        │
     │    "pubKeyCredParams": [...]               │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
     │  [Browser navigator.credentials.create()]  │
     │  [User touches fingerprint sensor]         │
     │                                            │
     │  POST /webauthn/register/finish            │
     │  {                                         │
     │    "id": "credential-id",                  │
     │    "response": {                           │
     │      "attestationObject": "...",           │
     │      "clientDataJSON": "..."               │
     │    }                                       │
     │  }                                         │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation challenge
     │                                            │ + stockage clé publique
     │                                            │
     │  200 OK                                    │
     │  {"success": true, "credential_id": "..."} │
     │◄───────────────────────────────────────────┤
     │                                            │
```

**Fichier** : `symbion-kernel/src/http.rs:2001-2042`

### Flow Authentication (Login avec Passkey)

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  POST /webauthn/auth/start                 │
     │  {"username": "admin"}                     │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Génération challenge
     │                                            │ + lookup credentials
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "challenge": "random-bytes",            │
     │    "allowCredentials": [                   │
     │      {"id": "cred-123", "type": "public-key"}│
     │    ]                                       │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
     │  [navigator.credentials.get()]             │
     │  [User touches fingerprint]                │
     │                                            │
     │  POST /webauthn/auth/finish                │
     │  {                                         │
     │    "id": "cred-123",                       │
     │    "response": {                           │
     │      "authenticatorData": "...",           │
     │      "signature": "..."                    │
     │    }                                       │
     │  }                                         │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Vérification signature
     │                                            │ avec clé publique
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "token": "eyJ0eXAiOiJKV1Qi...",         │
     │    "expires_at": 1699887600                │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
```

**Fichier** : `symbion-kernel/src/http.rs:2212-2273`

### Gestion Passkeys

**Liste passkeys enregistrées** :
```bash
curl https://localhost:8443/webauthn/credentials \
  -H "Authorization: Bearer $JWT"

# Réponse
{
  "credentials": [
    {
      "id": "cred-123",
      "name": "TouchID MacBook Pro",
      "created_at": 1699800800,
      "last_used": 1699887200
    },
    {
      "id": "cred-456",
      "name": "Windows Hello Desktop",
      "created_at": 1699800900,
      "last_used": null
    }
  ]
}
```

**Suppression passkey** :
```bash
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

curl -X DELETE https://localhost:8443/webauthn/credentials/cred-456 \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN"
```

---

## 🛡️ Device Trust (Bypass MFA Temporaire)

### Principe

Éviter de demander MFA à chaque login sur **appareils de confiance**.

**Caractéristiques** :
- Durée : **30 jours** (configurable)
- Identification : **Device fingerprint** (browser UUID + User-Agent)
- Stockage : **Cookie sécurisé** (`HttpOnly`, `Secure`, `SameSite=Strict`)
- Révocation : Manuelle via `/sessions` ou automatique après expiration

### Flow Trust Device

**1. Login initial avec MFA** :
```bash
curl -X POST https://localhost:8443/login/mfa \
  -H "Content-Type: application/json" \
  -d '{
    "mfa_token": "temp-123",
    "code": "654321",
    "trust_device": true,
    "device_fingerprint": "browser-uuid-5678"
  }'

# Réponse
{
  "token": "eyJ0eXAiOiJKV1Qi...",
  "device_trusted": true,
  "trust_expires_at": 1702479600  // 30 jours
}
```

**2. Logins suivants (sans MFA)** :
```bash
curl -X POST https://localhost:8443/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "password",
    "device_fingerprint": "browser-uuid-5678"
  }'

# Réponse (direct, sans MFA)
{
  "token": "eyJ0eXAiOiJKV1Qi...",
  "requires_mfa": false,
  "device_recognized": true
}
```

### Révocation Trust

**Liste devices de confiance** :
```bash
curl https://localhost:8443/sessions \
  -H "Authorization: Bearer $JWT"

# Réponse
{
  "sessions": [
    {
      "id": "session-1",
      "device": "Firefox on Linux (Trusted)",
      "trusted_until": 1702479600
    }
  ]
}
```

**Révoquer device** :
```bash
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

curl -X DELETE https://localhost:8443/sessions/session-1 \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN"
```

---

## 🔑 API Keys (Inter-Services)

### Principe

Authentification **fallback** pour communication entre services (agents → kernel).

**Caractéristiques** :
- Header : `X-Api-Key: your-secret-key`
- Secret : Variable d'environnement `SYMBION_API_KEY`
- Pas d'expiration : clé statique
- Usage : Agents, plugins, scripts internes

### Utilisation

```bash
# Agent → Kernel
curl https://localhost:8443/agents \
  -H "X-Api-Key: s3cr3t-42"
```

**Middleware** : `require_auth` (`symbion-kernel/src/http.rs:93-107`)
```rust
// Fallback API Key si JWT absent
let api_key_header = request.headers()
    .get("x-api-key")
    .and_then(|h| h.to_str().ok());

if let Some(key) = api_key_header {
    if key == expected_api_key {
        // Injecter user "system"
        request.extensions_mut().insert(User {
            id: "system".to_string(),
            username: "system".to_string(),
            mfa_verified: false,
        });
    }
}
```

---

## 🔒 Bonnes Pratiques Sécurité

### Stockage Tokens

❌ **Jamais dans localStorage** (vulnérable XSS)
✅ **Cookies HttpOnly** + `Secure` + `SameSite=Strict`
✅ **Memory (JavaScript variable)** pour SPA

### Rotation Secrets

```bash
# Régénérer JWT secret (invalide tous tokens)
export SYMBION_JWT_SECRET=$(openssl rand -base64 64)

# Régénérer API key
export SYMBION_API_KEY=$(openssl rand -hex 32)
```

### Audit Trail

Tous les événements d'authentification sont loggés :

```rust
// symbion-kernel/src/http.rs
println!("[auth] Login attempt: user={}, success={}, ip={}",
    username, success, ip_addr);
println!("[auth] MFA verification: user={}, success={}",
    user_id, success);
println!("[auth] WebAuthn authentication: user={}, credential={}",
    username, credential_id);
```

### Timeouts

| Élément | Durée | Raison |
|---------|-------|--------|
| JWT | 24h | Session standard |
| MFA Token (temporaire) | 5 min | Fenêtre complétion MFA |
| CSRF Nonce | 5 min | Protection replay attack |
| Device Trust | 30 jours | Balance sécurité/UX |

---

## 📖 Références

- **JWT RFC 7519** : https://datatracker.ietf.org/doc/html/rfc7519
- **TOTP RFC 6238** : https://datatracker.ietf.org/doc/html/rfc6238
- **WebAuthn Spec** : https://www.w3.org/TR/webauthn-2/
- **OWASP Auth Cheatsheet** : https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html

---

**Dernière mise à jour** : 2025-11-12
**Fichiers sources** :
- `symbion-kernel/src/http.rs` (endpoints auth)
- `symbion-kernel/src/auth.rs` (logique JWT/MFA)
