# Sécurité API - Symbion Kernel

> 🛡️ 5 couches de protection : CSRF, Rate Limiting, CORS, Input Validation, TLS

## 🎯 Architecture Sécurité Globale

Symbion implémente une **défense en profondeur** (defense-in-depth) :

```
┌─────────────────────────────────────────────────┐
│  1. TLS/HTTPS (Transport Layer)                 │ ← Encryption
├─────────────────────────────────────────────────┤
│  2. CORS (Origin Validation)                    │ ← Browser protection
├─────────────────────────────────────────────────┤
│  3. Rate Limiting (Brute-force Protection)      │ ← DoS mitigation
├─────────────────────────────────────────────────┤
│  4. Authentication (JWT/MFA/WebAuthn)           │ ← Identity verification
├─────────────────────────────────────────────────┤
│  5. CSRF (State-changing Operations)            │ ← CSRF attack prevention
├─────────────────────────────────────────────────┤
│  6. Input Validation (Injection Protection)     │ ← Data sanitization
└─────────────────────────────────────────────────┘
```

---

## 🔒 1. CSRF Protection (Cross-Site Request Forgery)

### Principe

Protection contre **attaques CSRF** : empêcher sites malveillants de déclencher actions non autorisées.

**Mécanisme** : **Nonces one-time** (tokens à usage unique, expiration 5 minutes)

### Endpoints Protégés

**CSRF requis sur tous les endpoints destructifs** :
- `POST`, `PUT`, `DELETE` (sauf endpoints publics)
- Exemples : création utilisateur, extinction agent, override contexte

### Flow CSRF

```
┌──────────┐                                ┌────────────┐
│  Client  │                                │   Kernel   │
└────┬─────┘                                └─────┬──────┘
     │                                            │
     │  GET /csrf-token                           │
     │  Authorization: Bearer <JWT>               │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Génération nonce
     │                                            │ + stockage Redis (5 min)
     │                                            │
     │  200 OK                                    │
     │  {                                         │
     │    "token": "csrf-abcd1234",               │
     │    "expires_at": 1699887300                │
     │  }                                         │
     │◄───────────────────────────────────────────┤
     │                                            │
     │                                            │
     │  POST /agents/eridwyn-Salon/shutdown       │
     │  Authorization: Bearer <JWT>               │
     │  X-CSRF-Token: csrf-abcd1234               │
     ├───────────────────────────────────────────►│
     │                                            │
     │                                            │ Validation nonce:
     │                                            │ 1. Existe dans Redis?
     │                                            │ 2. Non expiré?
     │                                            │ 3. Invalider (one-time)
     │                                            │
     │  200 OK                                    │
     │  {"success": true}                         │
     │◄───────────────────────────────────────────┤
     │                                            │
```

### Implémentation

**Middleware** : `require_csrf` (`symbion-kernel/src/http.rs:117-157`)

```rust
async fn require_csrf(
    State(app): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // 1. Extraction header X-CSRF-Token
    let csrf_token = request.headers()
        .get("x-csrf-token")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::FORBIDDEN, "CSRF token missing".to_string()))?;

    // 2. Validation nonce (existe + non expiré)
    let valid = app.csrf_store.validate(csrf_token).await;

    if !valid {
        return Err((StatusCode::FORBIDDEN, "Invalid CSRF token".to_string()));
    }

    // 3. Invalidation nonce (one-time use)
    app.csrf_store.invalidate(csrf_token).await;

    Ok(next.run(request).await)
}
```

**Génération nonce** :
```rust
// GET /csrf-token
use rand::Rng;

let nonce: String = rand::thread_rng()
    .sample_iter(&rand::distributions::Alphanumeric)
    .take(32)
    .map(char::from)
    .collect();

let expires_at = Utc::now() + Duration::minutes(5);

app.csrf_store.insert(nonce.clone(), expires_at).await;

Json(CsrfResponse {
    token: nonce,
    expires_at: expires_at.timestamp(),
})
```

### Bonnes Pratiques

✅ **Régénérer nonce après chaque utilisation**
✅ **Expiration courte** (5 minutes max)
✅ **Stockage sécurisé** (Redis ou in-memory map thread-safe)
❌ **Jamais inclure dans URL** (only headers)
❌ **Jamais réutiliser** un nonce invalidé

---

## 🚦 2. Rate Limiting (Protection Brute-Force)

### Principe

Limiter nombre de requêtes par IP pour **prévenir brute-force** et **abus API**.

**Implémentation** : **tower_governor** middleware

### Configuration

**Fichier** : `symbion-kernel/src/http.rs:189-206`

```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

// Configuration rate limiter
let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(5)          // 5 requêtes/seconde
        .burst_size(10)         // Burst max 10 requêtes
        .finish()
        .unwrap(),
);

let app = Router::new()
    .route("/login", post(login))
    .layer(GovernorLayer { config: governor_conf });
```

### Limites par Type d'Endpoint

| Endpoint | Limite | Burst | Raison |
|----------|--------|-------|--------|
| `/login` | 5 req/s | 10 | Anti brute-force passwords |
| `/mfa/verify` | 3 req/s | 5 | Anti brute-force TOTP |
| `/webauthn/*` | 10 req/s | 20 | Challenges multiples acceptables |
| **Endpoints API généraux** | 50 req/s | 100 | Usage normal |

### Réponse en Cas de Dépassement

**Status Code** : `429 Too Many Requests`

**Headers** :
```
X-RateLimit-Limit: 5
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1699887360
Retry-After: 60
```

**Body** :
```json
{
  "error": "Too many requests",
  "retry_after": 60
}
```

### Contournement (Whitelist)

Certaines IPs peuvent être **whitelistées** :

```rust
// Exemples : localhost, réseau local, monitoring tools
const WHITELISTED_IPS: &[&str] = &[
    "127.0.0.1",
    "::1",
    "192.168.1.0/24",  // Réseau domestique
];
```

---

## 🌐 3. CORS (Cross-Origin Resource Sharing)

### Principe

Contrôler **quels domaines** peuvent appeler l'API depuis navigateur web.

**Protection** : Empêcher sites malveillants d'appeler API avec credentials utilisateur.

### Configuration

**Fichier** : `symbion-kernel/src/http.rs:333-352`

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin([
        "https://symbion.local:3000".parse().unwrap(),  // PWA Dashboard
        "https://192.168.1.14:3000".parse().unwrap(),   // PWA via IP
        "http://localhost:3000".parse().unwrap(),       // Dev local
    ])
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        HeaderName::from_static("x-csrf-token"),
        HeaderName::from_static("x-api-key"),
    ])
    .allow_credentials(true)  // Cookies, JWT dans headers
    .max_age(Duration::from_secs(3600));

let app = Router::new()
    .route("/agents", get(get_agents))
    .layer(cors);
```

### Origins Autorisées

**Production** :
- `https://symbion.local:3000` (PWA Dashboard via hostname)
- `https://192.168.1.14:3000` (PWA Dashboard via IP locale)

**Développement** :
- `http://localhost:3000` (Dev frontend local)

### Preflight Requests

**Browser envoie OPTIONS avant requête réelle** :

```
OPTIONS /agents/eridwyn-Salon/shutdown
Origin: https://symbion.local:3000
Access-Control-Request-Method: POST
Access-Control-Request-Headers: x-csrf-token, authorization
```

**Kernel répond avec permissions** :
```
Access-Control-Allow-Origin: https://symbion.local:3000
Access-Control-Allow-Methods: POST
Access-Control-Allow-Headers: x-csrf-token, authorization
Access-Control-Allow-Credentials: true
Access-Control-Max-Age: 3600
```

### Blocage Requêtes Non Autorisées

**Exemple : site malveillant `evil.com`** :

```javascript
// Sur evil.com (tentative CSRF)
fetch('https://symbion.local:8443/agents/shutdown', {
    method: 'POST',
    credentials: 'include',  // Envoie cookies utilisateur
    headers: {
        'Authorization': 'Bearer <stolen-token>'
    }
});

// ❌ BLOQUÉ par CORS : evil.com n'est pas dans allow_origin
```

---

## 🔐 4. TLS/HTTPS Encryption

### Principe

**Chiffrement transport layer** : empêcher interception credentials/données.

**Protocole** : **TLS 1.3** (recommandé) ou TLS 1.2

### Configuration Actuelle

**Développement** :
- Certificats **auto-signés** (mkcert)
- Port : **8443** (HTTPS)
- CA : `/etc/mosquitto/certs/symbion-ca.crt`

**Fichiers** :
- **Certificat** : `/etc/mosquitto/certs/cert-mkcert.pem`
- **Clé privée** : `/etc/mosquitto/certs/key-mkcert.pem`
- **CA** : `/etc/mosquitto/certs/symbion-ca.crt`

### Lancement Kernel TLS

```bash
export SYMBION_TLS_CERT_PATH=/etc/mosquitto/certs/cert-mkcert.pem
export SYMBION_TLS_KEY_PATH=/etc/mosquitto/certs/key-mkcert.pem

cargo run --release -p symbion-kernel

# Output
[kernel] HTTPS listening on 0.0.0.0:8443 with TLS
```

### Téléchargement CA Certificat

**Endpoint public** : `GET /ca-certificate`

```bash
# Télécharger CA pour installation navigateur/OS
curl https://localhost:8443/ca-certificate -o symbion-ca.crt

# Installation
# - Linux : sudo cp symbion-ca.crt /usr/local/share/ca-certificates/ && sudo update-ca-certificates
# - macOS : sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain symbion-ca.crt
# - Windows : certutil -addstore -f "ROOT" symbion-ca.crt
```

### Production (Recommandations)

**Let's Encrypt** pour certificats gratuits valides :

```bash
# Installation certbot
sudo apt install certbot

# Génération certificat (domaine public requis)
sudo certbot certonly --standalone -d symbion.yourdomain.com

# Certificats générés dans /etc/letsencrypt/live/symbion.yourdomain.com/
# - fullchain.pem (certificat)
# - privkey.pem (clé privée)

# Configuration Kernel
export SYMBION_TLS_CERT_PATH=/etc/letsencrypt/live/symbion.yourdomain.com/fullchain.pem
export SYMBION_TLS_KEY_PATH=/etc/letsencrypt/live/symbion.yourdomain.com/privkey.pem
```

**Renouvellement automatique** :
```bash
# Crontab : renouvellement tous les 2 mois
0 0 1 */2 * certbot renew --quiet && systemctl restart symbion-kernel
```

---

## 🧹 5. Input Validation & Sanitization

### Principe

**Valider et sanitizer toutes les entrées utilisateur** pour prévenir injections.

### Validations Implémentées

**1. JSON Schema Validation**

```rust
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 3, max = 32))]
    username: String,

    #[validate(length(min = 8))]
    password: String,

    #[validate(email)]
    email: Option<String>,
}

async fn create_user(
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    // Validation automatique
    payload.validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // ...
}
```

**2. Command Whitelisting (Agents)**

**Fichier** : `symbion-agent-host/src/main.rs:796`

```rust
// Liste blanche commandes autorisées
const ALLOWED_COMMANDS: &[&str] = &[
    "systemctl",
    "shutdown",
    "reboot",
    "hibernate",
    "sensors",
    "df",
    "free",
    "uptime",
];

fn validate_command(cmd: &str) -> bool {
    let first_word = cmd.split_whitespace().next().unwrap_or("");
    ALLOWED_COMMANDS.contains(&first_word)
}

// Rejet commandes dangereuses
if !validate_command(&command) {
    return Err("Command not allowed");
}
```

**3. ANSI Escape Code Sanitization**

**Fichier** : `symbion-agent-host/src/main.rs:821-838`

```rust
use regex::Regex;

fn sanitize_output(output: &str) -> String {
    // Suppression codes ANSI (couleurs, formatage)
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(output, "").to_string()
}

// Avant envoi MQTT
let sanitized = sanitize_output(&raw_output);
publish_response(sanitized);
```

**4. SQL Injection Protection**

**ORM utilisé** : **SQLx** avec **requêtes préparées**

```rust
// ✅ SAFE : paramètres bindés
let user = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE username = $1",
    username
)
.fetch_one(&pool)
.await?;

// ❌ VULNÉRABLE (ne JAMAIS faire)
let query = format!("SELECT * FROM users WHERE username = '{}'", username);
```

### Limites de Taille

| Champ | Limite | Raison |
|-------|--------|--------|
| **Username** | 3-32 caractères | Standards sécurité |
| **Password** | 8+ caractères | Complexité minimale |
| **Note content** | 10 000 caractères | Éviter abus stockage |
| **Command output** | 50 000 caractères | Prévenir overflow MQTT |
| **JSON payload** | 1 MB | DoS protection |

---

## 🔍 6. Audit & Logging

### Principe

**Traçabilité complète** des événements sécurité.

### Événements Loggés

**Authentification** :
```rust
println!("[auth] Login attempt: user={}, success={}, ip={}, timestamp={}",
    username, success, ip_addr, Utc::now());

println!("[auth] MFA verification: user={}, success={}, method={}",
    user_id, success, "totp");

println!("[auth] WebAuthn authentication: user={}, credential={}, success={}",
    username, credential_id, success);

println!("[auth] JWT validation failed: token_expired={}, user={}",
    expired, user_id);
```

**Opérations Sensibles** :
```rust
println!("[security] Agent shutdown: agent={}, initiated_by={}, ip={}",
    agent_id, user_id, ip_addr);

println!("[security] User deleted: user={}, deleted_by={}, admin={}",
    deleted_user, admin_user, is_admin);

println!("[security] Master override: action={}, user={}, mfa_verified={}",
    action_type, user_id, mfa_verified);
```

**CSRF/Rate Limiting** :
```rust
println!("[security] CSRF validation failed: token={}, user={}, ip={}",
    csrf_token, user_id, ip_addr);

println!("[security] Rate limit exceeded: ip={}, endpoint={}, limit={}",
    ip_addr, endpoint, limit);
```

### Format Logs

**JSON structuré** pour parsing automatique :

```json
{
  "timestamp": "2025-11-12T10:30:45Z",
  "level": "WARN",
  "category": "security",
  "event": "login_failed",
  "user": "admin",
  "ip": "192.168.1.42",
  "reason": "invalid_password"
}
```

### Rétention

**Recommandation** :
- **Logs sécurité** : 1 an minimum
- **Logs audit** : 3 ans (compliance)
- **Logs debug** : 30 jours

---

## 🛡️ Checklist Sécurité

Avant déploiement production :

### Transport & Network
- [ ] TLS 1.3 activé (ou TLS 1.2 minimum)
- [ ] Certificats valides (Let's Encrypt ou CA entreprise)
- [ ] HSTS headers configurés
- [ ] Firewall règles configurées (port 8443 seulement)

### Authentication
- [ ] JWT secret ≥ 64 caractères aléatoires
- [ ] API key ≥ 32 caractères aléatoires
- [ ] Passwords hachés bcrypt (cost 12+)
- [ ] MFA activé pour utilisateurs admin
- [ ] WebAuthn configuré pour utilisateurs critiques

### API Protection
- [ ] CSRF protection activée sur tous POST/PUT/DELETE
- [ ] Rate limiting configuré par endpoint
- [ ] CORS origins whitelistées (pas de wildcard)
- [ ] Input validation sur tous endpoints
- [ ] Command whitelist configurée (agents)

### Monitoring
- [ ] Logs audit activés
- [ ] Alertes authentification échouée (3+ tentatives)
- [ ] Monitoring rate limit dépassements
- [ ] Dashboard métriques sécurité

### Code Quality
- [ ] Pas de secrets hardcodés (code scanning)
- [ ] Dependencies à jour (cargo audit)
- [ ] OWASP Top 10 vérifié
- [ ] Penetration testing effectué

---

## 📖 Références

- **OWASP Top 10** : https://owasp.org/www-project-top-ten/
- **OWASP Cheat Sheets** : https://cheatsheetseries.owasp.org/
- **CSRF Prevention** : https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
- **Rate Limiting Best Practices** : https://blog.logrocket.com/rate-limiting-go-application/
- **TLS Best Practices** : https://wiki.mozilla.org/Security/Server_Side_TLS

---

**Dernière mise à jour** : 2025-11-12
**Fichiers sources** :
- `symbion-kernel/src/http.rs` (CSRF, CORS, rate limiting)
- `symbion-agent-host/src/main.rs` (command whitelisting, sanitization)
