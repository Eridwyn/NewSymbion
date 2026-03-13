# Symbion Telegram-Claude Bridge

Bridge entre Telegram et Claude Code CLI. Permet de piloter Claude Code depuis Telegram en utilisant l'abonnement Claude existant (pas de cle API necessaire).

## Fonctionnalites

- **Streaming en direct** — la reponse s'affiche progressivement dans Telegram
- **Sessions persistantes** — Claude se souvient du contexte entre les messages
- **Indicateur d'outils** — affiche quand Claude lit un fichier, execute une commande, etc.
- **Modeles configurables** — switch entre haiku (rapide), sonnet, opus
- **Effort reglable** — low (rapide) a high (approfondi)
- **Acces complet au projet** — Claude peut lire, modifier, executer dans NewSymbion

## Architecture

```
Telegram → Bot (@Monsymbion_bot)
              ↓
         bridge.py (systemd service)
              ↓
         claude -p "..." --output-format stream-json
              ↓
         Reponse streaming → Telegram
```

## Installation

```bash
# Prerequis
sudo apt install python3-pip

# Installation
cd scripts/telegram-bridge
bash install.sh
```

Le script `install.sh` :
1. Installe les dependances Python (`python-telegram-bot`)
2. Cree le service systemd `symbion-telegram-bridge`
3. Demarre le service

## Configuration

Editer `config.env` :

```env
# Token du bot Telegram (via @BotFather)
TELEGRAM_BOT_TOKEN=...

# IDs Telegram autorises (separes par des virgules)
ALLOWED_USER_IDS=8119327529

# Chemin vers le CLI Claude Code
CLAUDE_PATH=/usr/local/bin/claude

# Timeout max par requete (secondes)
CLAUDE_TIMEOUT=600

# Repertoire de travail pour Claude Code
CLAUDE_WORKDIR=/home/eridwyn/RustroverProjects/NewSymbion
```

## Commandes Telegram

| Commande | Description |
|---|---|
| `/new` | Nouvelle conversation (reset contexte) |
| `/continue` | Reprendre la derniere session |
| `/cancel` | Annuler la requete en cours |
| `/status` | Etat du bridge |
| `/model haiku` | Modele rapide (defaut) |
| `/model sonnet` | Modele equilibre |
| `/model opus` | Modele le plus puissant |
| `/effort low` | Reponses rapides (defaut) |
| `/effort medium` | Reponses moderees |
| `/effort high` | Reponses approfondies |

## Gestion du service

```bash
# Status
sudo systemctl status symbion-telegram-bridge

# Logs en direct
sudo journalctl -u symbion-telegram-bridge -f

# Redemarrer
sudo systemctl restart symbion-telegram-bridge

# Arreter
sudo systemctl stop symbion-telegram-bridge
```

## Securite

- Seuls les user IDs dans `ALLOWED_USER_IDS` peuvent interagir avec le bot
- Le service tourne sous l'utilisateur `eridwyn` (pas root)
- Claude Code a acces complet au projet (`--dangerously-skip-permissions`)
- Budget max de 5$ par requete

## Fichiers

```
scripts/telegram-bridge/
├── bridge.py          # Script principal du bridge
├── config.env         # Configuration (tokens, paths)
├── install.sh         # Script d'installation systemd
├── requirements.txt   # Dependances Python
└── README.md          # Cette documentation
```
