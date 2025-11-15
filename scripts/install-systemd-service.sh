#!/bin/bash

# PR5: Script d'installation du service systemd symbion-kernel
# Usage: sudo ./scripts/install-systemd-service.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Symbion Kernel - Systemd Service Installation (PR5)       ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}❌ This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Check if service file exists
if [ ! -f "symbion-kernel.service" ]; then
    echo -e "${RED}❌ symbion-kernel.service not found in current directory${NC}"
    echo -e "${YELLOW}   Run this script from the NewSymbion project root${NC}"
    exit 1
fi

# Copy service file to systemd directory
echo -e "${YELLOW}📋 Copying symbion-kernel.service to /etc/systemd/system/${NC}"
cp symbion-kernel.service /etc/systemd/system/

# Set proper permissions
chmod 644 /etc/systemd/system/symbion-kernel.service

# Reload systemd daemon
echo -e "${YELLOW}🔄 Reloading systemd daemon${NC}"
systemctl daemon-reload

# Check if kernel is currently running
if systemctl is-active --quiet symbion-kernel; then
    echo -e "${YELLOW}⚠️  Symbion kernel is currently running as a service${NC}"
    echo -e "${YELLOW}   Restarting service to apply new configuration...${NC}"
    systemctl restart symbion-kernel
else
    echo -e "${YELLOW}▶️  Starting symbion-kernel service${NC}"
    systemctl start symbion-kernel
fi

# Enable service to start on boot
echo -e "${YELLOW}🔧 Enabling symbion-kernel to start on boot${NC}"
systemctl enable symbion-kernel

# Show service status
echo ""
echo -e "${GREEN}✅ Installation complete!${NC}"
echo ""
echo -e "${GREEN}Service status:${NC}"
systemctl status symbion-kernel --no-pager -l

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Useful commands:                                           ║${NC}"
echo -e "${GREEN}╠════════════════════════════════════════════════════════════╣${NC}"
echo -e "${GREEN}║  • View logs:      journalctl -u symbion-kernel -f          ║${NC}"
echo -e "${GREEN}║  • Stop service:   sudo systemctl stop symbion-kernel       ║${NC}"
echo -e "${GREEN}║  • Start service:  sudo systemctl start symbion-kernel      ║${NC}"
echo -e "${GREEN}║  • Restart:        sudo systemctl restart symbion-kernel    ║${NC}"
echo -e "${GREEN}║  • Status:         systemctl status symbion-kernel          ║${NC}"
echo -e "${GREEN}║  • Disable boot:   sudo systemctl disable symbion-kernel    ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
