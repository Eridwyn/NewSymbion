# Scripts de Monitoring Symbion

## 📊 Monitoring Automatique

Le script `monitor-symbion.sh` surveille l'état de votre infrastructure Symbion et envoie des alertes mail en cas de problème.

### 🎯 Checks effectués

- ✅ **Kernel alive** - Vérifie que le kernel répond sur http://localhost:8080
- ✅ **MQTT status** - Vérifie que MQTT est "connected"
- ✅ **Agents** - Compte les agents online/offline, vérifie les métriques
- ✅ **Plugins** - Vérifie que tous les plugins sont "Running"

### 🚀 Installation rapide

```bash
# Installation interactive avec crontab
./scripts/install-monitoring.sh

# Ou manuel avec email d'alerte
./scripts/monitor-symbion.sh votre-email@example.com

# Sans email (logs uniquement)
./scripts/monitor-symbion.sh
```

### ⚙️ Configuration mail

Le script supporte deux clients mail :

**Option 1 : msmtp (recommandé pour config simple)**
```bash
# Installation
sudo apt install msmtp msmtp-mta

# Configuration
cp scripts/config-mail-example.conf ~/.msmtprc
chmod 600 ~/.msmtprc
nano ~/.msmtprc  # Éditer avec vos credentials

# Test
echo "Test" | msmtp votre-email@example.com
```

**Option 2 : mail (postfix/sendmail)**
```bash
# Installation
sudo apt install mailutils

# Test
echo "Test" | mail -s "Test" votre-email@example.com
```

### 📅 Automatisation avec crontab

```bash
# Méthode 1: Installation automatique
./scripts/install-monitoring.sh

# Méthode 2: Manuel
crontab -e
# Ajouter selon la fréquence souhaitée:

# Toutes les 15 minutes (recommandé)
*/15 * * * * /chemin/vers/monitor-symbion.sh votre-email@example.com >> /tmp/symbion-monitor.log 2>&1

# Toutes les heures
0 * * * * /chemin/vers/monitor-symbion.sh votre-email@example.com >> /tmp/symbion-monitor.log 2>&1

# Tous les jours à 8h
0 8 * * * /chemin/vers/monitor-symbion.sh votre-email@example.com >> /tmp/symbion-monitor.log 2>&1
```

### 📝 Logs et états

```bash
# Voir les logs en temps réel
tail -f /tmp/symbion-monitor.log

# Voir le dernier état
cat /tmp/symbion-monitor.state

# Voir les alertes non envoyées (si pas de client mail)
cat /tmp/symbion-alerts.txt
```

### 🔔 Types d'alertes envoyées

- **Kernel DOWN** - Le kernel ne répond pas
- **MQTT Disconnected** - MQTT n'est pas en état "connected"
- **All Agents Offline** - Aucun agent actif
- **Plugin Failed** - Un ou plusieurs plugins ont crashé
- **System Recovered** - Le système est revenu à la normale après une panne

### 🧪 Test manuel

```bash
# Test complet avec affichage
./scripts/monitor-symbion.sh

# Test avec email
./scripts/monitor-symbion.sh test@example.com

# Simuler une panne (arrêter le kernel)
pkill symbion-kernel
./scripts/monitor-symbion.sh test@example.com
# → Devrait envoyer "Kernel DOWN"

# Relancer et tester la récupération
cargo run --release -p symbion-kernel &
sleep 5
./scripts/monitor-symbion.sh test@example.com
# → Devrait envoyer "System Recovered"
```

### 🔧 Variables d'environnement

```bash
# URL du kernel (défaut: http://localhost:8080)
KERNEL_URL=http://192.168.1.10:8080 ./scripts/monitor-symbion.sh

# API Key (défaut: s3cr3t-42)
SYMBION_API_KEY="autre-key" ./scripts/monitor-symbion.sh
```

### 📈 Exemple de sortie

```
[2025-10-15 21:26:51] 🔍 Monitoring Symbion - Démarrage
[2025-10-15 21:26:51] 📧 Alertes configurées vers: admin@example.com
[2025-10-15 21:26:51] 📊 Health: MQTT=connected, Agents=2, Uptime=435s, Plugins=1/0
[2025-10-15 21:26:51] 🤖 2 agent(s) enregistré(s)
[2025-10-15 21:26:51] ✅ Agent eridwyn-Salon: online - CPU: 9.9%, RAM: 10.0%
[2025-10-15 21:26:51]    Agent DESKTOP-3BT760L: offline
[2025-10-15 21:26:51]    📈 Résumé: 1 online, 1 offline, 0 stale
[2025-10-15 21:26:51] 🔌 1 plugin(s) découvert(s)
[2025-10-15 21:26:51] ✅ Plugin notes-manager: Running (uptime: 436s)
[2025-10-15 21:26:51] ✅ ✨ Tous les checks OK
```

### 🐛 Troubleshooting

**Mail non envoyé**
```bash
# Vérifier que msmtp ou mail est installé
which msmtp
which mail

# Tester la config msmtp
msmtp --serverinfo --account=default

# Vérifier les logs
cat ~/.msmtp.log
```

**Script ne trouve pas le kernel**
```bash
# Vérifier que le kernel tourne
lsof -i :8080

# Tester manuellement
curl http://localhost:8080/health

# Vérifier l'API key
curl -H "x-api-key: s3cr3t-42" http://localhost:8080/system/health
```

**jq non trouvé (facultatif)**
```bash
# Installation
sudo apt install jq

# Le script fonctionne sans mais avec moins de détails
```

## 📧 Configuration Gmail

Pour utiliser Gmail avec msmtp, vous devez générer un **mot de passe d'application** :

1. Aller sur https://myaccount.google.com/security
2. Activer la validation en 2 étapes
3. Aller dans "Mots de passe d'application"
4. Générer un mot de passe pour "Mail"
5. Utiliser ce mot de passe dans `~/.msmtprc`

```conf
account        gmail
host           smtp.gmail.com
port           587
from           votre-email@gmail.com
user           votre-email@gmail.com
password       xxxx xxxx xxxx xxxx  # Mot de passe d'application
```

## 🔐 Sécurité

⚠️ **Important** : Le fichier `~/.msmtprc` contient votre mot de passe en clair.

```bash
# TOUJOURS protéger ce fichier
chmod 600 ~/.msmtprc

# Vérifier les permissions
ls -la ~/.msmtprc
# Devrait afficher: -rw------- (600)
```
