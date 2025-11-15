# Troubleshooting - Symbion

Guide de résolution des problèmes courants pour l'écosystème Symbion.

---

## 🔍 Diagnostic Rapide

### Vérification État Système

```bash
# 1. Kernel actif ?
curl -k https://localhost:8443/health
# Attendu : {"status":"healthy","uptime_seconds":...}

# 2. MQTT broker actif ?
sudo systemctl status mosquitto
# Attendu : active (running)

# 3. Agents en ligne ?
curl -k https://localhost:8443/agents
# Attendu : Liste agents avec status "online"

# 4. Ports écoutent ?
sudo netstat -tulpn | grep -E '8080|8443|1883|9001'
# Attendu : 4 lignes avec LISTEN
```

---

## 🧬 Kernel - Symbion Kernel

### Symptôme : Kernel ne démarre pas

**Diagnostic** :
```bash
# Vérifier logs systemd
sudo journalctl -u symbion-kernel -n 50 --no-pager

# Vérifier logs fichier
tail -100 /tmp/kernel.log

# Tester lancement manuel
cd /home/symbion/NewSymbion/symbion-kernel
SYMBION_API_KEY="test" \
SYMBION_MQTT_BROKER="127.0.0.1:1883" \
SYMBION_JWT_SECRET="test-secret-min-64-chars-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" \
cargo run --release
```

**Causes Fréquentes** :

#### 1. Variables d'environnement manquantes

**Erreur** :
```
Error: Missing SYMBION_API_KEY environment variable
```

**Solution** :
```bash
# Vérifier /etc/symbion/kernel.env
cat /etc/symbion/kernel.env

# S'assurer que systemd charge le fichier
sudo systemctl edit symbion-kernel
# Ajouter :
[Service]
EnvironmentFile=/etc/symbion/kernel.env
```

#### 2. JWT secret trop court

**Erreur** :
```
Error: JWT secret must be at least 64 characters
```

**Solution** :
```bash
# Générer nouveau secret sécurisé
openssl rand -hex 64

# Mettre à jour /etc/symbion/kernel.env
SYMBION_JWT_SECRET="<nouveau_secret_128_caractères>"

# Redémarrer
sudo systemctl restart symbion-kernel
```

#### 3. Certificats TLS manquants ou invalides

**Erreur** :
```
Error: Failed to load TLS certificate: No such file or directory
```

**Solution** :
```bash
# Vérifier fichiers existent
ls -lh certs/cert.pem certs/key.pem

# Permissions correctes
sudo chown symbion:symbion certs/*.pem
chmod 600 certs/key.pem
chmod 644 certs/cert.pem

# Tester certificat valide
openssl x509 -in certs/cert.pem -text -noout
# Vérifier dates Valid from/to

# Régénérer si expiré (développement)
mkcert localhost 127.0.0.1 ::1
mv localhost+2.pem certs/cert.pem
mv localhost+2-key.pem certs/key.pem

# Régénérer si expiré (production)
sudo certbot renew
sudo cp /etc/letsencrypt/live/symbion.domain.com/fullchain.pem certs/cert.pem
sudo cp /etc/letsencrypt/live/symbion.domain.com/privkey.pem certs/key.pem
sudo chown symbion:symbion certs/*.pem
```

#### 4. Port 8443 déjà utilisé

**Erreur** :
```
Error: Address already in use (os error 98)
```

**Solution** :
```bash
# Identifier processus utilisant le port
sudo lsof -i :8443

# Tuer processus en conflit
sudo kill -9 <PID>

# Ou changer port dans code (http.rs)
# Puis recompiler
cargo build --release
```

#### 5. MQTT broker inaccessible

**Erreur** :
```
Error: Failed to connect to MQTT broker at 127.0.0.1:1883
```

**Solution** :
```bash
# Vérifier Mosquitto actif
sudo systemctl status mosquitto

# Démarrer si arrêté
sudo systemctl start mosquitto

# Tester connexion manuelle
mosquitto_sub -h 127.0.0.1 -t 'test' -v
# Si erreur "Connection refused" → broker pas démarré
# Si timeout → firewall bloque

# Vérifier logs broker
sudo tail -50 /var/log/mosquitto/mosquitto.log
```

---

### Symptôme : Kernel crashe au démarrage

**Diagnostic** :
```bash
# Stack trace dans logs
sudo journalctl -u symbion-kernel -n 200 | grep -A 20 "panic\|PANIC"

# Vérifier intégrité binary
md5sum target/release/symbion-kernel

# Recompiler propre
cargo clean
cargo build --release
```

**Causes Fréquentes** :

#### 1. Corruption fichiers JSON (users.json, agents.json)

**Erreur** :
```
thread 'main' panicked at 'Failed to parse users.json: EOF while parsing'
```

**Solution** :
```bash
# Restaurer backup
cp users.json.backup users.json

# Ou réinitialiser (ATTENTION : perd données)
echo '{"users":[]}' > users.json
echo '{"agents":[]}' > agents.json

# Redémarrer
sudo systemctl restart symbion-kernel
```

#### 2. Dépendances système manquantes

**Erreur** :
```
error while loading shared libraries: libssl.so.3
```

**Solution** :
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install libssl3 libssl-dev

# Fedora/RHEL
sudo dnf install openssl-libs openssl-devel

# Recompiler
cargo build --release
```

---

### Symptôme : Kernel répond lentement

**Diagnostic** :
```bash
# Vérifier charge CPU/RAM
curl -k https://localhost:8443/v1/metrics/system | jq

# Logs performances
tail -f /tmp/kernel.log | grep -i "slow\|timeout\|latency"

# Connexions actives
ss -tn | grep :8443 | wc -l
```

**Causes Fréquentes** :

#### 1. Trop de connexions simultanées

**Solution** : Rate limiting déjà actif (5 req/sec), vérifier DoS

#### 2. Logs trop verbeux

**Solution** :
```bash
# Réduire niveau logs
export RUST_LOG=error  # Au lieu de debug
sudo systemctl restart symbion-kernel
```

#### 3. Disque saturé

**Solution** :
```bash
# Vérifier espace
df -h

# Nettoyer logs anciens
sudo journalctl --vacuum-time=7d

# Rotation logs
sudo logrotate -f /etc/logrotate.d/symbion
```

---

## 📡 MQTT Broker - Mosquitto

### Symptôme : Broker ne démarre pas

**Diagnostic** :
```bash
# Status service
sudo systemctl status mosquitto

# Logs détaillés
sudo journalctl -u mosquitto -n 100 --no-pager

# Tester config
mosquitto -c /etc/mosquitto/mosquitto.conf -v
```

**Causes Fréquentes** :

#### 1. Port 1883 déjà utilisé

**Erreur** :
```
Error: Address already in use
```

**Solution** :
```bash
# Identifier processus
sudo lsof -i :1883

# Tuer ancien processus
sudo pkill mosquitto
sudo systemctl start mosquitto
```

#### 2. Certificats TLS invalides (WebSocket :9001)

**Erreur** :
```
Error loading certificate file /etc/mosquitto/certs/cert-mkcert.pem
```

**Solution** :
```bash
# Vérifier fichiers
ls -lh /etc/mosquitto/certs/

# Permissions correctes
sudo chown mosquitto:mosquitto /etc/mosquitto/certs/*.pem
sudo chmod 644 /etc/mosquitto/certs/*.crt
sudo chmod 644 /etc/mosquitto/certs/*.pem
sudo chmod 600 /etc/mosquitto/certs/key-mkcert.pem

# Tester certificat
openssl x509 -in /etc/mosquitto/certs/cert-mkcert.pem -text -noout
```

#### 3. Configuration syntaxe invalide

**Erreur** :
```
Error: Unknown configuration variable "protocole"
```

**Solution** :
```bash
# Vérifier config
cat /etc/mosquitto/conf.d/websocket.conf

# Format attendu :
listener 1883 0.0.0.0
protocol mqtt
allow_anonymous true

listener 9001 0.0.0.0
protocol websockets
allow_anonymous true
cafile /etc/mosquitto/certs/symbion-ca.crt
certfile /etc/mosquitto/certs/cert-mkcert.pem
keyfile /etc/mosquitto/certs/key-mkcert.pem

# Redémarrer
sudo systemctl restart mosquitto
```

---

### Symptôme : Clients ne peuvent pas se connecter

**Diagnostic** :
```bash
# Test connexion locale
mosquitto_sub -h localhost -t 'test' -v

# Test connexion réseau
mosquitto_sub -h 192.168.1.100 -t 'test' -v

# Vérifier firewall
sudo ufw status | grep 1883
```

**Causes Fréquentes** :

#### 1. Firewall bloque port 1883

**Solution** :
```bash
# Autoriser port (uniquement si agents externes)
sudo ufw allow from 192.168.1.0/24 to any port 1883

# Redémarrer firewall
sudo ufw reload
```

#### 2. Broker écoute uniquement 127.0.0.1

**Solution** :
```bash
# Vérifier listener
sudo netstat -tulpn | grep 1883
# Attendu : 0.0.0.0:1883 (pas 127.0.0.1:1883)

# Corriger config
sudo nano /etc/mosquitto/conf.d/websocket.conf
# Changer :
listener 1883 0.0.0.0  # Pas 127.0.0.1

sudo systemctl restart mosquitto
```

---

## 🤖 Agents - Symbion Agent Host

### Symptôme : Agent ne se connecte pas au kernel

**Diagnostic** :
```bash
# Logs agent
tail -50 ~/NewSymbion/symbion-agent-host/agent.log

# Tester connectivité réseau
ping <kernel_ip>
telnet <kernel_ip> 1883

# Vérifier DNS/résolution
nslookup symbion-kernel.local
```

**Causes Fréquentes** :

#### 1. Variable SYMBION_KERNEL_HOST incorrecte

**Erreur** :
```
Error: Failed to connect to MQTT broker at 127.0.0.1:1883
```

**Solution** :
```bash
# Spécifier IP kernel correcte
export SYMBION_KERNEL_HOST="192.168.1.100:1883"
./target/release/symbion-agent-host

# Ou dans systemd service
sudo systemctl edit symbion-agent-host
# Ajouter :
[Service]
Environment="SYMBION_KERNEL_HOST=192.168.1.100:1883"
```

#### 2. Agent ne trouve pas le kernel (auto-discovery échoue)

**Solution** :
```bash
# Mode manuel au lieu d'auto-discovery
SYMBION_KERNEL_HOST="192.168.1.100:1883" \
  ./target/release/symbion-agent-host
```

#### 3. Certificats réseau non fiables

**Solution** :
```bash
# Pour TLS broker (si configuré)
# Copier CA vers agent
scp kernel:/etc/mosquitto/certs/symbion-ca.crt ~/symbion-ca.crt

# Configurer agent pour utiliser CA
# (Actuellement non TLS sur port 1883)
```

---

### Symptôme : Agent apparaît offline dans dashboard

**Diagnostic** :
```bash
# Vérifier heartbeat agent
mosquitto_sub -h localhost -t 'symbion/agents/heartbeat@v1' -v

# Vérifier registry kernel
curl -k https://localhost:8443/agents | jq

# Logs kernel pour heartbeat timeout
tail -f /tmp/kernel.log | grep heartbeat
```

**Causes Fréquentes** :

#### 1. Agent_id en conflit

**Solution** :
```bash
# Chaque agent doit avoir ID unique
# Vérifier agent_id dans logs
tail agent.log | grep agent_id

# Supprimer ancien agent du registry si dupliqué
curl -X DELETE -k https://localhost:8443/agents/<agent_id>
```

#### 2. Heartbeat timeout (> 90s)

**Solution** :
```bash
# Redémarrer agent
sudo systemctl restart symbion-agent-host

# Vérifier heartbeat arrive
mosquitto_sub -t 'symbion/agents/heartbeat@v1' -C 1 -v
```

---

## 📱 PWA Dashboard

### Symptôme : PWA ne charge pas

**Diagnostic** :
```bash
# Dev server actif ?
curl http://localhost:3000

# Vérifier processus Vite
ps aux | grep vite

# Logs build
cd pwa-dashboard
npm run dev
```

**Causes Fréquentes** :

#### 1. Dépendances npm manquantes

**Erreur** :
```
Module not found: Error: Can't resolve 'lit'
```

**Solution** :
```bash
cd pwa-dashboard
rm -rf node_modules package-lock.json
npm install
npm run dev
```

#### 2. Port 3000 déjà utilisé

**Solution** :
```bash
# Tuer processus sur port 3000
sudo lsof -i :3000
sudo kill -9 <PID>

# Ou changer port
npm run dev -- --port 3001
```

---

### Symptôme : PWA ne se connecte pas au kernel

**Diagnostic** :
```bash
# Tester API depuis navigateur console
fetch('https://localhost:8443/health', {method: 'GET'})
  .then(r => r.json())
  .then(console.log)

# Vérifier CORS
curl -k -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: GET" \
  -X OPTIONS https://localhost:8443/health -v
```

**Causes Fréquentes** :

#### 1. Certificat auto-signé non accepté

**Erreur Console** :
```
net::ERR_CERT_AUTHORITY_INVALID
```

**Solution** :
```bash
# Navigateur : Visiter https://localhost:8443/health
# Accepter certificat manuellement (Advanced → Proceed)

# Ou télécharger CA mkcert
mkcert -install  # Sur machine développement
```

#### 2. CORS bloque requête

**Erreur Console** :
```
Access to fetch at 'https://localhost:8443/agents' has been blocked by CORS
```

**Solution** :
```bash
# Vérifier origin PWA autorisé dans kernel
# symbion-kernel/src/http.rs
# CorsLayer should allow http://localhost:3000

# Redémarrer kernel si modifié
```

---

### Symptôme : MQTT WebSocket ne se connecte pas

**Diagnostic** :
```bash
# Tester WebSocket manuellement
# Navigateur console :
const ws = new WebSocket('wss://localhost:9001');
ws.onopen = () => console.log('Connected');
ws.onerror = (e) => console.error(e);

# Vérifier broker écoute :9001
sudo netstat -tulpn | grep 9001
```

**Causes Fréquentes** :

#### 1. Broker WebSocket pas configuré

**Solution** :
```bash
# Vérifier /etc/mosquitto/conf.d/websocket.conf
cat /etc/mosquitto/conf.d/websocket.conf

# Doit contenir :
listener 9001 0.0.0.0
protocol websockets

# Redémarrer
sudo systemctl restart mosquitto
```

#### 2. Certificat TLS WebSocket invalide

**Erreur Console** :
```
WebSocket connection failed: SSL certificate problem
```

**Solution** :
```bash
# Vérifier certificat WSS
openssl s_client -connect localhost:9001

# Régénérer si expiré
mkcert localhost 127.0.0.1 ::1
sudo cp localhost+2.pem /etc/mosquitto/certs/cert-mkcert.pem
sudo cp localhost+2-key.pem /etc/mosquitto/certs/key-mkcert.pem
sudo systemctl restart mosquitto
```

---

## 🔥 Firewall & Réseau

### Symptôme : Connexions bloquées entre composants

**Diagnostic** :
```bash
# Vérifier règles UFW
sudo ufw status numbered

# Tester connectivité
telnet <ip> <port>

# Logs firewall
sudo tail -f /var/log/ufw.log | grep BLOCK
```

**Solution** :
```bash
# Autoriser ports Symbion (ajuster selon architecture)
sudo ufw allow 8080/tcp   # HTTP redirect
sudo ufw allow 8443/tcp   # HTTPS API
sudo ufw allow 1883/tcp   # MQTT (si agents externes)
sudo ufw allow 9001/tcp   # MQTT WSS

# Reload
sudo ufw reload
```

---

## 🛠️ Commandes Utiles

### Diagnostic Complet

```bash
#!/bin/bash
# symbion-diagnostic.sh

echo "=== SYMBION DIAGNOSTIC ==="
echo

echo "1. Services Status"
systemctl is-active symbion-kernel mosquitto
echo

echo "2. Ports Listening"
sudo netstat -tulpn | grep -E '8080|8443|1883|9001'
echo

echo "3. Kernel Health"
curl -k https://localhost:8443/health 2>/dev/null | jq
echo

echo "4. Agents Online"
curl -k https://localhost:8443/agents 2>/dev/null | jq '.[] | {id, status}'
echo

echo "5. MQTT Connectivity"
timeout 2 mosquitto_sub -h localhost -t 'symbion/#' -C 1 -v
echo

echo "6. Disk Space"
df -h / | tail -1
echo

echo "7. Recent Errors (Kernel)"
sudo journalctl -u symbion-kernel -n 10 --no-pager | grep -i error
echo

echo "=== END DIAGNOSTIC ==="
```

### Logs Streaming

```bash
# Kernel + MQTT logs côte à côte
sudo journalctl -u symbion-kernel -u mosquitto -f

# Filtrer erreurs uniquement
sudo journalctl -u symbion-kernel -p err -f

# Logs avec timestamps précis
sudo journalctl -u symbion-kernel --since "5 minutes ago" -o short-precise
```

### Nettoyage/Reset

```bash
# Reset complet (ATTENTION : perd données)
sudo systemctl stop symbion-kernel
rm users.json agents.json
echo '{"users":[]}' > users.json
echo '{"agents":[]}' > agents.json
sudo systemctl start symbion-kernel

# Purge logs
sudo journalctl --vacuum-time=1d

# Rebuild propre
cd symbion-kernel
cargo clean
cargo build --release
```

---

## 📊 Messages d'Erreur Fréquents

### `JWT signature verification failed`

**Cause** : Secret JWT changé, tokens invalides

**Solution** :
```bash
# Effacer cookies/localStorage navigateur
# Ou re-login via /login
```

### `Rate limit exceeded`

**Cause** : > 5 requêtes/sec depuis même IP

**Solution** : Attendre 60 secondes, optimiser client

### `MQTT message too large`

**Cause** : Payload MQTT > 256MB (limite Mosquitto par défaut)

**Solution** :
```bash
# Augmenter limite broker
sudo nano /etc/mosquitto/mosquitto.conf
# Ajouter :
message_size_limit 0  # Illimité (déconseillé)
# Ou :
message_size_limit 10485760  # 10MB

sudo systemctl restart mosquitto
```

### `Failed to bind address: Permission denied`

**Cause** : Ports < 1024 nécessitent root, ou port déjà utilisé

**Solution** :
```bash
# Option 1: Capabilities Linux
sudo setcap 'cap_net_bind_service=+ep' target/release/symbion-kernel

# Option 2: Systemd gère permissions
# (Déjà configuré dans symbion-kernel.service)

# Option 3: Utiliser ports > 1024
# (8080, 8443 OK - 80, 443 nécessitent root)
```

---

## 📚 Références

- **Architecture** : [docs/architecture/SYSTEM_OVERVIEW.md](architecture/SYSTEM_OVERVIEW.md)
- **Deployment** : [docs/DEPLOYMENT.md](DEPLOYMENT.md)
- **API Endpoints** : [docs/api/endpoints.md](api/endpoints.md)
- **MQTT Topics** : [docs/mqtt/topics.md](mqtt/topics.md)

---

## 🆘 Support & Logs

### Rapporter un Bug

Inclure dans le rapport :

```bash
# 1. Version système
uname -a
rustc --version

# 2. Logs kernel (100 dernières lignes)
sudo journalctl -u symbion-kernel -n 100 --no-pager > kernel.log

# 3. Logs MQTT
sudo journalctl -u mosquitto -n 50 --no-pager > mqtt.log

# 4. Configuration
cat /etc/symbion/kernel.env | sed 's/\(SECRET\|KEY\)=.*/\1=REDACTED/'

# 5. Diagnostic complet
./symbion-diagnostic.sh > diagnostic.txt
```

Envoyer à : issues@symbion.local ou GitHub Issues

---

**Dernière mise à jour** : 15 Novembre 2025
