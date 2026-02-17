#!/bin/bash
# Script de monitoring Symbion avec alertes mail
# Usage: ./monitor-symbion.sh [email@example.com]

set -euo pipefail

# Forcer un PATH complet pour les binaires système (important dans le contexte cron)
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# Configuration
KERNEL_URL="${KERNEL_URL:-https://localhost:8443}"
API_KEY="${SYMBION_API_KEY:-s3cr3t-42}"
ALERT_EMAIL="${1:-}"
LOG_FILE="/tmp/symbion-monitor.log"
STATE_FILE="/tmp/symbion-monitor.state"
ERROR_DIR="/tmp/symbion-errors"
CURL_OPTS="-k"  # Accept self-signed certificates

# Healthcheck.io ping (optionnel) — creer un check sur https://healthchecks.io
# et coller l'UUID ici ou dans l'env HEALTHCHECK_UUID
HEALTHCHECK_UUID="${HEALTHCHECK_UUID:-}"

# Créer le répertoire d'erreurs si nécessaire
mkdir -p "$ERROR_DIR"

# Buffer pour accumuler toutes les erreurs et envoyer un seul email
declare -a CONFIRMED_ERRORS_SUBJECTS
declare -a CONFIRMED_ERRORS_BODIES

# Couleurs pour logs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

error() {
    log "${RED}❌ ERROR: $1${NC}"
}

warn() {
    log "${YELLOW}⚠️  WARNING: $1${NC}"
}

success() {
    log "${GREEN}✅ $1${NC}"
}

# Fonction pour tracker une erreur et l'ajouter au batch si confirmée
send_alert() {
    local error_type="$1"
    local subject="$2"
    local body="$3"
    local error_file="$ERROR_DIR/${error_type}"

    # Vérifier si cette erreur a déjà été vue au check précédent
    if [ -f "$error_file" ]; then
        # Erreur confirmée (2ème occurrence consécutive) → AJOUTER AU BATCH
        warn "⚠️  Erreur confirmée (2ème check consécutif): $error_type"
        CONFIRMED_ERRORS_SUBJECTS+=("$subject")
        CONFIRMED_ERRORS_BODIES+=("$body")
    else
        # Première occurrence → LOGGER uniquement (pas d'email)
        warn "⏳ Erreur détectée (1er check): $error_type - en attente de confirmation"
        echo "$(date '+%Y-%m-%d %H:%M:%S')" > "$error_file"
    fi
}

# Fonction pour envoyer un seul email groupé avec toutes les erreurs
send_batch_alert() {
    # Compter les erreurs confirmées (désactiver set -u temporairement)
    set +u
    local error_count="${#CONFIRMED_ERRORS_SUBJECTS[@]}"
    set -u

    if [ "$error_count" -eq 0 ]; then
        # Aucune erreur confirmée
        return 0
    fi

    if [ -z "$ALERT_EMAIL" ]; then
        warn "Pas d'email configuré, alerte non envoyée"
        return 1
    fi

    # Construire le sujet et le corps de l'email groupé
    local subject="[Symbion] $error_count problème(s) détecté(s)"
    local body="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🚨 RAPPORT D'ALERTES SYMBION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Date: $(date '+%Y-%m-%d %H:%M:%S')
Nombre d'erreurs: $error_count

"

    # Ajouter chaque erreur au corps de l'email
    for i in "${!CONFIRMED_ERRORS_SUBJECTS[@]}"; do
        local err_subject="${CONFIRMED_ERRORS_SUBJECTS[$i]}"
        local err_body="${CONFIRMED_ERRORS_BODIES[$i]}"
        body+="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
❌ ERREUR $((i+1)): $err_subject
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

$err_body

"
    done

    body+="━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 ACTIONS RECOMMANDÉES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. Vérifier les logs: /tmp/symbion-monitor.log
2. Vérifier le statut: curl -k https://localhost:8443/health
3. Vérifier MQTT: mosquitto_sub -h localhost -p 1883 -t 'symbion/#' -v
4. Redémarrer si nécessaire: systemctl restart symbion-kernel

Ce rapport groupe toutes les erreurs confirmées (détectées lors de 2 checks consécutifs).
"

    # Envoyer l'email groupé
    if command -v mail &> /dev/null; then
        echo "$body" | mail -s "$subject" "$ALERT_EMAIL"
        log "📧 Email groupé envoyé à $ALERT_EMAIL ($error_count erreur(s))"
    elif command -v msmtp &> /dev/null; then
        # RFC 2822 compliant headers pour msmtp
        cat <<EOF | msmtp "$ALERT_EMAIL"
From: symbion@$(hostname)
To: $ALERT_EMAIL
Subject: $subject
Date: $(date -R)

$body
EOF
        log "📧 Email groupé envoyé à $ALERT_EMAIL via msmtp ($error_count erreur(s))"
    else
        error "Aucun client mail trouvé (mail ou msmtp)"
        echo "$body" >> /tmp/symbion-alerts.txt
        warn "Alerte sauvegardée dans /tmp/symbion-alerts.txt"
    fi
}

# Fonction pour nettoyer les erreurs résolues
clear_error() {
    local error_type="$1"
    local error_file="$ERROR_DIR/${error_type}"

    if [ -f "$error_file" ]; then
        rm -f "$error_file"
        log "✅ Erreur résolue: $error_type"
    fi
}

# Vérifier si le kernel répond
check_kernel_alive() {
    if ! curl $CURL_OPTS -s -f -m 5 "$KERNEL_URL/health" > /dev/null 2>&1; then
        error "Kernel ne répond pas sur $KERNEL_URL"
        send_alert "kernel_down" "Kernel DOWN" "Le kernel Symbion ne répond pas sur $KERNEL_URL/health"
        return 1
    fi
    clear_error "kernel_down"
    return 0
}

# Vérifier le health du système
check_system_health() {
    local health_json
    health_json=$(curl $CURL_OPTS -s -H "x-api-key: $API_KEY" "$KERNEL_URL/system/health" 2>/dev/null)

    if [ -z "$health_json" ]; then
        error "Impossible de récupérer le health"
        return 1
    fi

    # Parser le JSON (nécessite jq)
    if ! command -v jq &> /dev/null; then
        warn "jq non installé, parsing JSON limité"
        echo "$health_json"
        return 0
    fi

    local mqtt_status=$(echo "$health_json" | jq -r '.mqtt_status')
    local agents_count=$(echo "$health_json" | jq -r '.agents_count')
    local uptime=$(echo "$health_json" | jq -r '.uptime_seconds')

    log "📊 Health: MQTT=$mqtt_status, Agents=$agents_count, Uptime=${uptime}s"

    # Alertes
    if [ "$mqtt_status" != "connected" ]; then
        error "MQTT status incorrect: $mqtt_status"
        send_alert "mqtt_disconnected" "MQTT Disconnected" "Status MQTT: $mqtt_status (devrait être 'connected')"
        return 1
    fi
    clear_error "mqtt_disconnected"

    return 0
}

# Vérifier les agents
check_agents() {
    local agents_json
    agents_json=$(curl $CURL_OPTS -s -H "x-api-key: $API_KEY" "$KERNEL_URL/agents" 2>/dev/null)

    if [ -z "$agents_json" ]; then
        error "Impossible de récupérer la liste des agents"
        return 1
    fi

    if ! command -v jq &> /dev/null; then
        echo "$agents_json"
        return 0
    fi

    local agent_count=$(echo "$agents_json" | jq 'length')
    log "🤖 $agent_count agent(s) enregistré(s)"

    # Vérifier chaque agent
    local online=0
    local offline=0
    local stale=0

    while IFS= read -r agent; do
        local agent_id=$(echo "$agent" | jq -r '.agent_id')
        local hostname=$(echo "$agent" | jq -r '.hostname')
        local status=$(echo "$agent" | jq -r '.status')
        local last_seen=$(echo "$agent" | jq -r '.last_seen')
        local cpu=$(echo "$agent" | jq -r '.cpu_percent')
        local ram=$(echo "$agent" | jq -r '.memory_percent')

        if [ "$status" == "online" ]; then
            ((online++))
            if [ "$cpu" != "null" ] && [ "$ram" != "null" ]; then
                success "Agent $hostname ($agent_id): online - CPU: ${cpu}%, RAM: ${ram}%"
            else
                warn "Agent $hostname ($agent_id): online mais sans métriques"
                ((stale++))
            fi
        else
            ((offline++))
            log "   Agent $hostname ($agent_id): $status (last seen: $last_seen)"
        fi
    done < <(echo "$agents_json" | jq -c '.[]')

    log "   📈 Résumé: $online online, $offline offline, $stale stale"

    # Alerte si tous les agents sont offline
    if [ "$online" -eq 0 ] && [ "$agent_count" -gt 0 ]; then
        error "Tous les agents sont offline!"
        send_alert "agents_offline" "All Agents Offline" "Aucun agent actif détecté ($agent_count agents enregistrés)"
        return 1
    fi
    clear_error "agents_offline"

    # Alerte si des agents online n'ont pas de métriques (stale)
    if [ "$stale" -gt 0 ]; then
        warn "$stale agent(s) online sans métriques (heartbeat non reçu)"
    fi

    return 0
}

# Vérifier les plugins via systemd
check_plugins() {
    log "🔌 Checking plugins via systemd..."

    local plugins_ok=0
    local plugins_total=0
    local all_ok=true

    # Liste des plugins à vérifier
    local plugins=("notes" "notifications" "sensors")

    for plugin in "${plugins[@]}"; do
        plugins_total=$((plugins_total + 1))

        if sudo systemctl is-active --quiet "symbion-plugin-$plugin" 2>/dev/null; then
            plugins_ok=$((plugins_ok + 1))
            success "Plugin $plugin: running"
            clear_error "plugin_${plugin}_stopped"
        else
            error "Plugin $plugin: stopped or not installed"
            send_alert "plugin_${plugin}_stopped" "Plugin $plugin Stopped" "Le plugin $plugin n'est pas actif (sudo systemctl status symbion-plugin-$plugin)

Actions recommandées:
- Vérifier les logs: sudo journalctl -u symbion-plugin-$plugin
- Redémarrer: sudo systemctl restart symbion-plugin-$plugin
- Vérifier la configuration: sudo systemctl status symbion-plugin-$plugin"
            all_ok=false
        fi
    done

    log "   📊 Résumé plugins: $plugins_ok/$plugins_total running"

    if [ "$plugins_ok" -eq 0 ]; then
        error "Aucun plugin actif!"
        send_alert "all_plugins_stopped" "All Plugins Stopped" "Aucun plugin n'est actuellement actif via systemd.

Vérifier:
- sudo systemctl list-units 'symbion-plugin-*'
- sudo journalctl -xe"
        return 1
    fi

    if $all_ok; then
        clear_error "all_plugins_stopped"
        return 0
    else
        return 1
    fi
}

# Sauvegarder l'état pour comparaison
save_state() {
    local state="$1"
    # Tenter d'écrire le state, ignorer les erreurs de permissions
    echo "$state" > "$STATE_FILE" 2>/dev/null || {
        warn "Impossible de sauvegarder l'état (permissions?), continuant..."
        return 0  # Ne pas crasher le script
    }
}

get_last_state() {
    if [ -f "$STATE_FILE" ]; then
        cat "$STATE_FILE"
    else
        echo "unknown"
    fi
}

# Fonction principale
main() {
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    log "🔍 Monitoring Symbion - Démarrage"

    if [ -n "$ALERT_EMAIL" ]; then
        log "📧 Alertes configurées vers: $ALERT_EMAIL"
    else
        warn "Pas d'email configuré - Usage: $0 email@example.com"
    fi

    local all_ok=true

    # Check 1: Kernel alive
    if ! check_kernel_alive; then
        all_ok=false
        save_state "kernel_down"
        return 1
    fi

    # Check 2: System health
    if ! check_system_health; then
        all_ok=false
    fi

    # Check 3: Agents
    if ! check_agents; then
        all_ok=false
    fi

    # Check 4: Plugins systemd status (DISABLED - false positives)
    # if ! check_plugins; then
    #     all_ok=false
    # fi

    # Envoyer l'email groupé avec toutes les erreurs confirmées
    send_batch_alert

    # Récupération si c'était down avant
    local last_state=$(get_last_state)
    if [ "$last_state" == "error" ] && $all_ok; then
        success "🎉 Système rétabli!"
        # Envoyer email de récupération uniquement si tout est OK maintenant
        if [ -n "$ALERT_EMAIL" ]; then
            local body="✅ Le système Symbion est de nouveau opérationnel

Date: $(date '+%Y-%m-%d %H:%M:%S')

Tous les checks sont maintenant OK:
- ✅ Kernel: Responsive
- ✅ MQTT: Connected
- ✅ Agents: Online
- ✅ Plugins: Running

Les erreurs précédentes ont été résolues."

            if command -v mail &> /dev/null; then
                echo "$body" | mail -s "[Symbion] ✅ Système rétabli" "$ALERT_EMAIL"
                log "📧 Email de récupération envoyé"
            elif command -v msmtp &> /dev/null; then
                # RFC 2822 compliant headers pour msmtp
                cat <<EOF | msmtp "$ALERT_EMAIL"
From: symbion@$(hostname)
To: $ALERT_EMAIL
Subject: [Symbion] ✅ Système rétabli
Date: $(date -R)

$body
EOF
                log "📧 Email de récupération envoyé via msmtp"
            fi
        fi
        save_state "ok"
        # Nettoyer toutes les erreurs trackées
        rm -f "$ERROR_DIR"/*
    elif $all_ok; then
        success "✨ Tous les checks OK"
        save_state "ok"
        # Nettoyer toutes les erreurs trackées
        rm -f "$ERROR_DIR"/*
    else
        error "Des problèmes ont été détectés"
        save_state "error"
    fi

    # Ping healthcheck.io (monitoring externe)
    if [ -n "$HEALTHCHECK_UUID" ]; then
        local hc_url="https://hc-ping.com/$HEALTHCHECK_UUID"
        if $all_ok; then
            curl -fsS -m 10 --retry 3 "$hc_url" > /dev/null 2>&1 && \
                log "📡 Healthcheck.io: ping OK" || \
                warn "Healthcheck.io: ping failed (network?)"
        else
            curl -fsS -m 10 --retry 3 "$hc_url/fail" > /dev/null 2>&1 && \
                log "📡 Healthcheck.io: ping FAIL envoyé" || \
                warn "Healthcheck.io: ping failed (network?)"
        fi
    fi

    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Exécution
main "$@"
