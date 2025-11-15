# Sécurité API - Symbion Kernel

> 🛡️ 7 couches de protection : TLS, CORS, CSP, Rate Limiting, Auth, CSRF, Input Validation

## 🎯 Architecture Sécurité Globale

Symbion implémente une **défense en profondeur** (defense-in-depth) :

```
┌─────────────────────────────────────────────────┐
│  1. TLS/HTTPS (Transport Layer)                 │ ← Encryption
├─────────────────────────────────────────────────┤
│  2. CORS (Origin Validation)                    │ ← Browser protection
├─────────────────────────────────────────────────┤
│  3. CSP (Content Security Policy)               │ ← XSS prevention
├─────────────────────────────────────────────────┤
│  4. Rate Limiting (Brute-force Protection)      │ ← DoS mitigation
├─────────────────────────────────────────────────┤
│  5. Authentication (JWT/MFA/WebAuthn)           │ ← Identity verification
├─────────────────────────────────────────────────┤
│  6. CSRF (State-changing Operations)            │ ← CSRF attack prevention
├─────────────────────────────────────────────────┤
│  7. Input Validation (Injection Protection)     │ ← Data sanitization
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

Limiter nombre de requêtes pour **prévenir brute-force** et **abus API**.

**Implémentation Actuelle** : **auth.rs username-based rate limiting**

### ⚠️ Architecture Rate Limiting (Mise à jour 14 Nov 2025)

**CHANGEMENT**: tower_governor middleware **RETIRÉ** suite à incompatibilité critique.

**Raison**:
- `PeerIpKeyExtractor` et `SmartIpKeyExtractor` causaient HTTP 500 "Unable To Extract Key!"
- Échec sur localhost ET connexions VPN
- Bloquait toute authentification (vulnérabilité critique VULN-009)

**Solution**: Retour à rate limiting application-level dans `auth.rs`

### Configuration Active

**Fichier** : `symbion-kernel/src/auth.rs:145-171`

**Protection Auth Login**:
```rust
// Rate limiting basé sur USERNAME (pas IP)
const MAX_LOGIN_ATTEMPTS: usize = 5;
const RATE_LIMIT_WINDOW_SECS: i64 = 900;  // 15 minutes

fn check_rate_limit(&self, username: &str) -> Result<()> {
    let mut attempts = self.login_attempts.write();
    let now = OffsetDateTime::now_utc().unix_timestamp();

    let user_attempts = attempts.entry(username.to_string()).or_insert_with(Vec::new);

    // Supprimer tentatives expirées (> 15 min)
    user_attempts.retain(|&timestamp| now - timestamp < RATE_LIMIT_WINDOW_SECS);

    // Bloquer si limite atteinte
    if user_attempts.len() >= MAX_LOGIN_ATTEMPTS {
        let wait_minutes = /* calcul temps restant */;
        anyhow::bail!("Too many login attempts. Please wait {} minute(s)", wait_minutes);
    }

    Ok(())
}
```

**Caractéristiques**:
- ✅ Basé sur **username** (fonctionne localhost, VPN, production)
- ✅ Protection brute-force efficace (5 tentatives / 15 min)
- ✅ Stockage in-memory (reset au redémarrage kernel)
- ✅ Message d'erreur avec temps d'attente restant
- ⚠️ Pas de rate limiting global par IP (attaquant peut tester différents usernames)

### Limites par Type d'Endpoint

| Endpoint | Limite | Window | Scope | Status |
|----------|--------|--------|-------|--------|
| `/auth/login` | 5 attempts | 15 min | per username | ✅ Active |
| `/v1/auth/mfa/verify` | 5 attempts | 15 min | per username | ✅ Active |
| **Autres endpoints API** | Aucune | - | - | ⚠️ Non limité |

### Réponse en Cas de Dépassement

**Status Code** : `401 Unauthorized` (rate limit login)

**Body** :
```json
{
  "error": "Too many login attempts. Please wait 12 minute(s) before trying again."
}
```

**Note**: Pas de headers `X-RateLimit-*` (implémentation application-level, pas middleware HTTP)

### Avantages vs Inconvénients

**✅ Avantages auth.rs rate limiting**:
- Fonctionne sur tous environnements (dev, VPN, production)
- Pas de dépendance sur extraction IP complexe
- Granularité par utilisateur (meilleure protection comptes individuels)
- Pas de dépendance externe (tower_governor retiré)

**⚠️ Inconvénients**:
- Pas de protection DoS global par IP
- Attaquant peut distribuer attaque sur plusieurs usernames
- Reset complet au redémarrage kernel (pas de persistence)

**Mitigation recommandée (P1)**:
- Monitoring Prometheus pour détection pattern anomalies
- Alertes email si > 50 tentatives login/heure (tous usernames)
- Considérer nginx rate limiting en production (upstream du kernel)

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

## 🛡️ 7. CSP (Content Security Policy)

### Principe

**Content Security Policy (CSP)** est un header de sécurité HTTP qui permet de **contrôler les sources de contenu** autorisées à être chargées par le navigateur. C'est une **défense en profondeur contre les attaques XSS** (Cross-Site Scripting).

**Mécanisme** : Le serveur envoie un header `Content-Security-Policy` listant les directives de sources autorisées. Le navigateur **bloque automatiquement** tout contenu ne respectant pas la politique.

### Politique Symbion

Symbion implémente une **politique CSP stricte** basée sur le principe du **moindre privilège** (default deny) :

```
Content-Security-Policy:
  default-src 'none';                                    ← Deny all by default
  script-src 'self';                                     ← Scripts from same origin only
  style-src 'self' 'unsafe-inline';                      ← Styles + inline (for <style> tag)
  img-src 'self' data:;                                  ← Images + data URIs
  font-src 'self';                                       ← Fonts from same origin
  connect-src 'self' http: https: ws: wss:;              ← API + WebSocket + LAN mobile access
  manifest-src 'self';                                   ← PWA manifest
  base-uri 'self';                                       ← Prevent <base> tag injection
  form-action 'self';                                    ← Forms submit to same origin
  frame-ancestors 'none'                                 ← No framing (clickjacking prevention)
```

**Note sur l'accès LAN** : La directive `connect-src` est volontairement permissive (`http: https: ws: wss:`) pour permettre l'accès depuis des appareils mobiles sur le réseau local (IP 192.168.x.x). Pour un déploiement production sur internet public, cette directive devrait être restreinte à des origines spécifiques.

### Directives Expliquées

| Directive | Valeur | Justification |
|-----------|--------|---------------|
| `default-src` | `'none'` | Deny all by default - principe du moindre privilège |
| `script-src` | `'self'` | Scripts uniquement depuis même origine (localhost:3000 PWA) |
| `style-src` | `'self' 'unsafe-inline'` | Styles locaux + inline requis pour `<style>` tag dans index.html |
| `img-src` | `'self' data:` | Images locales + data URIs (pour base64 images) |
| `font-src` | `'self'` | Fonts système (Monaco, Menlo, Consolas) |
| `connect-src` | `'self' http: https: ws: wss:` | API + WebSocket + LAN mobile access (192.168.x.x) |
| `manifest-src` | `'self'` | PWA manifest.json depuis même origine |
| `base-uri` | `'self'` | Prévient injection de `<base>` tag pour redirection malveillante |
| `form-action` | `'self'` | Forms ne peuvent soumettre que vers même origine |
| `frame-ancestors` | `'none'` | Interdiction totale de framing (prévient clickjacking) |

### Attaques Prévenues

**XSS (Cross-Site Scripting)** :
- ❌ Injection de `<script>` malveillant → Bloqué par `script-src 'self'`
- ❌ Chargement de scripts externes (CDN compromis) → Bloqué
- ❌ Inline event handlers (`onclick="evil()"`) → Bloqué (pas de `'unsafe-inline'` pour scripts)

**Clickjacking** :
- ❌ Framing dans `<iframe>` malveillant → Bloqué par `frame-ancestors 'none'`

**Data Exfiltration** :
- ❌ Fetch vers domaine externe → Bloqué par `connect-src` restreint

**Base Tag Injection** :
- ❌ Injection `<base href="https://evil.com">` → Bloqué par `base-uri 'self'`

### Implémentation

**Middleware Axum** : `symbion-kernel/src/http.rs:362-397`

```rust
/// Middleware pour ajouter le header CSP (Content Security Policy)
/// Prévient les attaques XSS en restreignant les sources de contenu autorisées
async fn add_csp_header(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    let csp_policy = "default-src 'none'; \
                      script-src 'self'; \
                      style-src 'self' 'unsafe-inline'; \
                      img-src 'self' data:; \
                      font-src 'self'; \
                      connect-src 'self' http: https: ws: wss:; \
                      manifest-src 'self'; \
                      base-uri 'self'; \
                      form-action 'self'; \
                      frame-ancestors 'none'";

    response.headers_mut().insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        csp_policy.parse().unwrap()
    );
    response
}
```

**Application** : Middleware global appliqué à **toutes les réponses HTTP** du kernel (ligne 345).

### Testing & Validation

**1. Vérifier header présent** :
```bash
curl -k -I https://localhost:8443/health | grep -i content-security-policy
# Output:
# content-security-policy: default-src 'none'; script-src 'self'; ...
```

**2. Browser DevTools** :
- Ouvrir Console (F12)
- Charger PWA dashboard
- Vérifier **aucune violation CSP** (warnings en rouge)
- Si violations : ajuster politique ou corriger code PWA

**3. CSP Evaluator** :
- Outil Google : https://csp-evaluator.withgoogle.com/
- Coller politique Symbion
- Vérifier score sécurité (doit être "High")

### Notes de Sécurité

⚠️ **`'unsafe-inline'` pour styles** : Actuellement requis car `<style>` tag inline dans `pwa-dashboard/index.html`. **Amélioration future** : extraire styles dans fichier CSS externe pour supprimer `'unsafe-inline'`.

✅ **Pas de `'unsafe-inline'` pour scripts** : Aucun inline script dans la PWA, tous les scripts sont externes (`/config.js`, `/src/main.js`) → Sécurité maximale contre XSS.

⚠️ **`connect-src` permissif pour LAN** : La directive `http: https: ws: wss:` permet les connexions depuis n'importe quelle IP du réseau local (192.168.x.x) pour l'accès mobile/tablette. **Compromis domotique** : Trade-off entre sécurité et accessibilité pour un système home automation. En production internet public, restreindre à des origines spécifiques (ex: `https://symbion.yourdomain.com wss://symbion.yourdomain.com`).

### Évolution Future

**v1.3.0+ (Production)** :
- Extraction styles inline vers CSS externe
- Suppression `'unsafe-inline'` pour `style-src`
- CSP Report-Only mode pour monitoring violations :
  ```
  Content-Security-Policy-Report-Only: ...
  report-uri /csp-violation-report
  ```

**v2.0.0+ (Multi-tenant)** :
- CSP dynamique par tenant
- Support `'nonce-'` pour inline scripts (si besoin)

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

**Dernière mise à jour** : 2025-11-15 (PR6 - CSP headers)
**Fichiers sources** :
- `symbion-kernel/src/http.rs` (CSRF, CORS, CSP, HSTS, rate limiting)
- `symbion-agent-host/src/main.rs` (command whitelisting, sanitization)
