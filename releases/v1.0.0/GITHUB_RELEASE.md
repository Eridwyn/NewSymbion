# Symbion Agent v1.0.0

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
```toml
[mqtt]
broker_host = "127.0.0.1"
broker_port = 1883

[update]
auto_update = true
channel = "Stable"
check_interval_hours = 24
github_repo = "youruser/yourrepo"
```

## 🔄 Auto-Update
- Enable `auto_update = true` in configuration
- Agent will automatically check and update from GitHub releases
- Critical security updates are installed immediately
