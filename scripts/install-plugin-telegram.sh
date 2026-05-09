#!/bin/bash
# Install / configure / restart symbion-plugin-telegram.
# Idempotent : peut être relancé sans risque, ne touche au config.env existant
# que si l'utilisateur le demande explicitement (--force-config).
#
# Crée scripts/telegram-bridge/config.env (gitignored, perm 600) avec le token
# et les ALLOWED_USER_IDS demandés à l'interactif.
#
# Évite la récidive de l'incident 9 mai 2026 où un restart d'un service tournant
# depuis 2 mois avait perdu son TELEGRAM_BOT_TOKEN qui n'était qu'en mémoire.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$REPO_ROOT/scripts/telegram-bridge"
CONFIG_FILE="$CONFIG_DIR/config.env"
SERVICE="symbion-plugin-telegram"

FORCE_CONFIG=0
SKIP_BUILD=0
for arg in "$@"; do
    case "$arg" in
        --force-config) FORCE_CONFIG=1 ;;
        --skip-build) SKIP_BUILD=1 ;;
        -h|--help)
            cat <<EOF
Usage: $0 [--force-config] [--skip-build]

Options:
  --force-config   Re-demande TELEGRAM_BOT_TOKEN et ALLOWED_USER_IDS même
                   si config.env existe déjà (re-création complète).
  --skip-build     Ne lance pas cargo build avant le restart.
EOF
            exit 0
            ;;
    esac
done

echo "[install-plugin-telegram] Repo : $REPO_ROOT"

# ---- 1. Build ----
if [ "$SKIP_BUILD" -eq 0 ]; then
    if [ -x "$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo" ]; then
        export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
    fi
    echo "[install-plugin-telegram] cargo build --release -p symbion-plugin-telegram"
    cd "$REPO_ROOT"
    cargo build --release -p symbion-plugin-telegram
fi

# ---- 2. Config dir ----
mkdir -p "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"

# ---- 3. config.env ----
if [ -f "$CONFIG_FILE" ] && [ "$FORCE_CONFIG" -eq 0 ]; then
    echo "[install-plugin-telegram] config.env existe déjà — vérification cohérence."
    # On vérifie juste que les 2 vars critiques sont présentes et non vides.
    set +e
    grep -qE '^TELEGRAM_BOT_TOKEN=.+' "$CONFIG_FILE"
    has_token=$?
    grep -qE '^ALLOWED_USER_IDS=[0-9]' "$CONFIG_FILE"
    has_ids=$?
    set -e

    if [ $has_token -ne 0 ] || [ $has_ids -ne 0 ]; then
        echo "ERREUR : config.env présent mais incomplet (token ou allowed_ids manquant)."
        echo "        Relance avec --force-config pour le recréer."
        exit 1
    fi
    echo "[install-plugin-telegram] config.env OK (token + allowed_ids présents)."
else
    echo "[install-plugin-telegram] Création de $CONFIG_FILE"
    echo
    echo "Saisis les credentials Telegram (rien n'est échoé en clair pour le token) :"

    # Token (silencieux, comme un mot de passe)
    read -r -s -p "  TELEGRAM_BOT_TOKEN  : " token
    echo
    if [ -z "$token" ]; then
        echo "ERREUR : token vide, abandon."
        exit 1
    fi

    # User IDs autorisés
    read -r -p "  ALLOWED_USER_IDS    (un ou plusieurs, séparés par virgule) : " ids
    if [ -z "$ids" ] || ! echo "$ids" | grep -qE '^[0-9, ]+$'; then
        echo "ERREUR : ALLOWED_USER_IDS doit être une liste d'entiers (ex: 123456789,987654321)."
        exit 1
    fi

    cat > "$CONFIG_FILE" <<EOF
# Symbion Telegram Plugin - Configuration (généré par install-plugin-telegram.sh)
# WARNING: contient des secrets — NE PAS COMMITTER (gitignored)

TELEGRAM_BOT_TOKEN=$token
ALLOWED_USER_IDS=$ids

CLAUDE_PATH=/usr/local/bin/claude
CLAUDE_TIMEOUT=600
CLAUDE_WORKDIR=$REPO_ROOT

SYMBION_MQTT_BROKER=127.0.0.1:1883
SYMBION_TELEGRAM_SOCKET=/run/symbion-plugins/telegram.sock
SYMBION_API_KEY=s3cr3t-42
EOF
    chmod 600 "$CONFIG_FILE"
    chown eridwyn:eridwyn "$CONFIG_FILE" 2>/dev/null || true
    echo "[install-plugin-telegram] config.env créé (perm 600, gitignored)."
fi

# ---- 4. Restart service ----
echo "[install-plugin-telegram] Restart $SERVICE"
sudo systemctl reset-failed "$SERVICE" 2>/dev/null || true
sudo systemctl restart "$SERVICE"

# Attente état stable
for i in $(seq 1 15); do
    state=$(sudo systemctl is-active "$SERVICE" || true)
    if [ "$state" = "active" ] || [ "$state" = "failed" ]; then break; fi
    sleep 1
done

state=$(sudo systemctl is-active "$SERVICE" || true)
if [ "$state" = "active" ]; then
    echo "[install-plugin-telegram] OK : service actif."
    sudo journalctl -u "$SERVICE" --since "30 seconds ago" --no-pager | grep -E "Plugin ready|Bot|Subscribed" | tail -5 || true
    exit 0
else
    echo "[install-plugin-telegram] ECHEC : service en état '$state'. Logs :"
    sudo journalctl -u "$SERVICE" --since "30 seconds ago" --no-pager | tail -10
    exit 1
fi
