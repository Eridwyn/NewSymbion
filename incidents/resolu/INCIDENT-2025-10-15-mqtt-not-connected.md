# Rapport d'Incident - MQTT Status "connecting" et Agents non mis à jour

**Date**: 2025-10-15
**Durée**: En cours (détecté ~19h00)
**Sévérité**: Haute (Agents non synchronisés, monitoring dégradé)
**Statut**: 🔍 En diagnostic

---

## 📋 Résumé Exécutif

Le kernel Symbion affiche un status MQTT "connecting" dans l'API health alors que la connexion MQTT est fonctionnelle. Conséquence : les agents qui s'enregistrent et envoient des heartbeats ne sont pas pris en compte par le kernel, créant un désynchronisation complète entre l'état réel du réseau et l'état affiché dans le dashboard.

**Impact**:
- Agents ne remontent pas leurs métriques (CPU, RAM, uptime = null)
- Agent Windows éteint toujours affiché comme "online" malgré 3h d'inactivité
- Agent Linux local enregistré mais marqué "offline" avec anciennes données
- Dashboard PWA affiche des données obsolètes et incorrectes
- Monitoring système inutilisable

**Cause racine**: `mark_mqtt_connected()` jamais appelé dans `mqtt.rs:spawn_mqtt_listener()`

---

## 🔍 Chronologie de l'Incident

### T+0 - Détection Initiale
```
User: "bon tu te souviens du probleme d'hier avec fzil to fetch toussa,
c'est toujour d'zctualite malgre le fix et je remarque aussi que l'agent
local ne remonte pas non plus, xomme hier windows c'est bie eteint malgre
tout mais des soucis sur les etat etc"
```

**Symptômes observés**:
- "fail to fetch" dans dashboard (problème d'hier avec API key manquante - résolu)
- Agent local ne remonte pas
- Agent Windows éteint mais affiché comme online
- États agents incorrects

### T+5min - Diagnostic API
```bash
$ curl http://localhost:8080/agents
# Timeout - aucune réponse

$ curl http://localhost:8080/agents -H "x-api-key: s3cr3t-42"
[{"agent_id":"345a604068a8","hostname":"DESKTOP-3BT760L","os":"windows",
  "status":"online","last_seen":"2025-10-15T16:59:32Z",
  "uptime_seconds":null,"cpu_percent":null,"memory_percent":null},
 {"agent_id":"7070fc0481d8","hostname":"eridwyn-Salon","os":"linux",
  "status":"offline","last_seen":"2025-10-15T06:29:20Z",
  "uptime_seconds":215187,"cpu_percent":4.927536,"memory_percent":8.388969}]
```

**Observations**:
- ✅ API fonctionne avec header x-api-key (fix hier fonctionnel)
- ❌ Agent Windows "online" avec métriques null (anormal)
- ❌ `last_seen` == `registration_time` → Aucun heartbeat reçu
- ❌ Agent Linux "offline" avec données anciennes (06h29 ce matin)

### T+10min - Diagnostic MQTT
```bash
$ curl -s http://localhost:8080/system/health -H "x-api-key: s3cr3t-42"
{
  "mqtt_status": "connecting",  # <-- PROBLÈME ICI
  "mqtt_reconnects": 0,
  "mqtt_messages_total": 1145,
  "agents_count": 2
}

$ mosquitto_sub -t "symbion/agents/#" -v -C 5
# (12 secondes d'attente)
# Aucun message reçu
```

**Observations**:
- ❌ MQTT status = "connecting" au lieu de "connected"
- ✅ 1145 messages MQTT traités (donc connexion fonctionne)
- ❌ Aucun message MQTT reçu (agents n'envoient pas de heartbeat)

### T+15min - Test Agent Local
```bash
$ RUST_LOG=debug cargo run --release -p symbion-agent-host
[INFO] Agent registered successfully
[INFO] Subscribed to commands on: symbion/agents/command@v1
[INFO] Detected 5 capabilities: ["power_management", ...]

$ curl http://localhost:8080/agents -H "x-api-key: s3cr3t-42"
# Agent Linux toujours "offline" avec anciennes données (06:29:20)
```

**Observations**:
- ✅ Agent local démarre et s'enregistre via MQTT
- ❌ Kernel ne met PAS à jour l'agent dans sa base
- ❌ `last_seen` reste à 06:29:20 (pas de mise à jour)

---

## 🔬 Analyse Technique

### Code Problem: `symbion-kernel/src/mqtt.rs:44`

```rust
pub fn spawn_mqtt_listener(..., health_tracker: Option<HealthTracker>) {
    task::spawn(async move {
        let (client, mut eventloop) = AsyncClient::new(opts, 10);

        // Subscriptions MQTT
        client.subscribe("symbion/agents/registration@v1", QoS::AtLeastOnce).await;
        client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await;

        // ❌ MANQUE ICI : health_tracker.mark_mqtt_connected();

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Publish(p))) => {
                    tracker.record_mqtt_message(); // ✅ Enregistre les messages
                    // Traite registration, heartbeat, etc.
                }
            }
        }
    });
}
```

**Problème** : `mark_mqtt_connected()` n'est JAMAIS appelé après connexion réussie

**Conséquence** :
- Health status reste figé à "connecting" (valeur par défaut)
- Donne une fausse impression que MQTT ne fonctionne pas
- Peut affecter logique métier qui vérifie le statut MQTT

### État du Health Tracker

```rust
// symbion-kernel/src/health.rs:91
impl HealthTracker {
    pub fn new() -> Self {
        Self {
            mqtt_status: Arc::new(Mutex::new("connecting".to_string())), // Valeur initiale
            // ...
        }
    }

    pub fn mark_mqtt_connected(&self) {
        *self.mqtt_status.lock() = "connected".to_string();
    }
}
```

Le status "connecting" est la valeur par défaut et n'est jamais changé.

### Problème Secondaire: Agent Windows

```json
{
  "agent_id": "345a604068a8",
  "status": "online",
  "last_seen": "2025-10-15T16:59:32Z",
  "registration_time": "2025-10-15T16:59:32Z",
  "uptime_seconds": null,
  "cpu_percent": null,
  "memory_percent": null
}
```

**Analyse**:
- `last_seen` == `registration_time` → Agent enregistré mais aucun heartbeat
- Métriques à `null` → Confirme aucun heartbeat reçu
- User confirme : "l'agent windows tourne plus le pc est eteint"
- Agent devrait être marqué "offline" après 90s timeout

**Agent monitoring fonctionne mal** :
- L'agent s'éteint à ~17h00
- À 19h15 (2h15 plus tard), toujours marqué "online"
- Timeout configuré : 90 secondes (1.5 min) dans monitoring

Code monitoring : `symbion-kernel/src/agents.rs:585`

```rust
pub fn start_agent_monitoring(registry: SharedAgentRegistry, timeout_minutes: i64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check toutes les minutes

        loop {
            interval.tick().await;

            // Identifier agents timeout
            let timeout_threshold = now - Duration::minutes(timeout_minutes);
            for (agent_id, agent) in agents_map.iter() {
                if agent.status == "online" && agent.last_seen < timeout_threshold {
                    agents_to_mark_offline.push(agent_id.clone());
                }
            }

            // Marquer offline
            for agent_id in agents_to_mark_offline {
                registry.mark_agent_offline(&agent_id).await;
            }
        }
    });
}
```

**Question** : Monitoring est-il démarré ? Timeout correctement configuré ?

---

## 🎯 Tests de Validation

### Test 1: Connexion MQTT baseline
```bash
$ mosquitto_sub -t "#" -v -C 1
# Devrait afficher messages système
```

**Résultat**: ✅ Mosquitto fonctionne

### Test 2: Agent envoie-t-il des heartbeats ?
```bash
$ mosquitto_sub -t "symbion/agents/heartbeat@v1" -v
# Attendre 30s (cycle heartbeat)
```

**Résultat**: ❌ Aucun message reçu

**Hypothèse** : Agent ne démarre PAS le heartbeat automatique après registration

### Test 3: Kernel reçoit-il les registrations ?
```bash
$ tail -f /path/to/kernel/logs | grep -i registration
# Lancer agent dans autre terminal
```

**Résultat**: À tester (kernel n'a pas de logs de debug activés actuellement)

---

## 🛠️ Plan de Résolution

### Fix 1: Ajouter `mark_mqtt_connected()` après subscriptions

**Fichier**: `symbion-kernel/src/mqtt.rs:79`

```rust
// S'abonner aux événements agents
if agents.is_some() {
    client.subscribe("symbion/agents/registration@v1", QoS::AtLeastOnce).await?;
    client.subscribe("symbion/agents/heartbeat@v1", QoS::AtLeastOnce).await?;
    client.subscribe("symbion/agents/response@v1", QoS::AtLeastOnce).await?;
}

// ✅ AJOUTER ICI
if let Some(ref tracker) = health_tracker {
    tracker.mark_mqtt_connected();
    println!("[kernel] MQTT connected successfully");
}

loop {
    // Eventloop polling...
}
```

**Test après fix**:
```bash
$ curl -s http://localhost:8080/system/health -H "x-api-key: s3cr3t-42" | jq .mqtt_status
"connected"  # ✅ Should now show "connected"
```

### Fix 2: Vérifier agent heartbeat timer

**Fichier**: `symbion-agent-host/src/main.rs`

Vérifier que le heartbeat est bien envoyé périodiquement après registration.

Chercher :
```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        agent.send_heartbeat().await;
    }
});
```

Si absent → AJOUTER

### Fix 3: Redémarrer kernel après changements

```bash
$ pkill symbion-kernel
$ SYMBION_API_KEY="s3cr3t-42" SYMBION_MQTT_BROKER="127.0.0.1:1883" cargo run --release
```

### Fix 4: Vérifier agent monitoring démarré

**Fichier**: `symbion-kernel/src/main.rs` ou point d'entrée

Vérifier présence de :
```rust
AgentRegistry::start_agent_monitoring(agent_registry.clone(), 2); // 2 min timeout
```

Si absent → AJOUTER dans `main()`

---

## 📊 Métriques d'Impact

**Avant incident**:
- Agents online: 2/2
- MQTT messages/min: ~4 (registration + heartbeats toutes les 30s)
- Dashboard utilisable: ✅

**Pendant incident**:
- Agents synchronisés: 0/2
- MQTT status: "connecting" (faux)
- Dashboard utilisable: ❌ (données obsolètes)

**Après résolution attendue**:
- Agents synchronisés: 2/2 (ou 1/2 si Windows éteint marqué offline)
- MQTT status: "connected"
- Dashboard utilisable: ✅

---

## 📝 Notes de Contexte

### Incident d'hier (2025-10-14)
- **Problème** : Deadlock sur RwLock agents + timeout HTTP
- **Fix** : Snapshot pattern + TimeoutLayer 30s + optimisation scopes locks
- **Lien** : `/incidents/resolu/INCIDENT-2025-10-14-kernel-deadlock.md`

Possible que le fix d'hier ait introduit une régression sur MQTT status update ?

### Configuration Actuelle
```bash
SYMBION_API_KEY="s3cr3t-42"
SYMBION_MQTT_BROKER="127.0.0.1:1883"
```

Kernel uptime : 76414 secondes (~21 heures) → Pas redémarré depuis le fix d'hier

### Agents Identifiés
1. **345a604068a8** (DESKTOP-3BT760L) - Windows, éteint depuis ~17h00
2. **7070fc0481d8** (eridwyn-Salon) - Linux, cette machine, agent démarré à 19h13

---

## ⏭️ Prochaines Étapes

1. ✅ **Documenter l'incident** (ce fichier)
2. ⏳ **Appliquer Fix 1** : mark_mqtt_connected() après subscriptions
3. ⏳ **Vérifier Fix 2** : Agent heartbeat loop existe
4. ⏳ **Vérifier Fix 3** : Agent monitoring démarré dans kernel
5. ⏳ **Redémarrer kernel** avec corrections
6. ⏳ **Valider agents remontent** : curl /agents montre métriques
7. ⏳ **Tester timeout offline** : Éteindre agent, vérifier marqué offline après 2min
8. ⏳ **Déplacer vers `/resolu`** si validations OK

---

## 🔗 Fichiers Impactés

- `symbion-kernel/src/mqtt.rs` - mark_mqtt_connected() manquant (ligne 79)
- `symbion-kernel/src/agents.rs` - Monitoring agents timeout
- `symbion-agent-host/src/main.rs` - Heartbeat loop à vérifier

## 🏷️ Tags

`mqtt` `agents` `health-check` `monitoring` `heartbeat` `synchronization` `bug`
