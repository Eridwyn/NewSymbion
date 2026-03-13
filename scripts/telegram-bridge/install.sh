#!/bin/bash
# Install Symbion Telegram-Claude Bridge
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Symbion Telegram-Claude Bridge ==="

# Install Python deps
echo "[1/3] Installation des dependances Python..."
pip3 install --break-system-packages -q -r "$SCRIPT_DIR/requirements.txt"

# Create systemd service
echo "[2/3] Creation du service systemd..."
sudo tee /etc/systemd/system/symbion-telegram-bridge.service > /dev/null << EOF
[Unit]
Description=Symbion Telegram-Claude Bridge
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=eridwyn
Group=eridwyn
WorkingDirectory=$SCRIPT_DIR
ExecStart=/usr/bin/python3 $SCRIPT_DIR/bridge.py
Restart=on-failure
RestartSec=10
Environment=PATH=/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=multi-user.target
EOF

echo "[3/3] Activation du service..."
sudo systemctl daemon-reload
sudo systemctl enable symbion-telegram-bridge
sudo systemctl start symbion-telegram-bridge

echo ""
echo "Bridge installe et demarre!"
echo ""
echo "Commandes utiles:"
echo "  sudo systemctl status symbion-telegram-bridge"
echo "  sudo journalctl -u symbion-telegram-bridge -f"
echo "  sudo systemctl restart symbion-telegram-bridge"
echo "  sudo systemctl stop symbion-telegram-bridge"
