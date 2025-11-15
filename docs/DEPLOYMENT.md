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

## 🔄 Migration & Upgrade Strategy

### Versioning Symbion

**Semantic Versioning** : `MAJOR.MINOR.PATCH` (ex: `1.2.3`)
- **MAJOR** : Breaking changes (API, MQTT topics, database schema)
- **MINOR** : New features (backward compatible)
- **PATCH** : Bug fixes, documentation

**Version actuelle** : `1.1.7` (Novembre 2025)

### Upgrade Path

#### Minor/Patch Upgrades (Safe, Rolling)

**Exemple** : `1.1.7` → `1.2.0` ou `1.1.8`

```bash
# 1. Backup données
./scripts/backup-symbion.sh

# 2. Pull nouvelle version
cd /home/symbion/NewSymbion
git fetch origin
git checkout v1.2.0  # Tag version

# 3. Build
cargo build --release -p symbion-kernel

# 4. Restart (downtime < 5s)
sudo systemctl restart symbion-kernel

# 5. Vérifier santé
curl -k https://localhost:8443/health
tail -f /var/log/syslog | grep symbion-kernel
```

**Backward Compatibility** :
- ✅ MQTT topics `@v1` continuent de fonctionner
- ✅ API HTTP endpoints inchangés
- ✅ JSON data files format compatible

#### Major Upgrades (Breaking Changes)

**Exemple** : `1.x.x` → `2.0.0`

**⚠️ Requires Planning** :
- MQTT topics versioning (`@v1` → `@v2` coexistence)
- API backward compatibility window (6 mois minimum)
- Database schema migration (JSON → SQLite/Postgres)

**Migration Steps** :

```bash
# 1. READ CHANGELOG & MIGRATION GUIDE
cat docs/CHANGELOG.md | grep "BREAKING"
cat docs/MIGRATION_v2.md  # Version-specific guide

# 2. Test migration sur environnement staging
git clone /home/symbion/NewSymbion /tmp/symbion-staging
cd /tmp/symbion-staging
git checkout v2.0.0

# Build & run on different port
SYMBION_API_PORT=9443 cargo run --release

# Test endpoints
curl -k https://localhost:9443/health

# 3. Backup production
./scripts/backup-symbion.sh --full

# 4. Maintenance mode (optionnel)
sudo systemctl stop symbion-kernel

# 5. Run migration scripts
./scripts/migrate-v1-to-v2.sh
# Exemple : Convertir users.json → SQLite
# sqlite3 symbion.db < migrations/001_users.sql

# 6. Deploy nouvelle version
git checkout v2.0.0
cargo build --release -p symbion-kernel

# 7. Start nouveau kernel
sudo systemctl start symbion-kernel

# 8. Monitor logs intensivement (24-48h)
journalctl -u symbion-kernel -f

# 9. Rollback si problèmes critiques
# (Voir section Rollback)
```

### MQTT Topic Version Migration

**Stratégie Coexistence** : Kernel supporte `@v1` et `@v2` simultanément

**Exemple Migration** : `symbion/agents/heartbeat@v1` → `@v2`

```rust
// Phase 1 : Kernel accepte les deux versions (6 mois)
client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await?;
client.subscribe("symbion/agents/heartbeat@v2", QoS::AtLeastOnce).await?;

match topic.as_str() {
    "symbion/agents/heartbeat@v1" => handle_heartbeat_v1(payload),
    "symbion/agents/heartbeat@v2" => handle_heartbeat_v2(payload),
}

// Phase 2 : Agents upgrade progressivement vers @v2
// - Agent 1 publish @v2 (nouveau schema avec métriques GPU)
// - Agent 2 encore @v1 (ancien schema)
// - Kernel gère les deux

// Phase 3 : Dépréciation @v1 (après 12 mois)
// - Kernel log warnings si @v1 détecté
// - Documentation update: "@v1 deprecated, use @v2"

// Phase 4 : Suppression @v1 (après 18 mois)
// - Kernel v3.0.0 ne subscribe plus @v1
// - BREAKING CHANGE documenté
```

**Référence** : [mqtt/README.md - Versioning Topics](mqtt/README.md#versioning-topics)

### Database Migration

**État actuel** : JSON files (`users.json`, `agents.json`)
**Migration future** : SQLite → PostgreSQL

**Example Migration Script** : `scripts/migrate-json-to-sqlite.sh`

```bash
#!/bin/bash
# migrate-json-to-sqlite.sh

set -e

DB_FILE="symbion.db"
BACKUP_DIR="backups/$(date +%Y%m%d_%H%M%S)"

# 1. Backup JSON
mkdir -p "$BACKUP_DIR"
cp users.json agents.json "$BACKUP_DIR/"

# 2. Create SQLite schema
sqlite3 "$DB_FILE" <<EOF
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    platform TEXT NOT NULL,
    last_seen INTEGER NOT NULL,
    status TEXT NOT NULL
);
EOF

# 3. Import data
jq -r '.users[] | [.id, .username, .password_hash, .created_at] | @csv' users.json | \
  while IFS=, read -r id username hash created; do
    sqlite3 "$DB_FILE" "INSERT INTO users VALUES ($id, $username, $hash, $created);"
  done

echo "Migration complete. Backup: $BACKUP_DIR"
```

**Testing Migration** :
```bash
# Dry-run
./scripts/migrate-json-to-sqlite.sh --dry-run

# Actual migration
./scripts/migrate-json-to-sqlite.sh

# Verify data
sqlite3 symbion.db "SELECT COUNT(*) FROM users;"
sqlite3 symbion.db "SELECT COUNT(*) FROM agents;"
```

### API Version Migration

**Current** : No API versioning (implicit v1)
**Future** : `/v1/agents`, `/v2/agents` coexistence

**Strategy** :
- **v1 endpoints** : `/agents`, `/notes` (current, deprecated in v2.0.0)
- **v2 endpoints** : `/v2/agents`, `/v2/notes` (introduced v2.0.0)
- **Coexistence** : 12 months, then v1 removed in v3.0.0

**Example** :
```rust
// symbion-kernel/src/http.rs (v2.0.0+)

// Legacy v1 (deprecated)
.route("/agents", get(get_agents_v1))
.route("/agents/:id", get(get_agent_v1))

// New v2 (recommended)
.route("/v2/agents", get(get_agents_v2))
.route("/v2/agents/:id", get(get_agent_v2))

// v1 responses include deprecation header
async fn get_agents_v1() -> Response {
    let agents = /* ... */;
    Response::builder()
        .header("X-API-Version", "1")
        .header("Deprecation", "true")
        .header("Sunset", "2026-12-31")  // RFC 8594
        .header("Link", "</v2/agents>; rel=\"successor-version\"")
        .json(agents)
}
```

### Agents Upgrade Coordination

**Challenge** : Agents déployés sur N machines domestiques

**Strategy** : Progressive rollout

```bash
# 1. Upgrade kernel (backward compatible)
# Kernel v2.0.0 accepte agents v1.x et v2.x

# 2. Upgrade agents progressivement
# Machine par machine, tester stabilité

# Agent 1 (PC-Salon)
ssh pc-salon
cd ~/symbion-agent-host
git pull && cargo build --release
sudo systemctl restart symbion-agent-host

# Vérifier dans dashboard : Agent online, métriques OK

# Agent 2 (PC-Bureau)
# ... repeat

# 3. Si un agent échoue → rollback cet agent uniquement
# Kernel continue avec mix v1/v2 agents
```

### Rollback Strategy

**Voir section** : [Rollback Procédure](#rollback-procédure)

**Key Points** :
- Backup avant toute migration
- Git tags pour versions stables (`git checkout v1.1.7`)
- Kernel downgrade possible si data format compatible
- Database migrations **must be reversible** (scripts `migrate-up.sh` + `migrate-down.sh`)

### Pre-Migration Checklist

- [ ] **Backup complet** : users.json, agents.json, certs/, kernel binary
- [ ] **Read CHANGELOG** : Breaking changes identifiés
- [ ] **Test staging** : Migration testée sur environnement non-prod
- [ ] **Downtime window** : Si migration nécessite arrêt (communiquer utilisateurs)
- [ ] **Rollback plan** : Procédure documentée et testée
- [ ] **Monitoring ready** : Logs, alertes, dashboard actifs
- [ ] **Database backup** : Export SQL/JSON avant schema changes

### Post-Migration Validation

```bash
# 1. Health check
curl -k https://localhost:8443/health
# Expected: {"status":"healthy"}

# 2. Agents online
curl -k https://localhost:8443/agents | jq '.[] | {id, status}'
# Expected: All agents "online"

# 3. MQTT connectivity
mosquitto_sub -h localhost -t 'symbion/agents/heartbeat@v1' -C 1 -v
# Expected: Heartbeat received dans 30s

# 4. API endpoints
curl -k https://localhost:8443/v1/metrics/system | jq '.uptime_seconds'
# Expected: Uptime > 0

# 5. Logs sans erreurs critiques
journalctl -u symbion-kernel -n 100 | grep -i error
# Expected: No critical errors
```

---

## 📚 Références

- **Architecture**: [docs/architecture/SYSTEM_OVERVIEW.md](architecture/SYSTEM_OVERVIEW.md)
- **API**: [docs/api/endpoints.md](api/endpoints.md)
- **MQTT**: [docs/mqtt/topics.md](mqtt/topics.md)
- **Troubleshooting**: [docs/TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

**Dernière mise à jour** : 15 Novembre 2025
