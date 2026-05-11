mod actions;
mod claude;
mod config;
mod events;
mod prefs;
mod state;
mod telegram;

use axum::routing::{get, post};
use axum::Router;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::time::Duration;
use symbion_plugin_common::PluginHttpServer;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::utils::html as tg_html;
use tokio::signal::unix::{signal, SignalKind};

use crate::actions::{handle_action, health_handler};
use crate::config::Config;
use crate::events::{publish_health, publish_manifest};
use crate::state::AppState;
use crate::telegram::{build_dispatcher, BotCommand};
use teloxide::utils::command::BotCommands;

const PLUGIN_ID: &str = "telegram";
const HEALTH_INTERVAL: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() {
    println!("[telegram] Starting symbion-plugin-telegram...");

    // 1. Load config
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[telegram] Config error: {}", e);
            std::process::exit(1);
        }
    };

    let socket_path = config.socket_path.clone();

    // 2. Setup MQTT
    let mut mqtt_opts = MqttOptions::new(
        PLUGIN_ID,
        &config.mqtt_broker_host,
        config.mqtt_broker_port,
    );
    mqtt_opts.set_keep_alive(Duration::from_secs(30));

    let (mqtt_client, mut mqtt_eventloop) = AsyncClient::new(mqtt_opts, 10);

    // 3. Setup Telegram bot
    let bot = Bot::new(&config.telegram_bot_token);

    // 4. Load notification prefs (catégories on/off)
    let prefs_path = std::env::var("SYMBION_TELEGRAM_PREFS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| prefs::default_path(&config.claude_workdir));
    let prefs = prefs::load(&prefs_path);
    println!("[telegram] Notif prefs loaded from {:?}", prefs_path);

    // 5. Build shared state
    let state = AppState::new(config, mqtt_client.clone(), bot.clone(), prefs, prefs_path);

    // 6. Build Axum router for Unix socket (Contract v1.0)
    let router = Router::new()
        .route("/health", get(health_handler))
        .route("/actions", post(handle_action))
        .route("/config", get(prefs::get_config_handler).put(prefs::put_config_handler))
        .route("/broadcast-summary", post(broadcast_summary_handler))
        .with_state(state.clone());

    // 6. Create HTTP server on Unix socket
    let http_server = PluginHttpServer::new(&socket_path, router);

    // 7. Subscribe to notification topics
    let sub_client = mqtt_client.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let topics = [
            "symbion/notifications/sent@v1",
            "symbion/plugins/+/status",
        ];
        for topic in &topics {
            if let Err(e) = sub_client.subscribe(*topic, QoS::AtLeastOnce).await {
                eprintln!("[telegram] Failed to subscribe to {}: {}", topic, e);
            }
        }
        println!("[telegram] Subscribed to notification topics");
    });

    // 8. Spawn MQTT event loop with notification forwarding
    let notif_state = state.clone();
    tokio::spawn(async move {
        loop {
            match mqtt_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(msg))) => {
                    handle_mqtt_message(&notif_state, &msg.topic, &msg.payload).await;
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[telegram] MQTT error: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    // 9. Publish manifest (retained)
    let socket_str = socket_path.to_string_lossy().to_string();
    publish_manifest(&mqtt_client, &socket_str).await;

    // 9. Spawn health heartbeat
    let health_client = mqtt_client.clone();
    let health_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(HEALTH_INTERVAL).await;
            let uptime = health_state.start_time.elapsed().as_secs();
            publish_health(&health_client, uptime, "healthy").await;
        }
    });

    // 10. Register with kernel + actions templates (Contract v1.0 wrap pour /actions)
    let socket_str = socket_path.to_string_lossy().to_string();
    tokio::spawn(async move {
        use symbion_plugin_common::{PluginAction, PluginActionParam};

        // Small delay to let kernel be ready
        tokio::time::sleep(Duration::from_secs(3)).await;
        if let Err(e) = symbion_plugin_common::PluginRegistrationBuilder::new(PLUGIN_ID, &socket_str)
            .route("/health")
            .route("/actions")
            .route("/config")
            .route("/broadcast-summary")
            .version("1.0.0")
            .description("Telegram-Claude Code bridge with Symbion integration")
            .action(PluginAction {
                name: "send_notification".into(),
                label: "Envoyer notification (broadcast)".into(),
                description: Some("Notification à tous les utilisateurs autorisés (ALLOWED_USER_IDS)".into()),
                icon: Some("📢".into()),
                route: "actions".into(),
                method: "POST".into(),
                impact_level: "Low".into(),
                wrap_protocol: Some("v1".into()),  // Contract v1.0 wrap pour /actions
                params: vec![
                    PluginActionParam {
                        name: "text".into(),
                        label: "Message".into(),
                        param_type: "text_area".into(),
                        required: true,
                        default: None,
                        options: vec![],
                        min: None, max: None,
                        placeholder: Some("Texte du message à envoyer".into()),
                    },
                    PluginActionParam {
                        name: "level".into(),
                        label: "Niveau".into(),
                        param_type: "select".into(),
                        required: false,
                        default: Some(serde_json::json!("info")),
                        options: vec![
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("info"), label: "ℹ️ Info".into() },
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("success"), label: "✅ Succès".into() },
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("warning"), label: "⚠️ Avertissement".into() },
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("error"), label: "🚨 Erreur".into() },
                        ],
                        min: None, max: None, placeholder: None,
                    },
                ],
            })
            .action(PluginAction {
                name: "broadcast_summary".into(),
                label: "Envoyer résumé du jour".into(),
                description: Some("Construit et envoie un résumé synthétique (mode, agents, automations, café) à tous les utilisateurs autorisés. Typiquement déclenché via automation scheduled à 8h chaque matin.".into()),
                icon: Some("📊".into()),
                route: "broadcast-summary".into(),
                method: "POST".into(),
                impact_level: "Low".into(),
                wrap_protocol: None,  // route directe, pas Contract v1.0
                params: vec![],  // aucun param, le contenu est généré côté plugin
            })
            .action(PluginAction {
                name: "send_message".into(),
                label: "Envoyer message à un user".into(),
                description: Some("Message direct à un chat_id Telegram spécifique".into()),
                icon: Some("💬".into()),
                route: "actions".into(),
                method: "POST".into(),
                impact_level: "Low".into(),
                wrap_protocol: Some("v1".into()),
                params: vec![
                    PluginActionParam {
                        name: "chat_id".into(),
                        label: "Chat ID Telegram".into(),
                        param_type: "int".into(),
                        required: true,
                        default: None,
                        options: vec![],
                        min: None, max: None,
                        placeholder: Some("ID du chat (entier)".into()),
                    },
                    PluginActionParam {
                        name: "text".into(),
                        label: "Texte".into(),
                        param_type: "text_area".into(),
                        required: true,
                        default: None,
                        options: vec![],
                        min: None, max: None,
                        placeholder: Some("Contenu du message".into()),
                    },
                    PluginActionParam {
                        name: "parse_mode".into(),
                        label: "Format".into(),
                        param_type: "select".into(),
                        required: false,
                        default: None,
                        options: vec![
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!(""), label: "Plain text".into() },
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("HTML"), label: "HTML".into() },
                            symbion_plugin_common::PluginActionOption { value: serde_json::json!("Markdown"), label: "Markdown".into() },
                        ],
                        min: None, max: None, placeholder: None,
                    },
                ],
            })
            .register()
            .await
        {
            eprintln!("[telegram] Failed to register with kernel: {}", e);
        } else {
            println!("[telegram] Registered with kernel");
        }
    });

    // 11. Spawn HTTP server
    tokio::spawn(async move {
        if let Err(e) = http_server.serve().await {
            eprintln!("[telegram] HTTP server error: {}", e);
        }
    });

    // 12. Register bot commands with Telegram menu
    if let Err(e) = bot.set_my_commands(BotCommand::bot_commands()).await {
        eprintln!("[telegram] Failed to set bot commands: {}", e);
    } else {
        println!("[telegram] Bot commands registered with Telegram");
    }

    println!("[telegram] Plugin ready. Starting Telegram bot...");

    // 13. Build and run Telegram dispatcher (with graceful shutdown)
    let mut dispatcher = build_dispatcher(bot, state.clone());

    let shutdown_token = dispatcher.shutdown_token();

    // SIGTERM handler
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
        sigterm.recv().await;
        println!("[telegram] SIGTERM received, shutting down...");
        shutdown_token.shutdown().expect("shutdown dispatcher").await;
    });

    dispatcher.dispatch().await;

    // Cleanup
    let _ = publish_health(&mqtt_client, state.start_time.elapsed().as_secs(), "stopping").await;
    println!("[telegram] Stopped.");
}

/// Forward MQTT notifications to Telegram with interactive action buttons
async fn handle_mqtt_message(state: &AppState, topic: &str, payload: &[u8]) {
    // Parse payload
    let json: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(_) => return,
    };

    match topic {
        "symbion/notifications/sent@v1" => {
            println!("[telegram] Received notification via MQTT");
            handle_notification(state, &json).await;
        }
        t if t.starts_with("symbion/plugins/") && t.ends_with("/status") => {
            handle_plugin_status(state, t, &json).await;
        }
        _ => {}
    }
}

/// Handle kernel notification with optional inline action buttons
async fn handle_notification(state: &AppState, json: &serde_json::Value) {
    let notif = json.get("notification").unwrap_or(json);
    let notif_id = notif.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = notif.get("title").and_then(|v| v.as_str()).unwrap_or("Notification");
    let body = notif.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let priority = notif.get("priority").and_then(|v| v.as_str()).unwrap_or("P2");
    let source = notif.get("source").and_then(|v| v.as_str()).unwrap_or("symbion");

    println!("[telegram] Notification: id={}, title={}, actions={}",
        notif_id, title,
        notif.get("actions").map(|a| a.to_string()).unwrap_or_else(|| "none".into()));

    // Filtre par catégorie (toggles utilisateur). P0 = jamais filtré (urgence).
    if priority != "P0" {
        let category = prefs::categorize(source);
        let prefs = state.prefs.read().await;
        if !prefs.is_enabled(category) {
            println!(
                "[telegram] Skip notif id={} category={} (disabled by user prefs)",
                notif_id, category
            );
            return;
        }
    }

    let icon = match priority {
        "P0" => "🚨",
        "P1" => "⚠️",
        _ => "ℹ️",
    };

    // HTML formatting (parse_mode HTML côté Telegram). Échappement obligatoire
    // pour éviter qu'un body contenant `<` `>` `&` ne casse le rendu.
    let title_html = tg_html::escape(title);
    let body_html = tg_html::escape(body);
    let source_html = tg_html::escape(source);

    let text = format!(
        "{icon} <b>{title_html}</b>\n{body_html}\n\n<i>📌 {source_html} · {priority}</i>"
    );

    // Extract actions for inline keyboard
    let actions = notif.get("actions").and_then(|v| v.as_array());
    let keyboard = if let Some(actions) = actions {
        if actions.is_empty() || notif_id.is_empty() {
            None
        } else {
            // Cache notification data for callback resolution
            state.cache_notification(notif_id, notif.clone());

            let buttons: Vec<InlineKeyboardButton> = actions
                .iter()
                .filter_map(|a| {
                    let action_id = a.get("id").and_then(|v| v.as_str())?;
                    let label = a.get("label").and_then(|v| v.as_str())?;
                    let action_type = a.get("action_type");

                    // Choose icon based on action type
                    let btn_icon = match action_type {
                        Some(serde_json::Value::String(s)) if s == "Reject" => "❌",
                        Some(serde_json::Value::String(s)) if s == "Approve" => "✅",
                        Some(serde_json::Value::String(s)) if s == "Snooze" => "⏰",
                        Some(serde_json::Value::Object(m)) if m.contains_key("Custom") => "✅",
                        _ => "▶️",
                    };

                    // Callback data: notif:{notif_id}:{action_id}
                    // Telegram limits callback data to 64 bytes, so truncate if needed
                    let callback = format!("notif:{}:{}", notif_id, action_id);
                    if callback.len() > 64 {
                        // Use short hash if too long
                        let short_id = &notif_id[..notif_id.len().min(20)];
                        let callback = format!("notif:{}:{}", short_id, action_id);
                        Some(InlineKeyboardButton::callback(
                            format!("{} {}", btn_icon, label),
                            callback,
                        ))
                    } else {
                        Some(InlineKeyboardButton::callback(
                            format!("{} {}", btn_icon, label),
                            callback,
                        ))
                    }
                })
                .collect();

            if buttons.is_empty() {
                None
            } else {
                Some(InlineKeyboardMarkup::new(vec![buttons]))
            }
        }
    } else {
        None
    };

    // Send to all allowed users (parse_mode HTML, P2 silencieux)
    let silent = priority == "P2";
    for &user_id in &state.config.allowed_user_ids {
        let mut msg = state.bot.send_message(ChatId(user_id), &text)
            .parse_mode(ParseMode::Html);
        if silent {
            msg = msg.disable_notification(true);
        }
        if let Some(ref kb) = keyboard {
            let _ = msg.reply_markup(kb.clone()).await;
        } else {
            let _ = msg.await;
        }
    }
}

/// POST /broadcast-summary — Génère le résumé du jour et l'envoie à tous les
/// ALLOWED_USER_IDS. Appelable via automation scheduled (typiquement 1×/jour à 8h).
async fn broadcast_summary_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let text = crate::telegram::build_daily_summary(&state).await;
    let mut sent = 0;
    let mut errors = Vec::new();
    for &user_id in &state.config.allowed_user_ids {
        match state.bot
            .send_message(ChatId(user_id), &text)
            .parse_mode(ParseMode::Html)
            .await
        {
            Ok(_) => sent += 1,
            Err(e) => errors.push(format!("user {}: {}", user_id, e)),
        }
    }
    axum::Json(serde_json::json!({
        "sent_to": sent,
        "errors": errors,
    }))
}

/// Handle plugin status changes
async fn handle_plugin_status(state: &AppState, topic: &str, json: &serde_json::Value) {
    let plugin_id = topic
        .strip_prefix("symbion/plugins/")
        .and_then(|s| s.strip_suffix("/status"))
        .unwrap_or("?");

    if plugin_id == "telegram" {
        return;
    }

    let status = json.get("status").and_then(|v| v.as_str()).unwrap_or("");
    let text = match status {
        "unhealthy" | "degraded" => {
            let icon = if status == "unhealthy" { "🔴" } else { "🟡" };
            format!("{} Plugin {} → {}", icon, plugin_id, status)
        }
        _ => return,
    };

    for &user_id in &state.config.allowed_user_ids {
        let _ = state.bot.send_message(ChatId(user_id), &text).await;
    }
}
