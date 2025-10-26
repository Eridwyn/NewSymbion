#!/bin/bash
set -e

echo "🔧 Installation des services Symbion systemd..."

# Vérifier droits sudo
if [ "$EUID" -ne 0 ]; then
    echo "❌ Ce script doit être exécuté avec sudo"
    echo "Usage: sudo ./install-services.sh"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSTEMD_DIR="/etc/systemd/system"

echo ""
echo "1️⃣ Arrêt des anciens process manuels..."
pkill -f symbion-kernel || true
pkill -f symbion-agent-host || true
sleep 2

echo ""
echo "2️⃣ Création du répertoire de déploiement..."
mkdir -p /opt/symbion
chown eridwyn:eridwyn /opt/symbion

echo ""
echo "3️⃣ Copie des binaires initiaux..."
BUILD_DIR="/home/eridwyn/RustroverProjects/NewSymbion/target/release"
if [ -f "$BUILD_DIR/symbion-kernel" ]; then
    cp "$BUILD_DIR/symbion-kernel" /opt/symbion/
    chmod +x /opt/symbion/symbion-kernel
    echo "   ✅ Kernel copié dans /opt/symbion/"
else
    echo "   ⚠️  Binaire kernel non trouvé, compilez d'abord: cargo build --release -p symbion-kernel"
fi

if [ -f "$BUILD_DIR/symbion-agent-host" ]; then
    cp "$BUILD_DIR/symbion-agent-host" /opt/symbion/
    chmod +x /opt/symbion/symbion-agent-host
    echo "   ✅ Agent copié dans /opt/symbion/"
else
    echo "   ⚠️  Binaire agent non trouvé, compilez d'abord: cargo build --release -p symbion-agent-host"
fi

echo ""
echo "4️⃣ Copie des fichiers service..."
cp "$SCRIPT_DIR/symbion-kernel.service" "$SYSTEMD_DIR/"
cp "$SCRIPT_DIR/symbion-agent.service" "$SYSTEMD_DIR/"

echo ""
echo "5️⃣ Rechargement systemd..."
systemctl daemon-reload

echo ""
echo "6️⃣ Activation des services au démarrage..."
systemctl enable symbion-kernel.service
systemctl enable symbion-agent.service

echo ""
echo "7️⃣ Démarrage des services..."
systemctl start symbion-kernel.service
sleep 3
systemctl start symbion-agent.service

echo ""
echo "✅ Installation terminée!"
echo ""
echo "📊 Status des services:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
systemctl status symbion-kernel.service --no-pager -l || true
echo ""
systemctl status symbion-agent.service --no-pager -l || true

echo ""
echo "8️⃣ Installation optionnelle du Dashboard (PWA)..."
read -p "   Installer le service Dashboard ? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Vérifier si le build existe
    DASHBOARD_DIR="/home/eridwyn/RustroverProjects/NewSymbion/pwa-dashboard"
    if [ -d "$DASHBOARD_DIR/dist" ]; then
        echo "   ✅ Build dashboard trouvé, déploiement..."
        mkdir -p /var/www/symbion-dashboard
        cp -r "$DASHBOARD_DIR/dist/"* /var/www/symbion-dashboard/
        cp "$SCRIPT_DIR/symbion-dashboard.service" "$SYSTEMD_DIR/"
        systemctl daemon-reload
        systemctl enable symbion-dashboard.service
        systemctl start symbion-dashboard.service
        echo "   ✅ Dashboard service installé et démarré"
    else
        echo "   ⚠️  Build dashboard non trouvé. Compilez d'abord:"
        echo "      cd $DASHBOARD_DIR && npm run build"
    fi
else
    echo "   ⏭️  Dashboard service ignoré (vous pouvez utiliser Vite dev server)"
fi

echo ""
echo "📝 Commandes utiles:"
echo "  - Voir logs kernel:     journalctl -u symbion-kernel -f"
echo "  - Voir logs agent:      journalctl -u symbion-agent -f"
echo "  - Voir logs dashboard:  journalctl -u symbion-dashboard -f"
echo "  - Restart kernel:       sudo systemctl restart symbion-kernel"
echo "  - Restart agent:        sudo systemctl restart symbion-agent"
echo "  - Restart dashboard:    sudo systemctl restart symbion-dashboard"
echo "  - Stop tout:            sudo systemctl stop symbion-kernel symbion-agent symbion-dashboard"
echo "  - Désactiver auto:      sudo systemctl disable symbion-kernel symbion-agent symbion-dashboard"
