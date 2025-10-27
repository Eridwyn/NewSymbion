From: Symbion Architecture <noreply@symbion.local>
To: Mark <Markchavatte@gmail.com>
Subject: [Symbion] P0 ROADMAP FINALE - 6 PRs + Exigences PR6 Verrouillées
Content-Type: text/plain; charset=UTF-8

Salut Mark,

Validation reçue pour PR6. Voici la roadmap P0 finale verrouillée avec exigences techniques détaillées.

═══════════════════════════════════════════════════════════
🎯 ROADMAP P0 FINALE - 6 PRs SÉQUENTIELLES
═══════════════════════════════════════════════════════════

PR1 : context/timezone+hysteresis → v0.2.0-alpha.1
──────────────────────────────────────────────────
• IANA timezone Europe/Zurich
• Instant monotone pour hystérésis
• Logs structurés start/commit/reset
• MQTT retain=true sur context/mode
• Tests frontières horaires

PR2 : api/v1+auth+MFA+nonce → v0.2.0-alpha.2
────────────────────────────────────────────
• Endpoints /v1/* versionnés
• Auth JWT + MFA verification
• CSRF nonce sur validation
• Rate limiting Tower
• Tests auth + rate-limit

PR3 : decision/guards-first+weights → v0.2.0-beta.1
───────────────────────────────────────────────────
• Guards évalués avant trust_score
• Matrice context_match 30%
• MFA bonus +0.10 clampé
• Idempotence command_id (intégré ici)
• Traits plugins figés
• Tests guards + idempotence

PR4 : observability-min → v0.2.1
────────────────────────────────
• GET /v1/health avec composants
• Métriques Prometheus-style
• Propagation trace_id bout en bout
• Tests health + trace_id

PR5 : fail-safe → v0.2.2
────────────────────────
• Decision Engine catch_unwind
• Mode degraded si panic
• Politique fail-safe RequireValidation
• Tests panic + degraded

PR6 : intentions/lifecycle → v0.2.3  ✅ AJOUT CRITIQUE
──────────────────────────────────────────────────
• Agent online check avant exécution
• Timeout monitor + cleanup auto
• Détection conflits intentions simultanées
• Notifications dashboard MQTT
• Persistence intentions.json + graceful shutdown
• Tests lifecycle complets (9 tests)

TAG FINAL P0 : v0.2.3 = PRODUCTION-READY

═══════════════════════════════════════════════════════════
🔒 EXIGENCES PR6 VERROUILLÉES
═══════════════════════════════════════════════════════════

1️⃣ CHECK AGENT AVANT EXÉCUTION
───────────────────────────────

RÈGLES :
• Refuser si agent status != "online"
• Refuser si last_heartbeat > threshold (configurable, défaut 60s)
• Log clair avec agent_id + last_heartbeat timestamp
• Code erreur déterministe

IMPLÉMENTATION :

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Agent {agent_id} is offline (status: {status})")]
    AgentOffline {
        agent_id: String,
        status: String,
    },
    
    #[error("Agent {agent_id} heartbeat stale ({elapsed_secs}s > {threshold_secs}s)")]
    AgentStale {
        agent_id: String,
        elapsed_secs: i64,
        threshold_secs: u64,
    },
}

async fn execute_intention(
    intention: &Intention,
    agents: &AgentRegistry,
    config: &IntentionsConfig,
) -> Result<String, ExecutionError> {
    // 1. Récupérer agent
    let agent = agents.get(&intention.agent_id).await
        .ok_or_else(|| ExecutionError::AgentOffline {
            agent_id: intention.agent_id.clone(),
            status: "not_found".to_string(),
        })?;
    
    // 2. Vérifier status online
    if agent.status != AgentStatus::Online {
        return Err(ExecutionError::AgentOffline {
            agent_id: intention.agent_id.clone(),
            status: format!("{:?}", agent.status),
        });
    }
    
    // 3. Vérifier heartbeat freshness
    let elapsed = (OffsetDateTime::now_utc() - agent.last_heartbeat).whole_seconds();
    if elapsed > config.offline_heartbeat_max_secs as i64 {
        return Err(ExecutionError::AgentStale {
            agent_id: intention.agent_id.clone(),
            elapsed_secs: elapsed,
            threshold_secs: config.offline_heartbeat_max_secs,
        });
    }
    
    // 4. Exécuter
    println!("[exec] Executing intention {} on agent {} (heartbeat {}s ago)",
        intention.id, intention.agent_id, elapsed);
    
    let command_id = agents.send_command(&intention.action).await?;
    
    Ok(command_id)
}
```

═══════════════════════════════════════════════════════════
2️⃣ TIMEOUT & CLEANUP
─────────────────────

RÈGLES :
• Tâche background tick 30s
• Expire intentions > expires_at
• Retrait atomique registry + persist
• Événement dashboard « intention.expired »

IMPLÉMENTATION :

```rust
pub fn spawn_intention_timeout_monitor(
    intentions: Arc<IntentionRegistry>,
    dashboard_events: DashboardEventPublisher,
    config: IntentionsConfig,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            Duration::from_secs(config.cleanup_interval_secs)
        );
        
        loop {
            interval.tick().await;
            
            // Récupérer intentions expirées
            let expired = intentions.get_expired().await;
            
            if expired.is_empty() {
                continue;
            }
            
            println!("[intentions] Processing {} expired intentions", expired.len());
            
            for intention in expired {
                // Retrait atomique
                match intentions.remove(&intention.id).await {
                    Ok(Some(_)) => {
                        println!("[intentions] Expired and removed: {} (action: {:?}, age: {}s)",
                            intention.id,
                            intention.action,
                            (OffsetDateTime::now_utc() - intention.created_at).whole_seconds()
                        );
                        
                        // Notifier dashboard
                        if let Err(e) = dashboard_events.publish_intention_expired(&intention).await {
                            eprintln!("[intentions] Failed to notify expiration: {}", e);
                        }
                        
                        // Metric
                        metrics::INTENTIONS_EXPIRED_TOTAL.inc();
                    }
                    Ok(None) => {
                        eprintln!("[intentions] Expired but already removed: {}", intention.id);
                    }
                    Err(e) => {
                        eprintln!("[intentions] Failed to remove expired {}: {}", intention.id, e);
                    }
                }
            }
        }
    });
    
    println!("[intentions] Timeout monitor started (cleanup every {}s)",
        config.cleanup_interval_secs);
}
```

═══════════════════════════════════════════════════════════
3️⃣ DÉTECTION DE CONFLITS
─────────────────────────

RÈGLES :
• Matrice classique : Shutdown/Reboot/Hibernate conflictuels sur MÊME agent
• POST /v1/intentions retourne 409 CONFLICT avec détails
• Option P1 : proposer RequireValidation pour annuler/remplacer

IMPLÉMENTATION :

```rust
impl Action {
    /// Vérifie si action conflictuelle avec autre action
    pub fn conflicts_with(&self, other: &Action) -> bool {
        // Extraire agent_id
        let self_agent = self.target_agent_id();
        let other_agent = other.target_agent_id();
        
        // Agents différents = pas de conflit
        if self_agent != other_agent {
            return false;
        }
        
        // Matrice conflits sur MÊME agent
        matches!(
            (self, other),
            (Action::Shutdown { .. }, Action::Reboot { .. }) |
            (Action::Reboot { .. }, Action::Shutdown { .. }) |
            (Action::Shutdown { .. }, Action::Hibernate { .. }) |
            (Action::Hibernate { .. }, Action::Shutdown { .. }) |
            (Action::Reboot { .. }, Action::Hibernate { .. }) |
            (Action::Hibernate { .. }, Action::Reboot { .. })
        )
    }
    
    pub fn target_agent_id(&self) -> &str {
        match self {
            Action::Shutdown { agent_id, .. } => agent_id,
            Action::Reboot { agent_id, .. } => agent_id,
            Action::Hibernate { agent_id } => agent_id,
            Action::SuggestContextChange { .. } => "",  // Pas de target agent
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConflictInfo {
    pub existing_id: String,
    pub existing_action: String,
    pub created_at: String,
    pub expires_at: String,
}

impl DecisionEngine {
    pub async fn detect_conflicts(&self, new_intention: &Intention) -> Vec<ConflictInfo> {
        let pending = self.intentions.get_pending().await;
        
        pending.iter()
            .filter(|existing| existing.action.conflicts_with(&new_intention.action))
            .map(|existing| ConflictInfo {
                existing_id: existing.id.clone(),
                existing_action: format!("{:?}", existing.action),
                created_at: existing.created_at.to_string(),
                expires_at: existing.expires_at.to_string(),
            })
            .collect()
    }
}
```

ENDPOINT :

```rust
// POST /v1/intentions
async fn create_intention_handler(
    State(app): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(req): Json<CreateIntentionRequest>,
) -> Result<Json<Intention>, (StatusCode, Json<ErrorResponse>)> {
    // Verify auth
    let claims = app.auth_manager.verify_token(auth.token())
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "Invalid token".to_string(),
        })))?;
    
    // Créer intention
    let intention = Intention::new(req.action, claims.sub);
    
    // Détecter conflits
    let conflicts = app.decision_engine.detect_conflicts(&intention).await;
    
    if !conflicts.is_empty() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("Conflicting intentions found: {:?}", conflicts),
            })
        ));
    }
    
    // Décider
    let decision = app.decision_engine.decide(intention.clone()).await;
    
    // Ajouter à registry si RequireValidation
    if matches!(decision, DecisionResult::RequireValidation { .. }) {
        app.intentions.add(intention.clone()).await?;
        
        // Notifier dashboard
        app.dashboard_events.publish_intention_pending(&intention).await?;
    }
    
    Ok(Json(intention))
}
```

═══════════════════════════════════════════════════════════
4️⃣ NOTIFICATIONS DASHBOARD
───────────────────────────

RÈGLES :
• Événements MQTT (QoS 1, retain=false)
• Topics :
  - symbion/v1/dashboard/intention_pending
  - symbion/v1/dashboard/intention_expired
  - symbion/v1/dashboard/intention_validated
• Pas de payload sensible (pas de commande shell)
• Action format lisible
• Inclure trace_id + expires_at

IMPLÉMENTATION :

```rust
impl DashboardEventPublisher {
    pub async fn publish_intention_pending(&self, intention: &Intention) -> Result<()> {
        let payload = serde_json::json!({
            "version": "1.0",
            "type": "intention.pending_validation",
            "timestamp": OffsetDateTime::now_utc().format(&Rfc3339)?,
            "trace_id": format!("evt-{}", intention.id),
            "data": {
                "intention_id": intention.id,
                "action_type": intention.action.action_type(),  // "Shutdown", "Reboot", etc.
                "agent_id": intention.action.target_agent_id(),
                "impact_level": format!("{:?}", intention.impact_level),
                "trust_score": format!("{:.2}", intention.trust_score),
                "validation_url": format!("/v1/intentions/{}/validate", intention.id),
                "expires_at": intention.expires_at.format(&Rfc3339)?,
                "reason": intention.reason,
            }
        });
        
        self.mqtt_client.publish(
            "symbion/v1/dashboard/intention_pending",
            rumqttc::QoS::AtLeastOnce,
            false,  // retain = false
            serde_json::to_string(&payload)?,
        ).await?;
        
        metrics::DASHBOARD_EVENTS_PUBLISHED_TOTAL
            .with_label_values(&["intention_pending"])
            .inc();
        
        Ok(())
    }
    
    pub async fn publish_intention_expired(&self, intention: &Intention) -> Result<()> {
        let payload = serde_json::json!({
            "version": "1.0",
            "type": "intention.expired",
            "timestamp": OffsetDateTime::now_utc().format(&Rfc3339)?,
            "trace_id": format!("evt-{}", intention.id),
            "data": {
                "intention_id": intention.id,
                "action_type": intention.action.action_type(),
                "agent_id": intention.action.target_agent_id(),
                "created_at": intention.created_at.format(&Rfc3339)?,
                "expired_at": intention.expires_at.format(&Rfc3339)?,
            }
        });
        
        self.mqtt_client.publish(
            "symbion/v1/dashboard/intention_expired",
            rumqttc::QoS::AtLeastOnce,
            false,
            serde_json::to_string(&payload)?,
        ).await?;
        
        metrics::DASHBOARD_EVENTS_PUBLISHED_TOTAL
            .with_label_values(&["intention_expired"])
            .inc();
        
        Ok(())
    }
    
    pub async fn publish_intention_validated(&self, intention: &Intention, approved: bool) -> Result<()> {
        let payload = serde_json::json!({
            "version": "1.0",
            "type": "intention.validated",
            "timestamp": OffsetDateTime::now_utc().format(&Rfc3339)?,
            "trace_id": format!("evt-{}", intention.id),
            "data": {
                "intention_id": intention.id,
                "approved": approved,
                "action_type": intention.action.action_type(),
                "agent_id": intention.action.target_agent_id(),
            }
        });
        
        self.mqtt_client.publish(
            "symbion/v1/dashboard/intention_validated",
            rumqttc::QoS::AtLeastOnce,
            false,
            serde_json::to_string(&payload)?,
        ).await?;
        
        metrics::DASHBOARD_EVENTS_PUBLISHED_TOTAL
            .with_label_values(&["intention_validated"])
            .inc();
        
        Ok(())
    }
}

impl Action {
    /// Retourne type action lisible (pas de détails commande)
    pub fn action_type(&self) -> &'static str {
        match self {
            Action::Shutdown { .. } => "Shutdown",
            Action::Reboot { .. } => "Reboot",
            Action::Hibernate { .. } => "Hibernate",
            Action::SuggestContextChange { .. } => "SuggestContextChange",
        }
    }
}
```

═══════════════════════════════════════════════════════════
5️⃣ PERSISTENCE + GRACEFUL SHUTDOWN
───────────────────────────────────

RÈGLES :
• intentions.json avec écriture ATOMIQUE (write temp + fsync + rename)
• Chargement au boot + purge immédiate intentions déjà expirées
• Handler SIGINT/SIGTERM (Windows CTRL_C_EVENT)
• Metric « intentions_restored_total »

IMPLÉMENTATION :

```rust
impl IntentionRegistry {
    /// Sauvegarde ATOMIQUE
    pub async fn save(&self) -> Result<()> {
        let pending = self.pending.read();
        let json = serde_json::to_string_pretty(&*pending)?;
        
        // Écriture atomique : temp + fsync + rename
        let temp_path = self.persistence_path.with_extension("tmp");
        
        tokio::fs::write(&temp_path, &json).await?;
        
        // Fsync (flush to disk)
        let file = tokio::fs::File::open(&temp_path).await?;
        file.sync_all().await?;
        
        // Rename atomique
        tokio::fs::rename(&temp_path, &self.persistence_path).await?;
        
        println!("[intentions] Saved {} intentions atomically", pending.len());
        Ok(())
    }
    
    /// Chargement au boot avec purge auto des expirés
    pub async fn load(&self) -> Result<usize> {
        if !self.persistence_path.exists() {
            println!("[intentions] No persistence file, starting fresh");
            return Ok(0);
        }
        
        let json = tokio::fs::read_to_string(&self.persistence_path).await?;
        let loaded: HashMap<String, Intention> = serde_json::from_str(&json)?;
        
        let now = OffsetDateTime::now_utc();
        let mut restored = 0;
        let mut purged = 0;
        
        let mut pending = self.pending.write();
        
        for (id, intention) in loaded {
            if intention.expires_at > now {
                // Encore valide
                pending.insert(id, intention);
                restored += 1;
            } else {
                // Déjà expiré → ignorer
                println!("[intentions] Purged expired on boot: {} (expired {}s ago)",
                    id,
                    (now - intention.expires_at).whole_seconds()
                );
                purged += 1;
            }
        }
        
        drop(pending);
        
        println!("[intentions] Restored {} intentions ({} purged)", restored, purged);
        
        metrics::INTENTIONS_RESTORED_TOTAL.set(restored as i64);
        
        Ok(restored)
    }
}
```

GRACEFUL SHUTDOWN HANDLER :

```rust
// symbion-kernel/src/main.rs

#[tokio::main]
async fn main() {
    // ...
    
    let intentions = Arc::new(IntentionRegistry::new(&config.intentions_persistence_path));
    
    // Charger intentions au boot
    match intentions.load().await {
        Ok(count) => println!("[kernel] Restored {} intentions", count),
        Err(e) => eprintln!("[kernel] Failed to load intentions: {}", e),
    }
    
    // Signal handler graceful shutdown
    let intentions_clone = intentions.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let mut sigint = signal(SignalKind::interrupt()).unwrap();
            
            tokio::select! {
                _ = sigterm.recv() => {
                    println!("[kernel] SIGTERM received, graceful shutdown...");
                }
                _ = sigint.recv() => {
                    println!("[kernel] SIGINT received, graceful shutdown...");
                }
            }
        }
        
        #[cfg(windows)]
        {
            use tokio::signal::windows;
            let mut ctrl_c = windows::ctrl_c().unwrap();
            
            ctrl_c.recv().await;
            println!("[kernel] CTRL+C received, graceful shutdown...");
        }
        
        // Sauvegarder intentions
        if let Err(e) = intentions_clone.save().await {
            eprintln!("[kernel] Failed to save intentions: {}", e);
        }
        
        std::process::exit(0);
    });
    
    // Reste du code...
}
```

═══════════════════════════════════════════════════════════
⚙️ POINTS DE RIGUEUR
═══════════════════════════════════════════════════════════

1. HORLOGES MONOTONES
─────────────────────

Timeouts basés sur Instant (monotone).
Timestamps civils (OffsetDateTime) uniquement pour logs.

```rust
struct PendingValidation {
    intention_id: String,
    started_at: Instant,           // Pour timeout check
    created_at: OffsetDateTime,     // Pour logs
}

// Check timeout
if started_at.elapsed() > Duration::from_secs(timeout_secs) {
    // Expiré
}
```

2. VERROUILLAGE SANS DEADLOCK
──────────────────────────────

Ordre des locks documenté.
RwLock préféré à Mutex.
Pas d'appels blocking IO sous lock.

```rust
// ORDRE : intentions AVANT agents
// 1. Lock intentions.pending (RwLock)
// 2. Si besoin agents, lock agents (RwLock)
// JAMAIS l'inverse

{
    let pending = intentions.pending.read();
    // ... lecture seule, pas de IO
}  // Release lock immédiatement

// IO après release
tokio::fs::write(...).await?;
```

3. BACKOFF MQTT
───────────────

Publish MQTT best-effort avec backoff.
Pas d'arrêt kernel si publish échoue.

```rust
async fn publish_with_backoff(&self, topic: &str, payload: String) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 3;
    
    while attempts < max_attempts {
        match self.mqtt_client.publish(topic, QoS::AtLeastOnce, false, payload.clone()).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    eprintln!("[mqtt] Failed to publish after {} attempts: {}", max_attempts, e);
                    return Err(e.into());
                }
                tokio::time::sleep(Duration::from_millis(100 * attempts)).await;
            }
        }
    }
    
    Ok(())
}
```

4. RATE LIMIT CREATE INTENTION
───────────────────────────────

POST /v1/intentions limité à 30/min/IP.

```rust
let intention_rate_limit = ServiceBuilder::new()
    .layer(RateLimitLayer::new(30, Duration::from_secs(60)));

Router::new()
    .route("/v1/intentions", post(create_intention_handler)
        .layer(intention_rate_limit))
```

═══════════════════════════════════════════════════════════
📋 CONFIGURATION INTENTIONS
═══════════════════════════════════════════════════════════

```rust
// symbion-kernel/src/intentions.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentionsConfig {
    /// Max age du last_heartbeat avant considérer agent stale (défaut: 60s)
    pub offline_heartbeat_max_secs: u64,
    
    /// Timeout intention Medium (défaut: 600s = 10 min)
    pub timeout_medium_secs: u64,
    
    /// Timeout intention High (défaut: 1800s = 30 min)
    pub timeout_high_secs: u64,
    
    /// Intervalle cleanup intentions expirées (défaut: 30s)
    pub cleanup_interval_secs: u64,
    
    /// Chemin persistence (défaut: "./data/intentions.json")
    pub persistence_path: String,
}

impl Default for IntentionsConfig {
    fn default() -> Self {
        Self {
            offline_heartbeat_max_secs: 60,
            timeout_medium_secs: 600,
            timeout_high_secs: 1800,
            cleanup_interval_secs: 30,
            persistence_path: "./data/intentions.json".to_string(),
        }
    }
}
```

CHARGEMENT CONFIG :

```toml
# config.toml
[intentions]
offline_heartbeat_max_secs = 60
timeout_medium_secs = 600
timeout_high_secs = 1800
cleanup_interval_secs = 30
persistence_path = "./data/intentions.json"
```

═══════════════════════════════════════════════════════════
🧪 TESTS À LIVRER (9 TESTS NOMMÉS)
═══════════════════════════════════════════════════════════

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_offline_blocks_execution() {
        // Agent status = Offline
        // Vérifier ExecutionError::AgentOffline
    }

    #[tokio::test]
    async fn test_agent_stale_blocks_execution() {
        // Agent last_heartbeat > 60s
        // Vérifier ExecutionError::AgentStale
    }

    #[tokio::test]
    async fn test_intention_timeout_expires_and_notified() {
        // Intention expires_at dépassé
        // Vérifier retrait registry
        // Vérifier événement intention.expired publié
    }

    #[tokio::test]
    async fn test_conflict_detection_shutdown_reboot_same_agent() {
        // Intention Shutdown pending pour agent-01
        // Créer intention Reboot pour agent-01
        // Vérifier 409 CONFLICT
    }

    #[tokio::test]
    async fn test_conflict_detection_shutdown_hibernate_same_agent() {
        // Intention Shutdown pending pour agent-01
        // Créer intention Hibernate pour agent-01
        // Vérifier 409 CONFLICT
    }

    #[tokio::test]
    async fn test_restore_persistence_and_auto_expire_on_boot() {
        // Sauvegarder intentions (1 valide, 1 expirée)
        // Redémarrer registry
        // Vérifier 1 restaurée, 1 purgée
        // Vérifier metric intentions_restored_total = 1
    }

    #[tokio::test]
    async fn test_graceful_shutdown_persists_pending() {
        // Créer 2 intentions pending
        // Trigger graceful shutdown
        // Vérifier intentions.json contient 2 intentions
    }

    #[tokio::test]
    async fn test_notifications_pending_expired_validated_counters() {
        // Créer intention → vérifier événement pending
        // Expirer intention → vérifier événement expired
        // Valider intention → vérifier événement validated
        // Vérifier métriques dashboard_events_published_total
    }

    #[tokio::test]
    async fn test_rate_limit_create_intention_429() {
        // Envoyer 31 requêtes POST /v1/intentions en 1 min
        // Vérifier 31ème retourne 429 TOO_MANY_REQUESTS
    }
}
```

═══════════════════════════════════════════════════════════
🚀 CI INCHANGÉE
═══════════════════════════════════════════════════════════

Pipeline CI existant conservé :
• cargo fmt --check
• cargo clippy -- -D warnings
• cargo test

Guards-before-trust et toutes règles P0 maintenues.

═══════════════════════════════════════════════════════════
✅ PLAN FINAL P0
═══════════════════════════════════════════════════════════

SEMAINE 1 :
• PR1 : context/timezone+hysteresis → v0.2.0-alpha.1
• PR2 : api/v1+auth+MFA+nonce → v0.2.0-alpha.2

SEMAINE 2 :
• PR3 : decision/guards-first+weights → v0.2.0-beta.1
• PR4 : observability-min → v0.2.1

SEMAINE 3 :
• PR5 : fail-safe → v0.2.2
• PR6 : intentions/lifecycle → v0.2.3

SEMAINE 4 :
• Tests intégration complète
• Documentation finale
• Polish

TAG PRODUCTION-READY : v0.2.3

═══════════════════════════════════════════════════════════

P0 VERROUILLÉ AVEC PR6 INTÉGRÉE.
EXIGENCES TECHNIQUES DÉTAILLÉES.
PRÊT POUR IMPLÉMENTATION SÉQUENTIELLE.

Feu vert pour démarrer PR1 ?

Claude Code - P0 Roadmap Finale + PR6 Verrouillée
27 Octobre 2025
