# Roadmap Technique - NewSymbion

**Dernière mise à jour** : 10 Mai 2026
**Statut** : Fondations complètes, 8 plugins actifs, en route vers domotique

---

## Codebase Actuelle

| Composant | Langage | LOC | Fichiers | Tests |
|-----------|---------|----:|----------|------:|
| **symbion-kernel** | Rust | ~49,659 | 112 | 429 |
| **pwa-dashboard** | JS (Lit) | ~42,917 | 75 | 689 |
| **symbion-agent-host** | Rust | ~11,730 | 56 | 225 |
| **Plugins** (8) | Rust | ~13,641 | 31 | 143 |
| **Infra** (scripts/CI/bridge) | Bash/YAML/Py | ~3,500 | 30 | - |
| **Total** | | **~121,447** | **304** | **1,486** |

| Métrique | Valeur |
|----------|--------|
| API Routes | 178+ endpoints |
| MQTT Topics | 62 (18 core + 44 plugins) |
| SQLite | 22 tables, 5 migrations |
| Plugins | notes, ssl, sensors, library, telegram, freebox, common, coffee |

---

## Prochaines Étapes

### R1 — Notifications Telegram 🟡 PARTIELLEMENT (≈50%)

**Objectif** : Recevoir les notifications Symbion directement sur Telegram via @Monsymbion_bot

**État réel (9 mai 2026)** : L'infra MQTT→Telegram fonctionne. Le kernel publie sur `symbion/notifications/sent` et `symbion-plugin-telegram` reçoit + forward (validé en E2E avec les notifs café). Reste à finir : formatage HTML riche, page config PWA pour granularité par catégorie, résumé quotidien automatique.

**Contexte** : Le plugin Rust `symbion-plugin-telegram` (teloxide) est actif. La glue notifications kernel → Telegram fonctionne pour la priorité P0/P1/P2.

**Fonctionnalités** :
- [ ] Notifications agent offline/online → message Telegram
- [ ] Alertes environnement (température, humidité) → message Telegram
- [ ] Alertes SSL (certificat expirant) → message Telegram
- [ ] Résumé quotidien automatique (agents, capteurs, automations)
- [ ] Commandes rapides depuis Telegram (status, restart agent, etc.)
- [ ] Configuration : choix des notifications activées (granularité par type)

**Implémentation** :
- Kernel : nouveau subscriber MQTT `symbion/notifications/sent@v1` → forward vers Telegram
- Plugin Telegram : endpoint `/notify` ou topic MQTT `symbion/telegram/send`
- Plugin Telegram : formatage riche (HTML Telegram)
- PWA Settings : page config notifications Telegram (on/off par catégorie)

**Fichiers concernés** :
- `symbion-plugin-telegram/src/events.rs` — réception events kernel
- `symbion-kernel/src/notifications.rs` — ajout canal Telegram
- `pwa-dashboard/src/components/user-settings-page.js` — UI config

---

### R2 — Formulaire Public Bibliothèque + Validation Admin 🔴 TODO

**Objectif** : Permettre à n'importe qui (via la page publique `/lib`) de proposer du contenu à la bibliothèque, avec validation manuelle par l'admin avant publication.

**Contexte** : La page publique `public-lib/index.html` et le proxy `/v1/public/library` existent en lecture seule. Il faut ajouter un formulaire de soumission + un workflow de modération.

**Fonctionnalités** :
- [ ] Formulaire public sur `/lib` : titre, contenu, template (épice/recette/libre), champs structurés
- [ ] Soumission sans authentification (anti-spam : captcha ou rate limit)
- [ ] Stockage des soumissions en attente (table `pending_submissions`)
- [ ] Notification Telegram à l'admin quand nouvelle soumission
- [ ] Page admin PWA : liste des soumissions en attente
- [ ] Actions admin : approuver (→ crée le node), rejeter (→ supprime), modifier avant approbation
- [ ] Historique des soumissions (qui, quand, statut)

**Implémentation** :
- Plugin Library : nouveaux endpoints
  - `POST /submissions` (public, no auth) — soumettre contenu
  - `GET /submissions` (auth required) — lister soumissions en attente
  - `PUT /submissions/{id}/approve` (auth) — approuver → crée node
  - `PUT /submissions/{id}/reject` (auth) — rejeter
  - `GET /submissions/{id}` (auth) — détail soumission
- Database : table `pending_submissions` (id, title, content, fields, template_id, submitter_name, submitter_email, status, created_at, reviewed_at)
- Kernel : proxy public étendu pour accepter POST sur `/v1/public/library/submissions`
- Public-lib : formulaire HTML/JS avec sélection template + champs dynamiques
- PWA : onglet "Soumissions" dans la page bibliothèque avec actions approve/reject
- Telegram : notification "Nouvelle soumission : {titre}" avec boutons inline approve/reject

**Fichiers concernés** :
- `symbion-plugin-library/src/database.rs` — table + CRUD submissions
- `symbion-plugin-library/src/routes.rs` — endpoints submissions
- `symbion-plugin-library/src/models.rs` — structs Submission
- `symbion-kernel/src/plugin_proxy.rs` — autoriser POST sur /submissions
- `public-lib/index.html` — formulaire soumission
- `pwa-dashboard/src/components/library-page.js` — onglet admin soumissions

---

### R3 — Gestion Lumière Domotique v1 🔴 TODO

**Objectif** : Contrôler les lumières de la maison depuis Symbion (PWA + Telegram + automations).

**Contexte** : Première vraie brique domotique. Intégration avec un système de lumières connectées (Zigbee/WiFi via bridge type Zigbee2MQTT, Philips Hue, ou Tasmota/ESPHome).

**Fonctionnalités** :
- [ ] Découverte des lumières disponibles (via bridge Zigbee2MQTT ou API Hue)
- [ ] Contrôle on/off par pièce et par lampe
- [ ] Réglage luminosité (dimmer 0-100%)
- [ ] Réglage température couleur (chaud/froid) si supporté
- [ ] Groupes de lumières par pièce (salon, chambre, bureau, cuisine)
- [ ] Automations contextuelles :
  - Mode Maison → lumières salon allumées chaud 60%
  - Mode Nuit → tout éteint sauf veilleuse
  - Coucher de soleil → allumage progressif
  - Absence détectée (agents offline) → extinction auto après 30min
- [ ] Widget PWA : contrôle lumières avec sliders
- [ ] Commandes Telegram : `/lumieres salon on`, `/lumieres tout off`

**Implémentation** :
- Nouveau plugin `symbion-plugin-lights` ou intégration dans kernel
- MQTT : bridge Zigbee2MQTT publie sur `zigbee2mqtt/+` → kernel traduit
- Modèle données : `lights` table (id, name, room, type, state, brightness, color_temp, bridge_topic)
- API endpoints :
  - `GET /v1/lights` — lister lumières
  - `GET /v1/lights/{id}` — état lumière
  - `PUT /v1/lights/{id}` — contrôler (on/off, brightness, color_temp)
  - `PUT /v1/lights/groups/{room}` — contrôler groupe
  - `POST /v1/lights/scenes` — créer scène (combinaison d'états)
  - `PUT /v1/lights/scenes/{id}/activate` — activer scène
- Automations : règles dans `data/automations.json` liées aux modes contextuels
- PWA Widget : `lights-widget.js` avec toggle par pièce + sliders
- Telegram : commandes `/lights` avec boutons inline

**Prérequis matériel** :
- Bridge Zigbee (Sonoff Zigbee 3.0 USB Dongle + Zigbee2MQTT) OU
- Ampoules WiFi (Tasmota/ESPHome) OU
- Philips Hue Bridge
- Au minimum : 1 ampoule connectée pour tester

**Fichiers concernés** :
- `symbion-plugin-lights/` (nouveau plugin) ou `symbion-kernel/src/lights.rs`
- `symbion-kernel/src/mqtt.rs` — subscription Zigbee2MQTT
- `pwa-dashboard/src/widgets/lights-widget.js` — widget contrôle
- `pwa-dashboard/src/components/lights-page.js` — page dédiée

---

### R4 — Plugin Cafetière v2 : Customisation Boissons 🔴 TODO

**Objectif** : Débloquer la personnalisation complète des boissons (intensité, volume ml, température) via reverse engineering du protocole cloud Philips.

**Contexte** : Le plugin `symbion-plugin-coffee` v1 contrôle la EP2520/10 en LAN via le protocole Condor (on/off, brew recettes fixes, statuts, AquaClean). Mais la machine **ignore** les paramètres `PrimDose`/`GrDose` envoyés en LAN — la customisation passe uniquement par le cloud RITA (protobuf). L'app officielle propose : intensité (légère/ordinaire/corsée), volume (espresso 30/40/70ml, long 100/120/200ml), température (basse/moyenne/élevée).

**Approche — Reverse engineering via Frida + mitmproxy** :
1. Téléphone Android rooté (vieux téléphone dédié)
2. Frida pour bypass certificate pinning de l'app Philips
3. mitmproxy pour capturer le trafic HTTPS vers les serveurs cloud
4. pbtk pour extraire les définitions .proto de l'APK
5. Décoder les requêtes protobuf de customisation
6. Reproduire le protocole dans notre plugin Rust

**Fonctionnalités visées** :
- [ ] Extraction .proto depuis APK Philips (structures protobuf)
- [ ] Capture trafic app → cloud lors de customisation café
- [ ] Décodage protocole RITA (requêtes protobuf customisation)
- [ ] Implémentation Rust : envoi direct des paramètres customisés
- [ ] UI PWA : sliders intensité (1-3), volume (ml), température (1-3)
- [ ] Telegram : `/cafe espresso corsé 50ml` — commande avec paramètres
- [ ] Sauvegarde profils personnalisés (ex: "Mon espresso du matin")

**Outils nécessaires** :
- Frida (`pip install frida-tools`) — instrumentation runtime
- mitmproxy (`pip install mitmproxy`) — proxy HTTPS
- pbtk (`github.com/marin-m/pbtk`) — extraction protobuf depuis APK
- Téléphone Android rooté avec ADB
- frida-interception-and-unpinning (`github.com/httptoolkit/frida-interception-and-unpinning`)

**Prérequis matériel** :
- Téléphone Android rooté (vieux téléphone dédié — en cours de préparation)

**Fichiers concernés** :
- `symbion-plugin-coffee/src/condor.rs` — nouveau client cloud ou LAN étendu
- `symbion-plugin-coffee/src/main.rs` — endpoints customisation
- `pwa-dashboard/src/widgets/coffee-widget.js` — UI sliders
- `symbion-plugin-telegram/src/telegram.rs` — commandes paramétrées

**Références** :
- [hcpy](https://github.com/hcpy2-0/hcpy) — approche similaire pour Home Connect (Bosch/Siemens)
- [pbtk](https://github.com/marin-m/pbtk) — extraction .proto depuis APK
- [frida-interception-and-unpinning](https://github.com/httptoolkit/frida-interception-and-unpinning)

---

## Ordre de Priorité

| # | Feature | Effort | Dépendances | Impact |
|---|---------|--------|-------------|--------|
| R1 | Notifications Telegram | Moyen | Plugin Telegram existant | Haut — alertes temps réel |
| R2 | Formulaire public + validation | Moyen | Plugin Library + Public-lib existants | Moyen — collaboratif |
| R3 | Lumières domotique v1 | Élevé | Matériel Zigbee/WiFi | Très haut — vraie domotique |
| R4 | Cafetière v2 customisation | Élevé | Téléphone rooté + reverse eng. | Moyen — confort quotidien |

**Suggestion** : R1 → R2 → R3 (le matériel pour R3 peut être commandé pendant R1/R2), R4 quand le téléphone rooté est prêt

---

## Historique Complété

Tout ce qui suit est **terminé et en production** :

- **Context Engine v2** — Détection SSID, horaires, patterns activité
- **Security Hardening** — JWT, bcrypt, CSRF, TLS 1.3, HSTS, CSP, WebAuthn, MFA
- **Decision Engine** — Trust scoring, validation multi-niveaux, audit trail
- **Metrics & Observability** — Health probes, /metrics endpoint, monitoring cron
- **Intelligence v2** — Pattern learning, inference engine, feature extraction
- **Automations Engine** — 16 règles actives, scheduler, event-driven
- **Agent Host v2.5** — Watchdog, file transfer, scheduler, log collector, 11 handlers
- **SQLite Migration** — 22 tables, JSON fallback, 5 migrations
- **Plugin System** — 8 plugins (notes, ssl, sensors, library, telegram, freebox, common, coffee)
- **Bibliothèque de Connaissances** — Graph, templates, FTS5, éditeur visuel, page publique
- **Bot Telegram** — Bridge Python + plugin Rust, commandes Claude Code
- **Plugin Cafetière v1** — Philips EP2520/10, Condor LAN, brew/power/status, AquaClean, widget PWA, Telegram
- **OpenAPI/Swagger** — 109 paths, Swagger UI
- **Thèmes Dynamiques** — 4 modes système + custom, logo colorisé
- **Environment Monitoring** — ESP32/BME280, alertes température/humidité
