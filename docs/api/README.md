# API HTTP - Symbion Kernel

> 🌐 API REST complète du cerveau central Symbion

## 📋 Vue d'Ensemble

L'API HTTP du Kernel Symbion est construite avec **Axum** (framework Rust) et expose 90+ endpoints organisés en 10 catégories fonctionnelles.

**Base URL** : `https://localhost:8443` (TLS avec certificats auto-signés)
**Format** : JSON
**Authentification** : JWT (HS256) + API Keys (fallback)

## 🗂️ Catégories d'Endpoints

| Catégorie | Endpoints | Description |
|-----------|-----------|-------------|
| **Public** | 3 | Santé système, statut, certificat CA |
| **Authentification** | 8 | Login, JWT, sessions, MFA |
| **WebAuthn** | 6 | Passkeys biométriques (FIDO2) |
| **Utilisateurs** | 6 | CRUD utilisateurs (admin only) |
| **Agents** | 15+ | Gestion agents domestiques |
| **Contexte** | 3 | Modes (Cravate/Intime/Neutre) |
| **Notes/Memo** | 4 | Mémoire externe via ports |
| **Plugins** | 5 | Orchestration modules |
| **Décisions** | 8 | Decision Engine (PR3) |
| **Système** | 4 | Configuration, métriques |

**Détails complets** : [endpoints.md](./endpoints.md)

## 🔐 Authentification

Symbion supporte **3 modes d'authentification** :

### 1. JWT (Mode Principal)

```bash
# 1. Login pour obtenir token
curl -X POST https://localhost:8443/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# Réponse :
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "expires_at": 1699887600,
  "requires_mfa": false
}

# 2. Utiliser token dans header Authorization
curl https://localhost:8443/agents \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGc..."
```

**Caractéristiques JWT** :
- Algorithme : HS256
- Durée : 24 heures
- Claims : `sub` (user_id), `exp`, `username`, `mfa_verified`

### 2. API Key (Fallback Inter-Services)

```bash
curl https://localhost:8443/agents \
  -H "X-Api-Key: your-secret-api-key"
```

Utilisé pour communication entre services (agents → kernel).

### 3. WebAuthn Passkeys (Biométrique)

Flow complet : [authentication.md#webauthn](./authentication.md#webauthn)

## 🛡️ Sécurité

L'API implémente 5 couches de protection :

| Mécanisme | Description | Endpoints Affectés |
|-----------|-------------|-------------------|
| **JWT Auth** | Token obligatoire | Tous sauf publics |
| **CSRF** | Nonce one-time | POST/PUT/DELETE destructifs |
| **MFA** | TOTP requis | Opérations sensibles |
| **Rate Limiting** | 5 req/sec/IP | Tous endpoints |
| **CORS** | Origins whitelistés | Tous endpoints |

**Détails complets** : [security.md](./security.md)

## 📚 Documentation Détaillée

- **[Endpoints complets](./endpoints.md)** - Référence exhaustive avec exemples
- **[Authentification](./authentication.md)** - JWT, MFA, WebAuthn, Device Trust
- **[WebAuthn/Passkeys](./webauthn.md)** - Guide complet authentification biométrique
- **[Sécurité](./security.md)** - CSRF, Rate Limiting, CORS, protections

## 🚀 Exemples Rapides

### Créer une note contextuelle

```bash
# 1. Obtenir token CSRF
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

# 2. Créer note
curl -X POST https://localhost:8443/ports/memo \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Acheter lait + pain",
    "context": "intime",
    "tags": ["courses"]
  }'
```

### Éteindre un agent domestique

```bash
# 1. Obtenir CSRF token
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

# 2. Envoyer commande shutdown
curl -X POST https://localhost:8443/agents/eridwyn-Salon/shutdown \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN"
```

### Activer mode Focus Pro

```bash
CSRF_TOKEN=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $JWT" | jq -r '.token')

curl -X POST https://localhost:8443/context/override \
  -H "Authorization: Bearer $JWT" \
  -H "X-CSRF-Token: $CSRF_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "mode": "cravate",
    "duration_minutes": 240,
    "reason": "Focus développement"
  }'
```

## 🔧 Développement

### Ajouter un nouvel endpoint

1. **Définir route** dans `symbion-kernel/src/http.rs`
2. **Implémenter handler** (fonction async)
3. **Ajouter middleware** si nécessaire (`require_auth`, `require_csrf`)
4. **Documenter** dans `docs/api/endpoints.md`
5. **Mettre à jour** `CLAUDE.md`

**Pattern recommandé** :
```rust
// symbion-kernel/src/http.rs
async fn my_endpoint(
    State(app): State<Arc<AppState>>,
    Extension(user): Extension<User>,  // Injecté par require_auth
    Json(payload): Json<MyPayload>,
) -> Result<Json<MyResponse>, (StatusCode, String)> {
    // Logique métier
    Ok(Json(MyResponse { ... }))
}

// Enregistrement route
let app = Router::new()
    .route("/my-endpoint", post(my_endpoint))
    .layer(middleware::from_fn_with_state(
        app_state.clone(),
        require_auth
    ))
    .layer(middleware::from_fn_with_state(
        app_state.clone(),
        require_csrf
    ));
```

### Tester endpoint

```bash
# Test santé système (public)
curl https://localhost:8443/health

# Test endpoint authentifié
JWT="your-jwt-token"
curl https://localhost:8443/agents \
  -H "Authorization: Bearer $JWT"
```

## 📖 Références

- **Axum Documentation** : https://docs.rs/axum/latest/axum/
- **JWT RFC 7519** : https://datatracker.ietf.org/doc/html/rfc7519
- **WebAuthn Spec** : https://www.w3.org/TR/webauthn-2/
- **CORS MDN** : https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS

---

**Dernière mise à jour** : 2025-11-12
**Fichier source** : `symbion-kernel/src/http.rs` (2273 lignes)
