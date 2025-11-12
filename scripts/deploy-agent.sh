#!/bin/bash
set -e

# 🤖 Script de déploiement automatique Symbion Agent (Linux)
# Usage: ./deploy-agent.sh [version] [kernel_host]
# Exemple: ./deploy-agent.sh agent-v1.0.0 192.168.1.14

GITHUB_REPO="${GITHUB_REPO:-votre-username/NewSymbion}"
VERSION="${1:-latest}"
KERNEL_HOST="${2:-192.168.1.14}"
MQTT_BROKER="${3:-${KERNEL_HOST}:1883}"
INSTALL_DIR="${INSTALL_DIR:-/opt/symbion}"
SERVICE_NAME="symbion-agent"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🤖 Déploiement Symbion Agent ${VERSION}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "   Kernel Central: ${KERNEL_HOST}"
echo "   MQTT Broker:    ${MQTT_BROKER}"
echo ""

# Vérifier droits sudo si nécessaire
if [ ! -w "$(dirname "$INSTALL_DIR")" ]; then
    echo "❌ Permissions insuffisantes"
    echo "Usage: sudo $0 [version] [kernel_host]"
    exit 1
fi

# Créer répertoire d'installation
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Déterminer URL de téléchargement
if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/symbion-agent-linux-x64-latest"
else
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/symbion-agent-linux-x64-${VERSION}"
fi

echo "1️⃣ Téléchargement depuis GitHub Releases..."
echo "   URL: $DOWNLOAD_URL"

# Télécharger nouveau binaire
if ! curl -sSL -f -o symbion-agent-host.new "$DOWNLOAD_URL"; then
    echo "❌ Échec du téléchargement"
    echo "   Vérifiez que la version existe: https://github.com/${GITHUB_REPO}/releases"
    exit 1
fi

chmod +x symbion-agent-host.new

echo ""
echo "2️⃣ Vérification du nouveau binaire..."
sha256sum symbion-agent-host.new | head -1

# Sauvegarder ancien binaire si existe
if [ -f "symbion-agent-host" ]; then
    echo ""
    echo "3️⃣ Sauvegarde de l'ancien binaire..."
    cp symbion-agent-host symbion-agent-host.backup
    echo "   ✅ Backup créé: ${INSTALL_DIR}/symbion-agent-host.backup"
fi

# Vérifier si service systemd existe
SERVICE_EXISTS=false
if systemctl list-unit-files | grep -q "^${SERVICE_NAME}.service"; then
    SERVICE_EXISTS=true
    echo ""
    echo "4️⃣ Service systemd détecté, arrêt en cours..."
    systemctl stop "$SERVICE_NAME" || true
    sleep 2
fi

# Remplacer binaire
echo ""
echo "5️⃣ Installation du nouveau binaire..."
mv symbion-agent-host.new symbion-agent-host
echo "   ✅ Binaire installé: ${INSTALL_DIR}/symbion-agent-host"

# Créer fichier de configuration
echo ""
echo "6️⃣ Configuration de connexion au kernel central..."

cat > "${INSTALL_DIR}/agent-config.env" <<EOF
# Configuration Symbion Agent
# Ce fichier est chargé par le service systemd

SYMBION_MQTT_BROKER=${MQTT_BROKER}
SYMBION_KERNEL_HOST=${KERNEL_HOST}
RUST_LOG=info

# ID Agent (auto-généré au premier lancement basé sur hostname)
# SYMBION_AGENT_ID=$(hostname)
EOF

echo "   ✅ Configuration créée: ${INSTALL_DIR}/agent-config.env"

# Mettre à jour service systemd si existe
if [ "$SERVICE_EXISTS" = true ]; then
    echo ""
    echo "7️⃣ Mise à jour configuration service systemd..."

    # Vérifier si le service utilise EnvironmentFile
    SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
    if [ -f "$SERVICE_FILE" ]; then
        # Mettre à jour variable MQTT_BROKER dans le service
        sed -i "s|Environment=\"SYMBION_MQTT_BROKER=.*\"|Environment=\"SYMBION_MQTT_BROKER=${MQTT_BROKER}\"|" "$SERVICE_FILE"
        systemctl daemon-reload
        echo "   ✅ Service configuré pour kernel: ${KERNEL_HOST}"
    fi
fi

# Redémarrer service
if [ "$SERVICE_EXISTS" = true ]; then
    echo ""
    echo "8️⃣ Redémarrage du service systemd..."
    systemctl start "$SERVICE_NAME"
    sleep 3

    # Vérifier status
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        echo "   ✅ Service actif"
    else
        echo "   ❌ Service failed to start"
        echo ""
        echo "   📋 Logs (dernières 20 lignes):"
        journalctl -u "$SERVICE_NAME" -n 20 --no-pager
        echo ""
        echo "   🔄 Rollback automatique..."
        if [ -f "symbion-agent-host.backup" ]; then
            cp symbion-agent-host.backup symbion-agent-host
            systemctl start "$SERVICE_NAME"
            echo "   ✅ Rollback effectué vers ancienne version"
        fi
        exit 1
    fi

    echo ""
    echo "9️⃣ Vérification de l'enregistrement au kernel..."
    sleep 5

    # Vérifier que l'agent est visible sur le kernel
    AGENT_ID=$(hostname)
    if curl -k -s -H "x-api-key: s3cr3t-42" "https://${KERNEL_HOST}:8443/agents" | grep -q "$AGENT_ID"; then
        echo "   ✅ Agent enregistré sur kernel central"
        echo "   Agent ID: $AGENT_ID"
    else
        echo "   ⚠️  Agent pas encore visible sur kernel (peut prendre 30s)"
        echo "   Vérifiez avec: curl -k -H 'x-api-key: s3cr3t-42' https://${KERNEL_HOST}:8443/agents"
    fi
else
    echo ""
    echo "8️⃣ Service systemd non détecté - installation manuelle"
    echo "   Pour installer le service, créez le fichier:"
    echo "   /etc/systemd/system/symbion-agent.service"
    echo ""
    echo "   Exemple de contenu:"
    cat <<'EOF'

[Unit]
Description=Symbion Agent Host - System Monitoring
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/symbion
Environment="SYMBION_MQTT_BROKER=192.168.1.14:1883"
Environment="RUST_LOG=info"
ExecStart=/opt/symbion/symbion-agent-host
Restart=always
RestartSec=15s

[Install]
WantedBy=multi-user.target
EOF
    echo ""
    echo "   Puis lancez:"
    echo "   sudo systemctl daemon-reload"
    echo "   sudo systemctl enable symbion-agent"
    echo "   sudo systemctl start symbion-agent"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Déploiement réussi!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "📊 Status:"
if [ "$SERVICE_EXISTS" = true ]; then
    systemctl status "$SERVICE_NAME" --no-pager -l | head -15
fi

echo ""
echo "📝 Commandes utiles:"
echo "  - Voir logs:         journalctl -u $SERVICE_NAME -f"
echo "  - Restart:           sudo systemctl restart $SERVICE_NAME"
echo "  - Rollback:          sudo cp ${INSTALL_DIR}/symbion-agent-host.backup ${INSTALL_DIR}/symbion-agent-host && sudo systemctl restart $SERVICE_NAME"
echo "  - Test connexion:    curl -k https://${KERNEL_HOST}:8443/agents"
echo ""

echo "🌐 Prochaines étapes:"
echo "  1. Vérifier que l'agent apparaît sur le dashboard: http://${KERNEL_HOST}:3000"
echo "  2. Vérifier les métriques temps réel (CPU/RAM)"
echo "  3. Tester contrôles système (shutdown/hibernate)"
echo ""
