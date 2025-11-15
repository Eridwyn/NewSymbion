# Deployment Checklist - Symbion Production

Guide de déploiement production pour l'écosystème Symbion.

---

## ✅ Pré-Déploiement

### Infrastructure
- [ ] **Serveur Linux**: Ubuntu 22.04+ LTS ou Debian 12+
- [ ] **RAM Minimum**: 512MB (recommandé : 1GB+)
- [ ] **Stockage**: 2GB disponible (système + logs)
- [ ] **Ports Ouverts**:
  - 8080 (HTTP redirect)
  - 8443 (HTTPS API)
  - 1883 (MQTT local, non exposé)
  - 9001 (MQTT WSS, optionnel PWA externe)

### Dépendances
- [ ] **Rust**: 1.75+ installé (`rustc --version`)
- [ ] **Mosquitto**: 2.0+ installé et démarré
- [ ] **Node.js**: 18+ pour build PWA (optionnel si pré-build)
- [ ] **Let's Encrypt**: Certbot installé pour TLS prod

### Sécurité
- [ ] **Firewall**: UFW ou iptables configuré
- [ ] **Fail2ban**: Installé + règles SSH
- [ ] **Users**: Utilisateur `symbion` non-root créé
- [ ] **Permissions**: `symbion` peut bind ports 8080/8443 (via systemd)

---

## 🔧 Installation Kernel

### 1. Clone & Build

```bash
# En tant qu'utilisateur symbion
git clone https://github.com/Eridwyn/NewSymbion.git
cd NewSymbion/symbion-kernel
cargo build --release

# Vérifier binary
ls -lh target/release/symbion-kernel
```

### 2. Configuration TLS

**Production (Let's Encrypt)** :
```bash
# Obtenir certificat (remplacer DOMAIN par votre domaine)
sudo certbot certonly --standalone -d symbion.votredomaine.com

# Copier vers projet
mkdir -p certs/
sudo cp /etc/letsencrypt/live/symbion.votredomaine.com/fullchain.pem certs/cert.pem
sudo cp /etc/letsencrypt/live/symbion.votredomaine.com/privkey.pem certs/key.pem
sudo chown symbion:symbion certs/*.pem
```

**Développement (mkcert)** :
```bash
# Installer mkcert
brew install mkcert  # macOS
# ou apt install mkcert  # Linux

# Générer CA local
mkcert -install
mkcert localhost 127.0.0.1 ::1

# Copier
mkdir -p certs/
mv localhost+2.pem certs/cert.pem
mv localhost+2-key.pem certs/key.pem
```

### 3. Variables Environnement

Créer `/etc/symbion/kernel.env` :
```env
SYMBION_API_KEY="CHANGEME-PRODUCTION-KEY-MINIMUM-32-CHARS"
SYMBION_MQTT_BROKER="127.0.0.1:1883"
SYMBION_JWT_SECRET="CHANGEME-PRODUCTION-SECRET-MINIMUM-64-CHARS-FOR-HS256"
SYMBION_TLS_CERT_PATH="/home/symbion/NewSymbion/symbion-kernel/certs/cert.pem"
SYMBION_TLS_KEY_PATH="/home/symbion/NewSymbion/symbion-kernel/certs/key.pem"
SYMBION_TOKEN_EXPIRY_HOURS="8"
```

**IMPORTANT** : Générer secrets sécurisés :
```bash
# API Key
openssl rand -hex 32

# JWT Secret
openssl rand -hex 64
```

### 4. Systemd Service

Copier `symbion-kernel.service` :
```bash
sudo cp symbion-kernel.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable symbion-kernel
sudo systemctl start symbion-kernel
```

Vérifier :
```bash
sudo systemctl status symbion-kernel
journalctl -u symbion-kernel -f
```

---

## 🤖 Déploiement Agents

### Agent sur chaque machine domestique

```bash
# Clone (ou copier binary pré-compilé)
git clone https://github.com/Eridwyn/NewSymbion.git
cd NewSymbion/symbion-agent-host

# Build
cargo build --release

# Lancer (en tant que service ou nohup)
SYMBION_KERNEL_HOST="192.168.1.100:1883" \
  nohup ./target/release/symbion-agent-host > agent.log 2>&1 &
```

**Systemd agent (optionnel)** :
```ini
[Unit]
Description=Symbion Agent Host
After=network.target

[Service]
Type=simple
User=votreuser
WorkingDirectory=/home/votreuser/NewSymbion/symbion-agent-host
Environment="SYMBION_KERNEL_HOST=192.168.1.100:1883"
ExecStart=/home/votreuser/NewSymbion/symbion-agent-host/target/release/symbion-agent-host
Restart=always
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

---

## 📱 Déploiement PWA

### Build Production

```bash
cd pwa-dashboard
npm install
npm run build

# Output dans dist/
ls -lh dist/
```

### Servir via Kernel (intégré)

Le kernel sert automatiquement `pwa-dashboard/dist/` sur HTTPS :8443.

**OU** Nginx (optionnel) :
```nginx
server {
    listen 443 ssl http2;
    server_name pwa.symbion.votredomaine.com;

    ssl_certificate /etc/letsencrypt/live/pwa.symbion.votredomaine.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/pwa.symbion.votredomaine.com/privkey.pem;

    root /home/symbion/NewSymbion/pwa-dashboard/dist;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    # Proxy API vers kernel
    location /api/ {
        proxy_pass https://localhost:8443;
        proxy_ssl_verify off;
    }
}
```

---

## 🚦 Post-Déploiement

### Health Checks

```bash
# Kernel santé
curl -k https://localhost:8443/health

# MQTT actif
mosquitto_sub -t 'symbion/#' -v

# Agents online
curl -k https://localhost:8443/agents
```

### Monitoring

#### Cron Monitoring (déjà configuré)
```bash
# Vérifier crontab
crontab -l | grep monitor-symbion

# Tester script
./scripts/monitor-symbion.sh
```

#### Logs
```bash
# Kernel logs
journalctl -u symbion-kernel -f

# MQTT broker
journalctl -u mosquitto -f

# Agent logs
tail -f ~/NewSymbion/symbion-agent-host/agent.log
```

---

## 🔐 Sécurité Production

### Firewall (UFW)
```bash
sudo ufw allow 22/tcp  # SSH
sudo ufw allow 80/tcp  # HTTP redirect
sudo ufw allow 443/tcp # HTTPS
sudo ufw allow 8443/tcp # Kernel HTTPS
sudo ufw deny 1883/tcp # MQTT local only
sudo ufw enable
```

### Rate Limiting (déjà intégré)
- 5 requêtes/sec par IP (kernel)
- Fail2ban pour SSH (recommandé)

### Secrets Rotation
- Régénérer `SYMBION_JWT_SECRET` tous les 6 mois
- Régénérer `SYMBION_API_KEY` tous les 12 mois
- Redémarrer kernel après changement : `sudo systemctl restart symbion-kernel`

---

## 📊 Checklist Finale

### Kernel
- [ ] Systemd service actif et enabled
- [ ] Health check HTTPS répond
- [ ] Logs sans erreurs critiques
- [ ] TLS certificat valide (Let's Encrypt ou mkcert)
- [ ] Secrets générés aléatoirement (pas de valeurs par défaut)

### MQTT
- [ ] Mosquitto actif
- [ ] Port 1883 écoute (local only)
- [ ] Agents peuvent se connecter

### Agents
- [ ] Au moins 1 agent online
- [ ] Heartbeats reçus (check dashboard ou API /agents)
- [ ] Télémétrie remontée (CPU, RAM, disk)

### PWA
- [ ] Build production généré (dist/)
- [ ] Accessible via HTTPS (kernel ou Nginx)
- [ ] MQTT WebSocket connecté (WSS :9001 si externe)

### Monitoring
- [ ] Cron monitoring actif (15 min)
- [ ] Emails alertes configurés
- [ ] Logs rotationnés (journalctl ou logrotate)

### Backups
- [ ] Cron backup daily pour :
  - `/home/symbion/NewSymbion/symbion-kernel/users.json`
  - `/home/symbion/NewSymbion/symbion-kernel/agents.json`
  - Certificats TLS (certs/)
- [ ] Backup stockés hors serveur (NAS, cloud)

---

## 🆘 Rollback Procédure

En cas de problème critique :

```bash
# 1. Arrêter nouveau kernel
sudo systemctl stop symbion-kernel

# 2. Restaurer version précédente
cd /home/symbion/NewSymbion
git checkout <commit-stable>
cargo build --release -p symbion-kernel

# 3. Redémarrer
sudo systemctl start symbion-kernel

# 4. Vérifier health
curl -k https://localhost:8443/health
```

---

## 📚 Références

- **Architecture**: [docs/architecture/SYSTEM_OVERVIEW.md](architecture/SYSTEM_OVERVIEW.md)
- **API**: [docs/api/endpoints.md](api/endpoints.md)
- **MQTT**: [docs/mqtt/topics.md](mqtt/topics.md)
- **Troubleshooting**: [docs/TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

**Dernière mise à jour** : 15 Novembre 2025
