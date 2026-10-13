# Conception — Plugin Lumières (symbion-plugin-lights)

> Document de **conception** pour la roadmap R3 (Gestion Lumière Domotique v1).
> Cible matérielle retenue : **ampoules WiFi Tuya / Smart Life existantes**, pilotées
> en **local sur le LAN** (protocole Tuya, sans cloud ni Alexa). Aucun code n'est écrit.
>
> NB : ce doc évoque aussi des drivers Tasmota/ESPHome — gardés comme drivers futurs
> possibles, mais **le driver prioritaire est Tuya-local** (matériel réellement possédé).

Statut : **CONSTRUIT le 13 octobre 2026** → `symbion-plugin-lights/`, en Python
avec `tinytuya` (option A du §3). Écarts assumés par rapport à ce document :

- la table de DP du §2 (`1` on/off, `2`/`22` luminosité, `3`/`23` température)
  n'est juste que pour les ampoules de **type B**. Les types A et C ont une
  autre numérotation ET une autre échelle (0-255). Le plugin délègue la
  détection à `tinytuya.BulbDevice.detect_bulb()` au lieu de coder une table ;
- les automations passent par `PluginCommand` (§8, inchangé) **et** par un
  déclencheur poussé, ajouté au kernel pour l'occasion : voir
  `docs/plugins/AUTOMATIONS_PLUGIN_EVENTS.md` ;
- les drivers Tasmota et ESPHome ne sont pas écrits (pas de matériel).

Dernière mise à jour du design : 1 juillet 2026.

---

## 1. Objectif et périmètre v1

Contrôler les lumières de la maison depuis Symbion (PWA + automations + Telegram),
en réutilisant le moule des plugins existants (cf. `symbion-plugin-synology`,
`symbion-plugin-ssl`).

**Dans v1** : on/off, luminosité (0-100%), température de couleur (chaud/froid),
groupes par pièce, scènes, automations liées aux modes contextuels, widget PWA.
**Hors v1** (notés mais reportés) : couleur RGB complète, transitions/animations,
découverte auto Home Assistant, commandes Telegram avancées.

---

## 2. Choix matériel : ampoules Tuya / Smart Life, pilotées en LOCAL

Matériel réel : ampoules WiFi **Tuya / Smart Life** déjà installées (aujourd'hui via
cloud + Alexa). On les garde telles quelles mais on les pilote **en local sur le LAN**,
**sans cloud ni Alexa** (cohérent avec la philosophie résiliente de Symbion).

### Le protocole Tuya local (vs Tasmota/ESPHome)
Contrairement à Tasmota/ESPHome (qui parlent MQTT), une ampoule Tuya parle un
**protocole binaire chiffré (AES)** en TCP sur le port **6668**, versions 3.1 → 3.5.
Pour la piloter en local il faut **3 infos par appareil** : `device_id`, `local_key`
(clé secrète), et l'`ip` LAN. Le `local_key` s'extrait **une seule fois** via un compte
Tuya IoT (voir §13).

| Aspect | Tuya local |
|--------|------------|
| Transport | TCP 6668, payload chiffré AES (proto 3.1-3.5) |
| Commande on/off | DP `1` = bool |
| Luminosité | DP `2` ou `22` = **10-1000** (échelle Tuya) |
| Temp couleur | DP `3` ou `23` = **0-1000** (0=chaud .. 1000=froid) |
| État | l'ampoule pousse ses DPs sur la même connexion |
| Disponibilité | TCP joignable / heartbeat (pas de LWT MQTT) |

**Conséquence de design** : protocole hétérogène ET non-MQTT (≠ Tasmota). D'où une
**abstraction driver** qui (a) gère son propre transport (TCP Tuya / MQTT pour les autres),
(b) normalise tout en unités Symbion (luminosité **%**, température en **Kelvin**), (c)
émet des `LightUpdate` normalisés. Le reste du plugin ignore Tuya.

> Le `local_key` peut changer si l'ampoule est ré-appairée dans l'appli → prévoir une
> procédure de ré-extraction. Sinon le contrôle local reste stable et 100% hors-ligne.

---

## 3. Architecture

Nouveau plugin **`symbion-plugin-lights`** (binaire Rust, socket Unix + MQTT),
identique en structure à synology :

```
symbion-plugin-lights/
├── Cargo.toml
├── manifest.json
└── src/
    ├── main.rs        # tasks: MQTT, API socket, registration kernel, SIGTERM
    ├── config.rs      # config TOML (devices déclarés, mqtt, http)
    ├── model.rs       # Light, Room, Scene, capabilities, normalisation
    ├── mqtt.rs        # publisher Symbion + subscriber devices
    ├── state.rs       # registre dynamique (état live + scènes), persistance JSON
    └── drivers/
        ├── mod.rs     # trait LightDriver (transport-agnostique)
        ├── tuya.rs    # PRIORITAIRE : TCP 6668 chiffré <-> état normalisé
        ├── tasmota.rs # (futur) cmnd/stat/tele MQTT
        └── esphome.rs # (futur) command/state JSON MQTT
```

### Trait driver (clé de l'abstraction)
Le driver **possède son transport** (TCP Tuya pour Tuya ; le bus MQTT pour Tasmota/ESPHome).
Il pousse les mises à jour normalisées via un canal, le reste du plugin ne voit que des `Light`.
```rust
#[async_trait]
trait LightDriver {
    /// Démarre la connexion device et émet les LightUpdate sur `tx` (état + dispo).
    async fn run(&self, tx: Sender<LightUpdate>);
    /// Applique une commande normalisée (on/off, brightness %, color_temp K).
    async fn apply(&self, cmd: &LightCommand) -> Result<()>;
}
```

### Décision à trancher : langage du driver Tuya
Le protocole Tuya local (AES, négociation de session, versions 3.1-3.5) est **la partie
difficile**. Deux options :

| Option | Avantage | Inconvénient |
|--------|----------|--------------|
| **A. Service Python + `tinytuya`** (recommandé) | `tinytuya` est LA lib locale mature (toutes versions proto, scan réseau, communauté). Précédent repo : les MCP servers sont en Python. | Pas en Rust ; réimplémenter l'enregistrement kernel + socket en Python (HTTP simple). |
| **B. Plugin Rust + crate Tuya** | Cohérent avec les autres plugins, réutilise `symbion-plugin-common`. | Crates Tuya Rust peu maintenues → risque sur proto 3.4/3.5, on debug du chiffrement au lieu de livrer. |

**Reco : A** (Python + tinytuya) — le but est « local et résilient qui marche », et tinytuya
résout le plus dur. Le service Python publie le contrat Symbion (MQTT `symbion/lights/*`,
features, petite API socket, enregistrement kernel) exactement comme un plugin Rust.

---

## 4. Modèle de données

```rust
struct Light {
    id: String,            // logique, stable (ex: "salon_lampadaire")
    name: String,          // affichage ("Lampadaire salon")
    room: String,          // "salon" | "chambre" | "bureau" | "cuisine"
    driver: DriverKind,    // Tasmota | Esphome
    base_topic: String,    // topic device (ex tasmota: "salon_lampadaire")
    caps: Caps,            // dimmable, color_temp, (rgb plus tard)
    // état live (rempli par les messages device)
    online: bool,
    on: bool,
    brightness: u8,        // 0-100 normalisé
    color_temp_k: Option<u16>, // Kelvin normalisé (2200 chaud .. 6500 froid)
}

struct Caps { dimmable: bool, color_temp: bool }

struct Scene {
    id: String,            // "nuit", "cinema", "salon_chaud"
    name: String,
    states: Vec<SceneTarget>, // (light_id, on, brightness?, color_temp_k?)
}
```

### Normalisation (le travail réel)
| Grandeur | Symbion (interne) | Tasmota | ESPHome |
|----------|-------------------|---------|---------|
| Luminosité | 0-100 % | Dimmer 0-100 (identique) | brightness 0-255 (× 2.55) |
| Temp couleur | Kelvin 2200-6500 | mireds 153-500 (`mired = 1e6/K`) | mireds idem |
| On/off | bool | POWER ON/OFF | `"state":"ON"` |

---

## 5. Contrat MQTT côté Symbion (ce que le plugin expose)

Comme les autres plugins (le kernel n'ingère QUE `symbion/features/update`) :

- **État normalisé par lampe** (retained) : `symbion/lights/{id}`
  ```json
  {"id":"salon_lampadaire","room":"salon","online":true,"on":true,
   "brightness":60,"color_temp_k":2700,"name":"Lampadaire salon"}
  ```
- **Résumé** (retained) : `symbion/lights/summary` (toutes les lampes + pièces)
- **Features** → `symbion/features/update` (pour automations) :
  - `lights.{room}.any_on` (bool) — au moins une lampe allumée dans la pièce
  - `lights.{id}.on` (bool), `lights.{id}.brightness` (int)
- **Manifest** : `symbion/plugins/lights/manifest` · **Health** : `symbion/plugins/lights/status`

---

## 6. API HTTP (socket Unix, proxifiée `/v1/plugin-api/lights/*`)

| Méthode | Route | Rôle |
|---------|-------|------|
| GET | `/health` | santé plugin |
| GET | `/lights` | lister (état live + capabilities) |
| GET | `/lights/{id}` | état d'une lampe |
| PUT | `/lights/{id}` | contrôler `{on?, brightness?, color_temp_k?}` |
| PUT | `/lights/groups/{room}` | contrôler toutes les lampes d'une pièce |
| GET | `/scenes` · POST `/scenes` | lister / créer une scène |
| PUT | `/scenes/{id}/activate` | activer une scène |

PUT applique la commande → driver encode → publish MQTT device → l'état revient
via les topics device et met à jour le registre (boucle fermée, pas d'optimistic blind).

---

## 7. PluginActions (rule builder PWA, icône 💡)

Exposées à l'enregistrement kernel (comme l'action `check_now` de ssl) :
- `room_on` / `room_off` (param: pièce) — impact Low
- `set_scene` (param: scène) — impact Low
- `all_off` — impact Low

→ utilisables directement dans le builder d'automations sans payload JSON libre.

---

## 8. Automations ↔ modes contextuels (la vraie valeur)

On vient de fiabiliser le pilotage de mode par l'Intelligence v2 → on peut chaîner
mode → lumières de façon fiable. Exemples (règles dans `data/automations.json`) :

| Déclencheur | Condition | Action |
|-------------|-----------|--------|
| `ModeChange` → maison | — | `PluginCommand lights set_scene=salon_chaud` |
| `ModeChange` → veille | heure > 22h | `PluginCommand lights set_scene=nuit` |
| `Scheduled` 5min | `agent.online == false` depuis 30min | `PluginCommand lights all_off` (absence) |
| `Scheduled` | coucher de soleil (feature soleil) | allumage progressif salon |

`PluginCommand` existe déjà dans le moteur (`ActionDefinition::PluginCommand`). Le
déclencheur `ModeChange` existe aussi → **aucune modif kernel nécessaire** pour ces règles.

---

## 9. Widget PWA + Telegram

- **`lights-widget.js`** (moule de `synology-widget.js`) : cartes par pièce, toggle,
  slider luminosité, slider temp couleur, boutons scènes. Conso via `mqtt-service`
  (ajout abonnement `symbion/lights/#` + handler + cache, comme synology). Contrôle via
  `/v1/plugin-api/lights/...`. Lampe offline → carte grisée.
- **`lights-page.js`** (optionnel v1.1) : page dédiée gestion lampes + édition scènes.
- **Telegram** (reporté v1.1) : `/lumieres salon on|off`, `/lumieres scene nuit` —
  câblé dans `symbion-plugin-telegram` qui appellera l'API du plugin lights.

---

## 10. Config (TOML) — déclaration explicite des appareils

Pas de découverte auto en v1 (simple, contrôlé, robuste — cf. ssl). Chaque appareil
est déclaré ; l'attribution de pièce/nom reste éditable via API ensuite.

```toml
[mqtt]
host = "127.0.0.1"
port = 1883
client_id = "symbion-plugin-lights"

[http]
socket_path = "/run/symbion-plugins/lights.sock"

[[lights]]
id = "salon_lampadaire"
name = "Lampadaire salon"
room = "salon"
driver = "tuya"
device_id = "bfxxxxxxxxxxxxxxxxxxxx"   # extrait via tinytuya wizard (§13)
local_key = "xxxxxxxxxxxxxxxx"          # SECRET — ne pas committer
ip = "192.168.1.50"                     # IP LAN de l'ampoule (idéalement réservée DHCP)
version = "3.4"                         # version protocole Tuya
dimmable = true
color_temp = true
```

> Les `local_key` sont des **secrets** → fichier de config hors-repo (ex.
> `/opt/symbion/config/lights.toml`, perms 600), comme `synology.toml`. Le template
> committé ne contient que des valeurs factices.

> **Option v2** : ingestion de la découverte Home Assistant (`homeassistant/light/+/config`),
> que Tasmota ET ESPHome savent publier → auto-peuplement du registre. Reporté pour
> garder v1 simple.

---

## 11. Découpage build (quand on passera au code)

1. **Lot 1 — Squelette + 1 driver** : plugin, config, registre, driver Tasmota,
   subscribe état/dispo, publish `symbion/lights/{id}` + features, API GET/PUT on-off.
   → testable avec un device Tasmota **simulé** (publier des messages à la main via mosquitto_pub).
2. **Lot 2 — Luminosité + temp couleur + groupes** + driver ESPHome.
3. **Lot 3 — Scènes** (état JSON persistant + activate) + PluginActions + manifest/health.
4. **Lot 4 — Widget PWA** (toggle + sliders) + abonnement mqtt-service.
5. **Lot 5 — Automations modes↔lumières** + (v1.1) page PWA + Telegram.

Chaque lot est testable sans matériel réel via `mosquitto_pub`/`mosquitto_sub`
(on simule un Tasmota), exactement comme on a validé synology.

---

## 12. Décisions à trancher / questions ouvertes

1. **Tasmota et/ou ESPHome dès v1 ?** Le design prévoit les deux drivers, mais on peut
   ne coder que Tasmota d'abord (le plus répandu/simple) et ajouter ESPHome au lot 2.
2. **Découverte auto (HA discovery) v1 ou v2 ?** Proposé v2. À confirmer.
3. **Scènes : statiques (config) ou éditables (API + JSON) ?** Proposé éditable (comme
   les domaines ssl). À confirmer.
4. **Nomenclature pièces** : liste fixe (salon/chambre/bureau/cuisine) ou libre ? Proposé libre.
5. **Lien avec les modes** : quelles règles mode→scène tu veux exactement (à définir une
   fois que des scènes existent).

---

## 13. Prérequis : extraction des clés locales Tuya (setup unique)

Le matériel existe déjà (ampoules Smart Life). Le seul setup est l'**extraction unique
des `local_key`** :

1. Créer un compte **Tuya IoT Platform** (iot.tuya.com, gratuit) + un « Cloud Project ».
2. Y lier ton compte **Smart Life** (via l'app account / QR) → les ampoules remontent.
3. `pip install tinytuya` puis `python -m tinytuya wizard` → récupère pour chaque ampoule :
   `device_id`, `local_key`, et l'`ip` (scan LAN inclus : `python -m tinytuya scan`).
4. Renseigner ces valeurs dans `lights.toml` (§10).
5. Test local immédiat : `python -m tinytuya` ou un court script tinytuya → on/off une ampoule
   **sans passer par le cloud**. Si ça répond → le Lot 1 est câblable.

> **Réserver les IP** des ampoules en DHCP statique sur ta box (sinon l'IP peut changer et
> casser le contrôle local). Alternative : résolution par `device_id` via scan au démarrage.

### Coexistence avec Alexa / Smart Life (important)
Le contrôle local **n'enlève pas** les ampoules du cloud. Elles restent appairées à
Smart Life + Alexa et continuent de marcher normalement. Le canal local (TCP 6668) et le
canal cloud (Alexa) **coexistent** sur l'ampoule. Conséquences :
- ✅ Alexa et Smart Life continuent de fonctionner.
- ✅ Il **faut** garder les ampoules dans l'app Smart Life (c'est ce qui fournit/rafraîchit
  le `local_key`).
- ✅ Si Alexa allume une ampoule, Symbion **voit** le changement (l'ampoule pousse son état
  sur la connexion locale) → widget/features restent synchro.
- ✅ Contrôle Symbion **fonctionne même internet coupé** (c'est le gain de résilience).
- ⚠️ Une seule connexion **locale** à la fois par ampoule : éviter un 2e contrôleur local
  concurrent (ex. Home Assistant LocalTuya) sur la même ampoule. Le cloud, lui, ne compte pas.
- ⚠️ Ré-appairer une ampoule dans l'app change son `local_key` → re-extraire.
