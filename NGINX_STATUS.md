# État Configuration Nginx - 22 Nov 2025

## ✅ Ce qui Fonctionne

### 1. Configuration Nginx Installée
- ✅ `/etc/nginx/sites-available/symbion` créé
- ✅ `/etc/nginx/conf.d/websocket-upgrade.conf` avec map directive
- ✅ Syntaxe validée (`nginx -t` passed)
- ✅ Nginx rechargé et actif

### 2. Services Backend Actifs
```bash
✅ Kernel    : localhost:8443 (symbion-kernel)
✅ MQTT      : localhost:9001 (mosquitto websockets)
✅ PWA       : localhost:3000 (vite dev server)
✅ Nginx     : localhost:443 (reverse proxy)
```

### 3. Routage Configuré
- `/api/*` → Kernel (8443)
- `/ws/mqtt` → MQTT WebSocket (9001)
- `/health` → Kernel health check
- `/*` → PWA Dashboard (3000)

## ⚠️ Problème Actuel: HTTP/2 vs WebSocket

### Symptôme
```bash
curl --http2 https://localhost/ws/mqtt
→ HTTP/2 502 Bad Gateway

curl --http1.1 https://localhost/ws/mqtt
→ Connection établie (timeout = success)
```

### Cause Technique

Le problème est **architectural** :

1. **HTTP/2** ne supporte PAS le statut `101 Switching Protocols`
2. **WebSocket** NÉCESSITE ce status code pour upgrader
3. **Nginx** avec `listen 443 ssl http2` accepte HTTP/2 du client
4. Même avec `proxy_http_version 1.1`, Nginx ne peut pas convertir la réponse `101` de Mosquitto en stream HTTP/2

### Pourquoi `proxy_http_version 1.1` ne suffit pas ?

```
Client (HTTP/2)
    ↓ GET /ws/mqtt + Upgrade: websocket
Nginx (HTTP/2 frontend)
    ↓ proxy_http_version 1.1  ← Force HTTP/1.1 vers backend
Mosquitto (HTTP/1.1)
    ↓ 101 Switching Protocols  ← WebSocket handshake réussi
Nginx
    ❌ IMPOSSIBLE de convertir "101" en HTTP/2 stream
    ↓
Client
    ← 502 Bad Gateway
```

## 🔧 Solutions Possibles

### Solution 1: Server Block Dédié SANS HTTP/2 (RECOMMANDÉ)

Créer un serveur Nginx séparé pour WebSocket sans HTTP/2 :

```nginx
# /etc/nginx/sites-available/symbion-mqtt

server {
    listen 8444 ssl;  # Port différent, SANS http2
    server_name symbion.markcha.fr;

    # Certificats SSL
    ssl_certificate /etc/mosquitto/certs/cert-mkcert.pem;
    ssl_certificate_key /etc/mosquitto/certs/key-mkcert.pem;

    # Configuration SSL
    ssl_protocols TLSv1.3 TLSv1.2;

    location / {
        proxy_pass http://127.0.0.1:9001/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_read_timeout 86400s;
        proxy_buffering off;
    }
}
```

**Avantages** :
- ✅ WebSocket fonctionne parfaitement (HTTP/1.1 uniquement)
- ✅ API REST garde HTTP/2 (performances optimales)
- ✅ Isolation clean

**Inconvénients** :
- ❌ Nécessite ouvrir port 8444 sur votre NAS
- ❌ Deux ports à gérer (443 + 8444)

**Configuration PWA** :
```javascript
MQTT_BROKER: 'wss://symbion.markcha.fr:8444'
```

### Solution 2: Subdomain Dédié WebSocket

```nginx
# ws.symbion.markcha.fr SANS HTTP/2
server {
    listen 443 ssl;  # SANS http2
    server_name ws.symbion.markcha.fr;

    # ... même config WebSocket que solution 1
}

# api.symbion.markcha.fr AVEC HTTP/2
server {
    listen 443 ssl http2;
    server_name api.symbion.markcha.fr;

    # ... API + PWA
}
```

**Avantages** :
- ✅ Port 443 uniquement
- ✅ Séparation DNS propre

**Inconvénients** :
- ❌ Nécessite 2 sous-domaines
- ❌ 2 certificats SSL (ou wildcard *.symbion.markcha.fr)

**Configuration PWA** :
```javascript
API_BASE: 'https://api.symbion.markcha.fr'
MQTT_BROKER: 'wss://ws.symbion.markcha.fr'
```

### Solution 3: Désactiver HTTP/2 Complètement (NON RECOMMANDÉ)

```nginx
server {
    listen 443 ssl;  # Retirer "http2"
    server_name symbion.markcha.fr;
    # ...
}
```

**Avantages** :
- ✅ WebSocket fonctionne
- ✅ Tout sur port 443

**Inconvénients** :
- ❌ Perte performance HTTP/2 pour API/PWA
- ❌ Régression pour utilisateurs modernes

### Solution 4: Cloudflare Zero Trust (FUTURE)

Cloudflare supporte WebSocket via leurs tunnels avec HTTP/2 ALG (Application Layer Gateway).

**Avantages** :
- ✅ Tout sur port 443
- ✅ HTTP/2 + WebSocket fonctionnent
- ✅ Zero Trust intégré
- ✅ DDoS protection

**Prochaines étapes** :
- Cloudflare Tunnel + Access policies
- Configuration après validation architecture actuelle

## 📊 Tests Actuels

### ✅ Ce qui Fonctionne

```bash
# Health check
curl -k https://localhost/health
→ {"status":"ok"}

# PWA Dashboard
curl -k https://localhost/
→ HTML (Vite dev server)

# WebSocket direct (sans Nginx)
curl -v http://localhost:9001/
→ Connexion acceptée
```

### ❌ Ce qui Ne Fonctionne Pas

```bash
# MQTT WebSocket via Nginx (HTTP/2)
curl -k https://localhost/ws/mqtt
→ 502 Bad Gateway

# API endpoints vides
curl -k -H "x-api-key: s3cr3t-42" https://localhost/api/agents
→ Réponse vide
```

### 🔍 Debug API Vide

Les requêtes API retournent vide. Testons directement le kernel :

```bash
# Direct kernel (devrait fonctionner)
curl -k -H "x-api-key: s3cr3t-42" https://localhost:8443/api/agents

# Via Nginx (problème)
curl -k -H "x-api-key: s3cr3t-42" https://localhost/api/agents
```

**Possible cause** : Path rewriting incorrect ou CORS

## 🎯 Recommandation Immédiate

**Option A - Quick Win (Dev Local)** :
1. Utiliser connexion directe ports (8443, 9001, 3000)
2. Tester architecture Nginx sur machine de staging
3. Implémenter Solution 1 (port 8444 pour MQTT)

**Option B - Production Ready** :
1. Implémenter **Solution 1** (Server block dédié port 8444)
2. Ouvrir ports 443 + 8444 sur NAS
3. Tester en production
4. Migrer vers Cloudflare Tunnel après validation

## 📁 Fichiers Créés

```
nginx-symbion.conf           - Configuration Nginx complète
NGINX_SETUP.md               - Guide installation détaillé
scripts/test-nginx-setup.sh  - Suite de tests automatisés
pwa-dashboard/public/config.js - Config adaptative dev/prod
```

## 🔄 Prochaines Actions

### Choix 1: Port Dédié WebSocket (Simple, Rapide)

```bash
# 1. Créer config MQTT dédiée
sudo cp nginx-symbion-mqtt.conf /etc/nginx/sites-available/symbion-mqtt
sudo ln -s /etc/nginx/sites-available/symbion-mqtt /etc/nginx/sites-enabled/

# 2. Recharger Nginx
sudo nginx -t && sudo systemctl reload nginx

# 3. Tester
curl --http1.1 -k -v \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  https://localhost:8444/
```

### Choix 2: Subdomain (Production, Propre)

```bash
# 1. Créer DNS entries
ws.symbion.markcha.fr → NAS IP
api.symbion.markcha.fr → NAS IP

# 2. Générer certificats SSL
sudo certbot certonly -d ws.symbion.markcha.fr -d api.symbion.markcha.fr

# 3. Configurer Nginx
sudo cp nginx-symbion-split.conf /etc/nginx/sites-available/symbion
```

## 📞 Support / Debug

### Logs à Surveiller

```bash
# Nginx global
sudo tail -f /var/log/nginx/symbion-error.log

# MQTT WebSocket
sudo tail -f /var/log/nginx/mqtt-ws-error.log

# Mosquitto
sudo tail -f /var/log/mosquitto/mosquitto.log

# Kernel
tail -f /tmp/kernel.log
```

### Commandes Utiles

```bash
# Vérifier ports
ss -tlnp | grep -E ":(443|8443|9001|3000)"

# Test direct MQTT WS
curl -v http://localhost:9001/

# Test Nginx syntax
sudo nginx -T | grep -A 20 "ws/mqtt"

# Recharger sans downtime
sudo systemctl reload nginx
```

---

**Créé** : 22 Nov 2025 21:00 CET
**Status** : Configuration installée, WebSocket HTTP/2 incompatibility identifiée
**Next** : Choisir Solution 1 ou 2 pour production
