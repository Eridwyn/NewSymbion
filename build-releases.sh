#!/bin/bash

# Symbion Agent Cross-Platform Build Script
# Génère les binaires pour auto-update GitHub releases

set -e

VERSION="1.0.0"
PROJECT_NAME="symbion-agent-host"
BINARY_NAME="symbion-agent-host"

echo "🚀 Building Symbion Agent v${VERSION} for multiple platforms..."

# Create release directory
RELEASE_DIR="releases/v${VERSION}"
mkdir -p "$RELEASE_DIR"

# Function to build for a specific target
build_target() {
    local target=$1
    local platform_name=$2
    local extension=$3
    
    echo "📦 Building for $platform_name ($target)..."
    
    # Add target if not already installed
    rustup target add "$target" 2>/dev/null || true
    
    # Build
    cd symbion-agent-host
    cargo build --release --target "$target"
    
    # Copy and rename binary
    local src_binary="target/$target/release/${BINARY_NAME}${extension}"
    local dst_binary="../$RELEASE_DIR/${BINARY_NAME}-${platform_name}${extension}"
    
    if [ -f "$src_binary" ]; then
        cp "$src_binary" "$dst_binary"
        echo "✅ Built: $dst_binary"
        
        # Show file info
        ls -lh "$dst_binary"
    else
        echo "❌ Failed to build for $target"
        return 1
    fi
    
    cd ..
}

# Build for Linux x86_64 (native) - using workspace build
echo "🐧 Building for Linux x86_64..."
cargo build --release -p symbion-agent-host
cp "target/release/${BINARY_NAME}" "$RELEASE_DIR/${BINARY_NAME}-linux-x64"
echo "✅ Built: $RELEASE_DIR/${BINARY_NAME}-linux-x64"

# Build for Windows x86_64 (cross-compile)
echo "🪟 Preparing Windows x86_64 cross-compilation..."
# Note: Windows cross-compilation on Linux requires additional setup
# For now, we'll create a placeholder and instructions

cat > "$RELEASE_DIR/BUILD_WINDOWS.md" << 'EOF'
# Building for Windows

To build the Windows binary:

1. On Windows machine with Rust installed:
```bash
cargo build --release
```

2. Rename the output:
```bash
copy target\release\symbion-agent-host.exe symbion-agent-host-windows-x64.exe
```

3. Or use cross-compilation (requires setup):
```bash
# Install Windows target
rustup target add x86_64-pc-windows-gnu

# Install mingw-w64 toolchain
sudo apt install gcc-mingw-w64-x86-64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```
EOF

# Create checksums
echo "🔐 Generating checksums..."
cd "$RELEASE_DIR"
sha256sum * > checksums.sha256
cd ../..

echo ""
echo "🎉 Build completed!"
echo "📁 Release files in: $RELEASE_DIR"
echo ""
ls -la "$RELEASE_DIR"

echo ""
echo "📋 Next steps:"
echo "1. Test the Linux binary: ./$RELEASE_DIR/${BINARY_NAME}-linux-x64"
echo "2. Build Windows binary (see BUILD_WINDOWS.md)"
echo "3. Create GitHub release with these files"
echo "4. Update repository URL in agent config"
echo "5. Enable auto_update = true"

# Create GitHub release template
cat > "$RELEASE_DIR/GITHUB_RELEASE.md" << EOF
# Symbion Agent v${VERSION}

## 🚀 Features
- ✅ Configuration management avec stockage sécurisé OS
- ✅ Auto-update GitHub releases system
- ✅ Tauri setup wizard foundation  
- ✅ First-time setup detection
- ✅ MQTT configuration dynamique
- ✅ Cross-platform support (Linux/Windows)

## 📥 Downloads

### Linux x86_64
- **symbion-agent-host-linux-x64** - Main agent binary for Linux

### Windows x86_64  
- **symbion-agent-host-windows-x64.exe** - Main agent binary for Windows

### Verification
- **checksums.sha256** - SHA256 checksums for integrity verification

## 🔧 Installation

### First Time Setup
1. Download the appropriate binary for your platform
2. Run the binary - it will detect first-time setup and show configuration instructions
3. Create configuration file as shown, or use the setup wizard (future release)

### Configuration Example
\`\`\`toml
[mqtt]
broker_host = "127.0.0.1"
broker_port = 1883

[update]
auto_update = true
channel = "Stable"
check_interval_hours = 24
github_repo = "youruser/yourrepo"
\`\`\`

## 🔄 Auto-Update
- Enable \`auto_update = true\` in configuration
- Agent will automatically check and update from GitHub releases
- Critical security updates are installed immediately
EOF

echo ""
echo "📄 GitHub release template created: $RELEASE_DIR/GITHUB_RELEASE.md"