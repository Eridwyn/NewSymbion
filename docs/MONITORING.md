# Symbion - Guide de monitoring

## Health checks

### Kernel

```bash
# Endpoint sante
curl -s http://localhost:8080/health | jq

# Reponse attendue
# {"status":"ok","uptime":12345,"agents":2,"mqtt":"connected"}
```

### Plugins (via kernel proxy)

```bash
# Sante du plugin notes
curl -s --unix-socket /run/symbion-plugins/notes.sock http://localhost/health

# Sante du plugin freebox
curl -s --unix-socket /run/symbion-plugins/freebox.sock http://localhost/health
```

## Logs systemd

```bash
# Logs kernel en temps reel
sudo journalctl -u symbion-kernel -f --no-pager

# Logs derniere heure
sudo journalctl -u symbion-kernel --since "1 hour ago" --no-pager

# Logs avec niveau specifique
sudo journalctl -u symbion-kernel -p err --no-pager   # Erreurs uniquement
sudo journalctl -u symbion-kernel -p warning --no-pager

# Taille des logs
sudo journalctl -u symbion-kernel --disk-usage
```

## Debug MQTT

### Ecouter tous les topics Symbion

```bash
# Installer mosquitto-clients si necessaire
sudo apt install mosquitto-clients

# Ecouter tous les topics
mosquitto_sub -h localhost -t 'symbion/#' -v

# Ecouter un topic specifique
mosquitto_sub -h localhost -t 'symbion/kernel/health@v1' -v

# Ecouter les heartbeats agents
mosquitto_sub -h localhost -t 'symbion/hosts/heartbeat@v2' -v

# Ecouter le contexte dashboard
mosquitto_sub -h localhost -t 'symbion/dashboard/context@v1' -v
```

### Topics principaux

| Topic | Frequence | Contenu |
|-------|-----------|---------|
| `symbion/kernel/health@v1` | 30s | Etat kernel |
| `symbion/hosts/heartbeat@v2` | 15s | Telemetrie agents |
| `symbion/dashboard/context@v1` | On change | Mode actif + features |
| `symbion/dashboard/agents/+` | 15s | Etat individuel agents |
| `symbion/notifications/sent@v1` | On event | Notifications push |

## Alertes automatiques

Le script `scripts/monitor-symbion.sh` execute toutes les 15 minutes (cron) :

```bash
# Installer le monitoring
crontab -e
# Ajouter:
*/15 * * * * /opt/symbion/scripts/monitor-symbion.sh

# Verifications effectuees:
# - Kernel HTTP health check
# - Mosquitto service status
# - Agents heartbeat age (alerte si > 5 min)
# - Disk space (alerte si > 90%)
```

## Metriques cles

| Metrique | Source | Seuil alerte |
|----------|--------|--------------|
| Kernel uptime | `/health` | < 60s (restart recent) |
| Agents connectes | `/health` | 0 (aucun agent) |
| MQTT status | `/health` | "disconnected" |
| CPU agent | heartbeat MQTT | > 90% sustained |
| Memory agent | heartbeat MQTT | > 85% |
| Heartbeat age | timestamp | > 5 minutes |

## Troubleshooting rapide

| Symptome | Commande diagnostic | Solution |
|----------|--------------------|-----------|
| Kernel ne demarre pas | `journalctl -u symbion-kernel -n 50` | Verifier env vars |
| MQTT deconnecte | `systemctl status mosquitto` | `systemctl restart mosquitto` |
| PWA ne charge pas | `curl localhost:8080/health` | Verifier kernel + MQTT |
| Agent absent | `mosquitto_sub -t 'symbion/hosts/#' -v` | Verifier agent-host |
