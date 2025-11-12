#!/bin/bash
set -e

# 🚀 Script de déploiement automatique Symbion Dashboard depuis GitHub Releases
# Usage: ./deploy-dashboard.sh [version]
# Exemple: ./deploy-dashboard.sh dashboard-v1.0.0

GITHUB_REPO="${GITHUB_REPO:-votre-username/NewSymbion}"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/var/www/symbion-dashboard}"
BACKUP_DIR="${INSTALL_DIR}.backup"
PORT="${DASHBOARD_PORT:-3000}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📱 Déploiement Symbion Dashboard ${VERSION}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Vérifier droits sudo si nécessaire
if [ ! -w "$(dirname "$INSTALL_DIR")" ]; then
    echo "❌ Permissions insuffisantes pour écrire dans $(dirname "$INSTALL_DIR")"
    echo "Usage: sudo $0 [version]"
    exit 1
fi

# Déterminer URL de téléchargement
if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/latest/download/symbion-dashboard-latest.tar.gz"
else
    DOWNLOAD_URL="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/symbion-dashboard-${VERSION}.tar.gz"
fi

echo "1️⃣ Téléchargement depuis GitHub Releases..."
echo "   URL: $DOWNLOAD_URL"

# Créer répertoire temporaire
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Télécharger archive
if ! curl -sSL -f -o "$TMP_DIR/dashboard.tar.gz" "$DOWNLOAD_URL"; then
    echo "❌ Échec du téléchargement"
    echo "   Vérifiez que la version existe: https://github.com/${GITHUB_REPO}/releases"
    exit 1
fi

echo ""
echo "2️⃣ Extraction de l'archive..."
mkdir -p "$TMP_DIR/dashboard"
tar -xzf "$TMP_DIR/dashboard.tar.gz" -C "$TMP_DIR/dashboard"

# Vérifier que le build contient index.html
if [ ! -f "$TMP_DIR/dashboard/index.html" ]; then
    echo "❌ Archive invalide: index.html manquant"
    exit 1
fi

echo "   ✅ Build PWA valide extrait"

# Sauvegarder ancien déploiement si existe
if [ -d "$INSTALL_DIR" ]; then
    echo ""
    echo "3️⃣ Sauvegarde du déploiement actuel..."
    rm -rf "$BACKUP_DIR" 2>/dev/null || true
    mv "$INSTALL_DIR" "$BACKUP_DIR"
    echo "   ✅ Backup créé: ${BACKUP_DIR}"
fi

# Déployer nouveau build
echo ""
echo "4️⃣ Installation du nouveau build..."
mkdir -p "$INSTALL_DIR"
cp -r "$TMP_DIR/dashboard/"* "$INSTALL_DIR/"
echo "   ✅ Dashboard déployé: ${INSTALL_DIR}"

# Vérifier si un serveur web systemd est configuré
SERVICE_NAME="symbion-dashboard"
SERVICE_EXISTS=false
if systemctl list-unit-files 2>/dev/null | grep -q "^${SERVICE_NAME}.service"; then
    SERVICE_EXISTS=true
    echo ""
    echo "5️⃣ Service systemd détecté, redémarrage..."
    systemctl restart "$SERVICE_NAME" || true
    sleep 2
fi

# Vérifier santé du dashboard
echo ""
echo "6️⃣ Vérification de la santé..."
sleep 2

# Essayer de vérifier que le dashboard répond
if command -v curl &> /dev/null; then
    if curl -s http://localhost:${PORT} | grep -q "Symbion"; then
        echo "   ✅ Dashboard répond correctement"
    else
        echo "   ⚠️  Dashboard ne répond pas sur port ${PORT}"
        echo "   Vérifiez manuellement avec: curl http://localhost:${PORT}"
    fi
else
    echo "   ⚠️  curl non disponible, impossible de vérifier automatiquement"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Déploiement réussi!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ "$SERVICE_EXISTS" = true ]; then
    echo "📊 Status service:"
    systemctl status "$SERVICE_NAME" --no-pager -l | head -15 || true
else
    echo "📊 Servir le dashboard manuellement:"
    echo "   cd $INSTALL_DIR && python3 -m http.server ${PORT}"
fi

echo ""
echo "📝 Informations:"
echo "  - Répertoire:     $INSTALL_DIR"
echo "  - Backup:         $BACKUP_DIR (si problème)"
echo "  - Port suggéré:   $PORT"
echo ""
echo "🌐 Accès:"
echo "  - Local:          http://localhost:${PORT}"
echo "  - Réseau:         http://$(hostname -I | awk '{print $1}'):${PORT}"
echo ""
echo "🔄 Rollback si problème:"
echo "  sudo rm -rf $INSTALL_DIR"
echo "  sudo mv $BACKUP_DIR $INSTALL_DIR"
echo "  sudo systemctl restart $SERVICE_NAME  # Si service existe"
echo ""
echo "🔐 Configuration API:"
echo "  Le dashboard utilise config.js pour se connecter au kernel."
echo "  Fichier de configuration: ${INSTALL_DIR}/config.js"
echo ""
echo "  Pour modifier l'URL API après déploiement:"
echo "  sudo nano ${INSTALL_DIR}/config.js"
echo ""
echo "  Exemple configuration production:"
echo "    API_BASE: 'https://$(hostname -I | awk '{print $1}'):8443'"
echo "    API_KEY: 'votre-clé-sécurisée'"
echo ""
echo "  Après modification config.js, rechargez simplement le dashboard (F5)."
