# 🌍 Compatibilité Multi-Plateforme Symbion

Guide complet de compatibilité et déploiement sur Linux, Windows et Android.

## 📱 Matrice de Compatibilité

| Composant | Linux | Windows | macOS | Android | iOS |
|-----------|-------|---------|-------|---------|-----|
| **symbion-kernel** | ✅ Natif | ✅ Natif | ✅ Natif | ❌ N/A | ❌ N/A |
| **symbion-agent-host** | ✅ Natif | ✅ Natif | ✅ Natif | 🔶 Termux | 🔶 Termux |
| **pwa-dashboard** | ✅ Navigateur | ✅ Navigateur | ✅ Navigateur | ✅ PWA | ✅ PWA |
| **Scripts déploiement** | ✅ Bash | ✅ PowerShell | ✅ Bash | ⚠️ Limité | ⚠️ Limité |
| **Services système** | ✅ systemd | ✅ NSSM/Task | ✅ launchd | ❌ N/A | ❌ N/A |

**Légende** :
- ✅ Support complet et testé
- 🔶 Support partiel ou expérimental
- ⚠️ Fonctionnel avec limitations
- ❌ Non applicable ou non supporté

---

## 🐧 Linux (Support Complet)

### Distributions Supportées
- ✅ **Ubuntu** 20.04+ (testé, recommandé)
- ✅ **Debian** 11+
- ✅ **Fedora** 35+
- ✅ **Arch Linux** (rolling)
- ✅ **Raspberry Pi OS** (ARM64)

### Installation Production

```bash
# 1. Installer dépendances système
sudo apt-get update
sudo apt-get install -y mosquitto pkg-config libssl-dev

# 2. Déployer kernel
sudo ./scripts/deploy-kernel.sh kernel-v1.1.0

# 3. Installer services systemd
cd systemd
sudo ./install-services.sh

# 4. Déployer dashboard
sudo ./scripts/deploy-dashboard.sh dashboard-v2.0.0
```

### Services Systemd

```bash
# Status
systemctl status symbion-kernel symbion-agent symbion-dashboard

# Logs
journalctl -u symbion-kernel -f

# Auto-start au boot
sudo systemctl enable symbion-kernel symbion-agent
```

### Architecture ARM (Raspberry Pi)

```bash
# Compiler pour ARM64
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu -p symbion-kernel

# Ou utiliser Docker pour cross-compilation
docker run --rm -v $(pwd):/workspace \
  rust:latest bash -c "cd /workspace && cargo build --release --target aarch64-unknown-linux-gnu"
```

---

## 🪟 Windows (Support Complet)

### Versions Supportées
- ✅ **Windows 10** (build 1909+)
- ✅ **Windows 11**
- ✅ **Windows Server** 2019+

### Installation Production

#### Option 1 : PowerShell Scripts (Recommandé)

```powershell
# Exécuter PowerShell en Administrateur

# 1. Télécharger binaires depuis GitHub Releases
.\scripts\Deploy-SymbionKernel.ps1 -Version "kernel-v1.1.0"

# 2. Installer service Windows avec NSSM
.\scripts\Install-SymbionService.ps1

# 3. Démarrer services
Start-Service SymbionKernel
Start-Service SymbionAgent
```

#### Option 2 : Windows Task Scheduler

```powershell
# Créer tâche planifiée qui démarre au boot
$action = New-ScheduledTaskAction -Execute "C:\Symbion\symbion-kernel.exe"
$trigger = New-ScheduledTaskTrigger -AtStartup
$principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -RunLevel Highest

Register-ScheduledTask -TaskName "SymbionKernel" `
  -Action $action `
  -Trigger $trigger `
  -Principal $principal `
  -Description "Symbion IoT Kernel"
```

### Binaires Pré-Compilés

GitHub Actions compile automatiquement pour Windows :

```powershell
# Télécharger depuis releases
Invoke-WebRequest -Uri "https://github.com/USER/NewSymbion/releases/download/kernel-v1.1.0/symbion-kernel-windows-x64-kernel-v1.1.0.exe" `
  -OutFile "C:\Symbion\symbion-kernel.exe"

# Vérifier signature
Get-FileHash C:\Symbion\symbion-kernel.exe -Algorithm SHA256
```

### Dashboard Windows

```powershell
# Servir dashboard avec Python
cd C:\Symbion\dashboard
python -m http.server 3000

# Ou utiliser IIS (Internet Information Services)
# 1. Activer IIS dans Windows Features
# 2. Créer site web pointant vers C:\Symbion\dashboard
# 3. Binding: http://localhost:3000
```

### Firewall Windows

```powershell
# Ouvrir ports nécessaires
New-NetFirewallRule -DisplayName "Symbion Kernel HTTPS" `
  -Direction Inbound -LocalPort 8443 -Protocol TCP -Action Allow

New-NetFirewallRule -DisplayName "Symbion Dashboard HTTP" `
  -Direction Inbound -LocalPort 3000 -Protocol TCP -Action Allow
```

### Certificats TLS Windows

```powershell
# Importer certificat CA dans Windows Trust Store
Import-Certificate -FilePath "symbion-ca.crt" `
  -CertStoreLocation Cert:\LocalMachine\Root
```

---

## 📱 Android (Support PWA + Agent Expérimental)

### Dashboard PWA (Recommandé)

Le dashboard Symbion est une **Progressive Web App** pleinement fonctionnelle sur Android.

#### Installation PWA

1. **Ouvrir dashboard dans Chrome/Edge** :
   ```
   https://votre-ip-local:8443
   ou
   http://votre-ip-local:3000
   ```

2. **Menu → "Ajouter à l'écran d'accueil"**
   - Chrome affiche automatiquement la bannière d'installation
   - L'app s'ouvre en mode standalone (sans barre d'adresse)

3. **Fonctionnalités PWA Android** :
   - ✅ Mode offline (service worker)
   - ✅ Notifications push (si configuré)
   - ✅ Widgets contextuels responsives
   - ✅ Navigation tactile optimisée
   - ✅ Contrôles domotiques temps réel

#### Configuration Certificat Android

```bash
# 1. Télécharger certificat CA depuis dashboard
# 2. Aller dans Paramètres Android
# 3. Sécurité → Chiffrement → Installer certificat
# 4. Sélectionner symbion-ca.crt
# 5. Confirmer installation
```

### Agent Android (Expérimental - Termux)

**Termux** permet d'exécuter `symbion-agent-host` nativement sur Android.

#### Installation Termux

```bash
# 1. Installer Termux depuis F-Droid (PAS Google Play)
# https://f-droid.org/packages/com.termux/

# 2. Mettre à jour packages
pkg update && pkg upgrade

# 3. Installer Rust
pkg install rust openssl

# 4. Cloner repo
git clone https://github.com/USER/NewSymbion.git
cd NewSymbion

# 5. Compiler agent
cargo build --release -p symbion-agent-host

# 6. Lancer agent
SYMBION_MQTT_BROKER="192.168.1.14:1883" \
  ./target/release/symbion-agent-host
```

#### Limitations Agent Termux

⚠️ **Restrictions Android** :
- Pas d'accès root (sauf téléphone rooté)
- Monitoring limité (CPU/RAM seulement)
- Pas de contrôle système (shutdown, hibernate)
- Termux doit rester en foreground (notification persistante)

#### Utilisation Recommandée Android

Au lieu d'un agent complet, utilisez Android comme **interface de contrôle** :

```
┌─────────────────────────┐
│   Smartphone Android    │
│   (PWA Dashboard)       │
│   - Contrôles maison    │
│   - Monitoring          │
│   - Notes contextuelles │
└─────────────────────────┘
            ↓ WiFi
┌─────────────────────────┐
│   Serveur Domestique    │
│   (Linux/Windows)       │
│   - symbion-kernel      │
│   - symbion-agent-host  │
│   - MQTT broker         │
└─────────────────────────┘
```

---

## 🍎 iOS (Support PWA)

### Dashboard PWA iOS

Fonctionne identiquement à Android :

1. **Ouvrir dans Safari** :
   ```
   http://votre-ip-local:3000
   ```

2. **Partager → "Sur l'écran d'accueil"**

3. **Fonctionnalités** :
   - ✅ Mode standalone
   - ✅ Widgets responsives
   - ✅ Navigation tactile
   - ⚠️ Service worker limité (restrictions iOS)
   - ⚠️ Pas de notifications push (limitation Safari)

### Certificat iOS

```bash
# 1. Envoyer symbion-ca.crt par AirDrop ou email
# 2. Ouvrir fichier → Installer profil
# 3. Réglages → Général → Profils
# 4. Installer le profil CA
# 5. Réglages → Général → Informations → Réglages des certificats
# 6. Activer certificat pour "Symbion CA"
```

---

## 🔧 Scripts Multi-Plateforme

### PowerShell pour Windows

Créés pour équivalence fonctionnelle avec bash Linux :

- **`scripts/Deploy-SymbionKernel.ps1`** - Déploiement kernel Windows
- **`scripts/Deploy-SymbionDashboard.ps1`** - Déploiement dashboard Windows
- **`scripts/Install-SymbionService.ps1`** - Installation services Windows (NSSM)
- **`scripts/Monitor-Symbion.ps1`** - Monitoring santé système Windows

### Cross-Compilation depuis Linux

```bash
# Compiler binaires Windows depuis Linux
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64

cargo build --release --target x86_64-pc-windows-gnu -p symbion-kernel

# Binaire généré : target/x86_64-pc-windows-gnu/release/symbion-kernel.exe
```

---

## 🌐 Configuration Réseau Multi-Plateforme

### Accès Local (LAN)

```javascript
// pwa-dashboard/public/config.js

window.SYMBION_CONFIG = {
  // Auto-détection hostname (fonctionne Linux/Windows/Android/iOS)
  API_BASE: window.location.protocol + '//' + window.location.hostname + ':8443',

  // Ou IP statique pour accès multi-devices
  // API_BASE: 'https://192.168.1.100:8443',
}
```

### MQTT Multi-Plateforme

| OS | MQTT Broker | Installation |
|----|-------------|--------------|
| **Linux** | Mosquitto | `sudo apt-get install mosquitto` |
| **Windows** | Mosquitto | [mosquitto.org/download](https://mosquitto.org/download/) |
| **macOS** | Mosquitto | `brew install mosquitto` |
| **Android** | Termux | `pkg install mosquitto` |

### Ports et Firewall

```bash
# Linux (UFW)
sudo ufw allow 8443/tcp
sudo ufw allow 3000/tcp
sudo ufw allow 1883/tcp

# Windows (PowerShell)
New-NetFirewallRule -DisplayName "Symbion" -LocalPort 8443,3000,1883 -Protocol TCP -Action Allow

# macOS (pfctl)
echo "pass in proto tcp to port {8443, 3000, 1883}" | sudo pfctl -f -
```

---

## 📊 Tableau Récapitulatif Déploiement

| Plateforme | Kernel | Agent | Dashboard | Service Auto-Start |
|------------|--------|-------|-----------|-------------------|
| **Ubuntu/Debian** | ✅ Natif | ✅ Natif | ✅ PWA | systemd |
| **Fedora/RHEL** | ✅ Natif | ✅ Natif | ✅ PWA | systemd |
| **Windows 10/11** | ✅ .exe | ✅ .exe | ✅ PWA | NSSM/Task Scheduler |
| **macOS** | ✅ Natif | ✅ Natif | ✅ PWA | launchd |
| **Raspberry Pi** | ✅ ARM64 | ✅ ARM64 | ✅ PWA | systemd |
| **Android** | ❌ N/A | 🔶 Termux | ✅ PWA Install | ⚠️ Foreground Service |
| **iOS** | ❌ N/A | ❌ N/A | ✅ PWA Install | ❌ Limitations Safari |

---

## 🚀 Scénarios d'Usage Multi-Plateforme

### Scénario 1 : Serveur Linux + Contrôle Android

```
Serveur Ubuntu (eridwyn-Salon)
├── symbion-kernel (systemd)
├── symbion-agent-host (systemd)
└── mosquitto (systemd)

Smartphone Android (Galaxy S21)
└── PWA Dashboard installée
    ├── Contrôles domotiques
    ├── Notes contextuelles
    └── Monitoring temps réel
```

**Avantages** :
- Serveur Linux stable 24/7
- Contrôle mobile partout dans la maison
- Notifications PWA sur Android

### Scénario 2 : PC Windows Bureau + PC Linux Salon

```
DESKTOP-3BT760L (Windows 11)
├── symbion-agent-host.exe (NSSM service)
├── Mode: Productivité/Bureau
└── Contexte: Cravate (travail)

eridwyn-Salon (Ubuntu)
├── symbion-kernel (systemd) [HUB CENTRAL]
├── symbion-agent-host (systemd)
├── Mode: Domestique/Loisirs
└── Contexte: Intime (maison)

Tablette Cuisine (Android)
└── PWA Dashboard
    ├── Suggestions repas
    ├── Contrôles ambiance
    └── Notes courses
```

**Avantages** :
- Agents multi-environnements (Windows bureau + Linux salon)
- Hub central Linux fiable
- Interface tactile cuisine optimisée

### Scénario 3 : Déploiement Cloud Multi-Régions

```
Kernel Principal (Serveur Cloud Linux)
└── Agents distribués :
    ├── Agent Maison (Raspberry Pi - Linux ARM)
    ├── Agent Bureau (Windows Desktop)
    ├── Agent Serveur (Linux VPS)
    └── Dashboard accessible depuis :
        ├── Android (PWA)
        ├── iOS (PWA)
        ├── Desktop (navigateur)
        └── Tablette (PWA)
```

---

## 🔐 Sécurité Multi-Plateforme

### Certificats TLS

| OS | Installation CA | Emplacement |
|----|-----------------|-------------|
| **Linux** | `sudo cp symbion-ca.crt /usr/local/share/ca-certificates/ && sudo update-ca-certificates` | `/usr/local/share/ca-certificates/` |
| **Windows** | `Import-Certificate -FilePath symbion-ca.crt -CertStoreLocation Cert:\LocalMachine\Root` | Certificate Manager (certmgr.msc) |
| **macOS** | `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain symbion-ca.crt` | Keychain Access |
| **Android** | Paramètres → Sécurité → Installer certificat | User credentials |
| **iOS** | Installer profil → Réglages → Général → Informations → Certificats | Settings |

### API Keys et Secrets

```javascript
// Production : NE JAMAIS hardcoder API keys
// Utiliser variables d'environnement ou secrets management

// config.js (éditable post-déploiement)
window.SYMBION_CONFIG = {
  API_BASE: 'https://192.168.1.100:8443',
  // API_KEY chargée dynamiquement depuis backend auth
}
```

---

## 📝 Commandes Utiles Cross-Platform

### Vérifier Services

```bash
# Linux
systemctl status symbion-kernel

# Windows (PowerShell)
Get-Service SymbionKernel
sc query SymbionKernel

# macOS
launchctl list | grep symbion
```

### Logs

```bash
# Linux
journalctl -u symbion-kernel -f

# Windows
Get-EventLog -LogName Application -Source SymbionKernel -Newest 50

# macOS
log show --predicate 'process == "symbion-kernel"' --last 1h
```

### Networking

```bash
# Linux
netstat -tlnp | grep 8443

# Windows
netstat -ano | findstr 8443

# macOS
lsof -i :8443
```

---

**Mis à jour** : 26 Octobre 2025
**Versions testées** : Linux (Ubuntu 22.04), Windows 11, Android 14, iOS 17
