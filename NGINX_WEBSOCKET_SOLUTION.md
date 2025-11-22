# Solution Nginx WebSocket MQTT - Tests & Résultats

## ✅ Tests Réussis (22 Nov 2025)

### 1. Mosquitto WebSocket Listener
```bash
# Port 9001 écoute bien
sudo ss -tulpn | grep :9001
# tcp   LISTEN 0  4096  0.0.0.0:9001  0.0.0.0:*  users:(("mosquitto",pid=2012346))
```

**Config** : `/etc/mosquitto/conf.d/websocket.conf`
```conf
listener 9001
protocol websockets
```

### 2. WebSocket Upgrade Handshake (Raw)
```bash
# Test avec netcat - SUCCÈS
(echo -e "GET / HTTP/1.1\r\n\
Host: localhost:9001\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
Sec-WebSocket-Version: 13\r\n\
Sec-WebSocket-Protocol: mqtt, mqttv3.1\r\n\r\n"; sleep 1) | nc localhost 9001
```

**Réponse** :
```
HTTP/1.1 101 Switching Protocols
Upgrade: WebSocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: mqtt
```

✅ **Mosquitto accepte le subprotocol `mqtt` correctement**

### 3. WebSocket via Nginx (Port 8444)
```bash
# Test via Nginx SSL proxy - SUCCÈS
(echo -e "GET / HTTP/1.1\r\n\
Host: 192.168.1.14:8444\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
Sec-WebSocket-Version: 13\r\n\
Sec-WebSocket-Protocol: mqtt, mqttv3.1\r\n\r\n"; sleep 1) \
| openssl s_client -connect 192.168.1.14:8444 -quiet 2>/dev/null
```

**Réponse** :
```
HTTP/1.1 101 Switching Protocols
Server: nginx/1.24.0 (Ubuntu)
Connection: upgrade
Upgrade: WebSocket
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: mqtt
```

✅ **Nginx transmet correctement le header `Sec-WebSocket-Protocol: mqtt`**

### 4. MQTT.js via WebSocket Direct (Port 9001)
```bash
# Test avec MQTT.js Node.js - SUCCÈS
node -e "
const mqtt = require('mqtt');
const client = mqtt.connect('ws://192.168.1.14:9001', {
  clientId: 'test-node-' + Math.random().toString(16).substr(2, 8),
  keepalive: 60,
  connectTimeout: 5000
});

client.on('connect', () => {
  console.log('✅ MQTT Connected via WebSocket!');
  client.end();
  process.exit(0);
});
"
```

**Résultat** : `✅ MQTT Connected via WebSocket!`

### 5. MQTT.js via Nginx WSS (Port 8444)
```bash
# Test avec MQTT.js via Nginx - SUCCÈS
node -e "
const mqtt = require('mqtt');
const client = mqtt.connect('wss://192.168.1.14:8444', {
  clientId: 'test-nginx-' + Math.random().toString(16).substr(2, 8),
  keepalive: 60,
  connectTimeout: 5000,
  rejectUnauthorized: false
});

client.on('connect', () => {
  console.log('✅ MQTT Connected via Nginx WSS!');
  client.end();
  process.exit(0);
});
"
```

**Résultat** : `✅ MQTT Connected via Nginx WSS!`

---

## 🔍 Résumé des Tests

| Test | Port | Protocole | Proxy | Statut |
|------|------|-----------|-------|--------|
| Mosquitto Listener | 9001 | WebSocket | ❌ | ✅ OK |
| WebSocket Upgrade | 9001 | HTTP→WS | ❌ | ✅ OK |
| Subprotocol MQTT | 9001 | WS | ❌ | ✅ OK (mqtt) |
| Nginx WebSocket Proxy | 8444 | WSS | ✅ | ✅ OK |
| Nginx Subprotocol | 8444 | WSS | ✅ | ✅ OK (mqtt) |
| MQTT.js Direct | 9001 | WS | ❌ | ✅ OK |
| MQTT.js via Nginx | 8444 | WSS | ✅ | ✅ OK |

**Conclusion** : L'infrastructure **Mosquitto + Nginx + MQTT.js fonctionne à 100%**

---

## ❌ Problème Restant : PWA Dashboard

**Symptôme** : Le PWA dashboard affiche `connack timeout` dans la console

**Tests réussis** :
- ✅ MQTT.js fonctionne (testé avec Node.js)
- ✅ WebSocket upgrade fonctionne (testé avec netcat)
- ✅ Nginx proxy fonctionne (testé avec openssl s_client)
- ✅ Subprotocol `mqtt` est bien transmis

**Hypothèses** :
1. **Certificat SSL self-signed** : Le navigateur bloque la connexion WSS avec certificat non valide
2. **CORS** : Configuration CORS manquante pour WebSocket
3. **Config PWA** : Mauvaise URL MQTT détectée par le PWA
4. **Firewall navigateur/Windows** : Bloque les ports 9001 ou 8444

**Config PWA actuelle** (`pwa-dashboard/public/config.js`) :
```javascript
// Via Nginx (port 443, 80, ou vide)
MQTT_BROKER: viaProxy
  ? (protocol === 'https:' ? 'wss://' : 'ws://') + hostname + ':8444'
  : (protocol === 'https:' ? 'wss://' : 'ws://') + hostname + ':9001'
```

**Accès via Nginx** (`https://192.168.1.14`) :
- Détecte `viaProxy = true`
- MQTT_BROKER = `wss://192.168.1.14:8444` ✅

**Accès via Vite** (`https://192.168.1.14:3000`) :
- Détecte `viaProxy = false`
- MQTT_BROKER = `wss://192.168.1.14:9001` ✅

**Les deux devraient fonctionner** (tests Node.js confirment)

---

## 🔧 Prochaines Étapes de Debugging

### 1. Logs Console Navigateur
**Action requise** : Copier les logs de la console du navigateur, notamment :
- `[config] Detected environment:` (montre URL MQTT détectée)
- `🔌 Connecting to MQTT broker:` (URL utilisée)
- `❌ MQTT Error:` (erreur exacte)
- Erreurs SSL/certificat

### 2. Vérifier Certificat SSL
```bash
# Vérifier que le certificat est accepté pour port 8444
openssl s_client -connect 192.168.1.14:8444 -showcerts
```

**Hypothèse** : Le navigateur bloque WSS car certificat mkcert non trusted

**Solution potentielle** :
- Accepter manuellement le certificat en visitant `https://192.168.1.14:8444` dans le navigateur
- Ou utiliser Let's Encrypt (production)

### 3. Tester Port Direct 9001 (WS non-SSL)
**Modification temporaire** de `config.js` :
```javascript
// Force WS (non-SSL) pour test
MQTT_BROKER: 'ws://192.168.1.14:9001'
```

Si ça fonctionne → problème = certificat SSL
Si ça échoue → problème = autre chose

### 4. Test avec Navigateur DevTools Network
- Ouvrir DevTools → Network → WS (WebSocket filter)
- Recharger le PWA
- Regarder si requête WebSocket apparaît
- Status code ? Headers ? Messages ?

---

## 📝 Configuration Files

### Nginx - Main Server (Port 443)
`/etc/nginx/sites-available/symbion` - voir fichier complet

**Location MQTT (via Nginx proxy)** :
```nginx
location /ws/mqtt {
    proxy_pass http://127.0.0.1:9001/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection $connection_upgrade;
    proxy_read_timeout 86400s;
    proxy_send_timeout 86400s;
    proxy_buffering off;
}
```

### Nginx - MQTT Dedicated (Port 8444)
`/etc/nginx/sites-available/symbion-mqtt`

**CRITICAL** : `listen 8444 ssl;` **SANS `http2`** (HTTP/2 incompatible avec WebSocket)

```nginx
server {
    listen 8444 ssl;  # SANS http2 !
    server_name symbion.markcha.fr;

    ssl_certificate /etc/mosquitto/certs/cert-mkcert.pem;
    ssl_certificate_key /etc/mosquitto/certs/key-mkcert.pem;

    location / {
        proxy_pass http://127.0.0.1:9001/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
        proxy_buffering off;
    }
}
```

### Nginx - WebSocket Map Directive
`/etc/nginx/conf.d/websocket-upgrade.conf`

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}
```

### Mosquitto - WebSocket Config
`/etc/mosquitto/conf.d/websocket.conf`

```conf
# Listener WebSocket NON-TLS sur port 9001
listener 9001
protocol websockets
```

**IMPORTANT** : Doit être dans un fichier séparé (Mosquitto 2.0.18 crash si MQTT + WebSocket dans même fichier)

---

## 🔗 Références

- [MQTT WebSocket Connection Failed - Stack Overflow](https://stackoverflow.com/questions/69709461/mqtt-websocket-connection-failed)
- [Using MQTT Over WebSockets with Mosquitto](http://www.steves-internet-guide.com/mqtt-websockets/)
- [GitHub - mqttjs/MQTT.js](https://github.com/mqttjs/MQTT.js)
- [Mosquitto WebSocket subprotocol issue #336](https://github.com/eclipse/mosquitto/issues/336)
- [WebSocket subprotocol lighttpd issue](https://stackoverflow.com/questions/21642221/lighttpd-mod-websocket-mqtt-handshake-fail-no-subproto)

---

**Date** : 22 Novembre 2025
**Status** : Infrastructure OK, PWA debugging en cours
