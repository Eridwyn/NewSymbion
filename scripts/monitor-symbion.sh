#!/bin/bash
# Script de monitoring Symbion avec alertes mail
# Usage: ./monitor-symbion.sh [email@example.com]

set -euo pipefail

# Configuration
KERNEL_URL="${KERNEL_URL:-https://localhost:8443}"
API_KEY="${SYMBION_API_KEY:-s3cr3t-42}"
ALERT_EMAIL="${1:-}"
LOG_FILE="/tmp/symbion-monitor.log"
STATE_FILE="/tmp/symbion-monitor.state"
ERROR_DIR="/tmp/symbion-errors"
CURL_OPTS="-k"  # Accept self-signed certificates

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
    local error_count="${#CONFIRMED_ERRORS_SUBJECTS[@]}"

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
        echo -e "Subject: $subject\n\n$body" | msmtp "$ALERT_EMAIL"
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
    local plugins_active=$(echo "$health_json" | jq -r '.plugins_active')
    local plugins_failed=$(echo "$health_json" | jq -r '.plugins_failed')

    log "📊 Health: MQTT=$mqtt_status, Agents=$agents_count, Uptime=${uptime}s, Plugins=$plugins_active/$plugins_failed"

    # Alertes
    if [ "$mqtt_status" != "connected" ]; then
        error "MQTT status incorrect: $mqtt_status"
        send_alert "mqtt_disconnected" "MQTT Disconnected" "Status MQTT: $mqtt_status (devrait être 'connected')"
        return 1
    fi
    clear_error "mqtt_disconnected"

    if [ "$plugins_failed" != "0" ]; then
        warn "$plugins_failed plugin(s) en échec"
        send_alert "plugins_failed" "Plugins Failed" "$plugins_failed plugin(s) en état d'échec"
    else
        clear_error "plugins_failed"
    fi

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

# Vérifier les plugins
check_plugins() {
    local plugins_json
    plugins_json=$(curl $CURL_OPTS -s -H "x-api-key: $API_KEY" "$KERNEL_URL/plugins" 2>/dev/null)

    if [ -z "$plugins_json" ]; then
        error "Impossible de récupérer la liste des plugins"
        return 1
    fi

    if ! command -v jq &> /dev/null; then
        echo "$plugins_json"
        return 0
    fi

    local plugin_count=$(echo "$plugins_json" | jq 'length')
    log "🔌 $plugin_count plugin(s) découvert(s)"

    while IFS= read -r plugin; do
        local name=$(echo "$plugin" | jq -r '.name')
        local status=$(echo "$plugin" | jq -r '.status')
        local uptime=$(echo "$plugin" | jq -r '.uptime_seconds')
        local restart_count=$(echo "$plugin" | jq -r '.restart_count // 0')

        if [ "$status" == "Running" ]; then
            # Surveillance spécifique du plugin notes
            if [ "$name" == "notes-manager" ]; then
                local last_restart_count=$(get_plugin_restart_count "$name")

                if [ "$last_restart_count" != "unknown" ] && [ "$restart_count" -gt "$last_restart_count" ]; then
                    local crashes=$((restart_count - last_restart_count))
                    error "Plugin $name a crashé $crashes fois! (restarts: $last_restart_count → $restart_count)"
                    send_alert "plugin_notes_crashed" "Plugin Notes Crashed" "Le plugin notes-manager a redémarré $crashes fois.\n\nRestart count: $last_restart_count → $restart_count\nUptime actuel: ${uptime}s\n\nVérifier les logs pour identifier la cause."
                    warn "Check recommandé: journalctl -xe | grep notes"
                else
                    clear_error "plugin_notes_crashed"
                fi

                # Test fonctionnel : vérifier que l'API notes répond
                local notes_count
                notes_count=$(curl $CURL_OPTS -s -H "x-api-key: $API_KEY" "$KERNEL_URL/v1/plugin-api/notes/notes" 2>/dev/null | jq '.notes | length' 2>/dev/null)

                if [ -z "$notes_count" ] || [ "$notes_count" == "null" ]; then
                    error "Plugin $name ne répond pas à l'API /v1/plugin-api/notes/notes (timeout ou erreur)"
                    send_alert "plugin_notes_api_failed" "Plugin Notes API Failed" "Le plugin notes-manager est Running mais l'API /v1/plugin-api/notes/notes ne répond pas.\n\nUptime: ${uptime}s\nRestart count: $restart_count\n\nLe plugin ou le reverse proxy est probablement bloqué."
                    warn "Vérifier: curl -H 'x-api-key: $API_KEY' $KERNEL_URL/v1/plugin-api/notes/notes"
                else
                    clear_error "plugin_notes_api_failed"
                    # Vérification simple du nombre de notes
                    if [ "$notes_count" -eq 0 ]; then
                        warn "Plugin $name ne retourne aucune note (0 notes stockées)"
                    else
                        success "Plugin $name: Running (uptime: ${uptime}s, restarts: $restart_count, notes: $notes_count)"
                    fi
                fi

                save_plugin_restart_count "$name" "$restart_count"
            else
                success "Plugin $name: Running (uptime: ${uptime}s, restarts: $restart_count)"
            fi
        else
            error "Plugin $name: $status"
            send_alert "plugin_${name}_failed" "Plugin Failed: $name" "Le plugin $name est en état: $status"
        fi
    done < <(echo "$plugins_json" | jq -c '.[]')

    return 0
}

# Sauvegarder le restart_count d'un plugin
save_plugin_restart_count() {
    local plugin_name="$1"
    local restart_count="$2"
    local restart_file="/tmp/symbion-plugin-${plugin_name}.restarts"
    echo "$restart_count" > "$restart_file"
}

# Récupérer le dernier restart_count connu
get_plugin_restart_count() {
    local plugin_name="$1"
    local restart_file="/tmp/symbion-plugin-${plugin_name}.restarts"
    if [ -f "$restart_file" ]; then
        cat "$restart_file"
    else
        echo "unknown"
    fi
}

# Sauvegarder le notes_count d'un plugin
save_plugin_notes_count() {
    local plugin_name="$1"
    local notes_count="$2"
    local notes_file="/tmp/symbion-plugin-${plugin_name}.notes"
    echo "$notes_count" > "$notes_file"
}

# Récupérer le dernier notes_count connu
get_plugin_notes_count() {
    local plugin_name="$1"
    local notes_file="/tmp/symbion-plugin-${plugin_name}.notes"
    if [ -f "$notes_file" ]; then
        cat "$notes_file"
    else
        echo "unknown"
    fi
}

# Sauvegarder l'état pour comparaison
save_state() {
    local state="$1"
    echo "$state" > "$STATE_FILE"
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

    # Check 4: Plugins
    if ! check_plugins; then
        all_ok=false
    fi

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
                echo -e "Subject: [Symbion] ✅ Système rétabli\n\n$body" | msmtp "$ALERT_EMAIL"
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

    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Exécution
main "$@"
