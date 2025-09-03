#!/bin/bash

# Script pour cloner uniquement le dossier symbion-agent-host
# Usage: ./clone-agent-only.sh [destination_folder]

DESTINATION=${1:-symbion-agent-standalone}

echo "🔽 Cloning only symbion-agent-host folder..."

# Clone avec sparse-checkout
git clone --no-checkout https://github.com/eridwyn/NewSymbion.git "$DESTINATION"
cd "$DESTINATION"

# Enable sparse-checkout
git sparse-checkout init --cone
git sparse-checkout set symbion-agent-host

# Checkout files
git checkout

echo "✅ Done! Agent folder is in: $DESTINATION/symbion-agent-host"
echo ""
echo "🚀 To build:"
echo "cd $DESTINATION/symbion-agent-host"
echo "cargo build --release"
echo ""
echo "📁 Files cloned:"
find "$DESTINATION/symbion-agent-host" -type f -name "*.rs" | head -10