# WebAuthn / Passkeys Biométriques - Guide Complet

> 🔐 Authentification passwordless avec clés cryptographiques matérielles

## 🎯 Vue d'Ensemble

**WebAuthn (Web Authentication)** est un standard W3C pour l'authentification **sans mot de passe** utilisant des **clés cryptographiques** stockées dans du matériel sécurisé.

### Pourquoi WebAuthn ?

| Problème Traditionnel | Solution WebAuthn |
|----------------------|-------------------|
| Mots de passe faibles | Pas de mot de passe |
| Phishing | Clé liée au domaine |
| Stockage plaintext | Clé privée jamais transmise |
| Brute-force | Impossible (crypto asymétrique) |
| Keyloggers | Biométrie locale |

### Avantages Symbion

✅ **Passwordless** - Aucun mot de passe à mémoriser
✅ **Anti-phishing** - Clé liée à `symbion.local`
✅ **Biométrie locale** - Empreinte/visage jamais envoyé au serveur
✅ **Multi-device** - Plusieurs passkeys par utilisateur
✅ **Backup** - Synchronisation iCloud/Google Password Manager
✅ **Hardware secure** - Clé privée dans TPM/Secure Enclave

---

## 🔑 Concepts Clés

### Authenticators

**Authenticator** = appareil qui stocke les clés privées et effectue la signature

| Type | Exemples | Utilisation |
|------|----------|-------------|
| **Platform** | TouchID, FaceID, Windows Hello | Intégré à l'appareil |
| **Roaming** | YubiKey, Titan Key | USB/NFC externe |
| **Hybrid** | Smartphone via QR code | Cross-device |

### Cryptographie

**Paire de clés asymétriques** :
- **Clé privée** : Stockée dans authenticator (jamais transmise)
- **Clé publique** : Stockée sur serveur Symbion

**Flow crypto** :
1. Serveur envoie **challenge** (bytes aléatoires)
2. Authenticator **signe** avec clé privée
3. Serveur **vérifie** signature avec clé publique

### Relying Party (RP)

**Relying Party ID** : Domaine du service (ex: `symbion.local`)

**Sécurité** : Passkey ne fonctionne QUE sur domaine enregistré
- Enregistré sur `symbion.local` → ✅ Fonctionne
- Utilisé sur `evil.com` → ❌ Bloqué par browser

---

## 🚀 Enregistrement Passkey (Registration)

### Flow Complet

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │ 1. POST /webauthn/register/start           │
     │    Authorization: Bearer <JWT>             │
     │    {"username": "admin"}                   │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Génération challenge
     │                                            │ + user ID unique
     │                                            │
     │ 2. 200 OK                                  │
     │    {                                       │
     │      "challenge": "rnd-bytes-base64",      │
     │      "rp": {"id": "symbion.local"},        │
     │      "user": {                             │
     │        "id": "user-123-b64",               │
     │        "name": "admin",                    │
     │        "displayName": "Admin User"         │
     │      },                                    │
     │      "pubKeyCredParams": [                 │
     │        {"type": "public-key", "alg": -7}   │
     │      ],                                    │
     │      "timeout": 60000,                     │
     │      "attestation": "none"                 │
     │    }                                       │
     │◄───────────────────────────────────────────┤
     │                                            │
     │ 3. navigator.credentials.create()          │
     │    [User touches fingerprint sensor]       │
     │    [Authenticator generates key pair]      │
     │                                            │
     │ 4. POST /webauthn/register/finish          │
     │    {                                       │
     │      "id": "credential-id-base64",         │
     │      "rawId": "credential-id-base64",      │
     │      "response": {                         │
     │        "attestationObject": "...",         │
     │        "clientDataJSON": "..."             │
     │      },                                    │
     │      "type": "public-key"                  │
     │    }                                       │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation:
     │                                            │ 1. Challenge match?
     │                                            │ 2. RP ID correct?
     │                                            │ 3. Extract public key
     │                                            │ 4. Store in DB
     │                                            │
     │ 5. 200 OK                                  │
     │    {                                       │
     │      "success": true,                      │
     │      "credential_id": "cred-123"           │
     │    }                                       │
     │◄───────────────────────────────────────────┤
     │                                            │
```

### Étape 1 : Start Registration

**Endpoint** : `POST /webauthn/register/start`

**Request** :
```json
{
  "username": "admin"
}
```

**Response** :
```json
{
  "challenge": "W8qWlJT3VhG5VwYZQhZqNw",
  "rp": {
    "id": "symbion.local",
    "name": "Symbion Home Automation Hub"
  },
  "user": {
    "id": "dXNlci0xMjM",
    "name": "admin",
    "displayName": "Administrator"
  },
  "pubKeyCredParams": [
    { "type": "public-key", "alg": -7 },   // ES256
    { "type": "public-key", "alg": -257 }  // RS256
  ],
  "authenticatorSelection": {
    "authenticatorAttachment": "platform",
    "userVerification": "required",
    "residentKey": "preferred"
  },
  "timeout": 60000,
  "attestation": "none"
}
```

**Paramètres** :
- `challenge` : Bytes aléatoires encodés base64 (16-32 bytes)
- `rp.id` : Domaine Symbion
- `user.id` : ID utilisateur encodé base64
- `pubKeyCredParams` : Algorithmes acceptés (ES256, RS256)
- `authenticatorAttachment` : `platform` (TouchID/Windows Hello) ou `cross-platform` (YubiKey)
- `userVerification` : `required` (biométrie obligatoire)
- `attestation` : `none` (pas de vérification fabricant authenticator)

### Étape 2 : Browser API

**Frontend (JavaScript)** :
```javascript
// Récupérer options du serveur
const optionsResponse = await fetch('/webauthn/register/start', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${jwt}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({ username: 'admin' }),
});

const options = await optionsResponse.json();

// Conversion base64url → ArrayBuffer
const challenge = Uint8Array.from(atob(options.challenge.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0));
const userId = Uint8Array.from(atob(options.user.id.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0));

// Appel WebAuthn API
const credential = await navigator.credentials.create({
  publicKey: {
    challenge: challenge,
    rp: options.rp,
    user: {
      id: userId,
      name: options.user.name,
      displayName: options.user.displayName,
    },
    pubKeyCredParams: options.pubKeyCredParams,
    authenticatorSelection: options.authenticatorSelection,
    timeout: options.timeout,
    attestation: options.attestation,
  }
});

// Conversion ArrayBuffer → base64url
function arrayBufferToBase64url(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

// Envoi au serveur
const finishResponse = await fetch('/webauthn/register/finish', {
  method: 'POST',
  headers: {
    'Authorization': `Bearer ${jwt}`,
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    id: credential.id,
    rawId: arrayBufferToBase64url(credential.rawId),
    response: {
      attestationObject: arrayBufferToBase64url(credential.response.attestationObject),
      clientDataJSON: arrayBufferToBase64url(credential.response.clientDataJSON),
    },
    type: credential.type,
  }),
});
```

### Étape 3 : Finish Registration

**Endpoint** : `POST /webauthn/register/finish`

**Request** :
```json
{
  "id": "AaFdkcm...",
  "rawId": "AaFdkcm...",
  "response": {
    "attestationObject": "o2NmbXRk...",
    "clientDataJSON": "eyJ0eXBl..."
  },
  "type": "public-key"
}
```

**Backend Validation (Rust)** :
```rust
// symbion-kernel/src/http.rs:2001-2042
use webauthn_rs::{Webauthn, WebauthnBuilder};

async fn webauthn_register_finish(
    State(app): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Json(payload): Json<RegisterFinishRequest>,
) -> Result<Json<RegisterFinishResponse>, (StatusCode, String)> {
    // 1. Récupérer challenge stocké
    let challenge = app.webauthn_challenges.lock().await
        .remove(&user.id)
        .ok_or((StatusCode::BAD_REQUEST, "Challenge not found".to_string()))?;

    // 2. Decoder attestation
    let attestation_object = base64::decode_config(&payload.response.attestation_object, base64::URL_SAFE_NO_PAD)?;
    let client_data_json = base64::decode_config(&payload.response.client_data_json, base64::URL_SAFE_NO_PAD)?;

    // 3. Validation WebAuthn
    let webauthn = WebauthnBuilder::new("symbion.local", &Url::parse("https://symbion.local:8443")?)?
        .build()?;

    let credential = webauthn.register_credential(
        &attestation_object,
        &client_data_json,
        &challenge,
    ).map_err(|e| (StatusCode::BAD_REQUEST, format!("Validation failed: {}", e)))?;

    // 4. Stockage DB
    let credential_id = format!("cred-{}", uuid::Uuid::new_v4().simple());
    app.db.insert_webauthn_credential(
        &credential_id,
        &user.id,
        &credential.public_key,
        credential.counter,
    ).await?;

    Ok(Json(RegisterFinishResponse {
        success: true,
        credential_id,
    }))
}
```

**Stockage DB** :
```sql
CREATE TABLE webauthn_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    public_key BLOB NOT NULL,
    counter INTEGER NOT NULL DEFAULT 0,
    name TEXT,
    created_at INTEGER NOT NULL,
    last_used INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

---

## 🔓 Authentification avec Passkey

### Flow Complet

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │ 1. POST /webauthn/auth/start               │
     │    {"username": "admin"}                   │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Lookup user
     │                                            │ + credentials
     │                                            │ + challenge
     │                                            │
     │ 2. 200 OK                                  │
     │    {                                       │
     │      "challenge": "rnd-bytes",             │
     │      "allowCredentials": [...]             │
     │    }                                       │
     │◄───────────────────────────────────────────┤
     │                                            │
     │ 3. navigator.credentials.get()             │
     │    [User touches fingerprint]              │
     │    [Authenticator signs challenge]         │
     │                                            │
     │ 4. POST /webauthn/auth/finish              │
     │    {                                       │
     │      "id": "credential-id",                │
     │      "response": {                         │
     │        "authenticatorData": "...",         │
     │        "signature": "..."                  │
     │      }                                     │
     │    }                                       │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Verify signature
     │                                            │ with public key
     │                                            │ + Generate JWT
     │                                            │
     │ 5. 200 OK                                  │
     │    {                                       │
     │      "token": "eyJ0eXAi...",               │
     │      "expires_at": 1699887600              │
     │    }                                       │
     │◄───────────────────────────────────────────┤
     │                                            │
```

### Étape 1 : Start Authentication

**Endpoint** : `POST /webauthn/auth/start`

**Request** :
```json
{
  "username": "admin"
}
```

**Response** :
```json
{
  "challenge": "Y3RqWlN2TmhHNVZ3WVpRaFpxTnc",
  "rpId": "symbion.local",
  "allowCredentials": [
    {
      "type": "public-key",
      "id": "AaFdkcm9Zmlk...",
      "transports": ["internal", "usb", "nfc"]
    }
  ],
  "userVerification": "required",
  "timeout": 60000
}
```

**Paramètres** :
- `allowCredentials` : Liste passkeys enregistrées pour cet utilisateur
- `transports` : `internal` (TouchID), `usb` (YubiKey), `nfc`, `ble`

### Étape 2 : Browser API

**Frontend (JavaScript)** :
```javascript
// Récupérer options
const optionsResponse = await fetch('/webauthn/auth/start', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ username: 'admin' }),
});

const options = await optionsResponse.json();

// Conversion base64url → ArrayBuffer
const challenge = Uint8Array.from(atob(options.challenge.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0));
const allowCredentials = options.allowCredentials.map(cred => ({
  type: cred.type,
  id: Uint8Array.from(atob(cred.id.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0)),
  transports: cred.transports,
}));

// Appel WebAuthn API
const assertion = await navigator.credentials.get({
  publicKey: {
    challenge: challenge,
    rpId: options.rpId,
    allowCredentials: allowCredentials,
    userVerification: options.userVerification,
    timeout: options.timeout,
  }
});

// Envoi au serveur
const finishResponse = await fetch('/webauthn/auth/finish', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    id: assertion.id,
    rawId: arrayBufferToBase64url(assertion.rawId),
    response: {
      authenticatorData: arrayBufferToBase64url(assertion.response.authenticatorData),
      clientDataJSON: arrayBufferToBase64url(assertion.response.clientDataJSON),
      signature: arrayBufferToBase64url(assertion.response.signature),
      userHandle: assertion.response.userHandle ? arrayBufferToBase64url(assertion.response.userHandle) : null,
    },
    type: assertion.type,
  }),
});

const result = await finishResponse.json();
// Stocker JWT
localStorage.setItem('token', result.token);
```

### Étape 3 : Finish Authentication

**Endpoint** : `POST /webauthn/auth/finish`

**Request** :
```json
{
  "id": "AaFdkcm9Zmlk...",
  "rawId": "AaFdkcm9Zmlk...",
  "response": {
    "authenticatorData": "SZYN5YgOjGh0...",
    "clientDataJSON": "eyJ0eXBlIjoi...",
    "signature": "MEUCIQDqV7Ps...",
    "userHandle": "dXNlci0xMjM"
  },
  "type": "public-key"
}
```

**Backend Validation (Rust)** :
```rust
// symbion-kernel/src/http.rs:2212-2273
async fn webauthn_auth_finish(
    State(app): State<Arc<AppState>>,
    Json(payload): Json<AuthFinishRequest>,
) -> Result<Json<AuthFinishResponse>, (StatusCode, String)> {
    // 1. Récupérer credential de la DB
    let credential = app.db.get_webauthn_credential(&payload.id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Credential not found".to_string()))?;

    // 2. Récupérer challenge
    let challenge = app.webauthn_challenges.lock().await
        .remove(&credential.user_id)
        .ok_or((StatusCode::BAD_REQUEST, "Challenge not found".to_string()))?;

    // 3. Decoder response
    let authenticator_data = base64::decode_config(&payload.response.authenticator_data, base64::URL_SAFE_NO_PAD)?;
    let client_data_json = base64::decode_config(&payload.response.client_data_json, base64::URL_SAFE_NO_PAD)?;
    let signature = base64::decode_config(&payload.response.signature, base64::URL_SAFE_NO_PAD)?;

    // 4. Validation WebAuthn
    let webauthn = WebauthnBuilder::new("symbion.local", &Url::parse("https://symbion.local:8443")?)?
        .build()?;

    webauthn.verify_credential(
        &authenticator_data,
        &client_data_json,
        &signature,
        &credential.public_key,
        &challenge,
    ).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Verification failed: {}", e)))?;

    // 5. Update counter (protection replay attacks)
    app.db.update_credential_counter(&credential.id, credential.counter + 1).await?;

    // 6. Génération JWT
    let user = app.db.get_user(&credential.user_id).await?;
    let token = generate_jwt(&user, &app.jwt_secret)?;

    Ok(Json(AuthFinishResponse {
        token,
        expires_at: (Utc::now() + Duration::hours(24)).timestamp(),
    }))
}
```

---

## 🛠️ Gestion Passkeys

### Liste Passkeys Enregistrées

**Endpoint** : `GET /webauthn/credentials`

**Response** :
```json
{
  "credentials": [
    {
      "id": "cred-123",
      "name": "TouchID MacBook Pro",
      "created_at": 1699800800,
      "last_used": 1699887200,
      "transports": ["internal"]
    },
    {
      "id": "cred-456",
      "name": "YubiKey 5",
      "created_at": 1699801000,
      "last_used": null,
      "transports": ["usb", "nfc"]
    }
  ]
}
```

### Renommage Passkey

**Endpoint** : `PUT /webauthn/credentials/{id}`

**Request** :
```json
{
  "name": "TouchID MacBook Pro (Perso)"
}
```

### Suppression Passkey

**Endpoint** : `DELETE /webauthn/credentials/{id}`

**Headers** :
```
Authorization: Bearer <JWT>
X-CSRF-Token: <nonce>
```

**Response** :
```json
{
  "success": true,
  "message": "Passkey deleted"
}
```

---

## 🔒 Sécurité WebAuthn

### Protection Phishing

**Binding au domaine** :
```javascript
// Passkey enregistré sur symbion.local
{
  "rpId": "symbion.local",
  "origin": "https://symbion.local:8443"
}

// Tentative utilisation sur evil.com
// ❌ Browser bloque automatiquement (RP ID mismatch)
```

### Protection Replay Attacks

**Counter vérification** :
```rust
// Chaque assertion incrémente counter
let new_counter = parse_counter(&authenticator_data);

if new_counter <= credential.counter {
    return Err("Replay attack detected");
}

// Update counter in DB
db.update_counter(credential.id, new_counter);
```

### User Verification

**Biométrie obligatoire** :
```json
{
  "userVerification": "required"
}
```

**Modes** :
- `required` : Biométrie/PIN obligatoire (Symbion utilise ça)
- `preferred` : Biométrie si disponible, sinon rien
- `discouraged` : Pas de biométrie (presence only)

---

## 📱 Support Multi-Plateforme

### Compatibilité

| Plateforme | Authenticator | Support |
|------------|---------------|---------|
| **macOS** | TouchID | ✅ Safari 14+, Chrome 67+ |
| **iOS** | FaceID/TouchID | ✅ Safari 14.5+, Chrome 108+ |
| **Windows** | Windows Hello | ✅ Edge 18+, Chrome 67+ |
| **Android** | Fingerprint | ✅ Chrome 70+ |
| **Linux** | Pas de platform | ⚠️ Roaming only (YubiKey) |

### Authenticators Recommandés

**Platform** (intégré) :
- **macOS/iOS** : TouchID, FaceID
- **Windows** : Windows Hello (PIN, Fingerprint, Face)
- **Android** : Fingerprint, Face Unlock

**Roaming** (externe) :
- **YubiKey 5 Series** : USB-A, USB-C, NFC
- **Google Titan Key** : USB, NFC
- **Feitian ePass** : USB, NFC

### Fallback pour Navigateurs Non-Supportés

```javascript
if (!window.PublicKeyCredential) {
  // WebAuthn non supporté → fallback JWT classique
  console.warn('WebAuthn not supported, using password authentication');
  showPasswordLogin();
} else {
  showWebAuthnLogin();
}
```

---

## 🧪 Tests et Debugging

### Test Registration Locale

```bash
# 1. Démarrer Kernel
cargo run --release -p symbion-kernel

# 2. Ouvrir navigateur
open https://symbion.local:8443

# 3. Login JWT classique
curl -X POST https://symbion.local:8443/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# 4. Start registration
curl -X POST https://symbion.local:8443/webauthn/register/start \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"username": "admin"}'
```

### Debugging Browser

**Chrome DevTools** :
```
F12 → Console → navigator.credentials.create(...)
```

**Logs utiles** :
```javascript
navigator.credentials.create(publicKey)
  .then(credential => {
    console.log('Credential created:', credential);
    console.log('ID:', credential.id);
    console.log('Type:', credential.type);
  })
  .catch(error => {
    console.error('WebAuthn error:', error.name, error.message);
  });
```

**Erreurs communes** :
- `NotAllowedError` : User cancelled ou timeout
- `InvalidStateError` : Credential déjà enregistré
- `NotSupportedError` : Authenticator incompatible
- `SecurityError` : RP ID mismatch (phishing protection)

### Logs Backend

```rust
println!("[webauthn] Registration started: user={}", user.username);
println!("[webauthn] Challenge generated: {} bytes", challenge.len());
println!("[webauthn] Credential registered: id={}", credential_id);
println!("[webauthn] Authentication success: user={}, credential={}", user_id, credential_id);
```

---

## 📖 Références

- **WebAuthn Spec (W3C)** : https://www.w3.org/TR/webauthn-2/
- **webauthn-rs (Rust)** : https://docs.rs/webauthn-rs/latest/webauthn_rs/
- **FIDO Alliance** : https://fidoalliance.org/
- **Can I Use WebAuthn** : https://caniuse.com/webauthn
- **WebAuthn Guide** : https://webauthn.guide/

---

**Dernière mise à jour** : 2025-11-12
**Fichiers sources** :
- `symbion-kernel/src/http.rs:2001-2273` (endpoints WebAuthn)
- `symbion-kernel/src/webauthn.rs` (logique validation)
- `pwa-dashboard/src/services/webauthn-service.js` (frontend)
