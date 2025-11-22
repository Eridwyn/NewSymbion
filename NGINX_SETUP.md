# Guide Nginx Reverse Proxy - Symbion

Configuration Nginx pour router tous les services Symbion via port 443 uniquement.

## Architecture

```
Internet → symbion.markcha.fr:443 (HTTPS)
    ↓
NAS Reverse Proxy
    ↓
Nginx (ce serveur)
    ↓
    ├─ /api/* → Kernel (localhost:8443)
    ├─ /ws/mqtt → MQTT WebSocket (localhost:9001)
    ├─ /health → Kernel health check
    └─ /* → PWA Dashboard (localhost:3000)
```

## Prérequis

1. **Nginx installé**
   ```bash
   sudo apt install nginx
   ```

2. **Services Symbion actifs**
   ```bash
   # Vérifier que les services sont lancés sur les bons ports
   sudo ss -tlnp | grep -E ":(8443|9001|3000)"
   # Attendu:
   # :8443  symbion-kernel
   # :9001  mosquitto
   # :3000  node (vite)
   ```

3. **Certificats SSL**
   - Option 1 (dev): Utiliser certificats mkcert existants dans `/etc/mosquitto/certs/`
   - Option 2 (prod): Générer Let's Encrypt (voir section ci-dessous)

## Installation

### Étape 1: Créer le fichier de configuration WebSocket map

```bash
# Créer la map directive pour WebSocket (requis au niveau http{})
sudo tee /etc/nginx/conf.d/websocket-upgrade.conf > /dev/null <<'EOF'
# WebSocket Upgrade Map
# Convertit header Upgrade en variable utilisable par proxy_set_header
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}
EOF
```

### Étape 2: Installer la configuration Symbion

```bash
# Copier la configuration
sudo cp nginx-symbion.conf /etc/nginx/sites-available/symbion

# Désactiver la config par défaut
sudo rm /etc/nginx/sites-enabled/default 2>/dev/null || true

# Activer Symbion
sudo ln -s /etc/nginx/sites-available/symbion /etc/nginx/sites-enabled/

# Créer les fichiers de logs
sudo mkdir -p /var/log/nginx
sudo touch /var/log/nginx/symbion-access.log
sudo touch /var/log/nginx/symbion-error.log
sudo touch /var/log/nginx/mqtt-ws-access.log
sudo touch /var/log/nginx/mqtt-ws-error.log
```

### Étape 3: Tester la configuration

```bash
# Vérifier la syntaxe Nginx
sudo nginx -t

# Attendu:
# nginx: the configuration file /etc/nginx/nginx.conf syntax is ok
# nginx: configuration file /etc/nginx/nginx.conf test is successful
```

### Étape 4: Recharger Nginx

```bash
# Recharger sans downtime
sudo systemctl reload nginx

# Ou redémarrer complètement si besoin
sudo systemctl restart nginx

# Vérifier le statut
sudo systemctl status nginx
```

## Configuration DNS (sur votre NAS)

Sur votre NAS (reverse proxy principal), configurer :

```nginx
# Exemple configuration NAS → Nginx local
server {
    listen 443 ssl;
    server_name symbion.markcha.fr;

    # Certificats SSL NAS
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        # Proxy vers Nginx Symbion (IP du serveur Symbion)
        proxy_pass https://192.168.x.x:443;  # Remplacer par IP réelle
        proxy_ssl_verify off;

        # Headers standards
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Support WebSocket
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;

        # Timeouts étendus
        proxy_read_timeout 86400s;
    }
}
```

## Tests de Validation

### Test 1: Health Check API

```bash
# Local (doit fonctionner)
curl -k https://localhost/health

# Via domaine (si DNS configuré)
curl https://symbion.markcha.fr/health

# Attendu:
# {"status":"ok","timestamp":"..."}
```

### Test 2: API REST

```bash
# Lister les agents
curl -k -H "x-api-key: s3cr3t-42" \
  https://localhost/api/agents

# Attendu: JSON array d'agents
```

### Test 3: MQTT WebSocket

```bash
# Test avec curl (doit upgrader vers WebSocket)
curl -v -k \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  https://localhost/ws/mqtt

# Attendu dans la réponse:
# HTTP/1.1 101 Switching Protocols
# Connection: upgrade
# Upgrade: websocket
```

### Test 4: PWA Dashboard

```bash
# Vérifier que la page charge
curl -k https://localhost/ | grep -i symbion

# Ou simplement ouvrir dans un navigateur:
# https://localhost/
```

### Test 5: WebSocket depuis le navigateur

Ouvrir la console développeur du navigateur (F12) et exécuter :

```javascript
// Test MQTT WebSocket
const ws = new WebSocket('wss://symbion.markcha.fr/ws/mqtt');

ws.onopen = () => {
  console.log('✅ MQTT WebSocket connected!');
};

ws.onerror = (err) => {
  console.error('❌ MQTT WebSocket error:', err);
};

ws.onclose = () => {
  console.log('🔌 MQTT WebSocket closed');
};
```

## Logs de Debug

### Suivre les logs en temps réel

```bash
# Logs généraux Nginx
sudo tail -f /var/log/nginx/symbion-error.log

# Logs WebSocket MQTT spécifiques
sudo tail -f /var/log/nginx/mqtt-ws-error.log

# Logs access
sudo tail -f /var/log/nginx/symbion-access.log

# Logs Mosquitto
sudo tail -f /var/log/mosquitto/mosquitto.log

# Logs Kernel
tail -f /tmp/kernel.log
```

### Erreurs Communes

#### 1. `502 Bad Gateway` sur `/api/*`

**Cause**: Kernel non démarré ou port 8443 fermé

**Fix**:
```bash
# Vérifier que le kernel tourne
ps aux | grep symbion-kernel

# Relancer si besoin
cd symbion-kernel
SYMBION_API_KEY="s3cr3t-42" \
SYMBION_MQTT_BROKER="127.0.0.1:1883" \
SYMBION_JWT_SECRET="your-secret" \
cargo run --release
```

#### 2. `502 Bad Gateway` sur `/ws/mqtt`

**Cause**: Mosquitto non démarré ou port 9001 fermé

**Fix**:
```bash
# Vérifier mosquitto
sudo systemctl status mosquitto

# Redémarrer
sudo systemctl restart mosquitto

# Tester l'accès direct
curl -v http://localhost:9001/
```

#### 3. WebSocket `Connection: close` au lieu de `upgrade`

**Cause**: Map directive manquante

**Fix**:
```bash
# Vérifier que websocket-upgrade.conf existe
cat /etc/nginx/conf.d/websocket-upgrade.conf

# Si manquant, créer:
sudo tee /etc/nginx/conf.d/websocket-upgrade.conf > /dev/null <<'EOF'
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}
EOF

sudo nginx -t && sudo systemctl reload nginx
```

#### 4. `upstream prematurely closed connection`

**Cause**: HTTP/2 incompatible avec WebSocket sans `proxy_http_version 1.1`

**Fix**: La configuration fournie inclut déjà `proxy_http_version 1.1` dans la location `/ws/mqtt`. Vérifier qu'elle est bien présente.

## Certificats SSL Let's Encrypt (Production)

### Installation Certbot

```bash
sudo apt install certbot python3-certbot-nginx
```

### Générer certificats

```bash
# Stopper Nginx temporairement
sudo systemctl stop nginx

# Générer certificats (standalone mode)
sudo certbot certonly --standalone -d symbion.markcha.fr

# Redémarrer Nginx
sudo systemctl start nginx
```

### Modifier nginx-symbion.conf

Remplacer les lignes certificats par :

```nginx
ssl_certificate /etc/letsencrypt/live/symbion.markcha.fr/fullchain.pem;
ssl_certificate_key /etc/letsencrypt/live/symbion.markcha.fr/privkey.pem;
```

### Auto-renewal

```bash
# Tester le renouvellement
sudo certbot renew --dry-run

# Ajouter renouvellement automatique (cron)
sudo crontab -e

# Ajouter ligne:
0 3 * * * certbot renew --quiet --post-hook "systemctl reload nginx"
```

## Production: Build et serve PWA statique

Actuellement, Nginx proxy vers Vite dev server (port 3000). En production :

```bash
# Build PWA
cd pwa-dashboard
npm run build

# La configuration Nginx commentée sert le dossier dist/
# Décommenter la section "Production PWA" dans nginx-symbion.conf
```

## Monitoring

### Vérifier tous les services en un coup

```bash
# Script de monitoring rapide
cat > /tmp/symbion-check.sh <<'EOF'
#!/bin/bash
echo "=== Symbion Services Check ==="
echo ""
echo "1. Nginx:"
systemctl is-active nginx && echo "   ✅ Running" || echo "   ❌ Stopped"
echo ""
echo "2. Mosquitto:"
systemctl is-active mosquitto && echo "   ✅ Running" || echo "   ❌ Stopped"
echo ""
echo "3. Kernel (8443):"
ss -tlnp | grep :8443 > /dev/null && echo "   ✅ Listening" || echo "   ❌ Not listening"
echo ""
echo "4. PWA (3000):"
ss -tlnp | grep :3000 > /dev/null && echo "   ✅ Listening" || echo "   ❌ Not listening"
echo ""
echo "5. Health Check:"
curl -s -k https://localhost/health | jq .status 2>/dev/null || echo "   ❌ Failed"
EOF

chmod +x /tmp/symbion-check.sh
/tmp/symbion-check.sh
```

## Sécurité Firewall

```bash
# Bloquer accès direct aux ports (forcer passage par Nginx)
sudo ufw allow 443/tcp comment 'Nginx HTTPS'
sudo ufw allow 80/tcp comment 'Nginx HTTP redirect'

# Bloquer accès externe aux ports internes
sudo ufw deny 8443/tcp comment 'Block direct kernel access'
sudo ufw deny 9001/tcp comment 'Block direct MQTT access'
sudo ufw deny 3000/tcp comment 'Block direct PWA access'

# Autoriser localhost (127.0.0.1) toujours
sudo ufw reload
```

## Prochaines Étapes

1. ✅ Nginx configuré et testé en local
2. ⏳ Configurer reverse proxy sur NAS
3. ⏳ Tester accès externe via `symbion.markcha.fr`
4. ⏳ Activer Let's Encrypt en production
5. ⏳ Implémenter Zero Trust (Cloudflare Access ou autre)

---

**Créé le**: 2025-11-22
**Version**: 1.0.0
**Status**: Ready for Testing
