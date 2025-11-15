# Configuration Email - msmtp

Guide pour configurer l'envoi d'emails via msmtp (utilisé par `/mail` et monitoring).

---

## 📧 Installation

```bash
sudo apt install msmtp msmtp-mta
```

---

## 🔧 Configuration

### 1. Créer le fichier de configuration

Créer `~/.msmtprc` :

```bash
# Configuration msmtp pour Symbion
# Gmail SMTP

# Default account
account default

# SMTP server
host smtp.gmail.com
port 587

# Authentication
auth on
user Markchavatte@gmail.com

# TLS/SSL
tls on
tls_starttls on
tls_trust_file /etc/ssl/certs/ca-certificates.crt

# Password (use app-specific password from Gmail)
passwordeval "cat ~/.msmtp-password"

# From address
from Markchavatte@gmail.com

# Logging
logfile /tmp/msmtp.log

# Account-specific settings
account gmail : default
```

### 2. Sécuriser les permissions

```bash
chmod 600 ~/.msmtprc
```

### 3. Générer mot de passe d'application Gmail

1. Aller sur : https://myaccount.google.com/apppasswords
2. Créer un mot de passe pour "Symbion Mail"
3. Copier le mot de passe généré (16 caractères)

### 4. Stocker le mot de passe de manière sécurisée

```bash
echo "VOTRE_MOT_DE_PASSE_APP" > ~/.msmtp-password
chmod 600 ~/.msmtp-password
```

---

## ✅ Test

Tester l'envoi :

```bash
echo "Test email body" | msmtp --debug Markchavatte@gmail.com
```

Ou via le script helper :

```bash
./scripts/send-mail.sh "Markchavatte@gmail.com" "Test Subject" "Test body"
```

---

## 🎯 Utilisation avec Slash Command

Une fois configuré, utiliser `/mail` :

```
/mail "Deploy Success" "Symbion v1.1.7 déployé avec succès"
/mail "Security Alert" "Tentatives de connexion échouées détectées"
/mail "Weekly Report" "Rapport hebdomadaire du projet"
```

---

## 🔍 Troubleshooting

### Erreur : "compte default introuvable"

```bash
# Vérifier que le fichier existe et a les bonnes permissions
ls -la ~/.msmtprc
chmod 600 ~/.msmtprc
```

### Erreur : "authentication failed"

```bash
# Vérifier le mot de passe
cat ~/.msmtp-password

# Tester avec debug
echo "Test" | msmtp --debug Markchavatte@gmail.com
```

### Erreur : "TLS handshake failed"

```bash
# Vérifier les certificats
ls -la /etc/ssl/certs/ca-certificates.crt

# Mettre à jour les certificats si nécessaire
sudo update-ca-certificates
```

---

## 📝 Alternative : Stockage mot de passe direct

**Moins sécurisé mais plus simple** :

Éditer `~/.msmtprc` et remplacer :

```bash
# Commenter cette ligne :
# passwordeval "cat ~/.msmtp-password"

# Décommenter et remplacer :
password VOTRE_MOT_DE_PASSE_APP
```

⚠️ **Attention** : Le mot de passe est en clair dans le fichier !

---

## 🔗 Utilisation dans les scripts

Le script `scripts/monitor-symbion.sh` utilise déjà msmtp :

```bash
# Monitoring avec alertes email
./scripts/monitor-symbion.sh "Markchavatte@gmail.com"
```

Installer dans cron pour monitoring automatique :

```bash
# Toutes les 15 minutes
*/15 * * * * /home/eridwyn/RustroverProjects/NewSymbion/scripts/monitor-symbion.sh "Markchavatte@gmail.com"
```

---

Voir aussi :
- [scripts/send-mail.sh](../../scripts/send-mail.sh) - Script helper
- [.claude/commands/mail.md](../../.claude/commands/mail.md) - Slash command
