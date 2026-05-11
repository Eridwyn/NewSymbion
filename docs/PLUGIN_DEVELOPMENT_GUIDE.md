# Guide de Développement de Plugins Symbion

## Vue d'ensemble

Les plugins Symbion sont des binaires Rust indépendants qui communiquent avec le kernel via MQTT. Chaque plugin est auto-contenu, découvert automatiquement au démarrage du kernel, et géré avec un cycle de vie complet (démarrage, surveillance santé, redémarrage automatique).

## Architecture Plugin

```
Kernel (Hub)
    ↓ MQTT
Plugin (Standalone Binary)
    ↓ MQTT Topics
ESP32 / Agents / Services
```

### Avantages
- **Isolation** : Crash d'un plugin n'affecte pas le kernel
- **Hot reload** : Redémarrage sans arrêter le kernel
- **Scalabilité** : Chaque plugin = processus séparé
- **Extensibilité** : Ajout de fonctionnalités sans modifier le kernel

## Étape 1 : Créer la Structure du Plugin

### 1.1 Créer le package Rust

```bash
# Créer le répertoire du plugin
mkdir symbion-plugin-<nom>
cd symbion-plugin-<nom>

# Initialiser Cargo
cargo init --name symbion-plugin-<nom>
```

### 1.2 Configurer Cargo.toml

```toml
[package]
name = "symbion-plugin-<nom>"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "symbion-plugin-<nom>"
path = "src/main.rs"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
rumqttc = "0.24.0"
time = { version = "0.3.41", features = ["serde", "formatting", "parsing"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "2.0"
parking_lot = "0.12"  # Pour thread-safe collections
```

### 1.3 Ajouter au workspace

Dans `Cargo.toml` racine:

```toml
[workspace]
members = [
    "symbion-kernel",
    "symbion-plugin-notes",
    "symbion-plugin-<nom>",  # ← Ajouter ici
    # ...
]
```

## Étape 2 : Implémenter le Plugin

### 2.1 Structure de base (src/main.rs)

```rust
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, Publish, QoS};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Message structure pour votre domaine
#[derive(Debug, Serialize, Deserialize)]
struct MyData {
    id: String,
    value: f32,
    timestamp: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[plugin-<nom>] Starting v0.1.0");

    // Configuration MQTT
    let mut mqttoptions = MqttOptions::new(
        "symbion-plugin-<nom>",
        "127.0.0.1",
        1883
    );
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscription aux topics
    client
        .subscribe("symbion/<domaine>/commands@v1", QoS::AtLeastOnce)
        .await?;

    println!("[plugin-<nom>] MQTT connected, waiting for messages...");

    // Event loop principal
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                handle_mqtt_message(&client, publish).await;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("[plugin-<nom>] MQTT error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn handle_mqtt_message(client: &AsyncClient, publish: Publish) {
    let topic = publish.topic.as_str();
    let payload = String::from_utf8_lossy(&publish.payload);

    // Router selon le topic
    if topic == "symbion/<domaine>/commands@v1" {
        // Traiter la commande
        println!("[plugin-<nom>] Received command: {}", payload);

        // Publier réponse
        let response = serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        let _ = client.publish(
            "symbion/<domaine>/responses@v1",
            QoS::AtLeastOnce,
            false,
            serde_json::to_string(&response).unwrap()
        ).await;
    }
}
```

### 2.2 Patterns recommandés

#### Thread-safe State Management

```rust
use parking_lot::RwLock;
use std::collections::HashMap;

struct PluginState {
    data: RwLock<HashMap<String, MyData>>,
}

impl PluginState {
    fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    fn add(&self, id: String, value: MyData) {
        self.data.write().insert(id, value);
    }

    fn get(&self, id: &str) -> Option<MyData> {
        self.data.read().get(id).cloned()
    }
}
```

## Étape 3 : Créer le Manifest du Plugin

### 3.1 Créer le fichier manifest

Créer `plugins/symbion-plugin-<nom>.json`:

```json
{
  "name": "<nom>-manager",
  "description": "Description du plugin et son rôle",
  "version": "0.1.0",
  "binary": "./target/release/symbion-plugin-<nom>",
  "contracts": [
    "<domaine>.command@v1",
    "<domaine>.response@v1",
    "<domaine>.event@v1"
  ],
  "auto_start": true,
  "restart_on_failure": true,
  "startup_timeout_seconds": 10,
  "shutdown_timeout_seconds": 5,
  "depends_on": [],
  "start_priority": 20,
  "env": {
    "PLUGIN_CUSTOM_VAR": "value"
  }
}
```

### 3.2 Champs obligatoires du manifest

| Champ | Type | Description | Exemple |
|-------|------|-------------|---------|
| `name` | string | Nom unique du plugin | `"sensors-manager"` |
| `description` | string | Description courte | `"Environment sensors (F1)"` |
| `version` | string | Version semver | `"0.1.0"` |
| `binary` | string | **Chemin RELATIF** au projet | `"./target/release/symbion-plugin-sensors"` |
| `contracts` | array | Topics MQTT gérés | `["sensors.env@v1"]` |
| `auto_start` | boolean | Démarrage automatique | `true` |
| `restart_on_failure` | boolean | Redémarrage si crash | `true` |
| `startup_timeout_seconds` | number | Timeout démarrage | `10` |
| `shutdown_timeout_seconds` | number | Timeout arrêt propre | `5` |
| `depends_on` | array | Dépendances plugins | `[]` |
| `start_priority` | number | Ordre démarrage (0-100) | `20` |
| `env` | object | Variables environnement | `{}` |

### 3.3 ⚠️ PIÈGES COMMUNS À ÉVITER

#### ❌ Chemin binary incorrect

```json
// ❌ INCORRECT - Chemin relatif depuis plugins/
"binary": "../target/release/symbion-plugin-sensors"

// ✅ CORRECT - Chemin relatif depuis racine projet
"binary": "./target/release/symbion-plugin-sensors"
```

**Raison** : Le kernel s'exécute depuis la racine du projet (`/home/.../NewSymbion`), pas depuis `plugins/`.

#### ❌ Champs manquants

Le kernel attend **TOUS** ces champs. Si un seul manque, le plugin ne sera pas chargé.

**Checklist obligatoire** :
- [ ] `name`
- [ ] `description`
- [ ] `version`
- [ ] `binary` (chemin correct)
- [ ] `contracts` (array, peut être vide)
- [ ] `auto_start`
- [ ] `restart_on_failure`
- [ ] `startup_timeout_seconds`
- [ ] `shutdown_timeout_seconds`
- [ ] `depends_on` (array, peut être vide)
- [ ] `start_priority`
- [ ] `env` (object, peut être vide)

#### ❌ Permissions incorrectes

```bash
# Vérifier permissions manifest
ls -la plugins/symbion-plugin-<nom>.json
# Doit être : -rw-r--r-- eridwyn eridwyn

# Vérifier permissions binary
ls -la target/release/symbion-plugin-<nom>
# Doit être : -rwxrwxr-x eridwyn eridwyn (exécutable)

# Corriger si nécessaire
sudo chown eridwyn:eridwyn plugins/symbion-plugin-<nom>.json
sudo chmod 644 plugins/symbion-plugin-<nom>.json

sudo chown eridwyn:eridwyn target/release/symbion-plugin-<nom>
sudo chmod +x target/release/symbion-plugin-<nom>
```

## Étape 4 : Compiler et Tester

### 4.1 Compilation

```bash
# Compiler le plugin
cargo build --release -p symbion-plugin-<nom>

# Vérifier que le binary existe
ls -lh target/release/symbion-plugin-<nom>
```

### 4.2 Test manuel (sans kernel)

```bash
# Lancer le plugin manuellement
./target/release/symbion-plugin-<nom>

# Surveiller les logs
# Le plugin doit afficher :
# [plugin-<nom>] Starting v0.1.0
# [plugin-<nom>] MQTT connected, waiting for messages...
```

### 4.3 Tester avec MQTT

Dans un autre terminal:

```bash
# Publier un message test
mosquitto_pub -h localhost -p 1883 \
  -t "symbion/<domaine>/commands@v1" \
  -m '{"action":"test","value":42}'

# Écouter les réponses
mosquitto_sub -h localhost -p 1883 \
  -t "symbion/<domaine>/responses@v1" \
  -v
```

## Étape 5 : Intégration avec le Kernel

### 5.1 Vérifier découverte du plugin

```bash
# Redémarrer le kernel (systemd)
sudo systemctl restart symbion-kernel

# Vérifier les logs de découverte
sudo journalctl -u symbion-kernel --since "1 minute ago" | grep plugin

# Doit afficher :
# [plugins] discovered: <nom>-manager (from symbion-plugin-<nom>)
# [plugins] started <nom>-manager (instance ...)
```

### 5.2 Vérifier via API

```bash
# Lister les plugins chargés
curl -s -k -H "X-API-Key: s3cr3t-42" \
  https://localhost:8443/v1/plugins | jq '.'

# Doit montrer votre plugin avec status "Running"
```

### 5.3 Vérifier processus

```bash
# Voir les plugins en cours d'exécution
pgrep -fa symbion-plugin

# Doit afficher :
# <PID> ./target/release/symbion-plugin-<nom>
```

## Étape 6 : Debugging

### 6.1 Plugin ne se charge pas

```bash
# 1. Vérifier logs kernel au démarrage
sudo journalctl -u symbion-kernel --since "2 minutes ago" | grep -E "plugin|discovered|failed"

# Chercher :
# - "failed to load manifest" → Problème JSON ou champs manquants
# - "binary not found" → Chemin binary incorrect
# - "Permission denied" → Problème permissions
```

### 6.2 Erreurs communes

| Erreur | Cause | Solution |
|--------|-------|----------|
| `JSON error: missing field 'contracts'` | Champ manquant | Ajouter tous les champs obligatoires |
| `binary not found: ../target/...` | Mauvais chemin | Utiliser `./target/release/...` |
| `Permission denied (os error 13)` | Permissions | `chown` + `chmod` sur manifest/binary |
| `Plugin not in workspace` | Pas dans Cargo.toml | Ajouter dans `members = [...]` |

### 6.3 Plugin crash au démarrage

```bash
# Tester manuellement pour voir l'erreur
./target/release/symbion-plugin-<nom>

# Vérifier connexion MQTT
mosquitto_sub -h localhost -p 1883 -t '#' -v | grep <nom>
```

## Exemple Complet : Plugin Sensors (F1)

Voir le code source:
- **Code** : `symbion-plugin-sensors/src/main.rs`
- **Manifest** : `plugins/symbion-plugin-sensors.json`

### Contrats MQTT utilisés

```
Subscribe:
- symbion/sensors/registration@v1  (auto-registration ESP32)
- symbion/sensors/+/env@v1          (wildcard pour tous les sensors)

Publish:
- symbion/plugin/sensors/response@v1  (réponses API)
- symbion/dashboard/environment@v1    (push dashboard)
```

### Features implémentées

- Auto-registration des sensors ESP32
- Stockage en mémoire avec RwLock
- Circular buffer (max 100 readings)
- Status evaluation (Normal, WarningVentilate, RiskMold, TempLow)
- Health check automatique via MQTT presence

## Checklist Création Plugin

- [ ] Package Rust créé (`cargo init`)
- [ ] Dépendances ajoutées (rumqttc, serde, tokio)
- [ ] Ajouté au workspace (`Cargo.toml` racine)
- [ ] Code principal implémenté (`src/main.rs`)
- [ ] Manifest créé avec **TOUS** les champs obligatoires
- [ ] Chemin binary **relatif depuis racine** (`./target/release/...`)
- [ ] Contracts MQTT définis
- [ ] Compilation réussie (`cargo build --release`)
- [ ] Binary exécutable (`chmod +x`)
- [ ] Permissions manifest correctes (`644 eridwyn:eridwyn`)
- [ ] Test manuel réussi (connexion MQTT)
- [ ] Découverte par kernel confirmée (journalctl)
- [ ] Status "Running" dans API `/v1/plugins`
- [ ] Processus visible (`pgrep`)
- [ ] Communication MQTT opérationnelle

## Ressources

- **Architecture MQTT** : `docs/mqtt/topics.md`
- **Contrats MQTT** : `docs/mqtt/contracts.md`
- **API Kernel** : `docs/api/endpoints.md`
- **Plugin Notes** : `symbion-plugin-notes/` (référence)
- **Plugin Sensors** : `symbion-plugin-sensors/` (exemple complet)

---

## Étape 6 : Déclarer des actions templates pour le rule builder

Depuis mai 2026, un plugin peut **exposer ses actions sous forme de templates structurés**
au rule builder PWA. Au lieu de forcer l'utilisateur à taper du JSON dans un textarea
(`plugin_command` brut), le plugin déclare ses endpoints avec leurs paramètres typés,
et le PWA génère automatiquement un formulaire (selects, sliders, sub-fields).

### 6.1 Pourquoi

Sans templates, créer une automation « allumer la machine à café » demande à l'utilisateur :
- de connaître la route exacte (`/power`)
- de saisir le payload JSON correct (`{"on": true}`)
- de gérer la sérialisation (les bool, les nombres, etc.)

Avec templates : l'utilisateur choisit *« Plugin café → Allumer la machine »* dans 2 selects.
Le payload est construit automatiquement.

### 6.2 Déclarer les actions au register

Dans le `tokio::spawn` de registration, utiliser `PluginRegistrationBuilder.action(...)` :

```rust
use symbion_plugin_common::{PluginAction, PluginActionParam, PluginActionOption};

PluginRegistrationBuilder::new(PLUGIN_ID, SOCKET_PATH)
    .route("/power")
    .route("/brew")
    .action(PluginAction {
        name: "power_on".into(),
        label: "Allumer la machine".into(),
        description: Some("Sort la machine du mode standby".into()),
        icon: Some("⚡".into()),
        route: "power".into(),       // sans slash de tête
        method: "POST".into(),
        impact_level: "Low".into(),
        params: vec![
            PluginActionParam {
                name: "on".into(),
                label: "État".into(),
                param_type: "bool".into(),
                required: true,
                default: Some(serde_json::json!(true)),
                options: vec![],
                min: None, max: None, placeholder: None,
            },
        ],
    })
    .action(PluginAction {
        name: "brew".into(),
        label: "Lancer un café".into(),
        icon: Some("☕".into()),
        route: "brew".into(),
        method: "POST".into(),
        impact_level: "Medium".into(),
        params: vec![
            PluginActionParam {
                name: "drink".into(),
                label: "Boisson".into(),
                param_type: "select".into(),
                required: true,
                default: Some(serde_json::json!("espresso")),
                options: vec![
                    PluginActionOption { value: serde_json::json!("espresso"), label: "Espresso".into() },
                    PluginActionOption { value: serde_json::json!("coffee"), label: "Café long".into() },
                ],
                min: None, max: None, placeholder: None,
                description: None,
            },
            PluginActionParam {
                name: "temperature".into(),
                label: "Température (1-3)".into(),
                param_type: "int".into(),
                required: false,
                default: Some(serde_json::json!(2)),
                options: vec![],
                min: Some(1.0), max: Some(3.0), placeholder: None,
            },
        ],
        description: Some("Démarre la préparation".into()),
    })
    .register()
    .await?;
```

### 6.3 Types de paramètres supportés

| `param_type` | Rendering PWA | Sérialisation JSON |
|-------------|---------------|--------------------|
| `bool`      | Select Vrai/Faux | `true` / `false` |
| `int`       | Number input | `42` |
| `float`     | Number input step 0.01 | `3.14` |
| `string`    | Text input | `"hello"` |
| `select`    | Select avec `options[]` | preserve type d'origine de `value` |
| `text_area` | Textarea | `"multi\nlines"` |

`min`/`max` pour les nombres, `placeholder` pour les inputs texte, `default` rempli
automatiquement à la sélection de l'action.

### 6.3bis Wrap Contract v1.0 (plugins exposant `/actions` générique)

Si ton plugin expose un endpoint `/actions` générique qui attend le format
Contract v1.0 (`{spec_version, action_id, action_type, payload}`) au lieu d'une
route directe par action, ajoute `wrap_protocol: Some("v1".into())` dans la
`PluginAction`. L'executor wrap automatiquement le payload avant POST :

```rust
.action(PluginAction {
    name: "send_notification".into(),
    label: "Envoyer notification".into(),
    route: "actions".into(),         // route /actions générique
    method: "POST".into(),
    impact_level: "Low".into(),
    wrap_protocol: Some("v1".into()), // ← active le wrap auto
    params: vec![
        PluginActionParam {
            name: "text".into(),
            label: "Message".into(),
            param_type: "text_area".into(),
            required: true,
            default: None,
            options: vec![],
            min: None, max: None,
            placeholder: Some("Texte du message".into()),
        },
    ],
    description: None, icon: None,
})
```

Le payload reçu côté plugin sera : `{"spec_version":"1.0","action_id":"<uuid>","action_type":"send_notification","payload":{"text":"..."}}`.

Sans `wrap_protocol` (ou `Some("raw")`), le payload est posté tel quel (cas coffee/power).

Référence : `symbion-plugin-telegram/src/main.rs` (2 actions wrap v1).

### 6.4 Côté kernel

Pas de modif. Le `PluginRegistration` JSON envoyé au kernel contient le champ `actions`,
le kernel le stocke dans `PluginInfo` (`symbion-kernel/src/plugin_proxy.rs:104`) et l'expose
via `GET /v1/plugins`. Le PWA pull au premier rendu du form `plugin_command` et cache.

### 6.5 Comportement runtime

Quand l'automation s'exécute :
1. Le rule builder PWA construit `ActionDefinition::PluginCommand { plugin, route, payload }`
   à partir du form (les valeurs des sub-fields → keys du payload object)
2. `PluginCommandExecutor` (`symbion-kernel/src/automations/executors.rs`) résout le socket
   via `PluginRegistry::find_socket(/v1/plugin-api/{plugin}/{route})`
3. POST HTTP via Unix socket avec le payload sérialisé
4. Plugin reçoit la requête comme un POST normal sur `/{route}` avec body JSON

### 6.6 Fallback

Si le plugin n'a pas déclaré de `actions` (champ vide), le PWA retombe automatiquement
sur le formulaire libre route + textarea payload JSON (back-compat totale).

### 6.7 Inventaire des plugins avec templates (mai 2026)

| Plugin | Actions | Wrap | Exemples d'usage automation |
|--------|---------|------|------------------------------|
| `coffee` | 4 (power_on, power_off, brew, stop) | raw | Préchauffer à 7h, brew espresso après réunion |
| `telegram` | 2 (send_notification, send_message) | v1 | Alerter en cas d'incident, message à user précis |
| `sensors` | 3 (list_sensors, get_environment, get_sensor) | v1 | Diagnostic, lire chambre dans une chaîne d'actions |
| `notes` | 2 (create_note, delete_note) | v1 | Journaling auto sur changement de mode, archivage |
| `ssl` | 1 (check_now) | raw | Force vérif certificats matinale 6h |

**Sans templates** (volontairement) :
- `library` : POST nodes/sections demande des structures dynamiques (template_id,
  fields typés selon template). Pas pertinent pour automation simple.
- `freebox` : que de la lecture via /health et MQTT.

### 6.8 Référence

Implémentations de référence :
- `symbion-plugin-coffee/src/main.rs:691-790` : 4 actions, routes directes (raw)
- `symbion-plugin-telegram/src/main.rs:131-225` : 2 actions wrap v1 sur /actions
- `symbion-plugin-sensors/src/main.rs:670-740` : 3 actions wrap v1
- `symbion-plugin-notes/src/main.rs:867-955` : 2 actions wrap v1
- `symbion-plugin-ssl/src/main.rs:202-235` : 1 action raw

### 6.9 Piège connu : auto-discovery vs register manuel

Fixé en mai 2026 (commit `7405804`) — `plugin_proxy::register_plugin` purge
maintenant les anciennes entries du même plugin name avant d'insérer les
nouvelles. Auparavant : si l'auto-discovery au boot avait inscrit le plugin
avec une route catch-all `""`, le re-register manuel rajoutait les nouvelles
routes SANS retirer l'ancien entry, et `list_plugins` pouvait renvoyer
l'instance stale (0 actions visibles). Plus d'inquiétude maintenant — chaque
register repart d'un état propre pour ce plugin name.

---

**Dernière mise à jour** : 10 Mai 2026
**Version Symbion** : 1.5.0
**Plugins actifs** : notes, ssl, sensors, library, telegram, freebox, common, coffee
