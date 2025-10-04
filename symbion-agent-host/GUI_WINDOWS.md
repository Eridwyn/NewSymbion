# 🪟 Agent Windows GUI - Guide Complet

## 🎯 Objectif

Transformer l'agent Symbion en **application Windows native** avec:
- ✅ **Systray** - Icône dans la barre système avec menu
- ✅ **WebView embarqué** - Dashboard local sans navigateur
- ✅ **Mode windowless** - Pas de console noire
- ✅ **Auto-update** - Mise à jour automatique préservée

## 📦 Compilation

### Windows (Systray + GUI)
```bash
# Compile avec interface graphique Windows
cargo build --release --features gui

# Génère: symbion-agent-host.exe (sans fenêtre console)
# - Démarre directement en systray
# - WebView2 embarqué pour dashboard local
# - Clic icône → affiche/masque fenêtre
```

### Linux/Mac (Terminal classique)
```bash
# Mode terminal standard (sans GUI)
cargo build --release

# Génère: symbion-agent-host (avec console)
```

## 🔧 Prérequis

### Windows
- **WebView2 Runtime** (pré-installé sur Windows 10/11)
  - Si manquant: https://developer.microsoft.com/microsoft-edge/webview2/
- **Rust toolchain**: `rustup target add x86_64-pc-windows-msvc`

### Linux (test/développement uniquement)
```bash
sudo apt-get install -y libxdo-dev libwebkit2gtk-4.1-dev libgtk-3-dev
```

## 🎨 Interface GUI

### Systray
**Icône dans barre système** avec menu contextuel:
- 🟢 **Afficher Dashboard** - Ouvre fenêtre locale (localhost:9899)
- ⚪ **Masquer Dashboard** - Cache fenêtre (agent reste actif)
- 🌐 **Ouvrir Dashboard Principal** - Lance PWA kernel (localhost:3001)
- ❌ **Quitter** - Arrête l'agent

### WebView embarqué
- **URL**: `http://localhost:9899` (API locale agent)
- **Contenu**: Dashboard temps réel (métriques, contrôles)
- **Fermeture X**: Masque fenêtre au lieu de quitter

## ⚙️ Fonctionnalités préservées

### ✅ Auto-update intact
- Check au démarrage avant fork GUI/Terminal
- Updates automatiques si version critique
- Background checker périodique
- Notifications possibles via systray

### ✅ MQTT + Kernel
- Agent MQTT tourne en arrière-plan (tokio spawn)
- Connexion au kernel maintenue
- Heartbeats + commandes fonctionnent normalement

### ✅ Local API
- Serveur HTTP local:9899 actif
- Dashboard accessible même sans GUI
- API REST pour contrôles externes

## 🚀 Démarrage automatique Windows

### Méthode 1: Raccourci Startup
```powershell
# Créer raccourci dans dossier démarrage
$WshShell = New-Object -ComObject WScript.Shell
$Startup = [Environment]::GetFolderPath("Startup")
$Shortcut = $WshShell.CreateShortcut("$Startup\Symbion Agent.lnk")
$Shortcut.TargetPath = "C:\Program Files\Symbion\symbion-agent-host.exe"
$Shortcut.WorkingDirectory = "C:\Program Files\Symbion"
$Shortcut.Save()
```

### Méthode 2: Service Windows (TODO)
À venir - installation comme service système.

## 🏗️ Architecture technique

### Structure fichiers
```
symbion-agent-host/
├── src/
│   ├── main.rs              # Point d'entrée + fork GUI/Terminal
│   ├── gui.rs               # Module GUI Windows (tao + wry + tray-icon)
│   ├── local_api.rs         # Serveur HTTP localhost:9899
│   ├── system_tray.rs       # Systray basique (Linux/Mac)
│   └── ...
├── Cargo.toml               # Feature "gui" optionnelle
└── BUILD_GUI.md             # Ce fichier
```

### Dépendances GUI
```toml
[dependencies]
tray-icon = "0.18"   # Systray multi-plateforme
tao = "0.30"         # Fenêtres natives
wry = "0.47"         # WebView (Edge/WebKit)
image = "0.25"       # Icônes
```

### Flux d'exécution

```rust
main() {
    // 1. Setup wizard (first time)
    // 2. Load config
    // 3. ✅ Auto-update check (avant GUI)
    // 4. Start local API server

    #[cfg(feature = "gui")] {
        // Windows GUI mode
        tokio::spawn(agent.run());  // MQTT en background
        gui.run()?;                 // Event loop GUI (bloquant)
    }

    #[cfg(not(feature = "gui"))] {
        // Terminal mode
        agent.run().await?;         // Foreground classique
    }
}
```

## 🐛 Debug

### Logs
En mode GUI windowless, les logs vont dans:
- **Windows**: Fichier ou Output Debug String (DebugView)
- **Possible amélioration**: Rotation logs dans `%APPDATA%\Symbion\`

### Test local
```bash
# Lancer sans GUI pour voir logs
RUST_LOG=debug cargo run --release

# Lancer avec GUI (Linux test)
RUST_LOG=debug cargo run --release --features gui
```

## 📋 TODO futur

- [ ] Icône Windows personnalisée (.ico)
- [ ] Notifications toast Windows 10/11
- [ ] Build.rs pour embed icône
- [ ] Installation MSI/NSIS
- [ ] Service Windows (pas juste systray)
- [ ] Auto-update avec notification GUI
- [ ] Logs rotatifs dans AppData
- [ ] Paramètres GUI (pas juste config.toml)

## 🔒 Sécurité

- ✅ API locale limitée à localhost (0.0.0.0:9899)
- ✅ Pas de credentials stockés en clair
- ✅ WebView2 sandboxé (isolation processus Edge)
- ⚠️ TODO: Authentification API locale (token)

---

**Version**: 1.1.1
**Date**: 2025-10-04
**Statut**: ✅ Fonctionnel (Linux testé, Windows à tester)
