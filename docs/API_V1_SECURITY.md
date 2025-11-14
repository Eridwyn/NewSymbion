# API v1 - Authentication & Security

**Version**: 0.3.0-alpha.1 (PR2)
**Date**: 30 Octobre 2025
**Base URL**: `https://localhost:8443` (dev) | `https://yourdomain.com:8443` (prod)

---

## 🔐 **Authentication**

Symbion utilise **JWT (JSON Web Tokens)** pour l'authentification des utilisateurs.

### **POST /auth/login**

Authentification utilisateur et récupération du token JWT.

**Headers**:
```
Content-Type: application/json
```

**Request Body**:
```json
{
  "username": "Mark",
  "password": "your_password",
  "totp_code": "123456"  // Optionnel - requis si MFA activé
}
```

**Response Success (200)**:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "username": "Mark",
  "role": "admin",
  "expires_at": 1730304000,
  "mfa_enabled": false
}
```

**Response Error - MFA Required (401)**:
```json
{
  "error": "MFA required",
  "mfa_required": true
}
```

**Response Error - Invalid Credentials (401)**:
```json
{
  "error": "Invalid credentials"
}
```

**Response Error - Rate Limit (429)**:
```json
{
  "error": "Too many failed attempts. Wait 15 minutes.",
  "retry_after": 900
}
```

**Notes**:
- Token valide pendant **8 heures** par défaut
- Rate limiting: **10 tentatives max / 15 minutes**
- Si MFA activé, le champ `totp_code` devient obligatoire

---

## 🛡️ **CSRF Protection**

Les routes destructrices (POST/PUT/DELETE) nécessitent un **nonce CSRF** en plus du JWT.

### **GET /auth/csrf/nonce**

Génère un nonce CSRF single-use avec TTL de 5 minutes.

**Headers**:
```
Authorization: Bearer {jwt_token}
```

**Response Success (200)**:
```json
{
  "nonce": "550e8400-e29b-41d4-a716-446655440000",
  "expires_in_seconds": 300
}
```

**Notes**:
- **TTL**: 5 minutes
- **Single-use**: Nonce consommé après première utilisation
- **Auto-refresh**: Frontend doit refresh 30s avant expiration
- **Stockage backend**: In-memory (perdu au redémarrage kernel)

### **Routes Protégées par CSRF**

Toutes les routes `/v1/*` avec méthodes **POST/PUT/DELETE** nécessitent:

**Headers requis**:
```
Authorization: Bearer {jwt_token}
X-CSRF-Token: {nonce}
Content-Type: application/json
```

**Liste des routes protégées**:
1. **Agents Control**:
   - `POST /v1/agents/{id}/shutdown`
   - `POST /v1/agents/{id}/reboot`
   - `POST /v1/agents/{id}/hibernate`
   - `POST /v1/agents/{id}/kill-process`

2. **Context Control**:
   - `POST /v1/context/override`
   - `POST /v1/context/clear`

3. **Plugins Control**:
   - `POST /v1/plugins/{name}/start`
   - `POST /v1/plugins/{name}/stop`
   - `POST /v1/plugins/{name}/restart`

4. **Notes Management**:
   - `PUT /v1/ports/memo/{id}`
   - `DELETE /v1/ports/memo/{id}`

**Error Response - Missing CSRF (403)**:
```json
{
  "error": "Missing or invalid CSRF token"
}
```

**Error Response - Expired/Consumed (403)**:
```json
{
  "error": "CSRF token expired or already consumed"
}
```

---

## 🔒 **MFA / TOTP (Two-Factor Authentication)**

Symbion supporte l'authentification à deux facteurs via **TOTP (RFC 6238)**.
Compatible avec: Google Authenticator, Microsoft Authenticator, Authy, 1Password, etc.

### **GET /v1/auth/mfa/status**

Récupère l'état MFA de l'utilisateur actuel.

**Headers**:
```
Authorization: Bearer {jwt_token}
```

**Response Success (200)**:
```json
{
  "enabled": false,
  "username": "Mark"
}
```

---

### **POST /v1/auth/mfa/setup**

Initie la configuration MFA pour l'utilisateur.
Génère un secret TOTP, QR code et codes de récupération.

**Headers**:
```
Authorization: Bearer {jwt_token}
```

**Response Success (200)**:
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL...",
  "backup_codes": [
    "a1b2-c3d4-e5f6",
    "g7h8-i9j0-k1l2",
    "m3n4-o5p6-q7r8",
    "s9t0-u1v2-w3x4",
    "y5z6-a7b8-c9d0"
  ],
  "app_name": "Symbion",
  "username": "Mark"
}
```

**Notes**:
- **QR Code**: Format SVG encodé en base64 (data URI)
- **Secret**: Base32 encoded TOTP secret
- **Backup Codes**: 5 codes de récupération à usage unique
- **⚠️ Important**: Conserver les backup codes dans un endroit sûr

---

### **POST /v1/auth/mfa/verify**

Vérifie le code TOTP et **active MFA** pour l'utilisateur.

**Headers**:
```
Authorization: Bearer {jwt_token}
Content-Type: application/json
```

**Request Body**:
```json
{
  "code": "123456"  // Code TOTP 6 chiffres
}
```

**Response Success (200)**:
```json
{
  "success": true,
  "message": "MFA enabled successfully"
}
```

**Response Error - Invalid Code (401)**:
```json
{
  "success": false,
  "error": "Invalid TOTP code"
}
```

**Notes**:
- Code TOTP valide pendant **30 secondes** (fenêtre standard)
- Après activation, tous les futurs logins nécessiteront MFA

---

### **POST /v1/auth/mfa/disable**

Désactive MFA pour l'utilisateur actuel.

**Headers**:
```
Authorization: Bearer {jwt_token}
```

**Response Success (200)**:
```json
{
  "success": true,
  "message": "MFA disabled successfully"
}
```

**Notes**:
- Supprime le secret TOTP et les backup codes
- Logins futurs ne nécessiteront plus MFA

---

## 🚦 **Rate Limiting**

### ⚠️ **Mise à Jour Architecture (14 Nov 2025)**

**CHANGEMENT MAJEUR**: tower_governor middleware **RETIRÉ** suite à vulnérabilité critique (VULN-009).

**Raison**:
- `GovernorLayer` causait HTTP 500 "Unable To Extract Key!" sur localhost ET VPN
- Bloquait **toute authentification** (impact critique)
- Tous les IP extractors (Peer, Smart) échouaient
- Commit retrait: `b358b9b`

### **Protection Active: auth.rs Rate Limiting**

Implémenté dans `symbion-kernel/src/auth.rs:145-171` - Protection brute-force login.

**Limites**:
- **5 tentatives max / 15 minutes** par username (corrigé de 10 → 5)
- Compteur in-memory (reset au redémarrage kernel)
- Basé sur **username** (pas IP) → fonctionne partout

**Comportement**:
- Après 5 échecs: HTTP 401 avec message temps d'attente
- Window sliding: tentatives expirées automatiquement après 15 minutes

**Exemple réponse rate limit**:
```json
{
  "error": "Too many login attempts. Please wait 12 minute(s) before trying again."
}
```

**Avantages**:
- ✅ Fonctionne sur localhost, VPN, production
- ✅ Pas de dépendance IP extraction complexe
- ✅ Protection efficace par compte utilisateur

**Limitations**:
- ⚠️ Pas de rate limiting global par IP (DoS possible sur différents usernames)
- ⚠️ Pas de headers `X-RateLimit-*` (application-level, pas middleware HTTP)

### **Endpoints Protégés**

| Endpoint | Rate Limit | Window | Status |
|----------|-----------|--------|--------|
| `POST /auth/login` | 5 attempts | 15 min | ✅ Active |
| `POST /v1/auth/mfa/verify` | 5 attempts | 15 min | ✅ Active |
| Autres endpoints API | Aucun | - | ⚠️ Non limité |

### **Mitigation DoS Recommandée (P1)**

En l'absence de rate limiting global HTTP:
- Monitoring Prometheus pour détecter patterns anomalies
- Alertes email si > 50 tentatives login/heure
- Considérer nginx rate limiting en production (upstream du kernel)

---

## 📚 **Exemples d'Usage**

### **Workflow Login Standard (sans MFA)**

```bash
# 1. Login
curl -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test"}' \
  -k

# Response:
{
  "token": "eyJ0eXAiOiJKV1QiLC...",
  "username": "Mark",
  "role": "admin",
  "expires_at": 1730304000
}

# 2. Utiliser le token pour requêtes protégées
curl -X GET https://localhost:8443/v1/agents \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLC..." \
  -k
```

---

### **Workflow Login avec MFA**

```bash
# 1. Tentative login sans TOTP code
curl -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test"}' \
  -k

# Response:
{
  "error": "MFA required",
  "mfa_required": true
}

# 2. Login avec code TOTP
curl -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test","totp_code":"123456"}' \
  -k

# Response (si code valide):
{
  "token": "eyJ0eXAiOiJKV1QiLC...",
  "username": "Mark",
  "role": "admin",
  "expires_at": 1730304000,
  "mfa_enabled": true
}
```

---

### **Workflow Action Protégée CSRF**

```bash
# 1. Login
TOKEN=$(curl -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test"}' \
  -k -s | jq -r '.token')

# 2. Obtenir nonce CSRF
NONCE=$(curl -X GET https://localhost:8443/auth/csrf/nonce \
  -H "Authorization: Bearer $TOKEN" \
  -k -s | jq -r '.nonce')

# 3. Effectuer action protégée
curl -X POST https://localhost:8443/v1/context/override \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE" \
  -H "Content-Type: application/json" \
  -d '{"mode":"intime","duration_minutes":60}' \
  -k

# 4. Tentative replay attack (doit échouer avec 403)
curl -X POST https://localhost:8443/v1/context/override \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $NONCE" \
  -H "Content-Type: application/json" \
  -d '{"mode":"cravate","duration_minutes":120}' \
  -k

# Response: HTTP 403 Forbidden
{
  "error": "CSRF token expired or already consumed"
}
```

---

### **Workflow Activation MFA**

```bash
# 1. Login
TOKEN=$(curl -X POST https://localhost:8443/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"Mark","password":"test"}' \
  -k -s | jq -r '.token')

# 2. Initier setup MFA
curl -X POST https://localhost:8443/v1/auth/mfa/setup \
  -H "Authorization: Bearer $TOKEN" \
  -k -s | jq

# Response:
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/svg+xml;base64,...",
  "backup_codes": ["a1b2-c3d4-e5f6", ...]
}

# 3. Scanner QR code avec Google Authenticator

# 4. Vérifier code TOTP et activer MFA
curl -X POST https://localhost:8443/v1/auth/mfa/verify \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code":"123456"}' \
  -k

# Response:
{
  "success": true,
  "message": "MFA enabled successfully"
}

# 5. Tous les futurs logins nécessitent maintenant MFA
```

---

## 🔍 **Routes Publiques (Sans Auth)**

Routes accessibles sans JWT token:

- `GET /health` - Health check kernel
- `GET /system/health` - Health check système complet
- `POST /auth/login` - Login utilisateur
- `GET /ca-certificate` - Téléchargement certificat TLS CA

---

## 🛠️ **Troubleshooting**

### **Erreur: "Missing or invalid CSRF token"**

**Cause**: Header `X-CSRF-Token` manquant ou nonce invalide
**Solution**:
1. Vérifier que le nonce est généré via `/auth/csrf/nonce`
2. Vérifier que le header `X-CSRF-Token` est présent
3. Vérifier que le nonce n'a pas expiré (TTL 5 min)

### **Erreur: "CSRF token expired or already consumed"**

**Cause**: Nonce déjà utilisé ou expiré
**Solution**: Générer un nouveau nonce avant chaque action protégée

### **Erreur: "Too many failed attempts"**

**Cause**: Rate limiting auth (10 tentatives / 15 min)
**Solution**: Attendre 15 minutes ou utiliser les bons credentials

### **Erreur: "MFA required"**

**Cause**: MFA activé mais code TOTP non fourni
**Solution**: Ajouter champ `"totp_code": "123456"` au body de login

### **Erreur: "Invalid TOTP code"**

**Cause**: Code TOTP incorrect ou expiré (fenêtre 30s)
**Solution**:
1. Vérifier synchronisation horloge système
2. Regénérer code dans l'app Authenticator
3. Utiliser backup code si problème persiste

---

## 📖 **Références**

- **JWT**: [RFC 7519](https://tools.ietf.org/html/rfc7519)
- **TOTP**: [RFC 6238](https://tools.ietf.org/html/rfc6238)
- **CSRF**: [OWASP CSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)

---

## 📝 **Changelog PR2**

**Version 0.3.0-alpha.1** (30 Octobre 2025)

- ✅ JWT Authentication avec tokens Bearer
- ✅ CSRF Protection middleware sur routes destructrices
- ✅ MFA/TOTP Support complet (setup, verify, disable)
- ✅ Rate Limiting auth (10 tentatives / 15 min)
- ✅ Interface PWA avec page Paramètres utilisateur
- ✅ QR code generation pour MFA setup
- ✅ Backup codes récupération (5 codes single-use)

**Limitations connues**:
- ⚠️ Tower rate limiting désactivé (incompatibilité localhost)
- ⚠️ Backup codes MFA non implémentés côté validation (TODO PR3)
- ⚠️ Pas de révocation tokens JWT (stateless - TODO session blacklist)

---

**Maintenu par**: Mark (avec assistance Claude Code)
**Contact**: markchavatte@gmail.com
