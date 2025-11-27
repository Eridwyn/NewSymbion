# Symbion Systemd Services

This directory contains systemd user service files for Symbion components.

## Available Services

### Core Services
- `symbion-kernel.service` - Main Symbion kernel (HTTPS/MQTT hub)
- `symbion-agent.service` - Local agent for device monitoring
- `symbion-dashboard.service` - PWA dashboard frontend

### Plugin Services
- `symbion-plugin-notes.service` - Notes management plugin
- `symbion-plugin-notifications.service` - Notification handler plugin
- `symbion-plugin-sensors.service` - Sensor data aggregation plugin

## Installation

### Core Services
```bash
# Install kernel, agent, and dashboard services
./systemd/install-services.sh
```

### Plugin Services
```bash
# Install all 3 plugin services
./scripts/install-plugin-services.sh
```

## Service Management

### Starting Services
```bash
# Start individual plugin
systemctl --user start symbion-plugin-notes

# Start all plugins
systemctl --user start symbion-plugin-notes symbion-plugin-notifications symbion-plugin-sensors
```

### Checking Status
```bash
# Check individual service
systemctl --user status symbion-plugin-notes

# Check all plugin services
systemctl --user status symbion-plugin-*
```

### Viewing Logs
```bash
# Follow logs for notes plugin
journalctl --user -u symbion-plugin-notes -f

# View last 50 lines
journalctl --user -u symbion-plugin-notes -n 50

# View logs since boot
journalctl --user -u symbion-plugin-notes -b
```

### Stopping Services
```bash
# Stop individual plugin
systemctl --user stop symbion-plugin-notes

# Stop all plugins
systemctl --user stop symbion-plugin-notes symbion-plugin-notifications symbion-plugin-sensors
```

### Restarting Services
```bash
# Restart after code changes
systemctl --user restart symbion-plugin-notes
```

### Disabling Auto-start
```bash
# Disable a plugin from auto-starting
systemctl --user disable symbion-plugin-notes

# Re-enable
systemctl --user enable symbion-plugin-notes
```

## Service Configuration

All plugin services share the following configuration:

- **User**: `eridwyn`
- **WorkingDirectory**: `/home/eridwyn/RustroverProjects/NewSymbion`
- **Environment**: `SYMBION_MQTT_BROKER=127.0.0.1:1883`
- **Restart Policy**: `on-failure` with 5s delay
- **Logs**: Captured by journalctl
- **Dependencies**: Require `symbion-kernel.service` to be running

## Dependency Chain

```
symbion-kernel.service
    ↓
    ├─→ symbion-plugin-notes.service
    ├─→ symbion-plugin-notifications.service
    └─→ symbion-plugin-sensors.service
```

Plugins will:
- Start automatically after the kernel
- Wait for the kernel to be ready
- Restart automatically on failure
- Stop when the kernel stops (via `Requires=`)

## Troubleshooting

### Service won't start
```bash
# Check if kernel is running
systemctl --user status symbion-kernel

# Check service definition
systemctl --user cat symbion-plugin-notes

# View detailed error logs
journalctl --user -u symbion-plugin-notes -n 100
```

### Binary not found
```bash
# Rebuild plugins
cd /home/eridwyn/RustroverProjects/NewSymbion
cargo build --release

# Verify binaries exist
ls -lh target/release/symbion-plugin-*
```

### MQTT connection issues
```bash
# Check MQTT broker is running
systemctl status mosquitto

# Test MQTT connection
mosquitto_sub -h 127.0.0.1 -p 1883 -t "symbion/#" -v
```

### Permission issues
```bash
# Reload systemd daemon
systemctl --user daemon-reload

# Check service file permissions
ls -l ~/.config/systemd/user/symbion-plugin-*
```

## Uninstalling

```bash
# Stop and disable all plugin services
systemctl --user stop symbion-plugin-notes symbion-plugin-notifications symbion-plugin-sensors
systemctl --user disable symbion-plugin-notes symbion-plugin-notifications symbion-plugin-sensors

# Remove service files
rm ~/.config/systemd/user/symbion-plugin-*.service

# Reload daemon
systemctl --user daemon-reload
```

## See Also

- [QUICK_REFERENCE.md](../docs/QUICK_REFERENCE.md) - Command cheat sheet
- [PHILOSOPHY.md](../docs/PHILOSOPHY.md) - System architecture
- [DEPLOYMENT.md](../docs/DEPLOYMENT.md) - Deployment guide
