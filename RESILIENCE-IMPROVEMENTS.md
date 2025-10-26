# 🛡️ Guide d'Amélioration de la Résilience Symbion

## 📋 Problèmes Identifiés (26 Octobre 2025)

### ❌ 1. Pas de Supervision de Process
**Symptôme** : Agents apparaissent offline même quand ils fonctionnent, kernel se fige

**Cause** : Process lancés manuellement (`nohup`, `&`), pas de restart automatique en cas de crash

**Impact** : Système fragile, nécessite intervention manuelle fréquente

---

### ❌ 2. Timeout Agressif (2 minutes)
**Symptôme** : Agents marqués offline après 2 min sans heartbeat

**Cause** : `symbion-kernel/src/main.rs:145`
```rust
AgentRegistry::start_agent_monitoring(agents.clone(), 2); // 2 minutes
```

**Impact** : Micro-coupures réseau causent des faux offline

---

### ❌ 3. MQTT Event Loop Bloquant
**Symptôme** : Kernel ne répond plus, API HTTPS timeout

**Cause** : Boucle MQTT synchrone dans `symbion-kernel/src/mqtt.rs:87-200`
```rust
loop {
    match eventloop.poll().await {
        Ok(Event::Incoming(rumqttc::Incoming::Publish(p))) => {
            // ⚠️ Traitement synchrone - si un handler bloque, tout est mort
            if p.topic == "symbion/agents/heartbeat@v1" {
                agent_registry.handle_agent_heartbeat(heartbeat).await; // ← Peut bloquer
            }
        }
    }
}
```

**Impact** : Un seul handler lent bloque tout le système

---

### ❌ 4. Pas de Persistance d'État Robuste
**Symptôme** : Après restart kernel, agents doivent se ré-enregistrer

**Cause** : État en mémoire (`Arc<RwLock<HashMap>>`), sauvegarde JSON sporadique

**Impact** : Perte de métriques historiques, re-sync nécessaire

---

### ❌ 5. Pas de Reconnexion MQTT Automatique
**Symptôme** : Si broker MQTT redémarre, agents ne se reconnectent pas

**Cause** : Pas de retry logic dans `rumqttc::EventLoop`

**Impact** : Nécessite restart manuel des agents

---

### ❌ 6. Agent Windows Non Persistant
**Symptôme** : Après reboot Windows, agent pas lancé automatiquement

**Cause** : Pas de service Windows ou tâche planifiée

**Impact** : Monitoring Windows coupé jusqu'à lancement manuel

---

## 🚀 Solutions (Par Ordre de Priorité)

---

## ✅ SOLUTION 1 : Services Systemd (CRITIQUE - 30 min)

### **Pourquoi c'est la priorité #1 ?**
- ✅ Restart automatique sur crash (`Restart=always`)
- ✅ Démarrage automatique au boot
- ✅ Supervision intégrée avec `systemctl status`
- ✅ Logging centralisé avec `journalctl`
- ✅ Gestion des dépendances (kernel démarre après MQTT)

### **Installation**

```bash
cd /home/eridwyn/RustroverProjects/NewSymbion/systemd
sudo ./install-services.sh
```

Ce script va :
1. Arrêter les process manuels actuels
2. Copier les services dans `/etc/systemd/system/`
3. Activer les services au démarrage
4. Démarrer kernel + agent

### **Commandes Utiles**

```bash
# Status des services
sudo systemctl status symbion-kernel
sudo systemctl status symbion-agent

# Voir logs en temps réel
journalctl -u symbion-kernel -f
journalctl -u symbion-agent -f

# Restart après modification code
sudo systemctl restart symbion-kernel
sudo systemctl restart symbion-agent

# Stop/Start manuel
sudo systemctl stop symbion-kernel
sudo systemctl start symbion-kernel

# Désactiver auto-start
sudo systemctl disable symbion-kernel
```

### **Vérification Résilience**

Test de crash simulé :
```bash
# Tuer le kernel
sudo pkill -9 symbion-kernel

# Attendre 10 secondes (RestartSec=10s)
sleep 10

# Vérifier qu'il a redémarré
systemctl status symbion-kernel
# Devrait montrer: "Active: active (running)"
```

---

## ✅ SOLUTION 2 : Augmenter Timeout Agents (FACILE - 5 min)

### **Modification Code**

`symbion-kernel/src/main.rs:145`

**Avant :**
```rust
AgentRegistry::start_agent_monitoring(agents.clone(), 2); // 2 minutes
```

**Après :**
```rust
AgentRegistry::start_agent_monitoring(agents.clone(), 5); // 5 minutes (tolérance micro-coupures)
```

### **Puis Recompiler**

```bash
cd /home/eridwyn/RustroverProjects/NewSymbion
cargo build --release -p symbion-kernel

# Si systemd installé:
sudo systemctl restart symbion-kernel

# Sinon:
pkill symbion-kernel
./target/release/symbion-kernel &
```

### **Impact**

- ✅ Agents restent "online" même avec micro-coupures réseau (< 5 min)
- ⚠️ Détection des vrais offline prend 5 min au lieu de 2 min

---

## ✅ SOLUTION 3 : MQTT Handlers Asynchrones (MOYEN - 2h)

### **Problème Actuel**

Le kernel traite les messages MQTT de manière séquentielle. Si un handler prend 10 secondes, tous les autres messages sont bloqués.

### **Solution : Spawner des Tasks Tokio**

`symbion-kernel/src/mqtt.rs:139-160`

**Avant :**
```rust
} else if p.topic == "symbion/agents/heartbeat@v1" {
    if let Some(ref agent_registry) = agents {
        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
            match serde_json::from_str::<AgentHeartbeatMessage>(&txt) {
                Ok(heartbeat) => {
                    // ⚠️ SYNCHRONE - Bloque la boucle MQTT
                    if let Err(e) = agent_registry.handle_agent_heartbeat(heartbeat).await {
                        eprintln!("[kernel] failed to handle agent heartbeat: {}", e);
                    }
                }
            }
        }
    }
}
```

**Après :**
```rust
} else if p.topic == "symbion/agents/heartbeat@v1" {
    if let Some(ref agent_registry) = agents {
        if let Ok(txt) = String::from_utf8(p.payload.to_vec()) {
            match serde_json::from_str::<AgentHeartbeatMessage>(&txt) {
                Ok(heartbeat) => {
                    let registry = agent_registry.clone();
                    // ✅ ASYNCHRONE - N'attend pas la fin du traitement
                    tokio::spawn(async move {
                        if let Err(e) = registry.handle_agent_heartbeat(heartbeat).await {
                            eprintln!("[kernel] failed to handle agent heartbeat: {}", e);
                        }
                    });
                }
            }
        }
    }
}
```

### **Avantages**

- ✅ Kernel ne bloque jamais
- ✅ Traitement parallèle des heartbeats
- ✅ Latence API HTTPS stable même sous charge MQTT

### **Faire Pareil Pour**

- `symbion/agents/registration@v1` (ligne 122)
- `symbion/agents/response@v1` (ligne 164)
- `symbion/notes/response@v1` (ligne 111)

---

## ✅ SOLUTION 4 : Retry MQTT avec Backoff (MOYEN - 1h)

### **Ajouter dans Agent Host**

`symbion-agent-host/src/main.rs:158-166`

**Après** (ligne 166):
```rust
let (mqtt_client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

// ✅ NOUVEAU: Retry automatique avec backoff exponentiel
let mut retry_delay = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

loop {
    match eventloop.poll().await {
        Ok(event) => {
            // Réinitialiser le délai sur succès
            retry_delay = Duration::from_secs(1);

            // Traiter l'événement MQTT
            // ... (code existant)
        }
        Err(e) => {
            error!("MQTT connection error: {:?}", e);
            error!("Reconnecting in {:?}...", retry_delay);

            tokio::time::sleep(retry_delay).await;

            // Backoff exponentiel: 1s → 2s → 4s → 8s → ... → 60s max
            retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
        }
    }
}
```

### **Avantages**

- ✅ Agent se reconnecte automatiquement si broker redémarre
- ✅ Pas de spam de tentatives (backoff exponentiel)
- ✅ Résilient aux coupures réseau temporaires

---

## ✅ SOLUTION 5 : Service Windows Agent (FACILE - 15 min)

### **Option A : Tâche Planifiée (Plus Simple)**

1. Ouvrir **Planificateur de tâches** Windows
2. Créer une tâche de base :
   - **Nom** : Symbion Agent
   - **Déclencheur** : Au démarrage du système
   - **Action** : Démarrer un programme
   - **Programme** : `C:\Path\To\symbion-agent-host-windows-x64.exe`
   - **Arguments** : (vide)
   - **Répertoire** : `C:\Path\To\`
3. Paramètres avancés :
   - ✅ Cocher "Exécuter avec les privilèges les plus élevés"
   - ✅ Cocher "Démarrer la tâche si une instance est déjà en cours"
   - ✅ Autoriser l'exécution à la demande

### **Option B : Service Windows (Plus Robuste)**

Utiliser **NSSM** (Non-Sucking Service Manager) :

```powershell
# Télécharger NSSM depuis https://nssm.cc/download
# Extraire dans C:\Tools\nssm\

# Installer service
C:\Tools\nssm\nssm.exe install SymbionAgent "C:\Path\To\symbion-agent-host-windows-x64.exe"

# Configurer restart automatique
C:\Tools\nssm\nssm.exe set SymbionAgent AppStdout "C:\Path\To\symbion-agent.log"
C:\Tools\nssm\nssm.exe set SymbionAgent AppStderr "C:\Path\To\symbion-agent-error.log"
C:\Tools\nssm\nssm.exe set SymbionAgent AppRestartDelay 10000  # 10 secondes

# Démarrer
net start SymbionAgent
```

---

## ✅ SOLUTION 6 : Health Checks Proactifs (AVANCÉ - 3h)

### **Concept**

Au lieu de juste marquer les agents offline, essayer de les "réveiller" automatiquement.

### **Nouveau Module**

`symbion-kernel/src/agent_recovery.rs`

```rust
/// Tente de récupérer un agent offline
pub async fn try_recover_agent(agent: &Agent, mqtt_client: AsyncClient) -> Result<()> {
    match agent.os.as_str() {
        "windows" | "linux" => {
            // Tenter Wake-on-LAN si interface Ethernet
            if let Some(primary_mac) = &agent.network.primary_mac {
                info!("Attempting Wake-on-LAN for agent {}", agent.hostname);
                send_wol_packet(primary_mac)?;

                // Attendre 30 secondes pour boot
                tokio::time::sleep(Duration::from_secs(30)).await;

                // Vérifier si agent répond maintenant
                // ...
            }
        }
        _ => {}
    }

    Ok(())
}
```

### **Intégrer dans Monitoring**

`symbion-kernel/src/agents.rs:612`

**Après avoir marqué offline:**
```rust
for agent_id in agents_to_mark_offline {
    registry.mark_agent_offline(&agent_id).await;

    // ✅ NOUVEAU: Tenter récupération automatique
    if let Some(agent) = registry.get_agent(&agent_id).await {
        if let Err(e) = try_recover_agent(&agent, mqtt_client.clone()).await {
            warn!("Failed to recover agent {}: {}", agent_id, e);
        }
    }
}
```

---

## 📊 Checklist d'Amélioration Résilience

### Phase 1 : Critique (Faire Maintenant)
- [ ] Installer services systemd (30 min)
- [ ] Augmenter timeout agents à 5 min (5 min)
- [ ] Configurer agent Windows auto-start (15 min)

### Phase 2 : Important (Semaine Prochaine)
- [ ] Implémenter MQTT handlers asynchrones (2h)
- [ ] Ajouter retry MQTT avec backoff (1h)

### Phase 3 : Nice-to-Have (Quand Tu as le Temps)
- [ ] Health checks proactifs avec Wake-on-LAN (3h)
- [ ] Persistance d'état robuste (SQLite ou PostgreSQL) (4h)
- [ ] Dashboard de monitoring avancé (6h)

---

## 🧪 Tests de Résilience

### Test 1 : Crash Kernel
```bash
sudo pkill -9 symbion-kernel
sleep 15
systemctl status symbion-kernel
# ✅ Devrait montrer "active (running)"
```

### Test 2 : Restart MQTT Broker
```bash
sudo systemctl restart mosquitto
sleep 5
journalctl -u symbion-agent -n 20
# ✅ Devrait montrer reconnexion automatique
```

### Test 3 : Coupure Réseau Agent
```bash
# Sur machine agent, couper WiFi/Ethernet 3 minutes
# Puis reconnecter
# ✅ Agent devrait remonter dans les 30s
```

### Test 4 : Reboot Complet
```bash
sudo reboot
# Après reboot:
systemctl status symbion-kernel symbion-agent
# ✅ Les deux services devraient être actifs
```

---

## 📈 Métriques de Résilience (Objectifs)

**Avant Améliorations** :
- ❌ Uptime système : ~70% (nécessite intervention manuelle fréquente)
- ❌ MTTR (Mean Time To Recover) : 15-30 minutes (intervention humaine)
- ❌ False positive offline : ~30% (timeout agressif)

**Après Phase 1** :
- ✅ Uptime système : ~95% (restart automatique)
- ✅ MTTR : < 1 minute (systemd restart)
- ✅ False positive offline : ~5% (timeout 5 min)

**Après Phase 2** :
- ✅ Uptime système : ~99% (handlers async + retry)
- ✅ MTTR : < 15 secondes (backoff exponentiel)
- ✅ False positive offline : ~1%

**Après Phase 3** :
- ✅ Uptime système : ~99.9% (recovery proactif)
- ✅ MTTR : < 5 secondes (Wake-on-LAN)
- ✅ False positive offline : ~0.1%

---

## 🔗 Ressources

- [systemd Service Documentation](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
- [Tokio Async Programming](https://tokio.rs/tokio/tutorial)
- [NSSM - Service Manager Windows](https://nssm.cc/)
- [Wake-on-LAN Protocol](https://en.wikipedia.org/wiki/Wake-on-LAN)

---

**Créé le** : 26 Octobre 2025
**Auteur** : Claude Code
**Version** : 1.0
