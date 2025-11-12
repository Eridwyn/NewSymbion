# Quick Reference - Symbion

> 🚀 Cheat sheet rapide pour développeurs

## 📡 Endpoints HTTP Essentiels

### Authentification
```bash
# Login (JWT)
POST /login
{"username": "admin", "password": "***"}

# Login avec MFA
POST /login/mfa
{"mfa_token": "temp-123", "code": "654321"}

# Obtenir CSRF token
GET /csrf-token
Authorization: Bearer <JWT>

# WebAuthn (Passkey)
POST /webauthn/register/start   # Enregistrement
POST /webauthn/auth/start        # Login
```

### Agents Domestiques
```bash
# Liste agents
GET /agents

# Détails agent
GET /agents/{id}

# Extinction machine
POST /agents/{id}/shutdown
X-CSRF-Token: <nonce>

# Commande custom
POST /agents/{id}/command
{"command": "systemctl status bluetooth"}

# Wake-on-LAN
POST /wake?host_id=eridwyn-Bureau
```

### Context Engine
```bash
# Mode actuel
GET /context/current

# Override manuel
POST /context/override
{"mode": "cravate", "duration_minutes": 240}

# Clear override
POST /context/clear
```

### Notes/Memo
```bash
# Liste notes
GET /ports/memo

# Nouvelle note (contexte auto-injecté)
POST /ports/memo
{"content": "Acheter lait", "tags": ["courses"]}

# Modifier note
PUT /ports/memo/{id}

# Supprimer note
DELETE /ports/memo/{id}
```

---

## 🔌 Topics MQTT Principaux

### Agent → Kernel
```
symbion/agents/registration@v1   # Enregistrement (au boot)
symbion/agents/heartbeat@v1      # Métriques (30s)
symbion/agents/response@v1       # Résultat commandes
```

### Kernel → Agents
```
symbion/agents/command@v1        # Commandes de contrôle
symbion/agents/wake@v1           # Wake-on-LAN
```

### Notes Plugin
```
symbion/notes/request@v1         # CRUD requests
symbion/notes/response@v1        # Réponses plugin
```

### Dashboard PWA
```
symbion/dashboard/update@v1      # Événements temps réel
symbion/dashboard/notification@v1 # Alertes utilisateur
```

---

## 🔐 Authentification Rapide

### Flow JWT Standard
```bash
# 1. Login
TOKEN=$(curl -s -X POST https://localhost:8443/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}' \
  | jq -r '.token')

# 2. Utiliser token
curl https://localhost:8443/agents \
  -H "Authorization: Bearer $TOKEN"
```

### Flow avec CSRF (POST/PUT/DELETE)
```bash
# 1. Get JWT
TOKEN=$(curl -s -X POST https://localhost:8443/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}' \
  | jq -r '.token')

# 2. Get CSRF token
CSRF=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $TOKEN" \
  | jq -r '.token')

# 3. Requête avec CSRF
curl -X POST https://localhost:8443/agents/eridwyn-Salon/shutdown \
  -H "Authorization: Bearer $TOKEN" \
  -H "X-CSRF-Token: $CSRF"
```

### Flow WebAuthn (Passkey)
```javascript
// 1. Start registration
const options = await fetch('/webauthn/register/start', {
  method: 'POST',
  headers: { 'Authorization': `Bearer ${jwt}` },
}).then(r => r.json());

// 2. Create credential
const credential = await navigator.credentials.create({
  publicKey: convertOptions(options)
});

// 3. Finish registration
await fetch('/webauthn/register/finish', {
  method: 'POST',
  body: JSON.stringify(convertCredential(credential)),
});
```

---

## 🛡️ Sécurité Checklist

### Headers Requis
```bash
# JWT (tous endpoints sauf publics)
Authorization: Bearer <token>

# CSRF (POST/PUT/DELETE)
X-CSRF-Token: <nonce>

# API Key (fallback inter-services)
X-Api-Key: <secret>
```

### Rate Limits
```
/login          5 req/s  (burst 10)
/mfa/verify     3 req/s  (burst 5)
/webauthn/*    10 req/s  (burst 20)
Endpoints API  50 req/s  (burst 100)
```

### CORS Origins Autorisées
```
https://symbion.local:3000
https://192.168.1.14:3000
http://localhost:3000
```

---

## 🧪 Développement Local

### Démarrer Kernel
```bash
export SYMBION_API_KEY="s3cr3t-42"
export SYMBION_MQTT_BROKER="127.0.0.1:1883"
export SYMBION_JWT_SECRET="test-secret-64chars+"
export SYMBION_TLS_CERT_PATH="/etc/mosquitto/certs/cert-mkcert.pem"
export SYMBION_TLS_KEY_PATH="/etc/mosquitto/certs/key-mkcert.pem"

cargo run --release -p symbion-kernel
# Listening on https://0.0.0.0:8443
```

### Démarrer Agent
```bash
cargo run --release -p symbion-agent-host
# Agent eridwyn-Salon registered
```

### Démarrer PWA Dashboard
```bash
cd pwa-dashboard
npm run dev
# Dashboard on http://localhost:3000
```

### MQTT Broker (Mosquitto)
```bash
# Démarrer
sudo systemctl start mosquitto

# Logs temps réel
sudo tail -f /var/log/mosquitto/mosquitto.log

# Test subscription
mosquitto_sub -h localhost -t 'symbion/#' -v

# Test publish
mosquitto_pub -h localhost -t 'symbion/test' -m 'Hello'
```

---

## 📊 Monitoring Santé

### Health Check
```bash
curl https://localhost:8443/health
# {"status": "healthy", "mqtt_connected": true, "agents_online": 2}
```

### System Status
```bash
curl https://localhost:8443/system/status
# Détails complets kernel + agents + plugins
```

### Logs Kernel
```bash
tail -f /tmp/kernel.log | grep -E "(ERROR|WARN|auth|mqtt)"
```

### Logs Agents
```bash
journalctl -u symbion-agent-host -f
```

---

## 🔧 Commandes Utiles

### Docs Lookup Script
```bash
./scripts/docs-lookup.sh              # Menu
./scripts/docs-lookup.sh endpoints    # Liste endpoints
./scripts/docs-lookup.sh mqtt         # Liste topics
./scripts/docs-lookup.sh security     # Résumé sécurité
./scripts/docs-lookup.sh search "JWT" # Recherche
```

### Git Workflow
```bash
# Status
git status

# Commit (avec CSRF/JWT/WebAuthn changes)
git add .
git commit -m "feat: add WebAuthn passkey support

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"

# Push
git push origin main
```

### Tests
```bash
# Tests unitaires
cargo test --workspace

# Test endpoint specific
cargo test test_login_success

# Test MQTT
cargo test test_mqtt_connection
```

---

## 🔍 Troubleshooting

### MQTT Disconnected
```bash
# 1. Vérifier broker
sudo systemctl status mosquitto

# 2. Restart broker
sudo systemctl restart mosquitto

# 3. Check kernel connection
curl https://localhost:8443/system/status | jq '.mqtt'
```

### Agent Offline
```bash
# 1. Check heartbeat
curl https://localhost:8443/agents/{id} | jq '.last_seen'

# 2. Restart agent
cargo run --release -p symbion-agent-host

# 3. Check MQTT subscription
mosquitto_sub -h localhost -t 'symbion/agents/heartbeat@v1'
```

### CSRF Token Invalid
```bash
# Régénérer token (expire 5 min)
CSRF=$(curl -s https://localhost:8443/csrf-token \
  -H "Authorization: Bearer $TOKEN" | jq -r '.token')
```

### JWT Expired
```bash
# Refresh token (si < 1h restante)
curl -X POST https://localhost:8443/refresh \
  -H "Authorization: Bearer $OLD_TOKEN"

# Ou re-login
TOKEN=$(curl -s -X POST https://localhost:8443/login \
  -d '{"username":"admin","password":"password"}' | jq -r '.token')
```

---

## 📚 Documentation Complète

**Docs** : `docs/`
- `docs/api/endpoints.md` - 90+ endpoints HTTP
- `docs/api/authentication.md` - JWT, MFA, WebAuthn
- `docs/api/security.md` - CSRF, Rate Limiting, CORS
- `docs/api/webauthn.md` - Passkeys biométriques
- `docs/mqtt/topics.md` - 13 topics MQTT
- `docs/mqtt/contracts.md` - Schémas JSON
- `docs/mqtt/flows.md` - Message flows

**Script** : `./scripts/docs-lookup.sh`

---

**Dernière mise à jour** : 2025-11-12
