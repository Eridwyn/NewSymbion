# Symbion - Guide d'installation

## Prerequis

| Composant | Version | Notes |
|-----------|---------|-------|
| Rust | 1.84+ | `rustup update stable` |
| Node.js | 20+ | Pour le PWA dashboard |
| Mosquitto | 2.0+ | Broker MQTT |
| OpenSSL | 3.x | TLS pour HTTPS |

## Variables d'environnement

### Kernel (obligatoires)

```bash
SYMBION_API_KEY="<secret>"              # Cle API pour authentification agents
SYMBION_JWT_SECRET="<secret>"           # Secret JWT (min 32 chars)
SYMBION_MQTT_BROKER="127.0.0.1:1883"   # Adresse broker MQTT
```

### Kernel (optionnelles)

```bash
SYMBION_TIMEZONE="Europe/Paris"         # Fuseau horaire (defaut: Europe/Paris)
SYMBION_MQTT_MAX_PACKET="1048576"       # Taille max paquets MQTT en bytes (defaut: 1MB)
SYMBION_TRUST_SUCCESS_INCREMENT="0.01"  # Increment confiance par succes
SYMBION_TRUST_FAILURE_DECREMENT="0.05"  # Decrement confiance par echec
SYMBION_TRUST_DECAY_RATE="0.001"        # Taux de decay confiance
```

### Plugin Notes

```bash
SYMBION_NOTES_FILE="./notes.json"                      # Chemin fichier stockage
SYMBION_NOTES_SOCKET="/run/symbion-plugins/notes.sock"  # Socket Unix
```

### Plugin Freebox

```bash
FREEBOX_CONFIG="/opt/symbion/config/freebox.toml"  # Chemin config Freebox
```

## Installation systemd

### 1. Compiler le kernel

```bash
cd symbion-kernel
cargo build --release
sudo cp target/release/symbion-kernel /opt/symbion/bin/
```

### 2. Creer le service systemd

```ini
# /etc/systemd/system/symbion-kernel.service
[Unit]
Description=Symbion Kernel
After=network.target mosquitto.service

[Service]
Type=simple
User=symbion
Group=symbion
WorkingDirectory=/opt/symbion
ExecStart=/opt/symbion/bin/symbion-kernel
EnvironmentFile=/opt/symbion/config/symbion.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### 3. Activer et demarrer

```bash
sudo systemctl daemon-reload
sudo systemctl enable symbion-kernel
sudo systemctl start symbion-kernel
sudo systemctl status symbion-kernel
```

## Premier demarrage

1. **Verifier Mosquitto** : `sudo systemctl status mosquitto`
2. **Demarrer le kernel** : `sudo systemctl start symbion-kernel`
3. **Verifier la sante** : `curl http://localhost:8080/health`
4. **Consulter les logs** : `sudo journalctl -u symbion-kernel -f --no-pager`

## PWA Dashboard

```bash
cd pwa-dashboard
npm install
npm run dev        # Developpement (port 3000)
npm run build      # Production (dist/)
```

## Structure des repertoires

```
/opt/symbion/
  bin/                  # Binaires (kernel, plugins)
  config/               # Configuration (symbion.env, freebox.toml, certs/)
  data/                 # Donnees persistantes (modes.json, notes.json)
  logs/                 # Logs (optionnel, journalctl prefere)
/run/symbion-plugins/   # Sockets Unix des plugins
```
