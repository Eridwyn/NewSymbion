use crate::claude::run_claude;
use crate::events::emit_event;
use crate::state::AppState;
use serde_json::json;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, MessageId,
};
use teloxide::utils::command::BotCommands;
use tokio_util::sync::CancellationToken;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommand {
    #[command(description = "Démarrer le bot")]
    Start,
    #[command(description = "Nouvelle session Claude")]
    New,
    #[command(description = "Continuer la session")]
    Continue,
    #[command(description = "Annuler la tâche en cours")]
    Cancel,
    #[command(description = "Statut du bot et système")]
    Status,
    #[command(description = "Changer le modèle")]
    Model(String),
    #[command(description = "Changer l'effort")]
    Effort(String),
    #[command(description = "Aide")]
    Help,
    // ── Symbion ──
    #[command(description = "Santé système")]
    Health,
    #[command(description = "Agents connectés")]
    Agents,
    #[command(description = "Mode Symbion")]
    Mode(String),
    #[command(description = "Notifications")]
    Notifs,
    #[command(description = "Bibliothèque")]
    Lib(String),
    #[command(description = "Notes")]
    Notes,
    #[command(description = "Plugins actifs")]
    Plugins,
    #[command(description = "Logs système")]
    Log(String),
    #[command(description = "Wake-on-LAN")]
    Wake(String),
    #[command(description = "Historique interactions")]
    History,
    // ── v2.0 Contrôle ──
    #[command(description = "Éteindre un agent")]
    Shutdown(String),
    #[command(description = "Redémarrer un agent")]
    Reboot(String),
    #[command(description = "Hiberner un agent")]
    Hibernate(String),
    #[command(description = "Certificats SSL")]
    Ssl,
    #[command(description = "Décisions en attente")]
    Decision(String),
    #[command(description = "Cafetière (espresso/long/eau/stop/status)")]
    Cafe(String),
    #[command(description = "Résumé synthétique du jour (modes, agents, automations, café)")]
    Summary,
}

pub fn build_dispatcher(
    bot: Bot,
    state: AppState,
) -> Dispatcher<Bot, teloxide::RequestError, teloxide::dispatching::DefaultKey> {
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter_command::<BotCommand>()
                        .endpoint(handle_command),
                )
                .branch(dptree::entry().endpoint(handle_message)),
        )
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
}

// ── Inline Keyboards ──

fn model_keyboard(current: &str) -> InlineKeyboardMarkup {
    let models = ["haiku", "sonnet", "opus"];
    let buttons: Vec<InlineKeyboardButton> = models
        .iter()
        .map(|m| {
            let label = if *m == current {
                format!("• {} •", m)
            } else {
                m.to_string()
            };
            InlineKeyboardButton::callback(label, format!("model:{}", m))
        })
        .collect();
    InlineKeyboardMarkup::new(vec![buttons])
}

fn effort_keyboard(current: &str) -> InlineKeyboardMarkup {
    let levels = ["low", "medium", "high"];
    let buttons: Vec<InlineKeyboardButton> = levels
        .iter()
        .map(|l| {
            let label = if *l == current {
                format!("• {} •", l)
            } else {
                l.to_string()
            };
            InlineKeyboardButton::callback(label, format!("effort:{}", l))
        })
        .collect();
    InlineKeyboardMarkup::new(vec![buttons])
}

#[allow(dead_code)]
fn confirm_keyboard(action: &str, label: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback(format!("✅ {}", label), format!("confirm:{}", action)),
        InlineKeyboardButton::callback("❌ Annuler", "confirm:cancel".to_string()),
    ]])
}

fn mode_keyboard(modes: &[serde_json::Value]) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();
    for chunk in modes.chunks(2) {
        let row: Vec<InlineKeyboardButton> = chunk
            .iter()
            .filter_map(|m| {
                let id = m.get("id").and_then(|v| v.as_str())?;
                let name = m.get("name").and_then(|v| v.as_str()).unwrap_or(id);
                let icon = m.get("icon").and_then(|v| v.as_str()).unwrap_or("");
                Some(InlineKeyboardButton::callback(
                    format!("{} {}", icon, name),
                    format!("mode:{}", id),
                ))
            })
            .collect();
        if !row.is_empty() {
            rows.push(row);
        }
    }
    InlineKeyboardMarkup::new(rows)
}

// ── Callback Query Handler ──

async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    bot.answer_callback_query(&q.id).await?;

    let data = match &q.data {
        Some(d) => d.clone(),
        None => return Ok(()),
    };

    let chat_id = match &q.message {
        Some(msg) => msg.chat().id,
        None => return Ok(()),
    };
    let user_id = chat_id.0;

    if !state.config.is_allowed(user_id) {
        return Ok(());
    }

    let msg_id = q.message.as_ref().map(|m| m.id());

    if let Some(rest) = data.strip_prefix("notif:") {
        handle_notif_callback(&state, &bot, chat_id, msg_id, rest).await;
    } else if let Some(node_id) = data.strip_prefix("lib:") {
        lib_show_node(&state, &bot, chat_id, msg_id, node_id).await;
    } else if let Some(action) = data.strip_prefix("model:") {
        if matches!(action, "haiku" | "sonnet" | "opus") {
            state.update_session(user_id, |s| s.model = action.to_string());
            if let Some(mid) = msg_id {
                let session = state.get_session(user_id);
                let _ = bot
                    .edit_message_text(chat_id, mid, format!("✅ Modèle → {}", action))
                    .reply_markup(model_keyboard(&session.model))
                    .await;
            }
        }
    } else if let Some(action) = data.strip_prefix("effort:") {
        if matches!(action, "low" | "medium" | "high") {
            state.update_session(user_id, |s| s.effort = action.to_string());
            if let Some(mid) = msg_id {
                let session = state.get_session(user_id);
                let _ = bot
                    .edit_message_text(chat_id, mid, format!("✅ Effort → {}", action))
                    .reply_markup(effort_keyboard(&session.effort))
                    .await;
            }
        }
    } else if let Some(action) = data.strip_prefix("mode:") {
        let result = kernel_post(
            &state,
            "/context/override",
            &json!({ "mode": action, "duration_minutes": 480 }),
        )
        .await;
        if let Some(mid) = msg_id {
            let _ = bot.edit_message_text(chat_id, mid, format!("🔄 Mode → {} (8h)\n{}", action, result)).await;
        }
    } else if let Some(action) = data.strip_prefix("confirm:") {
        match action {
            "cancel" => {
                if let Some(mid) = msg_id {
                    let _ = bot.edit_message_text(chat_id, mid, "❌ Action annulée.").await;
                }
            }
            _ => {
                if let Some(mac) = action.strip_prefix("wake:") {
                    execute_wake(&state, &bot, chat_id, msg_id, mac).await;
                } else if let Some(rest) = action.strip_prefix("agent:") {
                    // Format: agent:shutdown:agent_id or agent:reboot:agent_id
                    let parts: Vec<&str> = rest.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let result = kernel_post(
                            &state,
                            &format!("/v1/agents/{}/{}", parts[1], parts[0]),
                            &json!({}),
                        )
                        .await;
                        if let Some(mid) = msg_id {
                            let _ = bot.edit_message_text(chat_id, mid, result).await;
                        }
                    }
                } else if let Some(rest) = action.strip_prefix("decide:") {
                    // Format: decide:approve:validation_id or decide:deny:validation_id
                    let parts: Vec<&str> = rest.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let approved = parts[0] == "approve";
                        let result = kernel_post(
                            &state,
                            &format!("/decision/validation/{}/resolve", parts[1]),
                            &json!({ "approved": approved, "username": "telegram" }),
                        )
                        .await;
                        if let Some(mid) = msg_id {
                            let icon = if approved { "✅" } else { "❌" };
                            let _ = bot.edit_message_text(chat_id, mid, format!("{} {}", icon, result)).await;
                        }
                    }
                }
            }
        }
    } else if let Some(rest) = data.strip_prefix("cafe:") {
        // Coffee inline keyboard callbacks — format: drink:cups
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        let drink = parts[0];
        let cups: u8 = parts.get(1).and_then(|c| c.parse().ok()).unwrap_or(1);
        let result = match drink {
            "espresso" | "coffee" | "hot_water" => {
                plugin_post(&state, "coffee", "/brew", &json!({"drink": drink, "temperature": 2, "cups": cups})).await
            }
            "stop" => plugin_post(&state, "coffee", "/stop", &json!({})).await,
            _ => "❌ Action inconnue".to_string(),
        };
        let icon = match drink {
            "espresso" | "coffee" => "☕",
            "hot_water" => "💧",
            "stop" => "⛔",
            _ => "❓",
        };
        let label = if cups > 1 { format!(" x{}", cups) } else { String::new() };
        if let Some(mid) = msg_id {
            let _ = bot.edit_message_text(chat_id, mid, format!("{}{} {}", icon, label, result)).await;
        }
    }

    Ok(())
}

// ── Notification Action Handler ──

async fn handle_notif_callback(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    msg_id: Option<MessageId>,
    rest: &str,
) {
    // Format: {notif_id}:{action_id}
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    if parts.len() != 2 {
        return;
    }
    let notif_id = parts[0];
    let action_id = parts[1];

    // Get cached notification data
    let cached = match state.get_cached_notif(notif_id) {
        Some(c) => c,
        None => {
            if let Some(mid) = msg_id {
                let _ = bot.edit_message_text(chat_id, mid, "⏰ Notification expirée.").await;
            }
            return;
        }
    };

    // Find the action in notification data
    let notif_data = &cached.data;
    let data_obj = notif_data.get("data");

    // Determine what to do based on action_id and notification data
    let result_text = match action_id {
        "apply" => {
            // Check if it's a mode suggestion
            if let Some(mode) = data_obj
                .and_then(|d| d.get("suggested_mode"))
                .and_then(|v| v.as_str())
            {
                // Apply mode via kernel
                let result = kernel_post(
                    state,
                    "/context/override",
                    &json!({ "mode": mode, "duration_minutes": 480 }),
                )
                .await;

                // Acknowledge notification
                let _ = kernel_post(
                    state,
                    &format!("/notifications/{}/acknowledge", notif_id),
                    &json!({}),
                )
                .await;

                format!("✅ Mode → {} (8h)\n{}", mode, result)
            } else {
                // Generic approve action
                let _ = kernel_post(
                    state,
                    &format!("/notifications/{}/acknowledge", notif_id),
                    &json!({}),
                )
                .await;
                "✅ Action appliquée".to_string()
            }
        }
        "dismiss" | "reject" => {
            // Just acknowledge
            let _ = kernel_post(
                state,
                &format!("/notifications/{}/acknowledge", notif_id),
                &json!({}),
            )
            .await;
            "❌ Ignoré".to_string()
        }
        "snooze" => {
            // Acknowledge (snooze = just dismiss for now)
            let _ = kernel_post(
                state,
                &format!("/notifications/{}/acknowledge", notif_id),
                &json!({}),
            )
            .await;
            "⏰ Rappel ignoré".to_string()
        }
        _ => {
            // Unknown action — just acknowledge
            let _ = kernel_post(
                state,
                &format!("/notifications/{}/acknowledge", notif_id),
                &json!({}),
            )
            .await;
            format!("▶️ Action '{}' exécutée", action_id)
        }
    };

    // Remove from cache
    state.remove_cached_notif(notif_id);

    // Update the message to show result (remove buttons)
    if let Some(mid) = msg_id {
        let _ = bot.edit_message_text(chat_id, mid, result_text).await;
    }
}

// ── Command Handler ──

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let user_id = chat_id.0;

    if !state.config.is_allowed(user_id) {
        bot.send_message(chat_id, "⛔ Non autorisé.").await?;
        return Ok(());
    }

    match cmd {
        BotCommand::Start | BotCommand::Help => {
            bot.send_message(
                chat_id,
                "🤖 Symbion Telegram\n\n\
                 📝 Envoie un message → Claude répond\n\
                 📷 Envoie une photo → Claude analyse\n\n\
                 ── Session ──\n\
                 /new — Nouvelle session\n\
                 /continue — Reprendre\n\
                 /cancel — Annuler\n\
                 /model — Changer modèle\n\
                 /effort — Changer effort\n\
                 /status — Statut bot\n\
                 /history — Historique\n\n\
                 ── Symbion ──\n\
                 /health — Santé système\n\
                 /agents — Agents connectés\n\
                 /mode — Voir/changer mode\n\
                 /notifs — Notifications\n\
                 /plugins — Plugins actifs\n\
                 /log [service] — Logs\n\
                 /wake [agent] — Wake-on-LAN\n\n\
                 ── Contrôle ──\n\
                 /shutdown <agent> — Éteindre\n\
                 /reboot <agent> — Redémarrer\n\
                 /hibernate <agent> — Hiberner\n\
                 /ssl — Certificats SSL\n\
                 /decision — Décisions en attente\n\n\
                 ── Données ──\n\
                 /lib <recherche> — Bibliothèque\n\
                 /notes — Notes récentes\n\n\
                 ── Cafetière ──\n\
                 /cafe — Menu interactif\n\
                 /cafe espresso — Espresso\n\
                 /cafe long — Café long\n\
                 /cafe eau — Eau chaude\n\
                 /cafe stop — Arrêter",
            )
            .await?;
        }
        BotCommand::New => {
            state.update_session(user_id, |s| s.session_id = None);
            bot.send_message(chat_id, "🔄 Nouvelle session.").await?;
        }
        BotCommand::Continue => {
            let session = state.get_session(user_id);
            if session.session_id.is_some() {
                bot.send_message(chat_id, "▶️ Session active, envoie ton message.")
                    .await?;
            } else {
                bot.send_message(chat_id, "❌ Aucune session à continuer.")
                    .await?;
            }
        }
        BotCommand::Cancel => {
            if let Some((_, token)) = state.active_tasks.remove(&user_id) {
                token.cancel();
                bot.send_message(chat_id, "❌ Tâche annulée.").await?;
            } else {
                bot.send_message(chat_id, "ℹ️ Aucune tâche en cours.")
                    .await?;
            }
        }
        BotCommand::Status => {
            let session = state.get_session(user_id);
            let busy = state.is_busy(user_id);
            let uptime_secs = state.start_time.elapsed().as_secs();
            let hours = uptime_secs / 3600;
            let mins = (uptime_secs % 3600) / 60;

            let sid_display = session
                .session_id
                .as_deref()
                .map(|s| &s[..8.min(s.len())])
                .unwrap_or("—");

            bot.send_message(
                chat_id,
                format!(
                    "📊 Symbion Telegram\n\n\
                     Modèle: {}\n\
                     Effort: {}\n\
                     Session: {}\n\
                     Tâche: {}\n\
                     Uptime: {}h{:02}m",
                    session.model,
                    session.effort,
                    sid_display,
                    if busy { "⏳ en cours" } else { "💤 idle" },
                    hours,
                    mins,
                ),
            )
            .await?;
        }
        BotCommand::Model(name) => {
            let name = name.trim().to_lowercase();
            if name.is_empty() {
                let session = state.get_session(user_id);
                bot.send_message(chat_id, format!("Modèle actuel: {}", session.model))
                    .reply_markup(model_keyboard(&session.model))
                    .await?;
            } else if matches!(name.as_str(), "haiku" | "sonnet" | "opus") {
                state.update_session(user_id, |s| s.model = name.clone());
                bot.send_message(chat_id, format!("✅ Modèle → {}", name))
                    .await?;
            } else {
                bot.send_message(chat_id, "❌ haiku, sonnet, ou opus")
                    .await?;
            }
        }
        BotCommand::Effort(level) => {
            let level = level.trim().to_lowercase();
            if level.is_empty() {
                let session = state.get_session(user_id);
                bot.send_message(chat_id, format!("Effort actuel: {}", session.effort))
                    .reply_markup(effort_keyboard(&session.effort))
                    .await?;
            } else if matches!(level.as_str(), "low" | "medium" | "high") {
                state.update_session(user_id, |s| s.effort = level.clone());
                bot.send_message(chat_id, format!("✅ Effort → {}", level))
                    .await?;
            } else {
                bot.send_message(chat_id, "❌ low, medium, ou high")
                    .await?;
            }
        }
        BotCommand::History => {
            let history = state.get_history(user_id);
            if history.is_empty() {
                bot.send_message(chat_id, "📭 Aucun historique.").await?;
            } else {
                let mut text = String::from("📜 Dernières interactions:\n\n");
                for (i, entry) in history.iter().enumerate().rev().take(10) {
                    let prompt_preview = truncate(&entry.prompt, 40);
                    let status = if entry.success { "✅" } else { "❌" };
                    text.push_str(&format!(
                        "{}. {} {} [{}]\n",
                        i + 1,
                        status,
                        prompt_preview,
                        entry.model
                    ));
                }
                bot.send_message(chat_id, text).await?;
            }
        }

        // ── Symbion Quick Commands ──

        BotCommand::Health => {
            let text = kernel_get(&state, "/system/health").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Agents => {
            let text = kernel_get(&state, "/agents").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Mode(arg) => {
            let arg = arg.trim().to_lowercase();
            if arg.is_empty() {
                // Fetch modes and show as inline keyboard
                let url = "https://localhost:8443/v1/modes";
                let client = make_client();
                match client.get(url).header("x-api-key", &state.config.kernel_api_key).send().await {
                    Ok(resp) => {
                        if let Ok(body) = resp.text().await {
                            if let Ok(modes) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                                let kb = mode_keyboard(&modes);
                                bot.send_message(chat_id, "🎨 Choisir un mode:")
                                    .reply_markup(kb)
                                    .await?;
                            } else {
                                bot.send_message(chat_id, format_json_from_str(&body)).await?;
                            }
                        }
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ {}", e)).await?;
                    }
                }
            } else {
                let result = kernel_post(&state, "/context/override", &json!({ "mode": arg, "duration_minutes": 480 })).await;
                bot.send_message(chat_id, format!("🔄 Mode → {} (8h)\n{}", arg, result)).await?;
            }
        }
        BotCommand::Notifs => {
            let text = kernel_get(&state, "/v1/notifications/active").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Lib(query) => {
            let query = query.trim();
            let path = if query.is_empty() {
                "/nodes?limit=10".to_string()
            } else {
                format!("/search?q={}", urlencoding(query))
            };
            lib_search_with_buttons(&state, &bot, chat_id, &path).await?;
        }
        BotCommand::Notes => {
            let text = plugin_get(&state, "notes", "/notes").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Plugins => {
            let text = kernel_get(&state, "/v1/plugins").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Log(service) => {
            let service = service.trim();
            let svc = match service {
                "" | "kernel" => "symbion-kernel",
                "telegram" | "tg" => "symbion-plugin-telegram",
                "mqtt" | "mosquitto" => "mosquitto",
                other => other,
            };
            match tokio::process::Command::new("journalctl")
                .args(["-u", svc, "-n", "15", "--no-pager", "-o", "short"])
                .output()
                .await
            {
                Ok(output) => {
                    let logs = String::from_utf8_lossy(&output.stdout);
                    let text = if logs.is_empty() {
                        format!("📭 Aucun log pour {}", svc)
                    } else {
                        format!("📋 {} (15 dernières lignes):\n\n{}", svc, truncate(&logs, 3800))
                    };
                    bot.send_message(chat_id, text).await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ {}", e)).await?;
                }
            }
        }
        BotCommand::Wake(target) => {
            let target = target.trim();
            if target.is_empty() {
                // List agents with MAC addresses
                let url = "https://localhost:8443/agents";
                let client = make_client();
                match client.get(url).header("x-api-key", &state.config.kernel_api_key).send().await {
                    Ok(resp) => {
                        if let Ok(body) = resp.text().await {
                            if let Ok(agents) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                                let mut text = String::from("💡 Agents disponibles pour WoL:\n\n");
                                let mut buttons = Vec::new();
                                for agent in &agents {
                                    let hostname = agent.get("hostname").and_then(|v| v.as_str()).unwrap_or("?");
                                    let mac = agent.get("primary_mac").and_then(|v| v.as_str()).unwrap_or("?");
                                    let status = agent.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                                    let icon = if status == "online" { "🟢" } else { "🔴" };
                                    text.push_str(&format!("{} {} — {}\n", icon, hostname, mac));
                                    if status != "online" {
                                        buttons.push(InlineKeyboardButton::callback(
                                            format!("💡 {}", hostname),
                                            format!("confirm:wake:{}", mac),
                                        ));
                                    }
                                }
                                if buttons.is_empty() {
                                    text.push_str("\nTous les agents sont en ligne.");
                                    bot.send_message(chat_id, text).await?;
                                } else {
                                    let kb = InlineKeyboardMarkup::new(vec![buttons]);
                                    bot.send_message(chat_id, text).reply_markup(kb).await?;
                                }
                            } else {
                                bot.send_message(chat_id, "❌ Impossible de lire les agents.").await?;
                            }
                        }
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ {}", e)).await?;
                    }
                }
            } else {
                // Direct wake with MAC or hostname
                execute_wake(&state, &bot, chat_id, None, target).await;
            }
        }

        // ── v2.0 Agent Control ──

        BotCommand::Shutdown(target) => {
            agent_control_cmd(&state, &bot, chat_id, &target, "shutdown", "Éteindre").await?;
        }
        BotCommand::Reboot(target) => {
            agent_control_cmd(&state, &bot, chat_id, &target, "reboot", "Redémarrer").await?;
        }
        BotCommand::Hibernate(target) => {
            agent_control_cmd(&state, &bot, chat_id, &target, "hibernate", "Hiberner").await?;
        }
        BotCommand::Ssl => {
            let text = plugin_get(&state, "ssl", "/domains").await;
            bot.send_message(chat_id, text).await?;
        }
        BotCommand::Decision(arg) => {
            let arg = arg.trim();
            if arg.is_empty() {
                // List pending decisions
                let text = kernel_get(&state, "/decision/validations/pending").await;
                bot.send_message(chat_id, text).await?;
            } else if let Some(rest) = arg.strip_prefix("approve ") {
                let result = kernel_post(
                    &state,
                    &format!("/decision/validation/{}/resolve", rest.trim()),
                    &json!({ "approved": true, "username": "telegram" }),
                )
                .await;
                bot.send_message(chat_id, result).await?;
            } else if let Some(rest) = arg.strip_prefix("deny ") {
                let result = kernel_post(
                    &state,
                    &format!("/decision/validation/{}/resolve", rest.trim()),
                    &json!({ "approved": false, "username": "telegram" }),
                )
                .await;
                bot.send_message(chat_id, result).await?;
            } else {
                bot.send_message(
                    chat_id,
                    "Usage:\n/decision — Liste en attente\n/decision approve <id>\n/decision deny <id>",
                )
                .await?;
            }
        }
        BotCommand::Cafe(arg) => {
            handle_cafe_command(&state, &bot, chat_id, &arg).await?;
        }

        BotCommand::Summary => {
            let text = build_daily_summary(&state).await;
            bot.send_message(chat_id, text).parse_mode(teloxide::types::ParseMode::Html).await?;
        }
    }

    Ok(())
}

/// Construit un résumé du jour : mode actif, agents, automations récentes, café.
/// Appelable via /summary (à la demande) ou via plugin_command depuis une automation
/// (ex: scheduled chaque jour à 8h).
pub async fn build_daily_summary(state: &AppState) -> String {
    use serde_json::Value;

    let client = make_client();
    let api_key = &state.config.kernel_api_key;

    let fetch = |path: &'static str| {
        let client = client.clone();
        let url = format!("https://localhost:8443{}", path);
        let api_key = api_key.clone();
        async move {
            client
                .get(&url)
                .header("x-api-key", api_key)
                .send()
                .await
                .ok()?
                .json::<Value>()
                .await
                .ok()
        }
    };

    let (context, agents, history, coffee_status) = tokio::join!(
        fetch("/v1/context/current"),
        fetch("/v1/agents"),
        fetch("/v1/automations/history?limit=30"),
        fetch("/v1/plugin-api/coffee/status"),
    );

    let mut out = String::from("<b>📊 Résumé du jour</b>\n\n");

    // Mode actuel
    if let Some(ctx) = context {
        let mode = ctx.get("mode_slug").or_else(|| ctx.get("mode"))
            .and_then(|v| v.as_str()).unwrap_or("?");
        let reason = ctx.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let conf = ctx.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        out.push_str(&format!("🎨 <b>Mode</b> : {} (conf {:.0}%)\n   <i>{}</i>\n\n", mode, conf * 100.0, reason));
    }

    // Agents
    if let Some(ag) = agents {
        if let Some(arr) = ag.as_array() {
            let online = arr.iter().filter(|a| a.get("status").and_then(|s| s.get("status")).and_then(|s| s.as_str()) == Some("online")).count();
            out.push_str(&format!("🖥️ <b>Agents</b> : {}/{} en ligne\n\n", online, arr.len()));
        }
    }

    // Automations exécutées aujourd'hui
    if let Some(hist) = history {
        let arr = hist.get("history").and_then(|v| v.as_array());
        if let Some(items) = arr {
            let today = time::OffsetDateTime::now_utc().date();
            let today_count = items.iter().filter(|item| {
                item.get("executed_at").and_then(|t| t.as_str())
                    .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
                    .map(|t| t.date() == today)
                    .unwrap_or(false)
            }).count();
            let success_count = items.iter().filter(|item| {
                item.get("success").and_then(|v| v.as_bool()).unwrap_or(false)
                && item.get("executed_at").and_then(|t| t.as_str())
                    .and_then(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok())
                    .map(|t| t.date() == today)
                    .unwrap_or(false)
            }).count();
            out.push_str(&format!("🤖 <b>Automations</b> : {} déclenchées aujourd'hui ({} OK)\n\n", today_count, success_count));
        }
    }

    // Café
    if let Some(cs) = coffee_status {
        let brews_today = cs.get("brew_count_today").and_then(|v| v.as_u64()).unwrap_or(0);
        let online = cs.get("online").and_then(|v| v.as_bool()).unwrap_or(false);
        let mainstate = cs.get("mainstate_text").and_then(|v| v.as_str()).unwrap_or("?");
        let water = cs.get("water_level").and_then(|v| v.as_u64()).unwrap_or(0);
        let beans = cs.get("bean_level").and_then(|v| v.as_u64()).unwrap_or(0);
        let maint = cs.get("maintenance_needed").and_then(|v| v.as_bool()).unwrap_or(false);
        let icon_on = if online { "🟢" } else { "🔴" };
        let maint_str = if maint { " ⚠️ maintenance" } else { "" };
        out.push_str(&format!(
            "☕ <b>Café</b> : {} {} {} brews aujourd'hui · eau {}% · grains {}%{}\n",
            icon_on, mainstate, brews_today, water, beans, maint_str
        ));
    }

    out
}

// ── Coffee Command Handler ──

async fn handle_cafe_command(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    arg: &str,
) -> Result<(), teloxide::RequestError> {
    let arg = arg.trim().to_lowercase();

    match arg.as_str() {
        "" => {
            // Show inline keyboard with drink choices
            let keyboard = InlineKeyboardMarkup::new(vec![
                vec![
                    InlineKeyboardButton::callback("☕ Espresso", "cafe:espresso:1"),
                    InlineKeyboardButton::callback("☕ Espresso x2", "cafe:espresso:2"),
                ],
                vec![
                    InlineKeyboardButton::callback("☕ Café long", "cafe:coffee:1"),
                    InlineKeyboardButton::callback("☕ Café long x2", "cafe:coffee:2"),
                ],
                vec![
                    InlineKeyboardButton::callback("💧 Eau chaude", "cafe:hot_water:1"),
                    InlineKeyboardButton::callback("⛔ Arrêter", "cafe:stop:0"),
                ],
            ]);
            bot.send_message(chat_id, "☕ Cafetière — Que veux-tu ?")
                .reply_markup(keyboard)
                .await?;
        }
        "espresso" | "expresso" => {
            let result = plugin_post(state, "coffee", "/brew", &json!({"drink": "espresso", "temperature": 2, "cups": 1})).await;
            bot.send_message(chat_id, format!("☕ Espresso\n{}", result)).await?;
        }
        "espresso2" | "expresso2" | "espresso x2" | "2espresso" => {
            let result = plugin_post(state, "coffee", "/brew", &json!({"drink": "espresso", "temperature": 2, "cups": 2})).await;
            bot.send_message(chat_id, format!("☕ Espresso x2\n{}", result)).await?;
        }
        "long" | "cafe" | "coffee" => {
            let result = plugin_post(state, "coffee", "/brew", &json!({"drink": "coffee", "temperature": 2, "cups": 1})).await;
            bot.send_message(chat_id, format!("☕ Café long\n{}", result)).await?;
        }
        "long2" | "cafe2" | "coffee2" | "long x2" | "2cafe" => {
            let result = plugin_post(state, "coffee", "/brew", &json!({"drink": "coffee", "temperature": 2, "cups": 2})).await;
            bot.send_message(chat_id, format!("☕ Café long x2\n{}", result)).await?;
        }
        "eau" | "water" | "hot_water" | "eau_chaude" => {
            let result = plugin_post(state, "coffee", "/brew", &json!({"drink": "hot_water", "temperature": 2, "cups": 1})).await;
            bot.send_message(chat_id, format!("💧 Eau chaude\n{}", result)).await?;
        }
        "stop" | "arreter" => {
            let result = plugin_post(state, "coffee", "/stop", &json!({})).await;
            bot.send_message(chat_id, format!("⛔ {}", result)).await?;
        }
        "status" | "statut" => {
            let raw = plugin_get(state, "coffee", "/status").await;
            bot.send_message(chat_id, format!("☕ Cafetière\n{}", raw)).await?;
        }
        "info" => {
            let raw = plugin_get(state, "coffee", "/info").await;
            bot.send_message(chat_id, format!("☕ Info machine\n{}", raw)).await?;
        }
        "on" | "allumer" => {
            let result = plugin_post(state, "coffee", "/power", &json!({"on": true})).await;
            bot.send_message(chat_id, format!("🔌 {}", result)).await?;
        }
        "off" | "eteindre" => {
            let result = plugin_post(state, "coffee", "/power", &json!({"on": false})).await;
            bot.send_message(chat_id, format!("🔌 {}", result)).await?;
        }
        _ => {
            bot.send_message(
                chat_id,
                "☕ Usage:\n\
                 /cafe — Menu interactif\n\
                 /cafe espresso — Espresso\n\
                 /cafe long — Café long\n\
                 /cafe eau — Eau chaude\n\
                 /cafe stop — Arrêter\n\
                 /cafe status — Statut machine\n\
                 /cafe on — Allumer\n\
                 /cafe off — Éteindre",
            )
            .await?;
        }
    }
    Ok(())
}

// ── Message Handler ──

async fn handle_message(
    bot: Bot,
    msg: Message,
    state: AppState,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let user_id = chat_id.0;

    if !state.config.is_allowed(user_id) {
        return Ok(());
    }

    // Handle photos — download and send to Claude as description
    if let Some(photos) = msg.photo() {
        if state.is_busy(user_id) {
            bot.send_message(chat_id, "⏳ Tâche en cours. /cancel pour annuler.")
                .await?;
            return Ok(());
        }

        // Get highest resolution photo
        let photo = photos.last().unwrap();
        let file = bot.get_file(&photo.file.id).await?;
        let file_path = file.path.clone();

        // Download to temp file
        let tmp_path = format!("/tmp/symbion-tg-{}.jpg", photo.file.unique_id);
        let download_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            state.config.telegram_bot_token, file_path
        );

        if let Ok(resp) = reqwest::get(&download_url).await {
            if let Ok(bytes) = resp.bytes().await {
                if tokio::fs::write(&tmp_path, &bytes).await.is_ok() {
                    let caption = msg.caption().unwrap_or("Décris cette image.");
                    let prompt = format!(
                        "Image reçue (sauvée à {}). Caption: {}",
                        tmp_path, caption
                    );

                    let cancel = CancellationToken::new();
                    state.active_tasks.insert(user_id, cancel.clone());

                    let state_clone = state.clone();
                    let prompt_clone = prompt.clone();
                    tokio::spawn(async move {
                        let result = run_claude(&state_clone, chat_id, &prompt_clone, cancel).await;
                        state_clone.active_tasks.remove(&user_id);
                        state_clone.add_history(user_id, &prompt_clone, result.is_ok(), &state_clone.get_session(user_id).model);
                        if let Err(e) = result {
                            let _ = state_clone.bot.send_message(chat_id, format!("❌ {}", e)).await;
                        }
                        // Cleanup temp file
                        let _ = tokio::fs::remove_file(&tmp_path).await;
                    });

                    return Ok(());
                }
            }
        }
        bot.send_message(chat_id, "❌ Impossible de télécharger la photo.").await?;
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => return Ok(()),
    };

    // Intercept unknown /commands
    if text.starts_with('/') {
        let cmd_name = text
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_start_matches('/');
        bot.send_message(
            chat_id,
            format!("❓ /{} inconnu. /help pour la liste.", cmd_name),
        )
        .await?;
        return Ok(());
    }

    if state.is_busy(user_id) {
        bot.send_message(chat_id, "⏳ Tâche en cours. /cancel pour annuler.")
            .await?;
        return Ok(());
    }

    let _ = emit_event(
        &state.mqtt_client,
        "message_received",
        json!({ "chat_id": user_id, "text_length": text.len() }),
    )
    .await;

    let cancel = CancellationToken::new();
    state.active_tasks.insert(user_id, cancel.clone());

    let state_clone = state.clone();
    let text_clone = text.clone();
    tokio::spawn(async move {
        let result = run_claude(&state_clone, chat_id, &text_clone, cancel).await;
        state_clone.active_tasks.remove(&user_id);
        state_clone.add_history(user_id, &text_clone, result.is_ok(), &state_clone.get_session(user_id).model);

        if let Err(e) = result {
            eprintln!("[telegram] Claude error for {}: {}", user_id, e);
            let _ = state_clone
                .bot
                .send_message(chat_id, format!("❌ Erreur: {}", e))
                .await;
        }

        let _ = emit_event(
            &state_clone.mqtt_client,
            "claude_response_completed",
            json!({ "chat_id": user_id, "prompt_length": text_clone.len() }),
        )
        .await;
    });

    Ok(())
}

// ── Library ──

async fn lib_search_with_buttons(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    path: &str,
) -> Result<(), teloxide::RequestError> {
    let url = format!("https://localhost:8443/v1/plugin-api/library{}", path);
    let client = make_client();
    let resp = client
        .get(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            bot.send_message(chat_id, format!("❌ {}", e)).await?;
            return Ok(());
        }
    };

    let body = resp.text().await.unwrap_or_default();
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            bot.send_message(chat_id, "❌ Réponse invalide").await?;
            return Ok(());
        }
    };

    // Extract nodes array (may be wrapped in { "nodes": [...] })
    let nodes = json
        .get("nodes")
        .and_then(|v| v.as_array())
        .or_else(|| json.as_array())
        .cloned()
        .unwrap_or_default();

    if nodes.is_empty() {
        bot.send_message(chat_id, "📭 Aucun résultat.").await?;
        return Ok(());
    }

    let mut text = String::from("📚 Résultats:\n\n");
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    for (i, node) in nodes.iter().enumerate().take(10) {
        let title = node.get("title").and_then(|v| v.as_str()).unwrap_or("?");
        let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let icon = node
            .get("fields")
            .and_then(|f| f.get("icon"))
            .and_then(|v| v.as_str())
            .unwrap_or("📄");
        let subtitle = node
            .get("fields")
            .and_then(|f| f.get("subtitle"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        text.push_str(&format!("{}. {} {}", i + 1, icon, title));
        if !subtitle.is_empty() {
            text.push_str(&format!("\n   {}", truncate(subtitle, 50)));
        }
        text.push('\n');

        // Callback data max 64 bytes — use short prefix + first 50 chars of ID
        let cb_id = if id.len() > 50 { &id[..50] } else { id };
        buttons.push(vec![InlineKeyboardButton::callback(
            format!("{} {}", icon, title),
            format!("lib:{}", cb_id),
        )]);
    }

    let kb = InlineKeyboardMarkup::new(buttons);
    bot.send_message(chat_id, text).reply_markup(kb).await?;
    Ok(())
}

async fn lib_show_node(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    _msg_id: Option<MessageId>,
    node_id: &str,
) {
    let url = format!(
        "https://localhost:8443/v1/plugin-api/library/nodes/{}",
        node_id
    );
    let client = make_client();
    let resp = match client
        .get(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = bot.send_message(chat_id, format!("❌ {}", e)).await;
            return;
        }
    };

    let body = resp.text().await.unwrap_or_default();
    let node: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            let _ = bot.send_message(chat_id, "❌ Fiche introuvable").await;
            return;
        }
    };

    // Format node detail
    let title = node.get("title").and_then(|v| v.as_str()).unwrap_or("?");
    let icon = node
        .get("fields")
        .and_then(|f| f.get("icon"))
        .and_then(|v| v.as_str())
        .unwrap_or("📄");
    let subtitle = node
        .get("fields")
        .and_then(|f| f.get("subtitle"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut text = format!("{} {}\n", icon, title);
    if !subtitle.is_empty() {
        text.push_str(&format!("{}\n", subtitle));
    }
    text.push_str("─────────────\n");

    // Show fields
    if let Some(fields) = node.get("fields").and_then(|v| v.as_object()) {
        for (key, val) in fields {
            // Skip meta fields
            if matches!(
                key.as_str(),
                "icon" | "subtitle" | "fiche_num" | "footer"
            ) {
                continue;
            }
            match val {
                serde_json::Value::String(s) if !s.is_empty() => {
                    text.push_str(&format!("\n{}: {}", key, truncate(s, 200)));
                }
                serde_json::Value::Number(n) => {
                    text.push_str(&format!("\n{}: {}", key, n));
                }
                serde_json::Value::Array(arr) => {
                    let items: Vec<String> = arr
                        .iter()
                        .filter_map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .or_else(|| v.get("nom").and_then(|n| n.as_str()).map(|s| s.to_string()))
                        })
                        .collect();
                    if !items.is_empty() {
                        text.push_str(&format!("\n{}: {}", key, items.join(", ")));
                    }
                }
                _ => {}
            }
        }
    }

    // Content
    if let Some(content) = node.get("content").and_then(|v| v.as_str()) {
        if !content.is_empty() {
            text.push_str(&format!("\n\n{}", truncate(content, 1000)));
        }
    }

    // Footer
    if let Some(footer) = node
        .get("fields")
        .and_then(|f| f.get("footer"))
        .and_then(|v| v.as_str())
    {
        text.push_str(&format!("\n\n📌 {}", footer));
    }

    let display = truncate(&text, 4000).to_string();
    let _ = bot.send_message(chat_id, display).await;
}

// ── Agent Control ──

async fn agent_control_cmd(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    target: &str,
    action: &str,
    label: &str,
) -> Result<(), teloxide::RequestError> {
    let target = target.trim();
    if target.is_empty() {
        // List agents with action buttons
        let url = "https://localhost:8443/agents";
        let client = make_client();
        match client
            .get(url)
            .header("x-api-key", &state.config.kernel_api_key)
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(body) = resp.text().await {
                    if let Ok(agents) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                        let online: Vec<&serde_json::Value> = agents
                            .iter()
                            .filter(|a| {
                                a.get("status").and_then(|v| v.as_str()) == Some("online")
                            })
                            .collect();

                        if online.is_empty() {
                            bot.send_message(chat_id, "📭 Aucun agent en ligne.")
                                .await?;
                        } else {
                            let buttons: Vec<InlineKeyboardButton> = online
                                .iter()
                                .filter_map(|a| {
                                    let hostname = a.get("hostname").and_then(|v| v.as_str())?;
                                    let id = a.get("agent_id").and_then(|v| v.as_str())?;
                                    Some(InlineKeyboardButton::callback(
                                        hostname.to_string(),
                                        format!("confirm:agent:{}:{}", action, id),
                                    ))
                                })
                                .collect();
                            let kb = InlineKeyboardMarkup::new(vec![
                                buttons,
                                vec![InlineKeyboardButton::callback(
                                    "❌ Annuler".to_string(),
                                    "confirm:cancel".to_string(),
                                )],
                            ]);
                            bot.send_message(
                                chat_id,
                                format!("⚠️ {} quel agent ?", label),
                            )
                            .reply_markup(kb)
                            .await?;
                        }
                    }
                }
            }
            Err(e) => {
                bot.send_message(chat_id, format!("❌ {}", e)).await?;
            }
        }
    } else {
        // Direct action with confirmation
        let agent_id = resolve_agent_id(state, target).await;
        match agent_id {
            Some(id) => {
                let kb = InlineKeyboardMarkup::new(vec![vec![
                    InlineKeyboardButton::callback(
                        format!("✅ {}", label),
                        format!("confirm:agent:{}:{}", action, id),
                    ),
                    InlineKeyboardButton::callback(
                        "❌ Annuler".to_string(),
                        "confirm:cancel".to_string(),
                    ),
                ]]);
                bot.send_message(
                    chat_id,
                    format!("⚠️ {} {} ?", label, target),
                )
                .reply_markup(kb)
                .await?;
            }
            None => {
                bot.send_message(chat_id, format!("❌ Agent '{}' non trouvé.", target))
                    .await?;
            }
        }
    }
    Ok(())
}

async fn resolve_agent_id(state: &AppState, hostname: &str) -> Option<String> {
    let url = "https://localhost:8443/agents";
    let client = make_client();
    let resp = client
        .get(url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await
        .ok()?;
    let body = resp.text().await.ok()?;
    let agents: Vec<serde_json::Value> = serde_json::from_str(&body).ok()?;

    for agent in &agents {
        let h = agent.get("hostname").and_then(|v| v.as_str()).unwrap_or("");
        let id = agent.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
        if h.eq_ignore_ascii_case(hostname) || id == hostname {
            return Some(id.to_string());
        }
    }
    None
}

// ── Wake-on-LAN ──

async fn execute_wake(
    state: &AppState,
    bot: &Bot,
    chat_id: ChatId,
    msg_id: Option<MessageId>,
    target: &str,
) {
    // Use kernel WoL endpoint: POST /wake?host_id=<id>
    let host_id = if target.contains(':') {
        // MAC address — try to resolve to agent_id
        match resolve_agent_id_by_mac(state, target).await {
            Some(id) => id,
            None => target.to_string(),
        }
    } else {
        // hostname or agent_id — resolve to agent_id
        resolve_agent_id(state, target).await.unwrap_or_else(|| target.to_string())
    };

    let result = kernel_post(
        state,
        &format!("/wake?host_id={}", urlencoding(&host_id)),
        &serde_json::json!({}),
    )
    .await;

    let text = format!("💡 WoL → {}\n{}", host_id, result);

    if let Some(mid) = msg_id {
        let _ = bot.edit_message_text(chat_id, mid, &text).await;
    } else {
        let _ = bot.send_message(chat_id, &text).await;
    }
}

async fn resolve_agent_id_by_mac(state: &AppState, mac: &str) -> Option<String> {
    let url = "https://localhost:8443/agents";
    let client = make_client();
    let resp = client
        .get(url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await
        .ok()?;
    let body = resp.text().await.ok()?;
    let agents: Vec<serde_json::Value> = serde_json::from_str(&body).ok()?;

    for agent in &agents {
        let m = agent.get("primary_mac").and_then(|v| v.as_str()).unwrap_or("");
        if m.eq_ignore_ascii_case(mac) {
            return agent.get("agent_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        }
    }
    None
}

// ── Kernel API helpers ──

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
}

async fn kernel_get(state: &AppState, path: &str) -> String {
    let url = format!("https://localhost:8443{}", path);
    let client = make_client();
    match client
        .get(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await
    {
        Ok(resp) => format_api_response(resp).await,
        Err(e) => format!("❌ Erreur kernel: {}", e),
    }
}

async fn kernel_post(state: &AppState, path: &str, body: &serde_json::Value) -> String {
    let url = format!("https://localhost:8443{}", path);
    let client = make_client();
    match client
        .post(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .json(body)
        .send()
        .await
    {
        Ok(resp) => format_api_response(resp).await,
        Err(e) => format!("❌ Erreur: {}", e),
    }
}

async fn plugin_get(state: &AppState, plugin: &str, path: &str) -> String {
    let url = format!("https://localhost:8443/v1/plugin-api/{}{}", plugin, path);
    let client = make_client();
    match client
        .get(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .send()
        .await
    {
        Ok(resp) => format_api_response(resp).await,
        Err(e) => format!("❌ Plugin {} inaccessible: {}", plugin, e),
    }
}

async fn plugin_post(state: &AppState, plugin: &str, path: &str, body: &serde_json::Value) -> String {
    let url = format!("https://localhost:8443/v1/plugin-api/{}{}", plugin, path);
    let client = make_client();
    match client
        .post(&url)
        .header("x-api-key", &state.config.kernel_api_key)
        .json(body)
        .send()
        .await
    {
        Ok(resp) => format_api_response(resp).await,
        Err(e) => format!("❌ Plugin {} inaccessible: {}", plugin, e),
    }
}

async fn format_api_response(resp: reqwest::Response) -> String {
    let status = resp.status();
    match resp.text().await {
        Ok(body) => {
            if !status.is_success() {
                return format!("❌ {} — {}", status.as_u16(), truncate(&body, 300));
            }
            format_json_from_str(&body)
        }
        Err(_) => format!("✅ {}", status.as_u16()),
    }
}

fn format_json_from_str(body: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        format_json_for_telegram(&json)
    } else {
        truncate(body, 3000).to_string()
    }
}

fn format_json_for_telegram(json: &serde_json::Value) -> String {
    // Unwrap common wrapper keys: { "plugins": [...] }, { "domains": [...] }, { "notes": [...] }, { "nodes": [...] }
    if let Some(obj) = json.as_object() {
        for key in &["plugins", "domains", "notes", "nodes", "items", "results"] {
            if let Some(inner) = obj.get(*key) {
                if inner.is_array() {
                    let mut result = format_json_for_telegram(inner);
                    // Append summary if present
                    if let Some(summary) = obj.get("summary") {
                        result.push_str("\n\n📊 ");
                        if let Some(s_obj) = summary.as_object() {
                            let parts: Vec<String> = s_obj
                                .iter()
                                .filter(|(k, _)| *k != "timestamp")
                                .map(|(k, v)| format!("{}: {}", k, v))
                                .collect();
                            result.push_str(&parts.join(" | "));
                        }
                    }
                    return result;
                }
            }
        }
    }

    if let Some(arr) = json.as_array() {
        if arr.is_empty() {
            return "📭 Aucun résultat.".to_string();
        }
        let mut out = String::new();
        for (i, item) in arr.iter().enumerate().take(15) {
            out.push_str(&format_json_item(item, i + 1));
            out.push('\n');
        }
        if arr.len() > 15 {
            out.push_str(&format!("\n... et {} de plus", arr.len() - 15));
        }
        return truncate(&out, 4000).to_string();
    }

    if let Some(obj) = json.as_object() {
        let mut out = String::new();
        for (key, val) in obj {
            if key == "timestamp" || key == "spec_version" {
                continue;
            }
            match val {
                serde_json::Value::String(s) => out.push_str(&format!("{}: {}\n", key, s)),
                serde_json::Value::Number(n) => out.push_str(&format!("{}: {}\n", key, n)),
                serde_json::Value::Bool(b) => {
                    out.push_str(&format!("{}: {}\n", key, if *b { "✅" } else { "❌" }))
                }
                serde_json::Value::Array(a) => {
                    out.push_str(&format!("{}: {} items\n", key, a.len()))
                }
                serde_json::Value::Null => out.push_str(&format!("{}: —\n", key)),
                _ => out.push_str(&format!("{}: ...\n", key)),
            }
        }
        return truncate(&out, 4000).to_string();
    }

    format!("{}", json)
}

fn format_json_item(item: &serde_json::Value, index: usize) -> String {
    if let Some(obj) = item.as_object() {
        let name = obj
            .get("label")
            .or_else(|| obj.get("name"))
            .or_else(|| obj.get("hostname"))
            .or_else(|| obj.get("title"))
            .or_else(|| obj.get("plugin_id"))
            .or_else(|| obj.get("agent_id"))
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        let status_icon = obj
            .get("status_level")
            .or_else(|| obj.get("status"))
            .or_else(|| obj.get("state"))
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "ok" | "healthy" | "online" | "active" | "connected" | "Running" => "🟢",
                "degraded" | "warning" => "🟡",
                "critical" | "unhealthy" | "offline" | "error" | "disconnected" => "🔴",
                _ => "⚪",
            })
            .unwrap_or("");

        let mut details = Vec::new();
        if let Some(os) = obj.get("os").and_then(|v| v.as_str()) {
            details.push(os.to_string());
        }
        if let Some(v) = obj.get("version").and_then(|v| v.as_str()) {
            details.push(format!("v{}", v));
        }
        if let Some(cpu) = obj.get("cpu_percent").and_then(|v| v.as_f64()) {
            details.push(format!("CPU {:.0}%", cpu));
        }
        if let Some(mem) = obj.get("memory_percent").and_then(|v| v.as_f64()) {
            details.push(format!("RAM {:.0}%", mem));
        }
        if let Some(desc) = obj
            .get("description")
            .or_else(|| obj.get("content"))
            .and_then(|v| v.as_str())
        {
            details.push(truncate(desc, 50).to_string());
        }
        if let Some(score) = obj.get("health_score").and_then(|v| v.as_u64()) {
            details.push(format!("santé {}%", score));
        }
        // SSL specific
        if let Some(days) = obj.get("days_remaining").and_then(|v| v.as_i64()) {
            details.push(format!("{}j restants", days));
        }
        if let Some(expiry) = obj.get("expiry_date").and_then(|v| v.as_str()) {
            details.push(format!("exp {}", expiry));
        }
        // Notes specific
        if let Some(data) = obj.get("data") {
            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                details.push(truncate(content, 60).to_string());
            }
        }

        let detail_str = if details.is_empty() {
            String::new()
        } else {
            format!(" — {}", details.join(", "))
        };

        format!("{}. {} {}{}", index, status_icon, name, detail_str)
    } else {
        format!("{}. {}", index, item)
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() > max {
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    } else {
        s
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('#', "%23")
}

#[cfg(test)]
mod pure_tests {
    use super::*;
    use serde_json::json;

    // ---- truncate ----

    #[test]
    fn truncate_no_op_when_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_cuts_long_strings() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn truncate_respects_utf8_boundaries() {
        // "café" = 5 bytes (c=1, a=1, f=1, é=2). Couper à 4 bytes coupe au milieu de é.
        // L'impl doit reculer jusqu'à un char boundary valide.
        let r = truncate("café", 4);
        assert!(r == "caf"); // tombe sur boundary 3
    }

    #[test]
    fn truncate_handles_emoji() {
        // emoji 4 bytes — couper au milieu doit reculer
        let r = truncate("ab🚀cd", 4);
        // 'a','b'=2 bytes, '🚀'=4 bytes. Couper à 4 = milieu de 🚀, recule à 2.
        assert_eq!(r, "ab");
    }

    // ---- urlencoding ----

    #[test]
    fn urlencoding_replaces_special_chars() {
        assert_eq!(urlencoding("a b&c=d#e"), "a%20b%26c%3Dd%23e");
    }

    #[test]
    fn urlencoding_no_op_on_safe_chars() {
        assert_eq!(urlencoding("abc-123_xyz"), "abc-123_xyz");
    }

    // ---- format_json_for_telegram ----

    #[test]
    fn format_empty_array_returns_friendly_message() {
        let r = format_json_for_telegram(&json!([]));
        assert!(r.contains("Aucun"));
    }

    #[test]
    fn format_array_lists_items_numbered() {
        let r = format_json_for_telegram(&json!([
            {"name": "alpha", "status": "online"},
            {"name": "beta", "status": "offline"}
        ]));
        assert!(r.contains("1. "));
        assert!(r.contains("2. "));
        assert!(r.contains("alpha"));
        assert!(r.contains("beta"));
    }

    #[test]
    fn format_array_truncates_at_15_items() {
        let items: Vec<_> = (0..20).map(|i| json!({"name": format!("item{}", i)})).collect();
        let r = format_json_for_telegram(&json!(items));
        // Doit mentionner "et N de plus"
        assert!(r.contains("de plus"));
        assert!(r.contains("5 de plus"));
    }

    #[test]
    fn format_unwraps_plugins_wrapper() {
        let r = format_json_for_telegram(&json!({
            "plugins": [{"name": "ssl", "status": "healthy"}]
        }));
        assert!(r.contains("ssl"));
        assert!(r.contains("1. "));
    }

    #[test]
    fn format_unwraps_summary_in_wrapped_array() {
        let r = format_json_for_telegram(&json!({
            "domains": [{"name": "example.com", "status_level": "ok"}],
            "summary": {"total": 1, "expired": 0, "timestamp": "2026-05-09"}
        }));
        assert!(r.contains("example.com"));
        // summary appended après les items
        assert!(r.contains("📊"));
        assert!(r.contains("total: 1"));
        // timestamp doit être ignoré dans le summary
        assert!(!r.contains("timestamp"));
    }

    #[test]
    fn format_object_skips_internal_keys() {
        let r = format_json_for_telegram(&json!({
            "title": "Hello",
            "timestamp": "2026-05-09",
            "spec_version": "1.0",
            "active": true
        }));
        assert!(r.contains("title: Hello"));
        assert!(r.contains("active: ✅"));
        assert!(!r.contains("timestamp"));
        assert!(!r.contains("spec_version"));
    }

    #[test]
    fn format_object_renders_bool_with_emoji() {
        let r = format_json_for_telegram(&json!({"online": false}));
        assert!(r.contains("online: ❌"));
    }

    // ---- format_json_item ----

    #[test]
    fn format_item_with_name_and_status() {
        let r = format_json_item(&json!({"name": "kernel", "status": "healthy"}), 1);
        assert!(r.starts_with("1. "));
        assert!(r.contains("🟢"));
        assert!(r.contains("kernel"));
    }

    #[test]
    fn format_item_uses_fallback_label_keys() {
        // Tente label > name > hostname > title > plugin_id > agent_id > id
        let r = format_json_item(&json!({"agent_id": "agt-42", "state": "Running"}), 3);
        assert!(r.contains("agt-42"));
        assert!(r.contains("🟢")); // "Running" mappe vert
    }

    #[test]
    fn format_item_unknown_status_uses_white_circle() {
        let r = format_json_item(&json!({"name": "thing", "status": "weird-state"}), 1);
        assert!(r.contains("⚪"));
    }

    #[test]
    fn format_item_includes_cpu_ram_when_present() {
        let r = format_json_item(
            &json!({"name": "host", "status": "online", "cpu_percent": 45.7, "memory_percent": 72.0}),
            1,
        );
        assert!(r.contains("CPU 46%"));
        assert!(r.contains("RAM 72%"));
    }

    #[test]
    fn format_item_includes_ssl_days_remaining() {
        let r = format_json_item(
            &json!({"hostname": "example.com", "status_level": "ok", "days_remaining": 30}),
            1,
        );
        assert!(r.contains("example.com"));
        assert!(r.contains("30j restants"));
    }

    #[test]
    fn format_item_falls_back_to_question_mark_when_no_label() {
        let r = format_json_item(&json!({"random": "data"}), 5);
        assert!(r.contains("5. "));
        assert!(r.contains("?"));
    }

    // ---- format_json_from_str ----

    #[test]
    fn format_str_handles_invalid_json() {
        let r = format_json_from_str("not a json {{ broken");
        assert!(r.contains("not a json"));
    }

    #[test]
    fn format_str_parses_valid_json() {
        let r = format_json_from_str(r#"[{"name":"x","status":"online"}]"#);
        assert!(r.contains("x"));
        assert!(r.contains("🟢"));
    }
}
