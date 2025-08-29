# Symbion (kernel)

Cerveau perso modulaire. Kernel en Rust + bus MQTT. 
API HTTP pour humains, MQTT pour les agents.

## Prérequis
- Rust (stable) + cargo
- Mosquitto (broker MQTT) local
- Linux/WSL recommandé

## Démarrage rapide
```bash
# 1) cloner
git clone https://github.com/Eridwyn/NewSymbion
cd NewSymbion/symbion-kernel

# 2) config
cp kernel.yaml.example kernel.yaml
export SYMBION_API_KEY="s3cr3t-42"

# 3) run
cargo run
# -> listening on 0.0.0.0:8080

# 4) publier un heartbeat de test
mosquitto_pub -t symbion/hosts/heartbeat@v2 \
  -m '{"host_id":"desktop-w11","ts":"2025-08-29T12:00:00Z","metrics":{"cpu":0.5,"ram":0.4},"net":{"ip":"192.168.1.44"}}'
```