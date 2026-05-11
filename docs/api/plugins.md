# Endpoints HTTP des plugins

> Routes internes exposées par chaque plugin via Unix socket
> (`/run/symbion-plugins/{plugin}.sock`), proxifiées par le kernel sur
> `/v1/plugin-api/{plugin}/*`.
>
> Dernière mise à jour : 11 mai 2026 (extraction automatique depuis le code).
>
> Pour les endpoints kernel core, voir [`endpoints.md`](endpoints.md).

## Accès

Toutes les routes plugins passent par le proxy kernel :

```
https://localhost:8443/v1/plugin-api/{plugin_name}/{route}
```

L'authentification (JWT bearer ou X-API-Key) est appliquée par le kernel **avant**
le proxy. Le plugin lui-même ne fait pas d'auth (socket Unix = trust ring).

## Inventaire (8 plugins, ~50 routes)

### coffee — Philips EP2520 LatteGo
Socket : `/run/symbion-plugins/coffee.sock` · Module : `symbion-plugin-coffee/src/main.rs:662-683`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin (uptime, machine online) |
| `GET` | `/status` | État machine (mainstate, brewing, levels, maintenance) |
| `GET` | `/info` | Device info Philips Condor (firmware, replenishment) |
| `GET` | `/configuration` | Config machine Condor |
| `POST` | `/brew` | Démarre une boisson `{drink, temperature, cups}` |
| `POST` | `/stop` | Arrête le brewing en cours |
| `POST` | `/power` | Power on/off `{on: bool}` |

### library — Bibliothèque de connaissances
Socket : `/run/symbion-plugins/library.sock` · Module : `symbion-plugin-library/src/routes.rs`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin |
| `GET POST` | `/nodes` | Liste / création de nodes |
| `GET PUT DELETE` | `/nodes/:id` | CRUD node |
| `GET` | `/nodes/:id/versions` | Historique des versions |
| `GET` | `/nodes/:id/desk` | Study desk (workflow lecture) |
| `POST` | `/nodes/:id/activate` | Activer un node |
| `GET POST` | `/sections` | Liste / création sections |
| `GET PUT DELETE` | `/sections/:id` | CRUD section |
| `GET` | `/sections/:id/nodes` | Nodes d'une section |
| `GET POST` | `/edges` | Liste / création relations |
| `GET DELETE` | `/edges/:id` | CRUD relation |
| `GET` | `/graph` | Vue graphe complet |
| `GET` | `/search` | FTS5 recherche full-text |
| `GET` | `/tags` | Liste tags uniques |
| `GET` | `/templates` | Liste templates structurés |
| `GET` | `/templates/:id` | Détail template |
| `GET POST` | `/pending-links` | Liens en attente de modération |
| `POST` | `/pending-links/:id/confirm` | Approuver un lien suggéré |
| `POST` | `/pending-links/:id/dismiss` | Rejeter un lien suggéré |
| `GET` | `/trash` | Nodes soft-deleted |

### notes — Notes contextuelles
Socket : `/run/symbion-plugins/notes.sock` · Module : `symbion-plugin-notes/src/main.rs:805-810`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin |
| `POST` | `/actions` | Contract v1.0 dispatcher (create_note, update_note, delete_note, list_notes) |
| `GET POST` | `/notes` | Liste / création notes (REST direct) |
| `PUT DELETE` | `/notes/:id` | Update / suppression note |

### sensors — Capteurs environnementaux
Socket : `/run/symbion-plugins/sensors.sock` · Module : `symbion-plugin-sensors/src/main.rs:570-573`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin |
| `POST` | `/actions` | Contract v1.0 dispatcher (list_sensors, get_environment, get_sensor) |
| `GET` | `/sensors` | Liste capteurs enregistrés |
| `GET` | `/environment/:room_id` | Snapshot env d'une pièce (température, humidité) |

### ssl — Surveillance certificats
Socket : `/run/symbion-plugins/ssl.sock` · Module : `symbion-plugin-ssl/src/main.rs:486-497`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin |
| `GET POST` | `/domains` | Liste / ajout domaines à surveiller |
| `GET PUT DELETE` | `/domains/:id` | CRUD domaine |
| `POST` | `/check` | Force vérif immédiate de tous les domaines |

### telegram — Bot Telegram + bridge Claude
Socket : `/run/symbion-plugins/telegram.sock` · Module : `symbion-plugin-telegram/src/main.rs:61-64`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut bot (uptime, sessions, allowed_users) |
| `POST` | `/actions` | Contract v1.0 dispatcher (send_message, send_notification) |
| `GET PUT` | `/config` | Préférences notifications (toggles par catégorie) |

### freebox — Présence réseau
Socket : `/run/symbion-plugins/freebox.sock` · Module : `symbion-plugin-freebox/src/main.rs:307`

| Méthode | Route | Description |
|---------|-------|-------------|
| `GET` | `/health` | Statut plugin (toutes les données passent par MQTT, pas HTTP) |

### common — Bibliothèque partagée
Pas de socket — utilisé en dépendance par les autres plugins
(`PluginHttpServer`, `PluginRegistrationBuilder`, types `PluginAction`).

## Patterns

### Contract v1.0 (`/actions` générique)

Trois plugins utilisent un endpoint `/actions` unique qui dispatche selon
le champ `action_type` du payload :

```json
POST /v1/plugin-api/{plugin}/actions
{
  "spec_version": "1.0",
  "action_id": "<uuid>",
  "action_type": "send_notification",
  "payload": { ... params spécifiques à l'action ... },
  "metadata": { ... optionnel ... }
}
```

Plugins concernés : `telegram`, `sensors`, `notes`. Pour les automations, voir
`wrap_protocol: "v1"` dans la doc rule builder
([`PLUGIN_DEVELOPMENT_GUIDE.md` §6.3bis](../PLUGIN_DEVELOPMENT_GUIDE.md)).

### Routes directes

Les autres plugins exposent une route HTTP par action (`/brew`, `/power`,
`/check`, etc.). C'est plus simple côté plugin mais demande de coder un
handler par endpoint. Utilisé par `coffee`, `ssl`, `library`.

## Comment ajouter une route plugin

1. Ajouter au router `axum::Router` du plugin (cf. modules cités ci-dessus)
2. Déclarer la route dans `PluginRegistrationBuilder.route("/ma-route")`
3. Optionnellement déclarer une action template via `.action(PluginAction { route: "ma-route", ... })`
   pour exposition automatique dans le rule builder PWA (cf. GUIDE §6)
4. Restart le plugin → auto-discovery ou re-register manuel propage au kernel

## Sécurité

- Sockets Unix avec perm `0o770` (lecture/écriture user+group eridwyn)
- Pas d'auth côté plugin (trust ring)
- L'auth (JWT/API-Key) est appliquée par le kernel avant le proxy
- Body payloads limités à 1 MB par défaut (configurable par plugin)
