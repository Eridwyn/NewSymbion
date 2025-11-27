# Symbion Plugin Services - Quick Start

## One-Command Installation

```bash
# Install all 3 plugin services
./scripts/install-plugin-services.sh
```

This will:
1. Copy service files to `~/.config/systemd/user/`
2. Reload systemd daemon
3. Enable auto-start for all plugins

## Post-Installation

### Start all plugins
```bash
systemctl --user start symbion-plugin-notes symbion-plugin-notifications symbion-plugin-sensors
```

### Verify services are running
```bash
systemctl --user status symbion-plugin-*
```

### Watch logs in real-time
```bash
# Terminal 1
journalctl --user -u symbion-plugin-notes -f

# Terminal 2
journalctl --user -u symbion-plugin-notifications -f

# Terminal 3
journalctl --user -u symbion-plugin-sensors -f
```

## Common Commands

| Action | Command |
|--------|---------|
| Start all plugins | `systemctl --user start symbion-plugin-{notes,notifications,sensors}` |
| Stop all plugins | `systemctl --user stop symbion-plugin-{notes,notifications,sensors}` |
| Restart after rebuild | `systemctl --user restart symbion-plugin-{notes,notifications,sensors}` |
| Check status | `systemctl --user status symbion-plugin-notes` |
| View logs | `journalctl --user -u symbion-plugin-notes -n 50` |
| Follow logs | `journalctl --user -u symbion-plugin-notes -f` |
| Disable auto-start | `systemctl --user disable symbion-plugin-notes` |

## Service Order

1. `symbion-kernel.service` (starts first)
2. Wait for kernel to be ready
3. All plugins start in parallel:
   - `symbion-plugin-notes.service`
   - `symbion-plugin-notifications.service`
   - `symbion-plugin-sensors.service`

## Troubleshooting

### "Unit not found"
```bash
systemctl --user daemon-reload
```

### "Permission denied"
```bash
ls -l ~/.config/systemd/user/symbion-plugin-*.service
# Should be owned by your user
```

### "Failed to start"
```bash
# Check kernel is running first
systemctl --user status symbion-kernel

# Check logs
journalctl --user -u symbion-plugin-notes -n 100
```

### After code changes
```bash
# Rebuild plugins
cargo build --release

# Restart services
systemctl --user restart symbion-plugin-{notes,notifications,sensors}
```

## Development Workflow

```bash
# 1. Make code changes
vim symbion-plugin-notes/src/main.rs

# 2. Rebuild
cargo build --release

# 3. Restart service
systemctl --user restart symbion-plugin-notes

# 4. Watch logs
journalctl --user -u symbion-plugin-notes -f
```

## Files Created

- `/home/eridwyn/RustroverProjects/NewSymbion/systemd/symbion-plugin-notes.service`
- `/home/eridwyn/RustroverProjects/NewSymbion/systemd/symbion-plugin-notifications.service`
- `/home/eridwyn/RustroverProjects/NewSymbion/systemd/symbion-plugin-sensors.service`
- `/home/eridwyn/RustroverProjects/NewSymbion/scripts/install-plugin-services.sh`

After installation, copied to:
- `~/.config/systemd/user/symbion-plugin-notes.service`
- `~/.config/systemd/user/symbion-plugin-notifications.service`
- `~/.config/systemd/user/symbion-plugin-sensors.service`
