# Configuration Reverse Proxy NAS pour Symbion

Ce document explique comment configurer votre NAS pour router `symbion.markcha.fr` vers votre serveur Symbion local.

## Prérequis

- ✅ Nginx installé et configuré sur le serveur Symbion (local)
- ✅ DNS `symbion.markcha.fr` pointant vers l'IP publique de votre NAS
- ✅ Ports 443 et 8444 ouverts sur le firewall du NAS
- ⏳ Certificat SSL pour `symbion.markcha.fr` sur le NAS

## Architecture Cible

```
Internet
    ↓
symbion.markcha.fr (DNS → IP publique NAS)
    ↓
NAS (IP publique)
    ↓ Port Forwarding / Reverse Proxy
    ↓
Serveur Symbion (IP LAN: 192.168.x.x)
    ↓
    ├─ Port 443 → Nginx → API (8443) + PWA (3000)
    └─ Port 8444 → Nginx → MQTT WebSocket (9001)
```

## Configuration Nginx sur NAS

### Option 1: Configuration Complète (Recommandé)

```nginx
# /etc/nginx/sites-available/symbion-nas

# Map directive pour WebSocket (requis)
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

# Server principal - Port 443 (API + PWA)
server {
    listen 443 ssl http2;
    server_name symbion.markcha.fr;

    # Certificats SSL NAS
    ssl_certificate /path/to/cert/symbion.markcha.fr.crt;
    ssl_certificate_key /path/to/cert/symbion.markcha.fr.key;

    # Configuration SSL moderne
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256';
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Logs
    access_log /var/log/nginx/symbion-nas-access.log;
    error_log /var/log/nginx/symbion-nas-error.log;

    # Proxy vers serveur Symbion local
    location / {
        # REMPLACER par l'IP réelle de votre serveur Symbion
        proxy_pass https://192.168.1.14:443;

        # Désactiver vérification SSL (certificat self-signed sur serveur local)
        proxy_ssl_verify off;

        # Headers standards
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Support WebSocket Vite HMR (dev)
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;

        # Timeouts
        proxy_read_timeout 60s;
        proxy_connect_timeout 10s;
    }
}

# Server MQTT WebSocket - Port 8444
server {
    listen 8444 ssl;  # SANS http2 (important!)
    server_name symbion.markcha.fr;

    # Certificats SSL (identiques au serveur principal)
    ssl_certificate /path/to/cert/symbion.markcha.fr.crt;
    ssl_certificate_key /path/to/cert/symbion.markcha.fr.key;

    # Configuration SSL moderne
    ssl_protocols TLSv1.3 TLSv1.2;
    ssl_ciphers 'ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256';

    # Logs
    access_log /var/log/nginx/symbion-mqtt-nas-access.log;
    error_log /var/log/nginx/symbion-mqtt-nas-error.log;

    # Proxy vers serveur Symbion MQTT WebSocket
    location / {
        # REMPLACER par l'IP réelle de votre serveur Symbion
        proxy_pass https://192.168.1.14:8444;

        proxy_ssl_verify off;

        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;

        # Headers standards
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts étendus pour MQTT
        proxy_read_timeout 86400s;  # 24 heures
        proxy_send_timeout 86400s;
        proxy_connect_timeout 75s;

        # Désactiver buffering
        proxy_buffering off;
    }
}

# Redirect HTTP → HTTPS
server {
    listen 80;
    server_name symbion.markcha.fr;

    return 301 https://$server_name$request_uri;
}
```

### Option 2: Port Forwarding Simple (Alternative)

Si votre NAS supporte le port forwarding uniquement :

```
Règles NAT/Firewall:
- Port externe 443  → IP serveur Symbion:443
- Port externe 8444 → IP serveur Symbion:8444
```

**Avantages** : Simple, pas de proxy sur NAS
**Inconvénients** : Pas de logs centralisés, pas de SSL termination sur NAS

## Installation NAS

### Étape 1: Créer la configuration

```bash
# Sur le NAS, créer le fichier de config
sudo nano /etc/nginx/sites-available/symbion-nas

# Coller la configuration Option 1 ci-dessus
# IMPORTANT: Remplacer 192.168.1.14 par l'IP réelle du serveur Symbion
```

### Étape 2: Activer la configuration

```bash
# Créer lien symbolique
sudo ln -s /etc/nginx/sites-available/symbion-nas /etc/nginx/sites-enabled/

# Tester syntaxe
sudo nginx -t

# Recharger Nginx
sudo systemctl reload nginx
```

### Étape 3: Vérifier les ports

```bash
# Vérifier que Nginx écoute sur les bons ports
sudo ss -tlnp | grep nginx | grep -E ":(443|8444)"

# Attendu:
# :443   nginx (HTTP/2)
# :8444  nginx (HTTP/1.1)
```

## Certificats SSL

### Option A: Let's Encrypt (Recommandé en Production)

```bash
# Sur le NAS, installer certbot
sudo apt install certbot python3-certbot-nginx

# Générer certificat
sudo certbot certonly --nginx -d symbion.markcha.fr

# Certificats créés dans:
# /etc/letsencrypt/live/symbion.markcha.fr/fullchain.pem
# /etc/letsencrypt/live/symbion.markcha.fr/privkey.pem

# Modifier la config Nginx pour pointer vers ces certificats
sudo nano /etc/nginx/sites-available/symbion-nas

# Remplacer:
ssl_certificate /etc/letsencrypt/live/symbion.markcha.fr/fullchain.pem;
ssl_certificate_key /etc/letsencrypt/live/symbion.markcha.fr/privkey.pem;
```

### Option B: Certificat Existant NAS

Si votre NAS a déjà un certificat wildcard `*.markcha.fr` :

```bash
# Utiliser le certificat existant
ssl_certificate /path/to/wildcard-markcha-fr.crt;
ssl_certificate_key /path/to/wildcard-markcha-fr.key;
```

## Configuration DNS

### Vérification DNS Actuelle

```bash
# Tester résolution DNS
dig symbion.markcha.fr

# Attendu: Doit pointer vers l'IP publique de votre NAS
# symbion.markcha.fr. 300 IN A xxx.xxx.xxx.xxx
```

### Si DNS Non Configuré

1. **Chez votre registrar de domaine** (OVH, Gandi, Cloudflare, etc.)
2. Créer un enregistrement A :
   ```
   Nom:   symbion
   Type:  A
   Valeur: <IP publique de votre NAS>
   TTL:   300 (5 minutes)
   ```
3. Attendre propagation DNS (5-30 minutes)

## Firewall NAS

### Ouvrir les Ports

```bash
# Si utilisation d'ufw
sudo ufw allow 443/tcp comment 'HTTPS Symbion'
sudo ufw allow 8444/tcp comment 'MQTT WebSocket Symbion'
sudo ufw reload

# Si utilisation d'iptables
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 8444 -j ACCEPT
sudo iptables-save
```

### Port Forwarding (si routeur séparé)

Si le NAS est derrière un routeur :

1. Se connecter à l'interface web du routeur
2. Créer règles NAT :
   ```
   Port externe 443  → IP NAS:443
   Port externe 8444 → IP NAS:8444
   ```

## Tests de Validation

### Test 1: Depuis le NAS

```bash
# Test health check
curl -k https://symbion.markcha.fr/health

# Attendu: {"status":"ok"}
```

### Test 2: WebSocket MQTT

```bash
# Test WebSocket upgrade
curl -k -v \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  https://symbion.markcha.fr:8444/

# Attendu: HTTP/1.1 101 Switching Protocols
```

### Test 3: Depuis Internet

```bash
# Sur une machine externe (réseau 4G, VPN, etc.)
curl https://symbion.markcha.fr/health

# Test PWA
# Ouvrir navigateur: https://symbion.markcha.fr
```

## Monitoring & Logs

### Logs NAS

```bash
# Logs accès
sudo tail -f /var/log/nginx/symbion-nas-access.log

# Logs erreurs
sudo tail -f /var/log/nginx/symbion-nas-error.log

# Logs MQTT WebSocket
sudo tail -f /var/log/nginx/symbion-mqtt-nas-error.log
```

### Logs Serveur Symbion

```bash
# Sur le serveur Symbion
sudo tail -f /var/log/nginx/symbion-error.log
sudo tail -f /var/log/nginx/mqtt-ws-error.log
```

## Troubleshooting

### Problème: 502 Bad Gateway

**Cause possible**: Serveur Symbion inaccessible depuis NAS

**Debug**:
```bash
# Depuis le NAS, tester connectivité
ping 192.168.1.14  # IP serveur Symbion
curl -k https://192.168.1.14:443/health
curl -k https://192.168.1.14:8444/
```

**Fix**: Vérifier firewall serveur Symbion autorise trafic depuis IP NAS

### Problème: Connexion WebSocket échoue

**Cause possible**: HTTP/2 activé sur port 8444 du NAS

**Fix**: Vérifier config NAS :
```nginx
# DOIT être:
listen 8444 ssl;  # SANS http2

# PAS:
listen 8444 ssl http2;  # ❌ INCORRECT
```

### Problème: Certificat SSL invalide

**Cause possible**: Certificat self-signed sur serveur local

**Fix**: Désactiver vérification SSL dans proxy:
```nginx
proxy_ssl_verify off;
```

## Sécurité

### Recommandations

1. **Utiliser Let's Encrypt** en production (certificats gratuits, auto-renouvelés)
2. **Activer fail2ban** sur NAS pour bloquer attaques brute-force
3. **Limiter accès SSH** au NAS (clés SSH uniquement, pas de password)
4. **Monitoring** : Configurer alertes si services tombent
5. **Backups** : Sauvegarder configuration Nginx régulièrement

### Firewall Restrictif (Optionnel)

```bash
# Bloquer tout sauf ports nécessaires
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp   # SSH
sudo ufw allow 443/tcp  # HTTPS
sudo ufw allow 8444/tcp # MQTT WS
sudo ufw enable
```

## Prochaine Étape: Zero Trust

Une fois la configuration Nginx validée, vous pourrez migrer vers **Cloudflare Zero Trust** pour :
- ✅ Pas besoin d'ouvrir ports sur NAS
- ✅ DDoS protection
- ✅ Authentification multi-facteurs
- ✅ Access policies granulaires
- ✅ Logs centralisés

Documentation à venir: `CLOUDFLARE_ZERO_TRUST.md`

---

**Créé**: 22 Nov 2025
**Version**: 1.0.0
**Status**: Ready for NAS Configuration
