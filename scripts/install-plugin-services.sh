#!/bin/bash
# Install Symbion plugin systemd services

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
SYSTEMD_DIR="$REPO_ROOT/systemd"
USER_SYSTEMD_DIR="$HOME/.config/systemd/user"

echo "Installing Symbion plugin systemd services..."

# Create user systemd directory
mkdir -p "$USER_SYSTEMD_DIR"

# Copy service files
cp "$SYSTEMD_DIR/symbion-plugin-notes.service" "$USER_SYSTEMD_DIR/"
cp "$SYSTEMD_DIR/symbion-plugin-notifications.service" "$USER_SYSTEMD_DIR/"
cp "$SYSTEMD_DIR/symbion-plugin-sensors.service" "$USER_SYSTEMD_DIR/"

echo "✅ Service files installed"

# Reload systemd daemon
systemctl --user daemon-reload
echo "✅ Systemd daemon reloaded"

# Enable services (auto-start)
systemctl --user enable symbion-plugin-notes.service
systemctl --user enable symbion-plugin-notifications.service
systemctl --user enable symbion-plugin-sensors.service
echo "✅ Services enabled (auto-start)"

echo ""
echo "Services installed successfully!"
echo ""
echo "To start services:"
echo "  systemctl --user start symbion-plugin-notes"
echo "  systemctl --user start symbion-plugin-notifications"
echo "  systemctl --user start symbion-plugin-sensors"
echo ""
echo "To check status:"
echo "  systemctl --user status symbion-plugin-notes"
echo ""
echo "To view logs:"
echo "  journalctl --user -u symbion-plugin-notes -f"
