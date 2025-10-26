# 🚀 Guide de Déploiement Symbion

Documentation complète pour déployer et mettre à jour l'écosystème Symbion en production.

## 📋 Table des Matières

- [Architecture de Déploiement](#architecture-de-déploiement)
- [GitHub Actions Workflows](#github-actions-workflows)
- [Scripts de Déploiement](#scripts-de-déploiement)
- [Services Systemd](#services-systemd)
- [Workflow Complet](#workflow-complet)
- [Rollback et Récupération](#rollback-et-récupération)

## 🏗️ Architecture de Déploiement

```
┌─────────────────────────────────────────────────────────────┐
│                    GitHub Actions CI/CD                      │
│  - Build automatique sur git push/tag                       │
│  - Tests et compilation multi-plateforme                    │
│  - Création releases GitHub avec binaires                   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  Scripts de Déploiement                      │
│  - Téléchargement depuis GitHub releases                    │
│  - Backup automatique version précédente                    │
│  - Vérification santé + rollback automatique                │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                   Services Systemd                           │
│  - Supervision processus 24/7                                │
│  - Auto-restart sur crash                                    │
│  - Gestion dépendances entre services                        │
│  - Logs centralisés journald                                 │
└─────────────────────────────────────────────────────────────┘
```

## ⚙️ GitHub Actions Workflows

### 1. **Kernel Deployment** (`.github/workflows/deploy-kernel.yml`)

**Déclencheurs** :
- Tag `kernel-v*` (ex: `kernel-v1.2.0`)
- Push sur branche `master` avec modifications dans `symbion-kernel/**`
- Déclenchement manuel via GitHub UI

**Actions** :
1. Compile `symbion-kernel` en mode release
2. Crée binaires pour Linux x64
3. Génère checksums SHA256
4. Publie GitHub Release avec assets téléchargeables

**Utilisation** :
```bash
# Créer une nouvelle version kernel
git tag kernel-v1.2.0
git push origin kernel-v1.2.0

# GitHub Actions build automatiquement
# Release disponible sous: https://github.com/USER/REPO/releases/tag/kernel-v1.2.0
```

### 2. **Dashboard Deployment** (`.github/workflows/deploy-dashboard.yml`)

**Déclencheurs** :
- Tag `dashboard-v*` (ex: `dashboard-v2.1.0`)
- Push sur `master` avec modifications dans `pwa-dashboard/**`
- Déclenchement manuel

**Actions** :
1. Build PWA avec Vite (`npm run build`)
2. Optimise assets (minification, tree-shaking)
3. Crée archive `.tar.gz` du répertoire `dist/`
4. Publie GitHub Release

**Utilisation** :
```bash
# Créer nouvelle version dashboard
git tag dashboard-v2.1.0
git push origin dashboard-v2.1.0
```

### 3. **Agent Deployment** (`.github/workflows/release.yml`)

**Déclencheurs** :
- Tag `agent-v*`
- Build multi-plateforme: Linux, Windows, macOS

**Platformes supportées** :
- Linux x64
- Windows x64
- macOS ARM64 (Apple Silicon)

## 📦 Scripts de Déploiement

### **Kernel** (`scripts/deploy-kernel.sh`)

Script intelligent pour déployer le kernel en production avec rollback automatique.

**Fonctionnalités** :
- ✅ Téléchargement depuis GitHub Releases
- ✅ Backup automatique version actuelle
- ✅ Test santé API après déploiement
- ✅ Rollback automatique si déploiement échoue
- ✅ Intégration systemd (restart service automatique)

**Utilisation** :
```bash
# Déployer version spécifique
sudo ./scripts/deploy-kernel.sh kernel-v1.2.0

# Déployer dernière version disponible
sudo ./scripts/deploy-kernel.sh latest

# Avec custom repo GitHub
GITHUB_REPO="votre-user/votre-repo" sudo ./scripts/deploy-kernel.sh kernel-v1.2.0
```

**Workflow interne** :
```bash
1. Télécharge binaire depuis GitHub releases
2. Crée backup: /opt/symbion/symbion-kernel.backup
3. Stop service systemd
4. Remplace binaire
5. Démarre service
6. Vérifie health endpoint (https://localhost:8443/health)
7. Si échec → rollback automatique vers backup
```

### **Dashboard** (`scripts/deploy-dashboard.sh`)

Déploiement PWA dashboard avec gestion versions.

**Fonctionnalités** :
- ✅ Télécharge archive build depuis releases
- ✅ Extraction vers `/var/www/symbion-dashboard`
- ✅ Backup répertoire précédent
- ✅ Restart service HTTP si configuré

**Utilisation** :
```bash
# Déployer version dashboard
sudo ./scripts/deploy-dashboard.sh dashboard-v2.1.0

# Déployer latest
sudo ./scripts/deploy-dashboard.sh latest

# Servir manuellement si pas de systemd
cd /var/www/symbion-dashboard
python3 -m http.server 3000
```

## 🔧 Services Systemd

### Installation Initiale

**Script tout-en-un** (`systemd/install-services.sh`):
```bash
cd /home/eridwyn/RustroverProjects/NewSymbion/systemd
sudo ./install-services.sh
```

**Actions effectuées** :
1. Arrêt processus manuels existants
2. Création `/opt/symbion/` pour binaires production
3. Copie binaires initiaux depuis `target/release/`
4. Installation fichiers `.service` dans `/etc/systemd/system/`
5. Activation auto-start au boot
6. Démarrage services
7. Optionnel: installation dashboard service

### Services Configurés

#### **symbion-kernel.service**
- **Rôle** : Hub IoT central avec MQTT + API REST
- **Port** : 8443 (HTTPS)
- **Restart** : Automatique (10s délai)
- **Logs** : `journalctl -u symbion-kernel -f`

#### **symbion-agent.service**
- **Rôle** : Agent monitoring système local
- **Dépendance** : Requiert `symbion-kernel.service` actif
- **Restart** : Automatique (15s délai)
- **Logs** : `journalctl -u symbion-agent -f`

#### **symbion-dashboard.service** (optionnel)
- **Rôle** : Interface web PWA
- **Port** : 3000 (HTTP)
- **Alternative** : Vite dev server ou nginx
- **Logs** : `journalctl -u symbion-dashboard -f`

### Commandes Systemd Utiles

```bash
# Status tous services
sudo systemctl status symbion-kernel symbion-agent symbion-dashboard

# Restart après modification config
sudo systemctl restart symbion-kernel

# Voir logs en temps réel
journalctl -u symbion-kernel -f

# Désactiver auto-start (temporaire)
sudo systemctl stop symbion-kernel

# Désactiver complètement
sudo systemctl disable symbion-kernel

# Recharger après modification .service
sudo systemctl daemon-reload
sudo systemctl restart symbion-kernel
```

## 🔄 Workflow Complet de Déploiement

### Scénario 1 : Nouvelle Version Kernel

```bash
# 1. Développement local
cd /home/eridwyn/RustroverProjects/NewSymbion/symbion-kernel
# ... modifications code ...

# 2. Tests locaux
cargo test
cargo build --release
./target/release/symbion-kernel  # Test manuel

# 3. Commit et tag
git add .
git commit -m "feat: amélioration système MQTT"
git tag kernel-v1.3.0
git push origin master
git push origin kernel-v1.3.0

# 4. GitHub Actions build automatique
# → Attend 5-10 minutes pour compilation
# → Vérifie release: https://github.com/USER/REPO/releases/tag/kernel-v1.3.0

# 5. Déploiement production
sudo ./scripts/deploy-kernel.sh kernel-v1.3.0

# 6. Vérification
curl -k https://localhost:8443/health
journalctl -u symbion-kernel -n 50
```

### Scénario 2 : Mise à Jour Dashboard

```bash
# 1. Développement
cd pwa-dashboard
# ... modifications UI ...

# 2. Test local
npm run dev  # Vérifier sur http://localhost:3000

# 3. Build production local (optionnel)
npm run build
npm run preview

# 4. Tag et push
git tag dashboard-v2.2.0
git push origin dashboard-v2.2.0

# 5. Déploiement
sudo ./scripts/deploy-dashboard.sh dashboard-v2.2.0

# 6. Vérification
curl http://localhost:3000
# Ouvrir navigateur: http://votre-ip:3000
```

### Scénario 3 : Déploiement Initial Nouveau Serveur

```bash
# 1. Cloner repo
git clone https://github.com/USER/NewSymbion.git
cd NewSymbion

# 2. Installer dépendances système
sudo apt-get update
sudo apt-get install -y mosquitto mosquitto-clients build-essential pkg-config libssl-dev

# 3. Compiler binaires initiaux (ou télécharger depuis releases)
cargo build --release -p symbion-kernel
cargo build --release -p symbion-agent-host

# 4. Installer services systemd
cd systemd
sudo ./install-services.sh

# 5. Vérifier tout fonctionne
systemctl status symbion-kernel symbion-agent
curl -k https://localhost:8443/health

# 6. Configurer déploiements futurs via scripts
# Créer tag initial
git tag kernel-v1.0.0
git push origin kernel-v1.0.0
```

## 🔙 Rollback et Récupération

### Rollback Kernel

**Automatique** (intégré dans `deploy-kernel.sh`):
- Si le health check échoue après déploiement, rollback automatique

**Manuel** :
```bash
# Restaurer depuis backup
sudo cp /opt/symbion/symbion-kernel.backup /opt/symbion/symbion-kernel
sudo systemctl restart symbion-kernel

# Ou déployer version précédente spécifique
sudo ./scripts/deploy-kernel.sh kernel-v1.2.0  # Version connue stable
```

### Rollback Dashboard

```bash
# Restaurer backup
sudo rm -rf /var/www/symbion-dashboard
sudo mv /var/www/symbion-dashboard.backup /var/www/symbion-dashboard
sudo systemctl restart symbion-dashboard

# Ou redéployer version précédente
sudo ./scripts/deploy-dashboard.sh dashboard-v2.0.0
```

### Récupération Service Crashé

```bash
# 1. Vérifier status
sudo systemctl status symbion-kernel

# 2. Voir logs erreur
journalctl -u symbion-kernel -n 100 --no-pager

# 3. Restart manuel si systemd n'a pas auto-restart
sudo systemctl restart symbion-kernel

# 4. Si problème persiste, revenir version stable
sudo ./scripts/deploy-kernel.sh kernel-v1.2.0  # Dernière version stable
```

## 🔐 Sécurité et Bonnes Pratiques

### Variables d'Environnement Sensibles

**Production** :
```bash
# NE JAMAIS commiter dans git
# Stocker dans systemd service files ou fichiers .env

# Exemple dans /etc/systemd/system/symbion-kernel.service
Environment="SYMBION_API_KEY=votre-clé-production-sécurisée"
Environment="SYMBION_JWT_SECRET=secret-64-caracteres-minimum"
```

**GitHub Secrets** :
- Configurer `SYMBION_API_KEY` dans GitHub Secrets
- Utiliser dans workflows: `${{ secrets.SYMBION_API_KEY }}`

### Certificats TLS

**Production recommandée** :
```bash
# Utiliser Let's Encrypt au lieu de certificats auto-signés
sudo apt-get install certbot
sudo certbot certonly --standalone -d votre-domaine.com

# Configurer kernel pour utiliser certificats Let's Encrypt
export SYMBION_TLS_CERT_PATH=/etc/letsencrypt/live/votre-domaine.com/fullchain.pem
export SYMBION_TLS_KEY_PATH=/etc/letsencrypt/live/votre-domaine.com/privkey.pem
```

### Firewall

```bash
# Ouvrir ports nécessaires uniquement
sudo ufw allow 8443/tcp comment 'Symbion Kernel HTTPS'
sudo ufw allow 3000/tcp comment 'Symbion Dashboard HTTP'
sudo ufw allow 1883/tcp comment 'MQTT broker local'  # Si accès externe nécessaire

# Bloquer MQTT externe si usage local uniquement
sudo ufw deny 1883/tcp
```

## 📊 Monitoring Production

### Health Checks

**Kernel** :
```bash
# Health endpoint
curl -k https://localhost:8443/health

# Agents actifs
curl -k -H "x-api-key: votre-clé" https://localhost:8443/agents

# Plugins
curl -k -H "x-api-key: votre-clé" https://localhost:8443/plugins
```

**Dashboard** :
```bash
# Accessibilité
curl -I http://localhost:3000

# PWA manifest
curl http://localhost:3000/manifest.json
```

### Logs Centralisés

```bash
# Tous services Symbion
journalctl -u symbion-* -f

# Filtre erreurs seulement
journalctl -u symbion-kernel -p err -f

# Export logs période
journalctl -u symbion-kernel --since "2025-10-20" --until "2025-10-26" > kernel-logs.txt
```

### Alertes Automatiques

Le système de monitoring existant (`scripts/monitor-symbion.sh`) envoie déjà des alertes email.

**Étendre avec** :
- Prometheus + Grafana pour métriques temps réel
- Healthchecks.io pour ping monitoring externe
- PagerDuty/Opsgenie pour alertes critiques

## 🚀 Optimisations Futures

### CI/CD Avancé

- [ ] Tests automatisés dans workflows
- [ ] Déploiement multi-environnements (staging, prod)
- [ ] Blue/Green deployments
- [ ] Canary releases progressives

### Infrastructure as Code

- [ ] Terraform pour provisionning serveurs
- [ ] Ansible playbooks pour configuration automatique
- [ ] Docker/Podman containers (alternative systemd)
- [ ] Kubernetes pour scalabilité (si besoin multi-serveurs)

### Monitoring Avancé

- [ ] Métriques Prometheus endpoints
- [ ] Distributed tracing avec OpenTelemetry
- [ ] Log aggregation avec Loki ou ELK
- [ ] APM (Application Performance Monitoring)

---

**Documentation maintenue le** : 26 Octobre 2025
**Version Symbion** : Kernel v1.1.0 | Dashboard v2.0.0 | Agent v1.0.0
