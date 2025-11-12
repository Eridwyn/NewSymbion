#!/bin/bash
# Installation du monitoring automatique Symbion

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MONITOR_SCRIPT="$SCRIPT_DIR/monitor-symbion.sh"

echo "🔧 Installation du monitoring Symbion"

# Vérifier que le script existe
if [ ! -f "$MONITOR_SCRIPT" ]; then
    echo "❌ Script de monitoring introuvable: $MONITOR_SCRIPT"
    exit 1
fi

# Demander l'email
read -p "📧 Email pour les alertes (laisser vide pour désactiver): " ALERT_EMAIL

# Créer l'entrée crontab
echo ""
echo "⏰ Configuration de la surveillance automatique"
echo ""
echo "Choisissez la fréquence:"
echo "  1) Toutes les 5 minutes (surveillance intensive)"
echo "  2) Toutes les 15 minutes (recommandé)"
echo "  3) Toutes les heures"
echo "  4) Une fois par jour à 8h"
read -p "Choix [2]: " FREQ_CHOICE
FREQ_CHOICE=${FREQ_CHOICE:-2}

case $FREQ_CHOICE in
    1)
        CRON_SCHEDULE="*/5 * * * *"
        FREQ_DESC="toutes les 5 minutes"
        ;;
    2)
        CRON_SCHEDULE="*/15 * * * *"
        FREQ_DESC="toutes les 15 minutes"
        ;;
    3)
        CRON_SCHEDULE="0 * * * *"
        FREQ_DESC="toutes les heures"
        ;;
    4)
        CRON_SCHEDULE="0 8 * * *"
        FREQ_DESC="tous les jours à 8h"
        ;;
    *)
        echo "❌ Choix invalide"
        exit 1
        ;;
esac

# Construire la ligne crontab
CRON_LINE="$CRON_SCHEDULE $MONITOR_SCRIPT $ALERT_EMAIL >> /tmp/symbion-monitor.log 2>&1"

echo ""
echo "📝 Configuration:"
echo "   Fréquence: $FREQ_DESC"
echo "   Email: ${ALERT_EMAIL:-aucun}"
echo "   Script: $MONITOR_SCRIPT"
echo ""

# Vérifier si une entrée existe déjà
if crontab -l 2>/dev/null | grep -q "monitor-symbion.sh"; then
    echo "⚠️  Une tâche de monitoring existe déjà"
    read -p "Voulez-vous la remplacer? (y/N): " REPLACE
    if [[ ! $REPLACE =~ ^[Yy]$ ]]; then
        echo "❌ Installation annulée"
        exit 0
    fi
    # Supprimer l'ancienne entrée
    crontab -l 2>/dev/null | grep -v "monitor-symbion.sh" | crontab -
fi

# Ajouter la nouvelle entrée
(crontab -l 2>/dev/null; echo "$CRON_LINE") | crontab -

echo "✅ Monitoring installé!"
echo ""
echo "📊 Commandes utiles:"
echo "   Tester maintenant:     $MONITOR_SCRIPT $ALERT_EMAIL"
echo "   Voir les logs:         tail -f /tmp/symbion-monitor.log"
echo "   Voir les crontabs:     crontab -l"
echo "   Désinstaller:          crontab -e  # supprimer la ligne monitor-symbion"
echo ""
echo "🔍 Premier test dans 30 secondes..."
sleep 5
echo "Lancement du test..."
$MONITOR_SCRIPT "$ALERT_EMAIL"
