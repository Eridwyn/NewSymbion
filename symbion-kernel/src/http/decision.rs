use super::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use time::OffsetDateTime;
use crate::decision_http::{EvaluateRequest, AuditQueryParams, ResolveValidationRequest, CreateOverrideRequest, RevokeOverrideRequest};
use crate::context_intelligence::DecisionSignal;

/// POST /decision/evaluate -- Evaluate an action through the Decision Engine.
#[utoipa::path(
    post,
    path = "/decision/evaluate",
    tag = "Decision",
    request_body = EvaluateRequest,
    responses(
        (status = 200, description = "Decision evaluation result", body = crate::decision::DecisionResult)
    ),
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce"))
)]
pub(super) async fn decision_evaluate(
    State(app): State<AppState>,
    Json(req): Json<EvaluateRequest>,
) -> Json<crate::decision::DecisionResult> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::evaluate_action(State(state), Json(req)).await
}

/// GET /decision/audit -- Retrieve the decision audit trail with optional query filters.
#[utoipa::path(
    get,
    path = "/decision/audit",
    tag = "Decision",
    params(AuditQueryParams),
    responses(
        (status = 200, description = "Audit trail entries", body = serde_json::Value)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_get_audit(
    State(app): State<AppState>,
    Query(params): Query<AuditQueryParams>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_audit_trail(State(state), Query(params)).await
}

/// GET /decision/metrics -- Return Decision Engine metrics in Prometheus text format.
#[utoipa::path(
    get,
    path = "/decision/metrics",
    tag = "Decision",
    responses(
        (status = 200, description = "Prometheus-format metrics", body = String),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_get_metrics(
    State(app): State<AppState>,
) -> Result<String, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_metrics(State(state)).await
}

/// GET /decision/validations/pending -- List all pending validation requests.
#[utoipa::path(
    get,
    path = "/decision/validations/pending",
    tag = "Decision",
    responses(
        (status = 200, description = "List of pending validations", body = Vec<crate::decision::ValidationRequest>)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_list_pending_validations(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_pending_validations(State(state)).await
}

/// POST /decision/validation/{id}/resolve -- Approve or reject a pending validation and execute the associated action if approved.
#[utoipa::path(
    post,
    path = "/decision/validation/{id}/resolve",
    tag = "Decision",
    request_body = ResolveValidationRequest,
    responses(
        (status = 200, description = "Resolved validation", body = crate::decision::ValidationRequest),
        (status = 404, description = "Validation not found")
    ),
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Validation request ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    )
)]
pub(super) async fn decision_resolve_validation(
    State(app): State<AppState>,
    Path(validation_id): Path<String>,
    Json(req): Json<ResolveValidationRequest>,
) -> Result<Json<crate::decision::ValidationRequest>, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };

    // Resolve the validation first
    let result = crate::decision_http::resolve_validation(
        State(state),
        Path(validation_id.clone()),
        Json(req.clone()),
    ).await?;

    // If approved, execute the pending action
    if req.approved {
        if let Some(pending) = app.pending_action_registry.take(&validation_id) {
            eprintln!(
                "[http] Executing pending action for approved validation {} (automation: {})",
                validation_id, pending.automation_name
            );

            // Même exécuteur que les automatisations, après une dernière
            // lecture des guards (cf. AutomationEngine::execute_approved).
            let ctx = crate::automations::ExecutionContext {
                context_engine: app.context_engine.clone(),
                agents: app.agents.clone(),
                sensors: app.sensors.clone(),
                notifications_manager: app.notifications_manager.clone(),
                event: crate::automations::AutomationEvent::Manual {
                    automation_id: pending.automation_id.clone(),
                    triggered_by: Some("user-validation".to_string()),
                    timestamp: OffsetDateTime::now_utc(),
                },
                decision_engine: Some(app.decision_engine.clone()),
                trust_tracker: Some(app.trust_tracker.clone()),
                validation_manager: Some(app.decision_validation_manager.clone()),
                pending_action_registry: Some(app.pending_action_registry.clone()),
                context_intelligence: Some(app.context_intelligence.clone()),
                mode_registry: Some(app.mode_registry.clone()),
                feature_registry: Some(app.feature_registry.clone()),
                plugin_registry: Some(app.plugin_registry.clone()),
            };
            let exec_result = match crate::automations::AutomationEngine::execute_approved(
                &result.0, &pending.action, &ctx,
            ).await {
                crate::automations::ApprovedOutcome::Executed => Ok(()),
                crate::automations::ApprovedOutcome::Failed(e) => Err(e),
                crate::automations::ApprovedOutcome::Refused(raisons) => {
                    // Ni succès ni échec de l'action : la confiance n'en apprend rien.
                    let raison = raisons.join(" ; ");
                    eprintln!(
                        "[http] ✋ Validation {} approuvée mais action retenue à l'exécution : {}",
                        validation_id, raison
                    );
                    let notification = crate::notifications::Notification {
                        id: uuid::Uuid::new_v4().to_string(),
                        priority: crate::notifications::NotificationPriority::P1,
                        title: format!("✋ Non exécuté : {}", pending.automation_name),
                        body: format!(
                            "Votre accord a bien été reçu, mais la situation a changé depuis la demande : {}. Relancez la règle si vous le voulez toujours.",
                            raison
                        ),
                        source: "automation-validation".to_string(),
                        timestamp: OffsetDateTime::now_utc(),
                        acknowledged: false,
                        acknowledged_at: None,
                        actions: vec![],
                        data: None,
                    };
                    if let Err(e) = app.notifications_manager.send(notification).await {
                        eprintln!("[http] Failed to send refusal notification: {}", e);
                    }
                    let record = crate::automations::ExecutionRecord {
                        automation_id: pending.automation_id.clone(),
                        automation_name: pending.automation_name.clone(),
                        executed_at: OffsetDateTime::now_utc(),
                        trigger_event: "validation_approved".to_string(),
                        conditions_met: true,
                        actions_executed: vec![crate::automations::ActionResult {
                            action_type: format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string(),
                            success: false,
                            error: Some(raison.clone()),
                            duration_ms: 0,
                            decision_id: Some(validation_id.clone()),
                            trust_score: Some(pending.trust_score),
                            decision_outcome: Some("refused_at_execution".to_string()),
                            blocked_reasons: Some(raisons),
                        }],
                        success: false,
                        error: Some(raison),
                        trust_score: Some(pending.trust_score),
                        decision_outcome: Some("refused_at_execution".to_string()),
                    };
                    if let Err(e) = app.automations.add_history(record) {
                        eprintln!("[http] Failed to add refusal record to history: {}", e);
                    }
                    return Ok(result);
                }
            };

            match &exec_result {
                Ok(_) => {
                    eprintln!(
                        "[http] ✅ Pending action executed successfully for validation {}",
                        validation_id
                    );

                    // Record success in trust tracker for evolving statistics
                    let action_type = format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string();
                    let agent_id = pending.action.agent_id();
                    app.trust_tracker.record_action(&action_type, agent_id.as_deref(), true);
                    eprintln!(
                        "[http] 📈 Trust tracker updated: {} (agent: {:?}) -> success",
                        action_type, agent_id
                    );

                    // Notify Intelligence: action approved after MFA (weak positive)
                    // Use automation's target mode - NO fallback to current_mode
                    if let Some(signals) = app.context_intelligence.last_signals() {
                        if pending.target_mode.is_none() {
                            eprintln!(
                                "[http] No explicit intent for '{}', skipping Intelligence feedback",
                                pending.automation_name
                            );
                        }
                        app.context_intelligence.record_decision_outcome(
                            DecisionSignal::ApprovedMFA,
                            pending.target_mode.as_deref(),
                            &signals,
                            None, // blocked_categories not applicable for ApprovedMFA
                        );
                    }

                    // Add success record to history
                    let action_result = crate::automations::ActionResult {
                        action_type: format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string(),
                        success: true,
                        error: None,
                        duration_ms: 0,
                        decision_id: Some(validation_id.clone()),
                        trust_score: Some(pending.trust_score),
                        decision_outcome: Some("approved_post_validation".to_string()),
                        blocked_reasons: None,
                    };

                    let record = crate::automations::ExecutionRecord {
                        automation_id: pending.automation_id.clone(),
                        automation_name: pending.automation_name.clone(),
                        executed_at: time::OffsetDateTime::now_utc(),
                        trigger_event: "validation_approved".to_string(),
                        conditions_met: true,
                        actions_executed: vec![action_result],
                        success: true,
                        error: None,
                        trust_score: Some(pending.trust_score),
                        decision_outcome: Some("approved".to_string()),
                    };

                    if let Err(e) = app.automations.add_history(record) {
                        eprintln!("[http] Failed to add success record to history: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[http] ❌ Pending action failed for validation {}: {}",
                        validation_id, e
                    );

                    // Record failure in trust tracker (reduces trust faster)
                    let action_type = format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string();
                    let agent_id = pending.action.agent_id();
                    app.trust_tracker.record_action(&action_type, agent_id.as_deref(), false);
                    eprintln!(
                        "[http] 📉 Trust tracker updated: {} (agent: {:?}) -> failure",
                        action_type, agent_id
                    );

                    // Add failure record to history
                    let action_result = crate::automations::ActionResult {
                        action_type: format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string(),
                        success: false,
                        error: Some(e.clone()),
                        duration_ms: 0,
                        decision_id: Some(validation_id.clone()),
                        trust_score: Some(pending.trust_score),
                        decision_outcome: Some("approved_but_failed".to_string()),
                        blocked_reasons: None,
                    };

                    let record = crate::automations::ExecutionRecord {
                        automation_id: pending.automation_id.clone(),
                        automation_name: pending.automation_name.clone(),
                        executed_at: time::OffsetDateTime::now_utc(),
                        trigger_event: "validation_approved".to_string(),
                        conditions_met: true,
                        actions_executed: vec![action_result],
                        success: false,
                        error: Some(e.clone()),
                        trust_score: Some(pending.trust_score),
                        decision_outcome: Some("approved_but_failed".to_string()),
                    };

                    if let Err(e) = app.automations.add_history(record) {
                        eprintln!("[http] Failed to add failure record to history: {}", e);
                    }
                }
            }
        } else {
            eprintln!(
                "[http] No pending action found for validation {} (may have expired)",
                validation_id
            );
        }
    } else {
        // Validation denied - create rejection record in history
        if let Some(pending) = app.pending_action_registry.take(&validation_id) {
            eprintln!(
                "[http] ❌ Validation {} rejected for automation '{}'",
                validation_id, pending.automation_name
            );

            // Notify Intelligence: action denied by user (strong negative)
            // Use automation's target mode - NO fallback to current_mode
            if let Some(signals) = app.context_intelligence.last_signals() {
                if pending.target_mode.is_none() {
                    eprintln!(
                        "[http] No explicit intent for '{}', skipping Intelligence feedback",
                        pending.automation_name
                    );
                }
                app.context_intelligence.record_decision_outcome(
                    DecisionSignal::Denied,
                    pending.target_mode.as_deref(),
                    &signals,
                    None, // blocked_categories not applicable for Denied
                );
            }

            let action_result = crate::automations::ActionResult {
                action_type: format!("{:?}", pending.action).split('{').next().unwrap_or("unknown").trim().to_string(),
                success: false,
                error: Some("Rejected by user".to_string()),
                duration_ms: 0,
                decision_id: Some(validation_id.clone()),
                trust_score: Some(pending.trust_score),
                decision_outcome: Some("rejected".to_string()),
                blocked_reasons: None,
            };

            let record = crate::automations::ExecutionRecord {
                automation_id: pending.automation_id.clone(),
                automation_name: pending.automation_name.clone(),
                executed_at: time::OffsetDateTime::now_utc(),
                trigger_event: "validation_rejected".to_string(),
                conditions_met: true,
                actions_executed: vec![action_result],
                success: false,
                error: Some("Rejected by user".to_string()),
                trust_score: Some(pending.trust_score),
                decision_outcome: Some("rejected".to_string()),
            };

            if let Err(e) = app.automations.add_history(record) {
                eprintln!("[http] Failed to add rejection record to history: {}", e);
            }
        }
    }

    Ok(result)
}

/// POST /decision/override -- Create a new master override for the Decision Engine.
#[utoipa::path(
    post,
    path = "/decision/override",
    tag = "Decision",
    request_body = CreateOverrideRequest,
    responses(
        (status = 200, description = "Created override", body = crate::decision::MasterOverride),
        (status = 400, description = "Invalid request")
    ),
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce"))
)]
pub(super) async fn decision_create_override(
    State(app): State<AppState>,
    Json(req): Json<CreateOverrideRequest>,
) -> Result<Json<crate::decision::MasterOverride>, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::create_override(State(state), Json(req)).await
}

/// GET /decision/overrides/active -- List all currently active master overrides.
#[utoipa::path(
    get,
    path = "/decision/overrides/active",
    tag = "Decision",
    responses(
        (status = 200, description = "List of active overrides", body = Vec<crate::decision::MasterOverride>)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_list_active_overrides(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::MasterOverride>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_active_overrides(State(state)).await
}

/// DELETE /decision/override/{id} -- Revoke an active master override by ID.
#[utoipa::path(
    delete,
    path = "/decision/override/{id}",
    tag = "Decision",
    request_body = RevokeOverrideRequest,
    responses(
        (status = 200, description = "Override revoked"),
        (status = 404, description = "Override not found")
    ),
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Override ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    )
)]
pub(super) async fn decision_revoke_override(
    State(app): State<AppState>,
    Path(override_id): Path<String>,
    Json(req): Json<RevokeOverrideRequest>,
) -> Result<StatusCode, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::revoke_override(State(state), Path(override_id), Json(req)).await
}

/// GET /decision/config -- Return the current Decision Engine configuration.
#[utoipa::path(
    get,
    path = "/decision/config",
    tag = "Decision",
    responses(
        (status = 200, description = "Decision engine configuration", body = crate::decision::DecisionConfig)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_get_config(
    State(app): State<AppState>,
) -> Json<crate::decision::DecisionConfig> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_config(State(state)).await
}

/// GET /decision/agent-health -- Return health status for all registered agents.
#[utoipa::path(
    get,
    path = "/decision/agent-health",
    tag = "Decision",
    responses(
        (status = 200, description = "Agent health status", body = serde_json::Value)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_get_agent_health(
    State(app): State<AppState>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_agent_health(State(state)).await
}

/// GET /decision/stats -- Return aggregate Decision Engine statistics.
#[utoipa::path(
    get,
    path = "/decision/stats",
    tag = "Decision",
    responses(
        (status = 200, description = "Decision engine statistics", body = crate::decision_http::DecisionStats)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_get_stats(
    State(app): State<AppState>,
) -> Json<crate::decision_http::DecisionStats> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::get_stats(State(state)).await
}

/// GET /decision/validations/expired -- List all expired validation requests.
#[utoipa::path(
    get,
    path = "/decision/validations/expired",
    tag = "Decision",
    responses(
        (status = 200, description = "List of expired validations", body = Vec<crate::decision::ValidationRequest>)
    ),
    security(("bearer_auth" = []))
)]
pub(super) async fn decision_list_expired_validations(
    State(app): State<AppState>,
) -> Json<Vec<crate::decision::ValidationRequest>> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::list_expired_validations(State(state)).await
}

/// DELETE /decision/validation/{id} -- Delete a specific validation request by ID.
#[utoipa::path(
    delete,
    path = "/decision/validation/{id}",
    tag = "Decision",
    responses(
        (status = 200, description = "Validation deleted"),
        (status = 404, description = "Validation not found")
    ),
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Validation request ID"),
        ("X-CSRF-Token" = String, Header, description = "CSRF nonce")
    )
)]
pub(super) async fn decision_delete_validation(
    State(app): State<AppState>,
    Path(validation_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::delete_validation(State(state), Path(validation_id)).await
}

/// DELETE /decision/validations/expired -- Delete all expired validation requests.
#[utoipa::path(
    delete,
    path = "/decision/validations/expired",
    tag = "Decision",
    responses(
        (status = 200, description = "All expired validations deleted", body = serde_json::Value)
    ),
    security(("bearer_auth" = [])),
    params(("X-CSRF-Token" = String, Header, description = "CSRF nonce"))
)]
pub(super) async fn decision_delete_all_expired_validations(
    State(app): State<AppState>,
) -> Json<serde_json::Value> {
    let state = crate::decision_http::DecisionEngineState {
        engine: app.decision_engine.clone(),
        validation_manager: app.decision_validation_manager.clone(),
        override_manager: app.decision_override_manager.clone(),
        audit_manager: app.decision_audit_manager.clone(),
        agent_health_manager: app.decision_agent_health_manager.clone(),
        metrics: app.decision_metrics.clone(),
    };
    crate::decision_http::delete_all_expired_validations(State(state)).await
}
