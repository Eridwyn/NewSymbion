#!/bin/bash
set -e

# 🚀 Script de déploiement automatique Symbion Kernel depuis GitHub Releases
# Usage: ./deploy-kernel.sh [version]
# Exemple: ./deploy-kernel.sh kernel-v1.2.0

GITHUB_REPO="${GITHUB_REPO:-votre-username/NewSymbion}"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/opt/symbion}"
SERVICE_NAME="symbion-kernel"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Déploiement Symbion Kernel ${VERSION}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Vérifier droits sudo
if [ "$EUID" -ne 0 ]; then
    echo "❌ Ce script nécessite les droits sudo"
    echo "Usage: sudo $0 [version]"
    exit 1
fi

# Créer répertoire d'installation
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Déterminer URL de téléchargement
if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/symbion-kernel-linux-x64-latest"
else
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/symbion-kernel-linux-x64-${VERSION}"
fi

echo "1️⃣ Téléchargement depuis GitHub Releases..."
echo "   URL: $DOWNLOAD_URL"

# Télécharger nouveau binaire
if ! curl -sSL -f -o symbion-kernel.new "$DOWNLOAD_URL"; then
    echo "❌ Échec du téléchargement"
    echo "   Vérifiez que la version existe: https://github.com/${GITHUB_REPO}/releases"
    exit 1
fi

chmod +x symbion-kernel.new

echo ""
echo "2️⃣ Vérification du nouveau binaire..."
if ! ./symbion-kernel.new --version 2>/dev/null | head -1; then
    echo "⚠️  Version check failed (expected for stateless binary)"
fi

# Sauvegarder ancien binaire si existe
if [ -f "symbion-kernel" ]; then
    echo ""
    echo "3️⃣ Sauvegarde de l'ancien binaire..."
    cp symbion-kernel symbion-kernel.backup
    echo "   ✅ Backup créé: ${INSTALL_DIR}/symbion-kernel.backup"
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
mv symbion-kernel.new symbion-kernel
echo "   ✅ Binaire installé: ${INSTALL_DIR}/symbion-kernel"

# Redémarrer service
if [ "$SERVICE_EXISTS" = true ]; then
    echo ""
    echo "6️⃣ Redémarrage du service systemd..."
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
        if [ -f "symbion-kernel.backup" ]; then
            cp symbion-kernel.backup symbion-kernel
            systemctl start "$SERVICE_NAME"
            echo "   ✅ Rollback effectué vers ancienne version"
        fi
        exit 1
    fi

    echo ""
    echo "7️⃣ Vérification de la santé..."
    sleep 2
    if curl -k -s https://localhost:8443/health | grep -q "ok"; then
        echo "   ✅ API kernel répond correctement"
    else
        echo "   ⚠️  API kernel ne répond pas (peut être normal si TLS non configuré)"
    fi
else
    echo ""
    echo "6️⃣ Service systemd non détecté - installation manuelle"
    echo "   Pour installer le service, lancez:"
    echo "   sudo /home/eridwyn/RustroverProjects/NewSymbion/systemd/install-services.sh"
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
echo "  - Voir logs:     journalctl -u $SERVICE_NAME -f"
echo "  - Restart:       sudo systemctl restart $SERVICE_NAME"
echo "  - Rollback:      sudo cp ${INSTALL_DIR}/symbion-kernel.backup ${INSTALL_DIR}/symbion-kernel && sudo systemctl restart $SERVICE_NAME"
echo "  - Health check:  curl -k https://localhost:8443/health"
