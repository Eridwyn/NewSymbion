# Compilation de Symbion Agent avec GUI Windows

## Mode GUI (Systray + WebView embarqué)

Pour compiler l'agent avec interface graphique Windows (sans terminal):

```bash
# Windows - Compile avec GUI intégré
cargo build --release --features gui

# Génère un exécutable windowless: symbion-agent-host.exe
# - Icône systray avec menu contextuel
# - WebView2 embarqué (dashboard local)
# - Pas de fenêtre console
```

## Mode Terminal (Linux/Windows classique)

Pour compiler sans GUI (mode terminal classique):

```bash
# Compilation standard sans GUI
cargo build --release

# Génère un exécutable avec console normale
```

## Prérequis Windows (GUI)

1. **WebView2 Runtime** doit être installé sur la machine cible
   - Généralement déjà présent sur Windows 10/11 modernes
   - Téléchargement: https://developer.microsoft.com/microsoft-edge/webview2/

2. **Rust toolchain** pour Windows
   ```bash
   rustup target add x86_64-pc-windows-msvc
   ```

## Fonctionnalités GUI

### Systray
- **Clic icône** : Affiche/masque la fenêtre dashboard
- **Menu contextuel** :
  - Afficher Dashboard (localhost:9899)
  - Masquer Dashboard
  - Ouvrir Dashboard Principal (PWA kernel)
  - Quitter

### WebView embarqué
- Interface locale `http://localhost:9899`
- Contrôles agent + métriques système
- Communication API REST avec agent MQTT

## Démarrage automatique Windows

Pour lancer l'agent au démarrage système:

```powershell
# Créer raccourci dans shell:startup
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\Symbion Agent.lnk")
$Shortcut.TargetPath = "C:\path\to\symbion-agent-host.exe"
$Shortcut.Save()
```

Ou installer comme service Windows (TODO).
